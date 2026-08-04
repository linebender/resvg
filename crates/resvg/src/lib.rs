// Copyright 2017 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

/*!
[resvg](https://github.com/linebender/resvg) is an SVG rendering library.
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

/// Encodes a pixmap as a PNG that declares a physical resolution.
///
/// A plain PNG carries no resolution, so a viewer or a word processor has to
/// guess one, and most guess 96 DPI. An image rendered for print or for a high
/// resolution display then shows up at the wrong physical size. Writing the
/// resolution into the `pHYs` chunk states the size the image is meant to be
/// shown at.
///
/// `dpi` is the resolution of the pixmap itself, which is only the same as
/// [`usvg::Options::dpi`] when the tree was rendered without scaling. Rendering
/// twice as large also doubles the resolution of the result.
///
/// # Example
///
/// ```no_run
/// # let tree = usvg::Tree::from_str("<svg/>", &usvg::Options::default()).unwrap();
/// let size = tree.size().to_int_size();
/// let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height()).unwrap();
/// resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());
/// let png = resvg::encode_png_with_dpi(pixmap.as_ref(), 300.0).unwrap();
/// ```
pub fn encode_png_with_dpi(
    pixmap: tiny_skia::PixmapRef,
    dpi: f32,
) -> Result<Vec<u8>, png::EncodingError> {
    // `pHYs` counts pixels per meter, and an inch is 0.0254 of one.
    let pixels_per_meter = (dpi / 0.0254).round().max(0.0) as u32;

    // A pixmap stores premultiplied pixels, while PNG wants straight alpha.
    let mut data = Vec::with_capacity(pixmap.data().len());
    for pixel in pixmap.pixels() {
        let color = pixel.demultiply();
        data.extend_from_slice(&[color.red(), color.green(), color.blue(), color.alpha()]);
    }

    let mut png_data = Vec::new();
    let mut encoder = png::Encoder::new(&mut png_data, pixmap.width(), pixmap.height());
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_pixel_dims(Some(png::PixelDimensions {
        xppu: pixels_per_meter,
        yppu: pixels_per_meter,
        unit: png::Unit::Meter,
    }));
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&data)?;
    writer.finish()?;

    Ok(png_data)
}

/// Saves a pixmap as a PNG file that declares a physical resolution.
///
/// See [`encode_png_with_dpi`].
pub fn save_png_with_dpi<P: AsRef<std::path::Path>>(
    pixmap: tiny_skia::PixmapRef,
    path: P,
    dpi: f32,
) -> Result<(), png::EncodingError> {
    let data = encode_png_with_dpi(pixmap, dpi)?;
    std::fs::write(path, data)?;
    Ok(())
}

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
    let max_bbox = max_filter_bbox(pixmap.width(), pixmap.height());

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

    let max_bbox = max_filter_bbox(pixmap.width(), pixmap.height());

    transform = transform.pre_translate(-bbox.x(), -bbox.y());

    let ctx = render::Context { max_bbox };
    render::render_node(node, &ctx, transform, pixmap);

    Some(())
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

fn max_filter_bbox(width: u32, height: u32) -> tiny_skia::IntRect {
    tiny_skia::IntRect::from_xywh(
        i32::try_from(width).unwrap_or(i32::MAX).saturating_mul(-2),
        i32::try_from(height).unwrap_or(i32::MAX).saturating_mul(-2),
        width.saturating_mul(5),
        height.saturating_mul(5),
    )
    .unwrap_or_else(|| {
        tiny_skia::IntRect::from_ltrb(i32::MIN / 2, i32::MIN / 2, i32::MAX / 2, i32::MAX / 2)
            .unwrap()
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn max_filter_bbox_is_clamped() {
        let bbox = super::max_filter_bbox(u32::MAX, u32::MAX);
        assert_eq!(bbox.left(), i32::MIN / 2);
        assert_eq!(bbox.top(), i32::MIN / 2);
        assert_eq!(bbox.right(), i32::MAX / 2);
        assert_eq!(bbox.bottom(), i32::MAX / 2);
    }
}
