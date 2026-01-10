// Copyright 2022 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::mem;
use std::sync::Arc;

use fontdb::{Database, ID};
use harfrust::Tag;
use skrifa::{
    bitmap::BitmapData,
    instance::{LocationRef, Size as SkrifaSize},
    outline::{pen::ControlBoundsPen, DrawSettings, HintingInstance, OutlinePen, Target},
    raw::TableProvider,
    setting::VariationSetting,
    FontRef, GlyphId, MetadataProvider,
};
use tiny_skia_path::{NonZeroRect, Size, Transform};

use crate::*;

/// Points per inch - standard typographic conversion factor for ppem calculation.
const POINTS_PER_INCH: f32 = 72.0;

fn resolve_rendering_mode(text: &Text) -> ShapeRendering {
    match text.rendering_mode {
        TextRendering::OptimizeSpeed => ShapeRendering::CrispEdges,
        TextRendering::OptimizeLegibility => ShapeRendering::GeometricPrecision,
        TextRendering::GeometricPrecision => ShapeRendering::GeometricPrecision,
    }
}

fn push_outline_paths(
    span: &layout::Span,
    builder: &mut tiny_skia_path::PathBuilder,
    new_children: &mut Vec<Node>,
    rendering_mode: ShapeRendering,
) {
    let builder = mem::replace(builder, tiny_skia_path::PathBuilder::new());

    if let Some(path) = builder.finish().and_then(|p| {
        Path::new(
            String::new(),
            span.visible,
            span.fill.clone(),
            span.stroke.clone(),
            span.paint_order,
            rendering_mode,
            Arc::new(p),
            Transform::default(),
        )
    }) {
        new_children.push(Node::Path(Box::new(path)));
    }
}

/// Hinting context for controlling font hinting behavior.
#[derive(Clone, Copy, Debug)]
pub struct HintingContext {
    /// Whether hinting is enabled globally.
    pub enabled: bool,
    /// DPI for ppem calculation.
    pub dpi: f32,
}

impl HintingContext {
    /// Calculate pixels per em from font size.
    pub fn ppem(&self, font_size: f32) -> f32 {
        // ppem = font_size * dpi / 72 (converting points to pixels)
        font_size * self.dpi / POINTS_PER_INCH
    }
}

/// Convert positioned glyphs to path outlines.
pub(crate) fn flatten(
    text: &mut Text,
    cache: &mut Cache,
    hinting_ctx: Option<HintingContext>,
) -> Option<(Group, NonZeroRect)> {
    flatten_impl(text, cache, hinting_ctx)
}

fn flatten_impl(
    text: &mut Text,
    cache: &mut Cache,
    hinting_ctx: Option<HintingContext>,
) -> Option<(Group, NonZeroRect)> {
    let mut new_children = vec![];

    let rendering_mode = resolve_rendering_mode(text);
    let hinting_mode = HintingMode::from_text_rendering(text.rendering_mode);

    // Determine if we should use hinting
    let use_hinting = hinting_ctx
        .map(|ctx| ctx.enabled && hinting_mode == HintingMode::Full)
        .unwrap_or(false);

    for span in &text.layouted {
        if let Some(path) = span.overline.as_ref() {
            let mut path = path.clone();
            path.rendering_mode = rendering_mode;
            new_children.push(Node::Path(Box::new(path)));
        }

        if let Some(path) = span.underline.as_ref() {
            let mut path = path.clone();
            path.rendering_mode = rendering_mode;
            new_children.push(Node::Path(Box::new(path)));
        }

        // Instead of always processing each glyph separately, we always collect
        // as many outline glyphs as possible by pushing them into the span_builder
        // and only if we encounter a different glyph, or we reach the very end of the
        // span to we push the actual outline paths into new_children. This way, we don't need
        // to create a new path for every glyph if we have many consecutive glyphs
        // with just outlines (which is the most common case).
        let mut span_builder = tiny_skia_path::PathBuilder::new();

        for glyph in &span.positioned_glyphs {
            // A (best-effort conversion of a) COLR glyph.
            if let Some(tree) = cache.fontdb_colr(glyph.font, glyph.id) {
                let mut group = Group {
                    transform: glyph.colr_transform(),
                    ..Group::empty()
                };
                // TODO: Probably need to update abs_transform of children?
                group.children.push(Node::Group(Box::new(tree.root)));
                group.calculate_bounding_boxes();

                new_children.push(Node::Group(Box::new(group)));
            }
            // An SVG glyph. Will return the usvg node containing the glyph descriptions.
            else if let Some(node) = cache.fontdb_svg(glyph.font, glyph.id) {
                push_outline_paths(span, &mut span_builder, &mut new_children, rendering_mode);

                let mut group = Group {
                    transform: glyph.svg_transform(),
                    ..Group::empty()
                };
                // TODO: Probably need to update abs_transform of children?
                group.children.push(node);
                group.calculate_bounding_boxes();

                new_children.push(Node::Group(Box::new(group)));
            }
            // A bitmap glyph.
            else if let Some(img) = cache.fontdb_raster(glyph.font, glyph.id) {
                push_outline_paths(span, &mut span_builder, &mut new_children, rendering_mode);

                let transform = if img.is_sbix {
                    glyph.sbix_transform(
                        img.x,
                        img.y,
                        img.glyph_bbox.map(|bbox| bbox.x_min as f32).unwrap_or(0.0),
                        img.glyph_bbox.map(|bbox| bbox.y_min as f32).unwrap_or(0.0),
                        img.pixels_per_em,
                        img.image.size.height(),
                    )
                } else {
                    glyph.cbdt_transform(img.x, img.y, img.pixels_per_em)
                };

                let mut group = Group {
                    transform,
                    ..Group::empty()
                };
                group.children.push(Node::Image(Box::new(img.image)));
                group.calculate_bounding_boxes();

                new_children.push(Node::Group(Box::new(group)));
            } else {
                // For variable fonts, we need to extract the outline with variations applied.
                // We can't use the cache here since the outline depends on variation values.
                // Also handle auto-opsz for variable fonts.
                let needs_variations = !glyph.variations.is_empty()
                    || glyph.font_optical_sizing() == crate::FontOpticalSizing::Auto;

                let outline = if use_hinting {
                    // Use skrifa for hinted outline extraction
                    let ppem = hinting_ctx.map(|ctx| ctx.ppem(glyph.font_size()));
                    extract_outline_skrifa(
                        &cache.fontdb,
                        glyph.font,
                        glyph.id,
                        &glyph.variations,
                        glyph.font_size(),
                        glyph.font_optical_sizing(),
                        ppem,
                        hinting_mode,
                    )
                } else if needs_variations {
                    cache.fontdb.outline_with_variations(
                        glyph.font,
                        glyph.id,
                        &glyph.variations,
                        glyph.font_size(),
                        glyph.font_optical_sizing(),
                    )
                } else {
                    cache.fontdb_outline(glyph.font, glyph.id)
                };

                if let Some(outline) = outline.and_then(|p| p.transform(glyph.outline_transform())) {
                    span_builder.push_path(&outline);
                }
            }
        }

        push_outline_paths(span, &mut span_builder, &mut new_children, rendering_mode);

        if let Some(path) = span.line_through.as_ref() {
            let mut path = path.clone();
            path.rendering_mode = rendering_mode;
            new_children.push(Node::Path(Box::new(path)));
        }
    }

    let mut group = Group {
        id: text.id.clone(),
        ..Group::empty()
    };

    for child in new_children {
        group.children.push(child);
    }

    group.calculate_bounding_boxes();
    let stroke_bbox = group.stroke_bounding_box().to_non_zero_rect()?;
    Some((group, stroke_bbox))
}

/// Extract glyph outline using skrifa with optional hinting.
fn extract_outline_skrifa(
    fontdb: &fontdb::Database,
    font_id: fontdb::ID,
    glyph_id: GlyphId,
    variations: &[crate::FontVariation],
    font_size: f32,
    font_optical_sizing: crate::FontOpticalSizing,
    ppem: Option<f32>,
    hinting_mode: HintingMode,
) -> Option<tiny_skia_path::Path> {
    fontdb.with_face_data(font_id, |data, face_index| -> Option<tiny_skia_path::Path> {
        let font = FontRef::from_index(data, face_index).ok()?;
        let outlines = font.outline_glyphs();
        let glyph = outlines.get(glyph_id)?;

        // Build variation coordinates if needed, using avar-aware normalization
        let needs_variations = !variations.is_empty()
            || font_optical_sizing == crate::FontOpticalSizing::Auto;

        let location = if needs_variations {
            let axes = font.axes();
            let mut coords: Vec<skrifa::instance::NormalizedCoord> =
                vec![Default::default(); axes.len()];

            // Build variation settings including auto-opsz
            let mut settings: Vec<VariationSetting> = variations
                .iter()
                .map(|v| VariationSetting::new(Tag::new(&v.tag), v.value))
                .collect();

            // Auto-set opsz if font-optical-sizing is auto and not explicitly set
            if font_optical_sizing == crate::FontOpticalSizing::Auto {
                let has_explicit_opsz = variations.iter().any(|v| v.tag == *b"opsz");
                if !has_explicit_opsz {
                    // Check if font has opsz axis
                    let has_opsz_axis = axes.iter().any(|a| a.tag() == Tag::new(b"opsz"));
                    if has_opsz_axis {
                        settings.push(VariationSetting::new(Tag::new(b"opsz"), font_size));
                    }
                }
            }

            // Use location_to_slice which applies avar (axis variations) table remapping.
            // This differs from ttf-parser's set_variation() which used raw user-space values.
            // Avar remapping transforms user-space axis values to design-space coordinates,
            // which is required for correct variable font rendering (especially for fonts
            // like Roboto Flex that rely heavily on avar for intermediate axis values).
            axes.location_to_slice(&settings, &mut coords);

            Some(coords)
        } else {
            None
        };

        let location_ref = location
            .as_ref()
            .map(|c| LocationRef::new(c))
            .unwrap_or_default();

        // Choose drawing settings based on hinting
        // Hinted output is in pixel units (scaled by ppem), while unhinted is in font units.
        // We scale hinted output back to font units so outline_transform() can apply consistent scaling.
        if let (Some(ppem_val), HintingMode::Full) = (ppem, hinting_mode) {
            let size = SkrifaSize::new(ppem_val);
            // Create hinting instance for smooth rendering.
            // Note: HintingInstance is created per-glyph. For performance optimization,
            // consider caching instances keyed by (font_id, ppem, location) if profiling
            // shows this is a bottleneck.
            let hinting_options = Target::Smooth {
                mode: skrifa::outline::SmoothMode::Normal,
                symmetric_rendering: true,
                preserve_linear_metrics: false,
            };

            if let Ok(hinting_instance) = HintingInstance::new(
                &outlines,
                size,
                location_ref,
                hinting_options,
            ) {
                // Use hinted drawing with the hinting instance
                // Output is in pixel units at ppem scale, so we need to scale back to font units
                let scale_back = font.head().unwrap().units_per_em() as f32 / ppem_val;
                let mut pen = ScalingPen::new(scale_back);
                let settings = DrawSettings::hinted(&hinting_instance, false);
                glyph.draw(settings, &mut pen).ok()?;
                return pen.finish();
            }
        }

        // Fallback to unhinted drawing (font units)
        let mut pen = SkrifaPen::new();
        let settings = DrawSettings::unhinted(SkrifaSize::unscaled(), location_ref);
        glyph.draw(settings, &mut pen).ok()?;
        pen.finish()
    })?
}

/// Pen adapter for skrifa's OutlinePen trait -> tiny_skia_path::PathBuilder
struct SkrifaPen {
    builder: tiny_skia_path::PathBuilder,
}

impl SkrifaPen {
    fn new() -> Self {
        Self {
            builder: tiny_skia_path::PathBuilder::new(),
        }
    }

    fn finish(self) -> Option<tiny_skia_path::Path> {
        self.builder.finish()
    }
}

/// Pen that scales coordinates by a factor (used to convert hinted pixel coords back to font units)
struct ScalingPen {
    builder: tiny_skia_path::PathBuilder,
    scale: f32,
}

impl ScalingPen {
    fn new(scale: f32) -> Self {
        Self {
            builder: tiny_skia_path::PathBuilder::new(),
            scale,
        }
    }

    fn finish(self) -> Option<tiny_skia_path::Path> {
        self.builder.finish()
    }
}

impl OutlinePen for ScalingPen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.builder.move_to(x * self.scale, y * self.scale);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.builder.line_to(x * self.scale, y * self.scale);
    }

    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.builder.quad_to(
            cx * self.scale, cy * self.scale,
            x * self.scale, y * self.scale,
        );
    }

    fn curve_to(&mut self, cx1: f32, cy1: f32, cx2: f32, cy2: f32, x: f32, y: f32) {
        self.builder.cubic_to(
            cx1 * self.scale, cy1 * self.scale,
            cx2 * self.scale, cy2 * self.scale,
            x * self.scale, y * self.scale,
        );
    }

    fn close(&mut self) {
        self.builder.close();
    }
}

impl OutlinePen for SkrifaPen {
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

/// Hinting mode derived from CSS text-rendering property
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HintingMode {
    /// No hinting (text-rendering: geometricPrecision)
    None,
    /// Full hinting (text-rendering: optimizeLegibility)
    Full,
}

impl HintingMode {
    /// Convert CSS TextRendering to HintingMode
    pub fn from_text_rendering(text_rendering: TextRendering) -> Self {
        match text_rendering {
            TextRendering::OptimizeSpeed => HintingMode::Full,
            TextRendering::OptimizeLegibility => HintingMode::Full,
            TextRendering::GeometricPrecision => HintingMode::None,
        }
    }
}

pub(crate) trait DatabaseExt {
    fn outline(&self, id: ID, glyph_id: GlyphId) -> Option<tiny_skia_path::Path>;
    fn outline_with_variations(
        &self,
        id: ID,
        glyph_id: GlyphId,
        variations: &[crate::FontVariation],
        font_size: f32,
        font_optical_sizing: crate::FontOpticalSizing,
    ) -> Option<tiny_skia_path::Path>;
    fn raster(&self, id: ID, glyph_id: GlyphId) -> Option<BitmapImage>;
    fn svg(&self, id: ID, glyph_id: GlyphId) -> Option<Node>;
    fn colr(&self, id: ID, glyph_id: GlyphId) -> Option<Tree>;
}

/// Bounding box for a glyph (x_min, y_min, x_max, y_max)
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub(crate) struct GlyphBbox {
    pub x_min: i16,
    pub y_min: i16,
    pub x_max: i16,
    pub y_max: i16,
}

#[derive(Clone)]
pub(crate) struct BitmapImage {
    image: Image,
    x: f32,
    y: f32,
    pixels_per_em: f32,
    glyph_bbox: Option<GlyphBbox>,
    is_sbix: bool,
}

impl DatabaseExt for Database {
    #[inline(never)]
    fn outline(&self, id: ID, glyph_id: GlyphId) -> Option<tiny_skia_path::Path> {
        self.with_face_data(id, |data, face_index| -> Option<tiny_skia_path::Path> {
            let font = FontRef::from_index(data, face_index).ok()?;
            let outlines = font.outline_glyphs();
            let glyph = outlines.get(glyph_id)?;

            let mut pen = SkrifaPen::new();
            let settings = DrawSettings::unhinted(SkrifaSize::unscaled(), LocationRef::default());
            glyph.draw(settings, &mut pen).ok()?;
            pen.finish()
        })?
    }

    #[inline(never)]
    fn outline_with_variations(
        &self,
        id: ID,
        glyph_id: GlyphId,
        variations: &[crate::FontVariation],
        font_size: f32,
        font_optical_sizing: crate::FontOpticalSizing,
    ) -> Option<tiny_skia_path::Path> {
        self.with_face_data(id, |data, face_index| -> Option<tiny_skia_path::Path> {
            let font = FontRef::from_index(data, face_index).ok()?;
            let outlines = font.outline_glyphs();
            let glyph = outlines.get(glyph_id)?;

            // Build variation coordinates using avar-aware normalization
            let axes = font.axes();
            let mut coords: Vec<skrifa::instance::NormalizedCoord> = vec![Default::default(); axes.len()];

            // Build variation settings including auto-opsz
            let mut settings: Vec<VariationSetting> = variations
                .iter()
                .map(|v| VariationSetting::new(Tag::new(&v.tag), v.value))
                .collect();

            // Auto-set opsz if font-optical-sizing is auto and not explicitly set
            if font_optical_sizing == crate::FontOpticalSizing::Auto {
                let has_explicit_opsz = variations.iter().any(|v| v.tag == *b"opsz");
                if !has_explicit_opsz {
                    let has_opsz_axis = axes.iter().any(|a| a.tag() == Tag::new(b"opsz"));
                    if has_opsz_axis {
                        settings.push(VariationSetting::new(Tag::new(b"opsz"), font_size));
                    }
                }
            }

            // Use location_to_slice which applies avar (axis variations) table remapping.
            // This differs from ttf-parser's set_variation() which used raw user-space values.
            // Avar remapping transforms user-space axis values to design-space coordinates,
            // which is required for correct variable font rendering (especially for fonts
            // like Roboto Flex that rely heavily on avar for intermediate axis values).
            axes.location_to_slice(&settings, &mut coords);

            let location = LocationRef::new(&coords);
            let mut pen = SkrifaPen::new();
            let settings = DrawSettings::unhinted(SkrifaSize::unscaled(), location);
            glyph.draw(settings, &mut pen).ok()?;
            pen.finish()
        })?
    }

    fn raster(&self, id: ID, glyph_id: GlyphId) -> Option<BitmapImage> {
        self.with_face_data(id, |data, face_index| -> Option<BitmapImage> {
            let font = FontRef::from_index(data, face_index).ok()?;

            // Try to get bitmap strikes
            let strikes = font.bitmap_strikes();
            // Get the largest available strike (use partial_cmp for f32)
            let strike = strikes
                .iter()
                .max_by(|a, b| a.ppem().partial_cmp(&b.ppem()).unwrap_or(std::cmp::Ordering::Equal))?;

            let bitmap_glyph = strike.get(glyph_id)?;
            let bitmap_data = bitmap_glyph.data;

            // Handle PNG data
            if let BitmapData::Png(png_data) = bitmap_data {
                // Get PNG dimensions using imagesize
                let (width, height) = if let Ok(size) = imagesize::blob_size(png_data) {
                    (size.width as u32, size.height as u32)
                } else {
                    // Fallback: estimate from strike ppem
                    let ppem = strike.ppem();
                    (ppem as u32, ppem as u32)
                };

                // Get the glyph outline bounding box for SBIX positioning.
                // SBIX requires the outline bbox for proper vertical alignment.
                let glyph_bbox = {
                    let outlines = font.outline_glyphs();
                    outlines.get(glyph_id).and_then(|glyph| {
                        let mut bounds_pen = ControlBoundsPen::new();
                        let settings = DrawSettings::unhinted(SkrifaSize::unscaled(), LocationRef::default());
                        glyph.draw(settings, &mut bounds_pen).ok()?;
                        bounds_pen.bounding_box().map(|bb| GlyphBbox {
                            x_min: bb.x_min as i16,
                            y_min: bb.y_min as i16,
                            x_max: bb.x_max as i16,
                            y_max: bb.y_max as i16,
                        })
                    })
                };

                // Detect SBIX format by checking if the font has an sbix table.
                // The previous heuristic using inner_bearing was unreliable.
                let is_sbix = font.table_data(Tag::new(b"sbix")).is_some();

                log::warn!(
                    "Bitmap glyph: bearing=({}, {}), inner_bearing=({}, {}), ppem={}, bbox={:?}, is_sbix={}, height={}",
                    bitmap_glyph.bearing_x, bitmap_glyph.bearing_y,
                    bitmap_glyph.inner_bearing_x, bitmap_glyph.inner_bearing_y,
                    strike.ppem(), glyph_bbox, is_sbix, height
                );

                // Use skrifa's inner_bearing values directly for both SBIX and CBDT.
                // inner_bearing_x/y contain the glyph positioning offsets we need.
                let (x, y) = (bitmap_glyph.inner_bearing_x, bitmap_glyph.inner_bearing_y);

                let bitmap_image = BitmapImage {
                    image: Image {
                        id: String::new(),
                        visible: true,
                        size: Size::from_wh(width as f32, height as f32)?,
                        rendering_mode: ImageRendering::OptimizeQuality,
                        kind: ImageKind::PNG(Arc::new(png_data.to_vec())),
                        abs_transform: Transform::default(),
                        abs_bounding_box: NonZeroRect::from_xywh(0.0, 0.0, width as f32, height as f32)?,
                    },
                    x,
                    y,
                    pixels_per_em: strike.ppem(),
                    glyph_bbox,
                    is_sbix,
                };

                return Some(bitmap_image);
            }

            None
        })?
    }

    fn svg(&self, id: ID, glyph_id: GlyphId) -> Option<Node> {
        // Parse SVG table manually since skrifa doesn't expose SVG table access yet.
        // SVG table format (OpenType spec):
        // - Header: version (u16), svgDocListOffset (u32), reserved (u32)
        // - Document list at offset: numEntries (u16), entries[]
        // - Each entry: startGlyphID (u16), endGlyphID (u16), svgDocOffset (u32), svgDocLength (u32)
        self.with_face_data(id, |data, face_index| -> Option<Node> {
            let font = FontRef::from_index(data, face_index).ok()?;

            let svg_table = font.table_data(Tag::new(b"SVG "))?;
            let svg_data = svg_table.as_ref();

            // Need at least header (10 bytes)
            if svg_data.len() < 10 {
                return None;
            }

            // Parse header
            let _version = u16::from_be_bytes([svg_data[0], svg_data[1]]);
            let doc_list_offset = u32::from_be_bytes([svg_data[2], svg_data[3], svg_data[4], svg_data[5]]) as usize;

            // Navigate to document list
            if doc_list_offset + 2 > svg_data.len() {
                return None;
            }

            let doc_list = &svg_data[doc_list_offset..];
            let num_entries = u16::from_be_bytes([doc_list[0], doc_list[1]]) as usize;

            // Each entry is 12 bytes
            let entries_start = 2;
            let glyph_id_val = glyph_id.to_u32() as u16;

            // Find the entry for this glyph
            for i in 0..num_entries {
                let entry_offset = entries_start + i * 12;
                if entry_offset + 12 > doc_list.len() {
                    break;
                }

                let entry = &doc_list[entry_offset..entry_offset + 12];
                let start_glyph = u16::from_be_bytes([entry[0], entry[1]]);
                let end_glyph = u16::from_be_bytes([entry[2], entry[3]]);
                let svg_doc_offset = u32::from_be_bytes([entry[4], entry[5], entry[6], entry[7]]) as usize;
                let svg_doc_length = u32::from_be_bytes([entry[8], entry[9], entry[10], entry[11]]) as usize;

                if glyph_id_val >= start_glyph && glyph_id_val <= end_glyph {
                    // Found the entry - extract SVG document
                    // Offset is relative to start of SVG table
                    let abs_offset = doc_list_offset + svg_doc_offset;
                    if abs_offset + svg_doc_length > svg_data.len() {
                        return None;
                    }

                    let svg_doc_data = &svg_data[abs_offset..abs_offset + svg_doc_length];

                    // Handle gzip compression (SVG documents may be gzip compressed)
                    let svg_bytes: std::borrow::Cow<[u8]> = if svg_doc_data.starts_with(&[0x1f, 0x8b]) {
                        // Gzip compressed
                        use std::io::Read;
                        let mut decoder = flate2::read::GzDecoder::new(svg_doc_data);
                        let mut decompressed = Vec::new();
                        if decoder.read_to_end(&mut decompressed).is_err() {
                            return None;
                        }
                        std::borrow::Cow::Owned(decompressed)
                    } else {
                        std::borrow::Cow::Borrowed(svg_doc_data)
                    };

                    // Parse the SVG document
                    let tree = crate::Tree::from_data(&svg_bytes, &crate::Options::default()).ok()?;

                    // If this record covers a single glyph, return the whole tree
                    // Otherwise, look for the specific glyph by ID
                    let node = if start_glyph == end_glyph {
                        Node::Group(Box::new(tree.root))
                    } else {
                        // Multi-glyph record - find the specific glyph by ID
                        let glyph_node_id = format!("glyph{}", glyph_id_val);
                        tree.node_by_id(&glyph_node_id).cloned()?
                    };

                    return Some(node);
                }
            }

            None
        })?
    }

    fn colr(&self, id: ID, glyph_id: GlyphId) -> Option<Tree> {
        // Use skrifa-based COLR painting
        // This provides COLRv1 support (sweep gradients, advanced blend modes)
        let result = self.with_face_data(id, |data, face_index| {
            super::skrifa_colr::paint_colr_glyph(data, face_index, glyph_id)
        })?;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skrifa_variable_font() {
        // Test that skrifa properly applies variable font axes
        let font_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../crates/resvg/tests/fonts/RobotoFlex.subset.ttf"
        );
        let font_data = std::fs::read(font_path).expect("Font not found");

        let font = FontRef::new(&font_data).expect("Failed to parse font");
        let outlines = font.outline_glyphs();

        // Get glyph for 'N'
        let charmap = font.charmap();
        let glyph_id = charmap.map('N').expect("Glyph not found");
        let glyph = outlines.get(glyph_id).expect("Outline not found");

        // Get axes
        let axes = font.axes();

        // Find wdth axis
        let wdth_idx = axes.iter().position(|a| a.tag() == Tag::new(b"wdth")).expect("wdth axis not found");

        // Draw with default location
        let mut pen1 = SkrifaPen::new();
        let settings1 = DrawSettings::unhinted(SkrifaSize::unscaled(), LocationRef::default());
        glyph.draw(settings1, &mut pen1).expect("Draw failed");
        let path1 = pen1.finish().expect("Path failed");
        let bounds1 = path1.bounds();

        // Draw with wdth=25 (narrow)
        let mut coords = vec![skrifa::instance::NormalizedCoord::default(); axes.len()];
        coords[wdth_idx] = axes.get(wdth_idx).unwrap().normalize(25.0);

        let location = LocationRef::new(&coords);
        let mut pen2 = SkrifaPen::new();
        let settings2 = DrawSettings::unhinted(SkrifaSize::unscaled(), location);
        glyph.draw(settings2, &mut pen2).expect("Draw failed");
        let path2 = pen2.finish().expect("Path failed");
        let bounds2 = path2.bounds();

        // The narrow version should have a smaller width
        assert!(
            bounds2.width() < bounds1.width(),
            "wdth=25 should be narrower than default! default width: {}, wdth=25 width: {}",
            bounds1.width(),
            bounds2.width()
        );
    }
}

