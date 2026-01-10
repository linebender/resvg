// Copyright 2017 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

/*!
[resvg](https://github.com/linebender/resvg) is an SVG rendering library.

## Main functions

- [`render`] - Renders an SVG tree onto a pixmap
- [`render_node`] - Renders a single node onto a pixmap
- [`encode_png_with_dpi`] - Encodes a pixmap as PNG with DPI metadata
- [`save_png_with_dpi`] - Saves a pixmap as PNG with DPI metadata

## Re-exports

This crate re-exports [`tiny_skia`] for pixmap handling and [`usvg`] for SVG parsing.
*/

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::identity_op)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::wrong_self_convention)]

pub use tiny_skia;
pub use usvg;

mod clip;
mod filter;
mod geom;
mod image;
mod mask;
mod path;
mod render;

/// Renders a tree onto the pixmap.
///
/// `transform` will be used as a root transform.
/// Can be used to position SVG inside the `pixmap`.
///
/// The produced content is in the sRGB color space.
pub fn render(
    tree: &usvg::Tree,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) {
    let target_size = tiny_skia::IntSize::from_wh(pixmap.width(), pixmap.height()).unwrap();
    let max_bbox = tiny_skia::IntRect::from_xywh(
        -(target_size.width() as i32) * 2,
        -(target_size.height() as i32) * 2,
        target_size.width() * 5,
        target_size.height() * 5,
    )
    .unwrap();

    let ctx = render::Context { max_bbox };
    render::render_nodes(tree.root(), &ctx, transform, pixmap);
}

/// Renders a node onto the pixmap.
///
/// `transform` will be used as a root transform.
/// Can be used to position SVG inside the `pixmap`.
///
/// The expected pixmap size can be retrieved from `usvg::Node::abs_layer_bounding_box()`.
///
/// Returns `None` when `node` has a zero size.
///
/// The produced content is in the sRGB color space.
pub fn render_node(
    node: &usvg::Node,
    mut transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) -> Option<()> {
    let bbox = node.abs_layer_bounding_box()?;

    let target_size = tiny_skia::IntSize::from_wh(pixmap.width(), pixmap.height()).unwrap();
    let max_bbox = tiny_skia::IntRect::from_xywh(
        -(target_size.width() as i32) * 2,
        -(target_size.height() as i32) * 2,
        target_size.width() * 5,
        target_size.height() * 5,
    )
    .unwrap();

    transform = transform.pre_translate(-bbox.x(), -bbox.y());

    let ctx = render::Context { max_bbox };
    render::render_node(node, &ctx, transform, pixmap);

    Some(())
}

/// Encodes a pixmap as PNG with DPI metadata in the pHYs chunk.
///
/// This is useful when you need the output PNG to have specific resolution metadata,
/// for example when targeting print or e-ink displays.
///
/// The DPI value is converted to pixels per meter for the PNG pHYs chunk.
///
/// # Example
///
/// ```no_run
/// let svg_data = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"></svg>"#;
/// let tree = usvg::Tree::from_str(&svg_data, &usvg::Options::default()).unwrap();
/// let size = tree.size().to_int_size();
/// let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height()).unwrap();
/// resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());
/// let png_data = resvg::encode_png_with_dpi(&pixmap, 96).unwrap();
/// ```
pub fn encode_png_with_dpi(
    pixmap: &tiny_skia::Pixmap,
    dpi: u32,
) -> Result<Vec<u8>, png::EncodingError> {
    // Convert DPI to pixels per meter for PNG pHYs chunk
    // 1 inch = 0.0254 meters, so pixels_per_meter = dpi / 0.0254
    let pixels_per_meter = (dpi as f64 / 0.0254).round() as u32;

    // Demultiply alpha (same as tiny-skia's encode_png)
    let mut tmp_data: Vec<u8> = pixmap.data().to_vec();
    for chunk in tmp_data.chunks_exact_mut(4) {
        let a = chunk[3];
        if a != 0 && a != 255 {
            let a_f = a as f32 / 255.0;
            chunk[0] = (chunk[0] as f32 / a_f).min(255.0) as u8;
            chunk[1] = (chunk[1] as f32 / a_f).min(255.0) as u8;
            chunk[2] = (chunk[2] as f32 / a_f).min(255.0) as u8;
        }
    }

    let mut data = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut data, pixmap.width(), pixmap.height());
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        // Set physical pixel dimensions (pHYs chunk) with unit = meters
        encoder.set_pixel_dims(Some(png::PixelDimensions {
            xppu: pixels_per_meter,
            yppu: pixels_per_meter,
            unit: png::Unit::Meter,
        }));
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&tmp_data)?;
    }

    Ok(data)
}

/// Saves a pixmap as a PNG file with DPI metadata.
///
/// This is a convenience wrapper around [`encode_png_with_dpi`] that writes directly to a file.
pub fn save_png_with_dpi(
    pixmap: &tiny_skia::Pixmap,
    path: &std::path::Path,
    dpi: u32,
) -> Result<(), png::EncodingError> {
    let data = encode_png_with_dpi(pixmap, dpi)?;
    std::fs::write(path, data)?;
    Ok(())
}

pub(crate) trait OptionLog {
    fn log_none<F: FnOnce()>(self, f: F) -> Self;
}

impl<T> OptionLog for Option<T> {
    #[inline]
    fn log_none<F: FnOnce()>(self, f: F) -> Self {
        self.or_else(|| {
            f();
            None
        })
    }
}
