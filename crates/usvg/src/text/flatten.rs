// Copyright 2022 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::mem;
use std::sync::Arc;

use crate::GlyphId;
use fontdb::{Database, ID};
use skrifa::MetadataProvider;
use skrifa::Tag;
use skrifa::bitmap::{BitmapData, BitmapFormat, MaskData};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::prelude::LocationRef;
use skrifa::raw::TableProvider as _;
use skrifa::raw::types::BoundingBox;
use svgtypes::Color;
use tiny_skia_path::{NonZeroRect, Size, Transform};
use xmlwriter::XmlWriter;

use crate::text::OPSZ;
use crate::text::colr::GlyphPainter;
use crate::*;

/// Encodes 8-bit RGBA pixels as PNG, the only raw image format `ImageKind` can
/// carry. `CBDT`/`EBDT` strikes store uncompressed bitmaps, so they have to be
/// re-encoded before they can be embedded into the tree.
fn encode_rgba_png(rgba: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    let mut png_data = Vec::new();
    let mut encoder = png::Encoder::new(&mut png_data, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    // Glyph bitmaps are small and usually decoded again right away, but the
    // tree can also be written back out as SVG with the image embedded, so
    // don't skip compression entirely.
    encoder.set_compression(png::Compression::Fast);
    let mut writer = encoder.write_header().ok()?;
    writer.write_image_data(rgba).ok()?;
    writer.finish().ok()?;
    Some(png_data)
}

/// Reads the coverage value of a single pixel of a bitmap mask and scales it to
/// the 0..=255 range, where 255 means "fully covered by the glyph".
fn mask_coverage(mask: &MaskData, x: u32, y: u32, width: u32) -> u8 {
    // A packed mask is a continuous bit stream, while an unpacked one restarts
    // at a byte boundary on every row.
    let bpp = mask.bpp as usize;
    let bit = if mask.is_packed {
        (y as usize * width as usize + x as usize) * bpp
    } else {
        let row_bits = (width as usize * bpp).next_multiple_of(8);
        y as usize * row_bits + x as usize * bpp
    };

    let Some(byte) = mask.data.get(bit / 8) else {
        return 0;
    };

    // Pixels are stored from the most to the least significant bit.
    let shift = 8 - bpp - (bit % 8);
    let value = (byte >> shift) & (((1u16 << bpp) - 1) as u8);

    // Scale to a full byte, e.g. 4bpp 0..=15 becomes 0, 17, 34, ..., 255.
    let max = ((1u16 << bpp) - 1) as u8;
    (u16::from(value) * 255 / u16::from(max)) as u8
}

/// Converts a 1, 2, 4 or 8 bits-per-pixel bitmap mask into PNG data.
///
/// A mask only stores coverage, so the glyph is painted in `color`, just like an
/// outline glyph would be. `opacity` is the fill opacity, which an `Image` node
/// cannot carry on its own.
fn mask_to_png(
    mask: &MaskData,
    width: u32,
    height: u32,
    color: crate::Color,
    opacity: u8,
) -> Option<Vec<u8>> {
    if !matches!(mask.bpp, 1 | 2 | 4 | 8) {
        log::warn!("Bitmap glyph has an invalid bit depth: {}.", mask.bpp);
        return None;
    }

    let len = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;
    let mut rgba = Vec::with_capacity(len);
    for y in 0..height {
        for x in 0..width {
            let coverage = mask_coverage(mask, x, y, width);
            rgba.push(color.red);
            rgba.push(color.green);
            rgba.push(color.blue);
            rgba.push((u16::from(coverage) * u16::from(opacity) / 255) as u8);
        }
    }

    encode_rgba_png(&rgba, width, height)
}

/// Converts a premultiplied BGRA color bitmap into PNG data.
fn bgra_to_png(data: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    let len = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;
    let data = data.get(..len)?;

    let mut rgba = Vec::with_capacity(len);
    for pixel in data.chunks_exact(4) {
        // PNG stores straight alpha, so the color channels have to be undone.
        let a = pixel[3];
        let unpremultiply = |c: u8| match a {
            0 => 0,
            _ => (u16::from(c) * 255 / u16::from(a)).min(255) as u8,
        };
        rgba.push(unpremultiply(pixel[2]));
        rgba.push(unpremultiply(pixel[1]));
        rgba.push(unpremultiply(pixel[0]));
        rgba.push(a);
    }

    encode_rgba_png(&rgba, width, height)
}

fn resolve_rendering_mode(text: &Text) -> ShapeRendering {
    match text.rendering_mode {
        TextRendering::OptimizeSpeed => ShapeRendering::CrispEdges,
        TextRendering::OptimizeLegibility => ShapeRendering::GeometricPrecision,
        TextRendering::GeometricPrecision => ShapeRendering::GeometricPrecision,
    }
}

/// Returns the effective variation settings for a glyph: the span's explicit
/// variations plus an automatically computed `opsz` value when
/// `font-optical-sizing: auto` is in effect and the font has an `opsz` axis
/// that wasn't set explicitly. This matches browser behavior
/// (CSS font-optical-sizing: auto).
fn effective_variations(
    cache: &mut Cache,
    span: &layout::Span,
    glyph: &layout::PositionedGlyph,
) -> Vec<FontVariation> {
    let mut variations = span.variations.clone();
    if span.font_optical_sizing == crate::FontOpticalSizing::Auto
        && !variations.iter().any(|v| &v.tag == b"opsz")
        && cache.has_opsz_axis(glyph.font)
    {
        variations.push(FontVariation::new(*b"opsz", glyph.font_size()));
    }
    variations
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

        // Bitmap masks store coverage only, so they are painted like an outline
        // glyph would be. Non-solid paints cannot be expressed by an image and
        // fall back to black, which is also what an absent fill resolves to.
        let (mask_color, mask_opacity) = match span.fill.as_ref() {
            Some(fill) => {
                let color = match fill.paint {
                    Paint::Color(color) => color,
                    _ => crate::Color::black(),
                };
                (color, (fill.opacity.get() * 255.0).round() as u8)
            }
            None => (crate::Color::black(), 255),
        };

        for glyph in &span.positioned_glyphs {
            let variations = effective_variations(cache, span, glyph);

            // A (best-effort conversion of a) COLR glyph.
            if let Some(tree) = cache.fontdb_colr(glyph.font, glyph.id, &variations) {
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
            else if let Some(img) = cache.fontdb_raster(
                glyph.font,
                glyph.id,
                glyph.font_size(),
                mask_color,
                mask_opacity,
            ) {
                push_outline_paths(
                    span,
                    &mut span_builder,
                    &mut new_children,
                    rendering_mode,
                    abs_transform,
                );

                let transform = if img.is_sbix {
                    glyph.sbix_transform(
                        img.x as f32,
                        img.y as f32,
                        img.glyph_bbox.map(|bbox| bbox.x_min).unwrap_or(0) as f32,
                        img.glyph_bbox.map(|bbox| bbox.y_min).unwrap_or(0) as f32,
                        img.pixels_per_em as f32,
                        img.image.size.height(),
                    )
                } else {
                    glyph.cbdt_transform(img.x as f32, img.y as f32, img.pixels_per_em as f32)
                };

                let mut group = Group {
                    transform,
                    ..Group::empty()
                };
                group.children.push(Node::Image(Box::new(img.image)));
                group.calculate_bounding_boxes();

                new_children.push(Node::Group(Box::new(group)));
            } else {
                let outline = cache.fontdb_outline(glyph.font, glyph.id, &variations);

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

#[derive(Default)]
struct PathBuilder {
    builder: tiny_skia_path::PathBuilder,
}

impl OutlinePen for PathBuilder {
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

pub(crate) trait DatabaseExt {
    fn outline(
        &self,
        id: ID,
        glyph_id: GlyphId,
        variations: &[crate::FontVariation],
    ) -> Option<tiny_skia_path::Path>;
    fn has_opsz_axis(&self, id: ID) -> bool;
    fn raster(
        &self,
        id: ID,
        glyph_id: GlyphId,
        font_size: f32,
        mask_color: crate::Color,
        mask_opacity: u8,
    ) -> Option<BitmapImage>;
    fn svg(&self, id: ID, glyph_id: GlyphId) -> Option<Node>;
    fn colr(&self, id: ID, glyph_id: GlyphId, variations: &[crate::FontVariation]) -> Option<Tree>;
}

#[derive(Clone)]
pub(crate) struct BitmapImage {
    image: Image,
    x: i16,
    y: i16,
    pixels_per_em: u16,
    glyph_bbox: Option<BoundingBox<i16>>,
    is_sbix: bool,
}

impl DatabaseExt for Database {
    #[inline(never)]
    fn outline(
        &self,
        id: ID,
        glyph_id: GlyphId,
        variations: &[crate::FontVariation],
    ) -> Option<tiny_skia_path::Path> {
        self.with_face_data(id, |data, face_index| -> Option<tiny_skia_path::Path> {
            let font = skrifa::FontRef::from_index(data, face_index).ok()?;
            let outline = font.outline_glyphs().get(glyph_id.into())?;

            let mut builder = PathBuilder::default();

            let size = skrifa::prelude::Size::unscaled();
            // An empty variation list resolves to the default value of every
            // variation axis, which is what we want for non-variable fonts and
            // for variable fonts used without variations.
            let location = font.axes().location(
                variations
                    .iter()
                    .map(|v| (Tag::from_be_bytes(v.tag), v.value)),
            );
            outline
                .draw(DrawSettings::unhinted(size, &location), &mut builder)
                .ok()?;

            builder.builder.finish()
        })?
    }

    fn has_opsz_axis(&self, id: ID) -> bool {
        self.with_face_data(id, |data, face_index| -> Option<bool> {
            let font = skrifa::FontRef::from_index(data, face_index).ok()?;
            Some(font.axes().get_by_tag(OPSZ).is_some())
        })
        .flatten()
        .unwrap_or(false)
    }

    fn raster(
        &self,
        id: ID,
        glyph_id: GlyphId,
        font_size: f32,
        mask_color: crate::Color,
        mask_opacity: u8,
    ) -> Option<BitmapImage> {
        self.with_face_data(id, |data, face_index| -> Option<BitmapImage> {
            let font = skrifa::FontRef::from_index(data, face_index).ok()?;
            let bitmap_strikes = font.bitmap_strikes();

            // An unscaled size asks for the largest image available.
            let size = skrifa::prelude::Size::unscaled();
            let location = LocationRef::default();
            let image = bitmap_strikes.glyph_for_size(size, glyph_id.into())?;

            // A pixel font draws every strike for one exact size, and is meant
            // to be used at those sizes only. Scaling one of its bitmaps looks
            // far worse than the outline it also ships, so prefer the strike
            // that matches, and leave the glyph to the outline otherwise.
            //
            // Color bitmaps work the other way around: an emoji font tends to
            // carry a single large strike for every size, and usually has no
            // outline to fall back to, so those keep using the largest one.
            let image = if matches!(image.data, BitmapData::Mask(_)) {
                let matching = bitmap_strikes
                    .glyph_for_size(skrifa::prelude::Size::new(font_size), glyph_id.into())
                    .filter(|image| image.ppem_y == font_size);

                match matching {
                    Some(matching) => matching,
                    // Keep the unscaled bitmap for a glyph that has nothing else.
                    None if font.outline_glyphs().get(glyph_id.into()).is_none() => image,
                    None => return None,
                }
            } else {
                image
            };

            // A mask comes from a pixel font, which is drawn for one specific
            // size. Smoothing one of those blurs the very pixel grid it was
            // drawn on, and bleeds into the transparent border of the image
            // where a stem touches the edge of the glyph box, so keep the
            // pixels intact instead.
            let (png_data, rendering_mode) = match image.data {
                BitmapData::Png(data) => (data.to_vec(), ImageRendering::OptimizeQuality),
                BitmapData::Bgra(data) => (
                    bgra_to_png(data, image.width, image.height)?,
                    ImageRendering::OptimizeQuality,
                ),
                BitmapData::Mask(mask) => (
                    mask_to_png(&mask, image.width, image.height, mask_color, mask_opacity)?,
                    ImageRendering::Pixelated,
                ),
            };

            let metrics = font.glyph_metrics(size, location);
            let bounding_box = metrics.bounds(glyph_id.into()).map(|bbox| BoundingBox {
                x_min: bbox.x_min as i16,
                y_min: bbox.y_min as i16,
                x_max: bbox.x_max as i16,
                y_max: bbox.y_max as i16,
            });

            let bitmap_image = BitmapImage {
                image: Image {
                    id: String::new(),
                    visible: true,
                    size: Size::from_wh(image.width as f32, image.height as f32)?,
                    rendering_mode,
                    kind: ImageKind::PNG(Arc::new(png_data)),
                    abs_transform: Transform::default(),
                    abs_bounding_box: NonZeroRect::from_xywh(
                        0.0,
                        0.0,
                        image.width as f32,
                        image.height as f32,
                    )?,
                },
                x: image.inner_bearing_x as i16,
                y: image.inner_bearing_y as i16,
                pixels_per_em: image.ppem_x as u16,
                glyph_bbox: bounding_box,
                is_sbix: bitmap_strikes.format() == Some(BitmapFormat::Sbix),
            };

            Some(bitmap_image)
        })?
    }

    fn svg(&self, id: ID, glyph_id: GlyphId) -> Option<Node> {
        // SEE: https://docs.rs/read-fonts/latest/read_fonts/tables/svg/type.Svg.html

        // TODO: Technically not 100% accurate because the SVG format in a OTF font
        // is actually a subset/superset of a normal SVG, but it seems to work fine
        // for Twitter Color Emoji, so might as well use what we already have.

        // TODO: Glyph records can contain the data for multiple glyphs. We should
        // add a cache so we don't need to reparse the data every time.
        self.with_face_data(id, |data, face_index| -> Option<Node> {
            let font = skrifa::FontRef::from_index(data, face_index).ok()?;
            let svg_table = font.svg().ok()?;
            let image_data = svg_table.glyph_data(glyph_id.into()).ok()??;
            let tree = Tree::from_data(image_data, &Options::default()).ok()?;

            // Twitter Color Emoji seems to always have one SVG record per glyph,
            // while Noto Color Emoji sometimes contains multiple ones. It's kind of hacky,
            // but the best we have for now.
            let document_list = svg_table.svg_document_list().ok()?;
            let doc_record = document_list.document_records().iter().find(|r| {
                (r.start_glyph_id.get().to_u32()..=r.end_glyph_id.get().to_u32())
                    .contains(&glyph_id.0)
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
        })?
    }

    fn colr(&self, id: ID, glyph_id: GlyphId, variations: &[crate::FontVariation]) -> Option<Tree> {
        self.with_face_data(id, |data, face_index| -> Option<Tree> {
            let font = skrifa::FontRef::from_index(data, face_index).ok()?;

            let location = font.axes().location(
                variations
                    .iter()
                    .map(|v| (Tag::from_be_bytes(v.tag), v.value)),
            );

            let mut svg = XmlWriter::new(xmlwriter::Options::default());

            svg.start_element("svg");
            svg.write_attribute("xmlns", "http://www.w3.org/2000/svg");
            svg.write_attribute("xmlns:xlink", "http://www.w3.org/1999/xlink");

            let mut path_buf = String::with_capacity(256);
            let gradient_index = 1;
            let clip_path_index = 1;

            svg.start_element("g");

            let mut glyph_painter = GlyphPainter {
                font: &font,
                location: LocationRef::from(&location),
                svg: &mut svg,
                path_buf: &mut path_buf,
                gradient_index,
                clip_path_index,
                foreground_color: Color::new_rgba(0, 0, 0, 255),
                transform: skrifa::color::Transform::default(),
                outline_transform: skrifa::color::Transform::default(),
                transforms_stack: vec![skrifa::color::Transform::default()],
                clip_stack: Vec::new(),
            };

            font.color_glyphs()
                .get(glyph_id.into())?
                .paint(&location, &mut glyph_painter)
                .ok()?;
            svg.end_element();

            Tree::from_data(svg.end_document().as_bytes(), &Options::default()).ok()
        })?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coverage(bpp: u8, is_packed: bool, data: &[u8], width: u32, height: u32) -> Vec<u8> {
        let mask = MaskData {
            bpp,
            is_packed,
            data,
        };
        (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .map(|(x, y)| mask_coverage(&mask, x, y, width))
            .collect()
    }

    #[test]
    fn mask_coverage_1bpp_byte_aligned_rows() {
        // Each row starts on a byte boundary, so the last 5 bits are padding.
        let data = [0b1010_0000, 0b0100_0000];
        assert_eq!(
            coverage(1, false, &data, 3, 2),
            [255, 0, 255, /**/ 0, 255, 0]
        );
    }

    #[test]
    fn mask_coverage_1bpp_packed_rows() {
        // The second row continues in the same byte as the first one.
        let data = [0b1010_1000];
        assert_eq!(
            coverage(1, true, &data, 3, 2),
            [255, 0, 255, /**/ 0, 255, 0]
        );
    }

    #[test]
    fn mask_coverage_2bpp() {
        let data = [0b11_01_00_00];
        assert_eq!(coverage(2, false, &data, 3, 1), [255, 85, 0]);
    }

    #[test]
    fn mask_coverage_4bpp() {
        // A row of three 4bpp pixels is padded from 12 to 16 bits.
        let data = [0x0F, 0x80, 0xF0, 0x00];
        assert_eq!(
            coverage(4, false, &data, 3, 2),
            [0, 255, 136, /**/ 255, 0, 0]
        );
    }

    #[test]
    fn mask_coverage_8bpp() {
        let data = [0, 128, 255];
        assert_eq!(coverage(8, false, &data, 3, 1), [0, 128, 255]);
    }

    #[test]
    fn mask_coverage_out_of_bounds_is_transparent() {
        assert_eq!(coverage(8, false, &[42], 3, 1), [42, 0, 0]);
    }
}
