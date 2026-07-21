// Copyright 2022 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::mem;
use std::sync::Arc;

use fontdb::{Database, ID};
use skrifa::bitmap::{BitmapData, Origin};
use skrifa::instance::{LocationRef, Size as SkrifaSize};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::raw::TableProvider;
use skrifa::{FontRef, GlyphId, MetadataProvider, Tag};
use tiny_skia_path::{NonZeroRect, Size, Transform};
use xmlwriter::XmlWriter;

use crate::text::colr::GlyphPainter;
use crate::*;

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
    abs_transform: Transform,
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
            abs_transform,
        )
    }) {
        new_children.push(Node::Path(Box::new(path)));
    }
}

pub(crate) fn flatten(text: &mut Text, cache: &mut Cache) -> Option<(Group, NonZeroRect)> {
    let mut new_children = vec![];

    let abs_transform = text.abs_transform;
    let rendering_mode = resolve_rendering_mode(text);

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

        // For variable fonts, we need to extract the outline with variations applied.
        // We can't use the cache here since the outline depends on variation values.
        let has_explicit_variations = !span.variations.is_empty();

        for glyph in &span.positioned_glyphs {
            // A (best-effort conversion of a) COLR glyph.
            if let Some(tree) = cache.fontdb_colr(glyph.font, glyph.id) {
                let mut group = Group {
                    transform: glyph.colr_transform(),
                    ..Group::empty()
                };
                // TODO: Probably need to update abs_transform of children? Same
                // for SVG and bitmap glyphs.
                group.children.push(Node::Group(Box::new(tree.root)));
                group.calculate_bounding_boxes();

                new_children.push(Node::Group(Box::new(group)));
            }
            // An SVG glyph. Will return the usvg node containing the glyph descriptions.
            else if let Some(node) = cache.fontdb_svg(glyph.font, glyph.id) {
                push_outline_paths(
                    span,
                    &mut span_builder,
                    &mut new_children,
                    rendering_mode,
                    abs_transform,
                );

                let mut group = Group {
                    transform: glyph.svg_transform(),
                    ..Group::empty()
                };
                group.children.push(node);
                group.calculate_bounding_boxes();

                new_children.push(Node::Group(Box::new(group)));
            }
            // A bitmap glyph.
            else if let Some(img) = cache.fontdb_raster(glyph.font, glyph.id) {
                push_outline_paths(
                    span,
                    &mut span_builder,
                    &mut new_children,
                    rendering_mode,
                    abs_transform,
                );

                let transform = if img.is_sbix {
                    glyph.sbix_transform(
                        img.x,
                        img.y,
                        img.glyph_bbox.map(|(x_min, _)| x_min).unwrap_or(0.0),
                        img.glyph_bbox.map(|(_, y_min)| y_min).unwrap_or(0.0),
                        img.pixels_per_em,
                        img.image.size.height(),
                    )
                } else {
                    glyph.cbdt_transform(img.x, img.y, img.pixels_per_em, img.image.size.height())
                };

                let mut group = Group {
                    transform,
                    ..Group::empty()
                };
                group.children.push(Node::Image(Box::new(img.image)));
                group.calculate_bounding_boxes();

                new_children.push(Node::Group(Box::new(group)));
            } else {
                // Only bypass cache if: explicit variations OR (auto opsz AND font has opsz axis)
                let needs_variations = has_explicit_variations
                    || (span.font_optical_sizing == crate::FontOpticalSizing::Auto
                        && cache.has_opsz_axis(glyph.font));

                let outline = if needs_variations {
                    cache.fontdb.outline_with_variations(
                        glyph.font,
                        glyph.id,
                        &span.variations,
                        glyph.font_size(),
                        span.font_optical_sizing,
                    )
                } else {
                    cache.fontdb_outline(glyph.font, glyph.id)
                };

                if let Some(outline) = outline.and_then(|p| p.transform(glyph.outline_transform()))
                {
                    span_builder.push_path(&outline);
                }
            }
        }

        push_outline_paths(
            span,
            &mut span_builder,
            &mut new_children,
            rendering_mode,
            abs_transform,
        );

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

pub(crate) struct PathBuilder {
    builder: tiny_skia_path::PathBuilder,
}

impl PathBuilder {
    pub(crate) fn new() -> Self {
        Self {
            builder: tiny_skia_path::PathBuilder::new(),
        }
    }
    pub(crate) fn finish(self) -> Option<tiny_skia_path::Path> {
        self.builder.finish()
    }
}

impl OutlinePen for PathBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.builder.move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.builder.line_to(x, y);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.builder.quad_to(x1, y1, x, y);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.builder.cubic_to(x1, y1, x2, y2, x, y);
    }

    fn close(&mut self) {
        self.builder.close();
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
    fn has_opsz_axis(&self, id: ID) -> bool;
    fn raster(&self, id: ID, glyph_id: GlyphId) -> Option<BitmapImage>;
    fn svg(&self, id: ID, glyph_id: GlyphId) -> Option<Node>;
    fn colr(&self, id: ID, glyph_id: GlyphId) -> Option<Tree>;
}

#[derive(Clone)]
pub(crate) struct BitmapImage {
    image: Image,
    x: f32,
    y: f32,
    pixels_per_em: f32,
    glyph_bbox: Option<(f32, f32)>,
    is_sbix: bool,
}

impl DatabaseExt for Database {
    #[inline(never)]
    fn outline(&self, id: ID, glyph_id: GlyphId) -> Option<tiny_skia_path::Path> {
        self.with_face_data(id, |data, face_index| -> Option<tiny_skia_path::Path> {
            let font = FontRef::from_index(data, face_index).ok()?;

            let mut builder = PathBuilder::new();

            // An empty location resolves to the default variation instance.
            let glyph = font.outline_glyphs().get(glyph_id)?;
            glyph
                .draw(
                    DrawSettings::unhinted(SkrifaSize::unscaled(), LocationRef::default()),
                    &mut builder,
                )
                .ok()?;
            builder.finish()
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

            let mut settings: Vec<(Tag, f32)> = variations
                .iter()
                .map(|v| (Tag::new(&v.tag), v.value))
                .collect();

            // Auto-set opsz if font-optical-sizing is auto and not explicitly set
            if font_optical_sizing == crate::FontOpticalSizing::Auto {
                let has_explicit_opsz = variations.iter().any(|v| v.tag == *b"opsz");
                if !has_explicit_opsz && font.axes().get_by_tag(Tag::new(b"opsz")).is_some() {
                    settings.push((Tag::new(b"opsz"), font_size));
                }
            }

            let location = font.axes().location(settings.iter().copied());

            let mut builder = PathBuilder::new();

            font.outline_glyphs()
                .get(glyph_id)?
                .draw(
                    DrawSettings::unhinted(SkrifaSize::unscaled(), &location),
                    &mut builder,
                )
                .ok()?;
            builder.finish()
        })?
    }

    fn has_opsz_axis(&self, id: ID) -> bool {
        self.with_face_data(id, |data, face_index| -> Option<bool> {
            let font = FontRef::from_index(data, face_index).ok()?;
            Some(font.axes().get_by_tag(Tag::new(b"opsz")).is_some())
        })
        .flatten()
        .unwrap_or(false)
    }

    fn raster(&self, id: ID, glyph_id: GlyphId) -> Option<BitmapImage> {
        self.with_face_data(id, |data, face_index| -> Option<BitmapImage> {
            let font = FontRef::from_index(data, face_index).ok()?;

            let image = font
                .bitmap_strikes()
                .glyph_for_size(SkrifaSize::unscaled(), glyph_id)?;

            if let BitmapData::Png(png_data) = image.data {
                // `sbix` is the only bitmap table with a bottom-left origin; `CBDT`
                // and `EBDT` use a top-left origin. This drives which positioning
                // transform we apply below.
                let is_sbix = image.placement_origin == Origin::BottomLeft;

                // For `sbix`, the outline bounding box is needed for positioning.
                let glyph_bbox = if is_sbix {
                    font.glyph_metrics(SkrifaSize::unscaled(), LocationRef::default())
                        .bounds(glyph_id)
                        .map(|b| (b.x_min, b.y_min))
                } else {
                    None
                };

                // skrifa reports the inner bearing relative to the top of the bitmap
                // (positive upwards), while the transform helpers expect a bottom-anchored
                // vertical offset.
                let y = if is_sbix {
                    // `sbix` uses a bottom-left origin, so its offset is used as-is.
                    image.inner_bearing_y
                } else {
                    // `CBDT`/`EBDT` use a top-left origin, so shift down by the height.
                    image.inner_bearing_y - image.height as f32
                };

                let bitmap_image = BitmapImage {
                    image: Image {
                        id: String::new(),
                        visible: true,
                        size: Size::from_wh(image.width as f32, image.height as f32)?,
                        rendering_mode: ImageRendering::OptimizeQuality,
                        kind: ImageKind::PNG(Arc::new(png_data.to_vec())),
                        abs_transform: Transform::default(),
                        abs_bounding_box: NonZeroRect::from_xywh(
                            0.0,
                            0.0,
                            image.width as f32,
                            image.height as f32,
                        )?,
                    },
                    x: image.inner_bearing_x,
                    y,
                    pixels_per_em: image.ppem_x,
                    glyph_bbox,
                    is_sbix,
                };

                return Some(bitmap_image);
            }

            None
        })?
    }

    fn svg(&self, id: ID, glyph_id: GlyphId) -> Option<Node> {
        // TODO: Technically not 100% accurate because the SVG format in a OTF font
        // is actually a subset/superset of a normal SVG, but it seems to work fine
        // for Twitter Color Emoji, so might as well use what we already have.

        // TODO: Glyph records can contain the data for multiple glyphs. We should
        // add a cache so we don't need to reparse the data every time.
        self.with_face_data(id, |data, face_index| -> Option<Node> {
            let font = FontRef::from_index(data, face_index).ok()?;
            let svg_table = font.svg().ok()?;
            let image_data = svg_table.glyph_data(glyph_id).ok()??;
            let tree = Tree::from_data(image_data, &Options::default()).ok()?;

            let records = svg_table.svg_document_list().ok()?.document_records();
            let gid = glyph_id.to_u32();
            for record in records {
                let start = record.start_glyph_id().to_u32();
                let end = record.end_glyph_id().to_u32();
                if gid >= start && gid <= end {
                    // Twitter Color Emoji seems to always have one SVG record per glyph,
                    // while Noto Color Emoji sometimes contains multiple ones.
                    // It's kind of hacky, but the best we have for now.
                    if record.start_glyph_id() == record.end_glyph_id() {
                        return Some(Node::Group(Box::new(tree.root)));
                    }
                    if let Some(node) = tree.node_by_id(&format!("glyph{}", gid)).cloned() {
                        return Some(node);
                    }
                }
            }
            log::warn!("Failed to find SVG glyph node for glyph {}", gid);
            None
        })?
    }

    fn colr(&self, id: ID, glyph_id: GlyphId) -> Option<Tree> {
        self.with_face_data(id, |data, face_index| -> Option<Tree> {
            let font = FontRef::from_index(data, face_index).ok()?;
            let color_glyph = font.color_glyphs().get(glyph_id)?;

            let mut svg = XmlWriter::new(xmlwriter::Options::default());

            svg.start_element("svg");
            svg.write_attribute("xmlns", "http://www.w3.org/2000/svg");
            svg.write_attribute("xmlns:xlink", "http://www.w3.org/1999/xlink");

            svg.start_element("g");

            let mut glyph_painter = GlyphPainter::new(&font, &mut svg);
            color_glyph
                .paint(LocationRef::default(), &mut glyph_painter)
                .ok()?;

            svg.end_element();

            Tree::from_data(svg.end_document().as_bytes(), &Options::default()).ok()
        })?
    }
}
