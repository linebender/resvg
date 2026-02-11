// Copyright 2022 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::mem;
use std::sync::Arc;

use fontdb::{Database, ID};
use harfrust::Tag;
use skrifa::{
    FontRef, GlyphId, MetadataProvider,
    bitmap::BitmapData,
    instance::{LocationRef, Size as SkrifaSize},
    outline::{DrawSettings, OutlinePen, pen::ControlBoundsPen},
    setting::VariationSetting,
};
use tiny_skia_path::{NonZeroRect, Size, Transform};

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

/// Convert positioned glyphs to path outlines.
pub(crate) fn flatten(text: &mut Text, cache: &mut Cache) -> Option<(Group, NonZeroRect)> {
    let mut new_children = vec![];
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

        let mut span_builder = tiny_skia_path::PathBuilder::new();

        // Check if we need variations for this span.
        let has_explicit_variations = !span.variations.is_empty();

        for glyph in &span.positioned_glyphs {
            // Only use variations path if we have explicit variations OR
            // if font-optical-sizing is auto AND the font has an opsz axis
            let needs_variations = has_explicit_variations
                || (span.font_optical_sizing == crate::FontOpticalSizing::Auto
                    && cache.has_opsz_axis(glyph.font));

            // A (best-effort conversion of a) COLR glyph.
            if let Some(tree) = cache.fontdb_colr(glyph.font, glyph.id) {
                let mut group = Group {
                    transform: glyph.colr_transform(),
                    ..Group::empty()
                };
                group.children.push(Node::Group(Box::new(tree.root)));
                group.calculate_bounding_boxes();
                new_children.push(Node::Group(Box::new(group)));
            }
            // An SVG glyph.
            else if let Some(node) = cache.fontdb_svg(glyph.font, glyph.id) {
                push_outline_paths(span, &mut span_builder, &mut new_children, rendering_mode);

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
                // Regular outline glyph
                let outline = if needs_variations {
                    cache
                        .fontdb
                        .outline_with_variations(glyph.font, glyph.id, &span.variations)
                } else {
                    cache.fontdb_outline(glyph.font, glyph.id)
                };

                if let Some(outline) = outline.and_then(|p| p.transform(glyph.outline_transform()))
                {
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

// SkrifaPen for outline drawing
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

// DatabaseExt trait for skrifa-based font operations
pub(crate) trait DatabaseExt {
    fn outline(&self, id: ID, glyph_id: GlyphId) -> Option<tiny_skia_path::Path>;
    fn outline_with_variations(
        &self,
        id: ID,
        glyph_id: GlyphId,
        variations: &[crate::FontVariation],
    ) -> Option<tiny_skia_path::Path>;
    fn has_opsz_axis(&self, id: ID) -> bool;
    fn raster(&self, id: ID, glyph_id: GlyphId) -> Option<BitmapImage>;
    fn svg(&self, id: ID, glyph_id: GlyphId) -> Option<Node>;
    fn colr(&self, id: ID, glyph_id: GlyphId) -> Option<Tree>;
}

/// Bounding box for a glyph
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
    pub(crate) image: Image,
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) pixels_per_em: f32,
    pub(crate) glyph_bbox: Option<GlyphBbox>,
    pub(crate) is_sbix: bool,
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
    ) -> Option<tiny_skia_path::Path> {
        self.with_face_data(id, |data, face_index| -> Option<tiny_skia_path::Path> {
            let font = FontRef::from_index(data, face_index).ok()?;
            let outlines = font.outline_glyphs();
            let glyph = outlines.get(glyph_id)?;

            // Build variation coordinates using avar-aware normalization
            let axes = font.axes();
            let mut coords: Vec<skrifa::instance::NormalizedCoord> =
                vec![Default::default(); axes.len()];

            // Build variation settings (auto-opsz is already included in variations)
            let settings: Vec<VariationSetting> = variations
                .iter()
                .map(|v| VariationSetting::new(Tag::new(&v.tag), v.value))
                .collect();

            // Use location_to_slice which applies avar table remapping
            axes.location_to_slice(&settings, &mut coords);

            let location = LocationRef::new(&coords);
            let mut pen = SkrifaPen::new();
            let draw_settings = DrawSettings::unhinted(SkrifaSize::unscaled(), location);
            glyph.draw(draw_settings, &mut pen).ok()?;
            pen.finish()
        })?
    }

    fn has_opsz_axis(&self, id: ID) -> bool {
        self.with_face_data(id, |data, face_index| -> Option<bool> {
            let font = FontRef::from_index(data, face_index).ok()?;
            let has_opsz = font.axes().iter().any(|a| a.tag() == Tag::new(b"opsz"));
            Some(has_opsz)
        })
        .flatten()
        .unwrap_or(false)
    }

    fn raster(&self, id: ID, glyph_id: GlyphId) -> Option<BitmapImage> {
        self.with_face_data(id, |data, face_index| -> Option<BitmapImage> {
            let font = FontRef::from_index(data, face_index).ok()?;

            // Get largest strike (like ttf-parser's u16::MAX behavior)
            let strikes = font.bitmap_strikes();
            let strike = strikes.iter().max_by(|a, b| {
                a.ppem()
                    .partial_cmp(&b.ppem())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })?;

            let bitmap_glyph = strike.get(glyph_id)?;

            // Only handle PNG format (matching original ttf-parser behavior)
            let png_data = match bitmap_glyph.data {
                BitmapData::Png(data) => data,
                _ => return None,
            };

            // Get dimensions from PNG header
            let (width, height) = if let Ok(size) = imagesize::blob_size(png_data) {
                (size.width as u32, size.height as u32)
            } else {
                let ppem = strike.ppem();
                (ppem as u32, ppem as u32)
            };

            let glyph_bbox = {
                let outlines = font.outline_glyphs();
                outlines.get(glyph_id).and_then(|glyph| {
                    let mut bounds_pen = ControlBoundsPen::new();
                    let settings =
                        DrawSettings::unhinted(SkrifaSize::unscaled(), LocationRef::default());
                    glyph.draw(settings, &mut bounds_pen).ok()?;
                    bounds_pen.bounding_box().map(|bb| GlyphBbox {
                        x_min: bb.x_min.clamp(i16::MIN as f32, i16::MAX as f32) as i16,
                        y_min: bb.y_min.clamp(i16::MIN as f32, i16::MAX as f32) as i16,
                        x_max: bb.x_max.clamp(i16::MIN as f32, i16::MAX as f32) as i16,
                        y_max: bb.y_max.clamp(i16::MIN as f32, i16::MAX as f32) as i16,
                    })
                })
            };

            let is_sbix = font.table_data(Tag::new(b"sbix")).is_some();

            let bitmap_image = BitmapImage {
                image: Image {
                    id: String::new(),
                    visible: true,
                    size: Size::from_wh(width as f32, height as f32)?,
                    rendering_mode: ImageRendering::OptimizeQuality,
                    kind: ImageKind::PNG(Arc::new(png_data.to_vec())),
                    abs_transform: Transform::default(),
                    abs_bounding_box: NonZeroRect::from_xywh(
                        0.0,
                        0.0,
                        width as f32,
                        height as f32,
                    )?,
                },
                x: bitmap_glyph.inner_bearing_x,
                y: bitmap_glyph.inner_bearing_y,
                pixels_per_em: strike.ppem(),
                glyph_bbox,
                is_sbix,
            };

            Some(bitmap_image)
        })?
    }

    fn svg(&self, id: ID, glyph_id: GlyphId) -> Option<Node> {
        self.with_face_data(id, |data, face_index| -> Option<Node> {
            let font = FontRef::from_index(data, face_index).ok()?;

            let svg_table = font.table_data(Tag::new(b"SVG "))?;
            let svg_data = svg_table.as_ref();

            if svg_data.len() < 10 {
                return None;
            }

            let _version = u16::from_be_bytes([svg_data[0], svg_data[1]]);
            let doc_list_offset =
                u32::from_be_bytes([svg_data[2], svg_data[3], svg_data[4], svg_data[5]]) as usize;

            if doc_list_offset + 2 > svg_data.len() {
                return None;
            }

            let doc_list = &svg_data[doc_list_offset..];
            let num_entries = u16::from_be_bytes([doc_list[0], doc_list[1]]) as usize;

            let entries_start = 2;
            let glyph_id_val = glyph_id.to_u32() as u16;

            for i in 0..num_entries {
                let entry_offset = entries_start + i * 12;
                if entry_offset + 12 > doc_list.len() {
                    break;
                }

                let entry = &doc_list[entry_offset..entry_offset + 12];
                let start_glyph = u16::from_be_bytes([entry[0], entry[1]]);
                let end_glyph = u16::from_be_bytes([entry[2], entry[3]]);
                let svg_doc_offset =
                    u32::from_be_bytes([entry[4], entry[5], entry[6], entry[7]]) as usize;
                let svg_doc_length =
                    u32::from_be_bytes([entry[8], entry[9], entry[10], entry[11]]) as usize;

                if glyph_id_val >= start_glyph && glyph_id_val <= end_glyph {
                    let abs_offset = doc_list_offset + svg_doc_offset;
                    if abs_offset + svg_doc_length > svg_data.len() {
                        return None;
                    }

                    let svg_doc_data = &svg_data[abs_offset..abs_offset + svg_doc_length];

                    let svg_bytes: std::borrow::Cow<[u8]> =
                        if svg_doc_data.starts_with(&[0x1f, 0x8b]) {
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

                    let tree =
                        crate::Tree::from_data(&svg_bytes, &crate::Options::default()).ok()?;

                    let node = if start_glyph == end_glyph {
                        Node::Group(Box::new(tree.root))
                    } else {
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
        let font_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../crates/resvg/tests/fonts/RobotoFlex.subset.ttf"
        );
        let font_data = std::fs::read(font_path).expect("Font not found");

        let font = FontRef::new(&font_data).expect("Failed to parse font");
        let outlines = font.outline_glyphs();

        let charmap = font.charmap();
        let glyph_id = charmap.map('N').expect("Glyph not found");
        let glyph = outlines.get(glyph_id).expect("Outline not found");

        let axes = font.axes();

        let wdth_idx = axes
            .iter()
            .position(|a| a.tag() == Tag::new(b"wdth"))
            .expect("wdth axis not found");

        let mut pen1 = SkrifaPen::new();
        let settings1 = DrawSettings::unhinted(SkrifaSize::unscaled(), LocationRef::default());
        glyph.draw(settings1, &mut pen1).expect("Draw failed");
        let path1 = pen1.finish().expect("Path failed");
        let bounds1 = path1.bounds();

        let mut coords = vec![skrifa::instance::NormalizedCoord::default(); axes.len()];
        coords[wdth_idx] = axes.get(wdth_idx).unwrap().normalize(25.0);

        let location = LocationRef::new(&coords);
        let mut pen2 = SkrifaPen::new();
        let settings2 = DrawSettings::unhinted(SkrifaSize::unscaled(), location);
        glyph.draw(settings2, &mut pen2).expect("Draw failed");
        let path2 = pen2.finish().expect("Path failed");
        let bounds2 = path2.bounds();

        assert!(
            bounds2.width() < bounds1.width(),
            "wdth=25 should be narrower than default! default width: {}, wdth=25 width: {}",
            bounds1.width(),
            bounds2.width()
        );
    }
}
