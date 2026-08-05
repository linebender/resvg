// Copyright 2026 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Bitmap glyphs, i.e. the `CBDT`, `EBDT` and `sbix` strikes a font may carry
//! next to, or instead of, its outlines.

use std::sync::Arc;

use fontdb::{Database, ID};
use skrifa::MetadataProvider;
use skrifa::bitmap::{BitmapData, BitmapFormat, MaskData};
use skrifa::prelude::LocationRef;
use skrifa::raw::types::BoundingBox;
use tiny_skia_path::{NonZeroRect, Size, Transform};

use crate::{Color, GlyphId, Image, ImageKind, ImageRendering};

/// Identifies a bitmap glyph, together with everything its appearance depends
/// on, so that it can be cached.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct BitmapGlyphKey {
    pub(crate) font: ID,
    pub(crate) glyph: GlyphId,
    /// The size picks the strike, so the same glyph differs between sizes.
    /// Kept as raw bits, since `f32` is neither `Eq` nor `Hash`.
    font_size: u32,
    /// A mask stores coverage only and is painted with the fill of the span, so
    /// the same glyph differs between fills.
    fill: [u8; 4],
}

impl BitmapGlyphKey {
    pub(crate) fn new(
        font: ID,
        glyph: GlyphId,
        font_size: f32,
        fill_color: Color,
        fill_opacity: u8,
    ) -> Self {
        Self {
            font,
            glyph,
            font_size: font_size.to_bits(),
            fill: [
                fill_color.red,
                fill_color.green,
                fill_color.blue,
                fill_opacity,
            ],
        }
    }

    fn font_size(&self) -> f32 {
        f32::from_bits(self.font_size)
    }

    fn fill_color(&self) -> Color {
        Color::new_rgb(self.fill[0], self.fill[1], self.fill[2])
    }

    fn fill_opacity(&self) -> u8 {
        self.fill[3]
    }
}

/// A bitmap glyph, ready to be placed by the caller.
#[derive(Clone)]
pub(crate) struct BitmapImage {
    pub(crate) image: Image,
    pub(crate) x: i16,
    pub(crate) y: i16,
    pub(crate) pixels_per_em: u16,
    pub(crate) glyph_bbox: Option<BoundingBox<i16>>,
    pub(crate) is_sbix: bool,
}

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

/// Looks up a bitmap glyph and converts it into an image.
///
/// Returns `None` when the glyph should be drawn from its outline instead.
pub(crate) fn glyph(fontdb: &Database, key: BitmapGlyphKey) -> Option<BitmapImage> {
    let font_size = key.font_size();
    let mask_color = key.fill_color();
    let mask_opacity = key.fill_opacity();
    let glyph_id = key.glyph;
    fontdb.with_face_data(key.font, |data, face_index| -> Option<BitmapImage> {
        let font = skrifa::FontRef::from_index(data, face_index).ok()?;
        let bitmap_strikes = font.bitmap_strikes();

        // An unscaled size asks for the largest image available.
        let size = skrifa::prelude::Size::unscaled();
        let location = LocationRef::default();
        let image = bitmap_strikes.glyph_for_size(size, glyph_id.into())?;

        // A mask is one size specific rendering of the same design the outline
        // already describes, drawn for the exact size of its strike. Scaling one
        // looks far worse than drawing that outline, so prefer a strike that
        // matches the size and leave the glyph to its outline otherwise.
        //
        // A color bitmap is the intended appearance of the glyph rather than a
        // rendering of the outline, so it stays preferred at any size. An sbix
        // font relies on that: it ships outlines behind its bitmaps as a
        // fallback for renderers without sbix support, so treating the outline
        // as the better choice would invert what the font intends.
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
