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
#[cfg(feature = "16bpc")]
pub mod dither;
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
    render_to_pixmap(tree, transform, pixmap);
}

/// Renders a tree onto a 16-bit per channel pixmap.
///
/// `transform` will be used as a root transform.
/// Can be used to position SVG inside the `pixmap`.
#[cfg(feature = "16bpc")]
pub fn render_u16(
    tree: &usvg::Tree,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapU16Mut,
) {
    render_to_pixmap(tree, transform, pixmap);
}

/// Renders a tree using 16bpc precision internally, then downsamples to 8bpc using blue noise dithering.
#[cfg(feature = "16bpc")]
pub fn render_u16_dithered(
    tree: &usvg::Tree,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) {
    let mut u16_pixmap = match tiny_skia::PixmapU16::new(pixmap.width(), pixmap.height()) {
        Some(p) => p,
        None => return,
    };
    render_u16(tree, transform, &mut u16_pixmap.as_mut());
    let dithered = dither::dither_u16_to_u8(&u16_pixmap);
    pixmap.data_mut().copy_from_slice(dithered.data());
}

/// Renders a tree onto a dynamic pixmap.
#[cfg(feature = "16bpc")]
pub fn render_dynamic(
    tree: &usvg::Tree,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::DynamicPixmapMut,
) {
    match pixmap {
        tiny_skia::DynamicPixmapMut::U8(p) => render(tree, transform, p),
        tiny_skia::DynamicPixmapMut::U16(p) => render_u16(tree, transform, p),
    }
}

/// Renders a tree onto a generic pixmap.
pub fn render_to_pixmap<P: tiny_skia::HighPixel>(
    tree: &usvg::Tree,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMutGeneric<'_, P>,
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
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) -> Option<()> {
    render_node_to_pixmap(node, transform, pixmap)
}

/// Renders a node onto a 16-bit per channel pixmap.
#[cfg(feature = "16bpc")]
pub fn render_node_u16(
    node: &usvg::Node,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapU16Mut,
) -> Option<()> {
    render_node_to_pixmap(node, transform, pixmap)
}

/// Renders a node onto a dynamic pixmap.
#[cfg(feature = "16bpc")]
pub fn render_node_dynamic(
    node: &usvg::Node,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::DynamicPixmapMut,
) -> Option<()> {
    match pixmap {
        tiny_skia::DynamicPixmapMut::U8(p) => render_node(node, transform, p),
        tiny_skia::DynamicPixmapMut::U16(p) => render_node_u16(node, transform, p),
    }
}

/// Renders a node onto a generic pixmap.
pub fn render_node_to_pixmap<P: tiny_skia::HighPixel>(
    node: &usvg::Node,
    mut transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMutGeneric<'_, P>,
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
