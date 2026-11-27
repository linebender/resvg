// Copyright 2026 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use skrifa::font::Variation;
use skrifa::font::glyph::bitmap::{BitmapSource, Pixels};
use skrifa::font::glyph::outline::{DrawOptions, PathSink};
use skrifa::raw::TableProvider as _;
use skrifa::raw::types::{Matrix, NameId};
use skrifa::{DrawMemory, Tag};
use svgtypes::Color;
use tiny_skia_path::{NonZeroRect, Size, Transform};
use xmlwriter::XmlWriter;

use super::colr::GlyphPainter;
use super::layout::ResolvedFont;
use crate::parser::OptionLog;
use crate::{
    Font, FontData, FontKey, FontOpticalSizing, FontResolver, FontVariation, GlyphId, Image,
    ImageKind, ImageRendering, Node, Options, Tree,
};

#[derive(Default)]
pub(crate) struct FontCache {
    pub(crate) resolved: HashMap<Font, Arc<ResolvedFont>>,
    // `None` marks faces that failed to load, so they are not requested again.
    loaded: HashMap<FontKey, Option<LoadedFont>>,
    used: HashSet<FontKey>,
}

pub(crate) struct LoadedFont {
    data: FontData,
    font: skrifa::Font,
    instances: HashMap<Vec<FontVariation>, FontInstance>,
    instance_key: Vec<FontVariation>,
    svg: HashMap<GlyphId, Option<Node>>,
    raster: HashMap<GlyphId, Option<BitmapImage>>,
}

pub(crate) struct FontInstance {
    pub(crate) font: skrifa::Font,
    pub(crate) plans: Vec<harfrust::ShapePlan>,
    outlines: HashMap<GlyphId, Option<tiny_skia_path::Path>>,
    colr: HashMap<GlyphId, Option<Tree>>,
}

#[derive(Clone)]
pub(crate) struct BitmapImage {
    pub(crate) image: Image,
    pub(crate) x: i16,
    pub(crate) y: i16,
    pub(crate) pixels_per_em: u16,
    pub(crate) bearing_x: f32,
    pub(crate) bearing_y: f32,
    pub(crate) is_sbix: bool,
}

impl FontCache {
    pub(crate) fn load(
        &mut self,
        key: FontKey,
        resolver: &dyn FontResolver,
    ) -> Option<&mut LoadedFont> {
        self.loaded
            .entry(key)
            .or_insert_with(|| LoadedFont::new(resolver.load(key)?))
            .as_mut()
    }

    /// Returns a font that was already loaded with [`FontCache::load`].
    pub(crate) fn get(&mut self, key: FontKey) -> Option<&mut LoadedFont> {
        self.loaded.get_mut(&key)?.as_mut()
    }

    pub(crate) fn mark_used(&mut self, key: FontKey) {
        self.used.insert(key);
    }

    pub(crate) fn into_used_fonts(mut self) -> HashMap<FontKey, FontData> {
        self.used
            .iter()
            .filter_map(|key| Some((*key, self.loaded.remove(key)??.data)))
            .collect()
    }

    pub(crate) fn outline(
        &mut self,
        key: FontKey,
        glyph: GlyphId,
        variations: impl IntoIterator<Item = FontVariation>,
    ) -> Option<tiny_skia_path::Path> {
        let instance = self.get(key)?.instance(variations);
        instance
            .outlines
            .entry(glyph)
            .or_insert_with(|| outline(&instance.font, glyph))
            .clone()
    }

    pub(crate) fn colr(
        &mut self,
        key: FontKey,
        glyph: GlyphId,
        variations: impl IntoIterator<Item = FontVariation>,
    ) -> Option<Tree> {
        let loaded = self.get(key)?;
        if loaded.font.color_glyphs().is_empty() {
            return None;
        }

        let instance = loaded.instance(variations);
        instance
            .colr
            .entry(glyph)
            .or_insert_with(|| colr(&instance.font, glyph))
            .clone()
    }

    pub(crate) fn svg(&mut self, key: FontKey, glyph: GlyphId) -> Option<Node> {
        let loaded = self.get(key)?;
        if loaded.font.tables().svg().is_err() {
            return None;
        }

        let font = &loaded.font;
        loaded
            .svg
            .entry(glyph)
            .or_insert_with(|| svg(font, glyph))
            .clone()
    }

    pub(crate) fn raster(&mut self, key: FontKey, glyph: GlyphId) -> Option<BitmapImage> {
        let loaded = self.get(key)?;
        if loaded.font.bitmap_strikes().is_empty() {
            return None;
        }

        let font = &loaded.font;
        loaded
            .raster
            .entry(glyph)
            .or_insert_with(|| raster(font, glyph))
            .clone()
    }
}

impl LoadedFont {
    fn new(data: FontData) -> Option<Self> {
        let font = skrifa::Font::new(data.data.clone(), data.index)?;

        Some(Self {
            data,
            font,
            instances: HashMap::new(),
            instance_key: Vec::new(),
            svg: HashMap::new(),
            raster: HashMap::new(),
        })
    }

    pub(crate) fn instance(
        &mut self,
        variations: impl IntoIterator<Item = FontVariation>,
    ) -> &mut FontInstance {
        let key = &mut self.instance_key;
        key.clear();
        key.extend(
            self.font
                .axes()
                .filter(variations.into_iter().map(to_skrifa_variation))
                .map(|v| FontVariation::new(v.tag.to_be_bytes(), v.value)),
        );

        if !self.instances.contains_key(key.as_slice()) {
            let font = if key.is_empty() {
                self.font.clone()
            } else {
                self.font
                    .instance_builder()
                    .variations(key.iter().copied().map(to_skrifa_variation))
                    .build()
            };

            let instance = FontInstance {
                font,
                plans: Vec::new(),
                outlines: HashMap::new(),
                colr: HashMap::new(),
            };
            self.instances.insert(key.clone(), instance);
        }

        self.instances.get_mut(key.as_slice()).unwrap()
    }
}

fn to_skrifa_variation(v: FontVariation) -> Variation {
    Variation::new(Tag::from_be_bytes(v.tag), v.value)
}

/// Adds the `opsz` value that `font-optical-sizing: auto` implies, like browsers do.
///
/// It comes first, so that an `opsz` in `variations` overrides it.
pub(crate) fn with_optical_sizing(
    variations: &[FontVariation],
    optical_sizing: FontOpticalSizing,
    font_size: f32,
) -> impl Iterator<Item = FontVariation> + Clone + '_ {
    let auto_opsz = (optical_sizing == FontOpticalSizing::Auto)
        .then(|| FontVariation::new(*b"opsz", font_size));
    auto_opsz.into_iter().chain(variations.iter().copied())
}

#[derive(Default)]
struct PathBuilder {
    builder: tiny_skia_path::PathBuilder,
}

impl PathSink for PathBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.builder.move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.builder.line_to(x, y);
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.builder.quad_to(cx0, cy0, x, y);
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.builder.cubic_to(cx0, cy0, cx1, cy1, x, y);
    }

    fn close(&mut self) {
        self.builder.close();
    }
}

#[inline(never)]
fn outline(font: &skrifa::Font, glyph_id: GlyphId) -> Option<tiny_skia_path::Path> {
    let outline = font.outline_glyphs().get(glyph_id.into())?;

    let mut builder = PathBuilder::default();
    outline
        .draw(&DrawOptions::default(), DrawMemory::Scratch, &mut builder)
        .ok()?;

    builder.builder.finish()
}

fn raster(font: &skrifa::Font, glyph_id: GlyphId) -> Option<BitmapImage> {
    // No size selects the largest image available.
    let image = font.bitmap_strikes().select_bitmap(glyph_id.into(), None)?;

    // Follows vello:
    // https://github.com/linebender/vello/blob/717a485dabd16249fae739a755a3da3ea4a8f69a/glifo/src/glyph.rs#L1322-L1367
    let (bearing_x, bearing_y) = if is_apple_color_emoji(font) {
        (0.0, -(font.metrics().units_per_em as f32) / 8.0)
    } else {
        (image.x_bearing() as f32, image.y_bearing() as f32)
    };

    match image.pixels() {
        Pixels::Png(data) => Some(BitmapImage {
            image: Image {
                id: String::new(),
                visible: true,
                size: Size::from_wh(image.width() as f32, image.height() as f32)?,
                rendering_mode: ImageRendering::OptimizeQuality,
                kind: ImageKind::PNG(Arc::new(data.to_vec())),
                abs_transform: Transform::default(),
                abs_bounding_box: NonZeroRect::from_xywh(
                    0.0,
                    0.0,
                    image.width() as f32,
                    image.height() as f32,
                )?,
            },
            x: image.inner_x_bearing() as i16,
            y: image.inner_y_bearing() as i16,
            pixels_per_em: image.ppem_x(),
            bearing_x,
            bearing_y,
            is_sbix: image.source() == BitmapSource::Sbix,
        }),
        // TODO: implement other bitmap formats
        Pixels::Bgra(_) | Pixels::Mask(_) => None,
    }
}

fn is_apple_color_emoji(font: &skrifa::Font) -> bool {
    font.names()
        .get(NameId::POSTSCRIPT_NAME)
        .is_some_and(|name| {
            name.iter()
                .any(|name| name.name().chars().eq("AppleColorEmoji".chars()))
        })
}

fn svg(font: &skrifa::Font, glyph_id: GlyphId) -> Option<Node> {
    // SEE: https://docs.rs/read-fonts/latest/read_fonts/tables/svg/type.Svg.html

    // TODO: Technically not 100% accurate because the SVG format in a OTF font
    // is actually a subset/superset of a normal SVG, but it seems to work fine
    // for Twitter Color Emoji, so might as well use what we already have.

    // TODO: Glyph records can contain the data for multiple glyphs. We should
    // add a cache so we don't need to reparse the data every time.
    let svg_table = font.tables().svg().ok()?;
    let image_data = svg_table.glyph_data(glyph_id.into())?;
    let tree = Tree::from_data(image_data, &Options::default()).ok()?;

    // Twitter Color Emoji seems to always have one SVG record per glyph,
    // while Noto Color Emoji sometimes contains multiple ones. It's kind of hacky,
    // but the best we have for now.
    let document_list = svg_table.svg_document_list().ok()?;
    let doc_record = document_list.document_records().iter().find(|r| {
        (r.start_glyph_id.get().to_u32()..=r.end_glyph_id.get().to_u32()).contains(&glyph_id.0)
    })?;
    let node = if doc_record.start_glyph_id == doc_record.end_glyph_id {
        Node::Group(Box::new(tree.root))
    } else {
        tree.node_by_id(&format!("glyph{}", glyph_id.0))
            .log_none(|| {
                log::warn!("Failed to find SVG glyph node for glyph {}", glyph_id.0);
            })
            .cloned()?
    };

    Some(node)
}

fn colr(font: &skrifa::Font, glyph_id: GlyphId) -> Option<Tree> {
    let color_glyph = font.color_glyphs().get(glyph_id.into())?;

    let mut svg = XmlWriter::new(xmlwriter::Options::default());

    svg.start_element("svg");
    svg.write_attribute("xmlns", "http://www.w3.org/2000/svg");
    svg.write_attribute("xmlns:xlink", "http://www.w3.org/1999/xlink");

    let mut path_buf = String::with_capacity(256);

    svg.start_element("g");

    let mut glyph_painter = GlyphPainter {
        font,
        svg: &mut svg,
        path_buf: &mut path_buf,
        gradient_index: 1,
        clip_path_index: 1,
        foreground_color: Color::new_rgba(0, 0, 0, 255),
        transform: Matrix::default(),
        outline_transform: Matrix::default(),
        transforms_stack: vec![Matrix::default()],
        clip_stack: Vec::new(),
    };

    color_glyph.paint(&mut glyph_painter).ok()?;
    svg.end_element();

    Tree::from_data(svg.end_document().as_bytes(), &Options::default()).ok()
}
