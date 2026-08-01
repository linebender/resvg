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
    let (x, width) = max_filter_axis(width);
    let (y, height) = max_filter_axis(height);
    tiny_skia::IntRect::from_xywh(x, y, width, height).unwrap()
}

fn max_filter_axis(length: u32) -> (i32, u32) {
    let max = i32::MAX as u32;
    let canvas = length.min(max);
    let margin = canvas.saturating_mul(2).min((max - canvas) / 2);
    let extent = canvas + margin * 2;
    (-(margin as i32), extent)
}

#[cfg(test)]
mod tests {
    #[test]
    fn max_filter_bbox_is_clamped() {
        for size in [1, i32::MAX as u32 / 5, i32::MAX as u32, u32::MAX] {
            let (origin, extent) = super::max_filter_axis(size);
            let canvas = size.min(i32::MAX as u32) as i32;
            assert!(origin <= 0);
            assert!(origin + extent as i32 >= canvas);
        }

        assert_eq!(super::max_filter_axis(100), (-200, 500));
    }
}
