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
    let target_size = tiny_skia::IntSize::from_wh(pixmap.width(), pixmap.height()).unwrap();
    let max_bbox = max_bbox(target_size);

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
    let max_bbox = max_bbox(target_size);

    transform = transform.pre_translate(-bbox.x(), -bbox.y());

    let ctx = render::Context { max_bbox };
    render::render_node(node, &ctx, transform, pixmap);

    Some(())
}

/// Builds a generous clipping region around the target pixmap.
///
/// The region extends two viewport-lengths into the negative direction and is
/// five viewport-lengths wide/tall. For very large pixmaps (e.g. a tiny SVG
/// scaled up by a huge factor) these multiplications would exceed `i32::MAX`,
/// which previously made `IntRect::from_xywh` return `None` and the following
/// `.unwrap()` panic. We compute in `i64` and clamp to a valid range instead,
/// so an over-large target degrades into a still-generous clip region rather
/// than a crash. See https://github.com/linebender/resvg/issues/939.
fn max_bbox(target_size: tiny_skia::IntSize) -> tiny_skia::IntRect {
    let w = target_size.width() as i64;
    let h = target_size.height() as i64;

    // `IntRect::from_xywh` takes `u32` extents but rejects any that exceed
    // `i32::MAX` (or whose `x + width` overflows `i32`). Clamp accordingly.
    let clamp_pos = |v: i64| v.clamp(i32::MIN as i64, i32::MAX as i64) as i32;
    let clamp_ext = |v: i64| v.clamp(0, i32::MAX as i64) as u32;

    let x = clamp_pos(-w * 2);
    let y = clamp_pos(-h * 2);
    let width = clamp_ext(w * 5);
    let height = clamp_ext(h * 5);

    // `x`/`y` are non-positive and the extents are bounded by `i32::MAX`, so
    // `x + width` / `y + height` cannot overflow and this always succeeds; the
    // fallback to the bare target rect (which is always valid) only guards
    // against future changes.
    tiny_skia::IntRect::from_xywh(x, y, width, height).unwrap_or_else(|| {
        tiny_skia::IntRect::from_xywh(0, 0, target_size.width(), target_size.height()).unwrap()
    })
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

#[cfg(test)]
mod tests {
    use super::max_bbox;

    #[test]
    fn max_bbox_handles_huge_target() {
        // Regression test for https://github.com/linebender/resvg/issues/939.
        // Rendering into a very tall pixmap (e.g. a tiny SVG scaled up so its
        // height becomes ~571 million pixels) used to panic because
        // `height * 5` exceeds `i32::MAX` and `IntRect::from_xywh` returned
        // `None`. `max_bbox` must clamp instead of panicking.
        let size = tiny_skia::IntSize::from_wh(100, 571_428_544).unwrap();
        let bbox = max_bbox(size);
        // The extents are clamped to the valid `i32` range.
        assert!(bbox.width() <= i32::MAX as u32);
        assert!(bbox.height() <= i32::MAX as u32);
    }

    #[test]
    fn max_bbox_small_target_is_unchanged() {
        // For ordinary sizes the region keeps its 2x-negative / 5x-extent shape.
        let size = tiny_skia::IntSize::from_wh(100, 200).unwrap();
        let bbox = max_bbox(size);
        assert_eq!(bbox.x(), -200);
        assert_eq!(bbox.y(), -400);
        assert_eq!(bbox.width(), 500);
        assert_eq!(bbox.height(), 1000);
    }
}
