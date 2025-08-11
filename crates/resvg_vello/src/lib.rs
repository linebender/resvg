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

pub use vello_cpu;
use vello_cpu::kurbo::{Affine, Vec2};
use vello_cpu::RenderContext;
pub use usvg;
use usvg::tiny_skia_path;

// mod clip;
// mod filter;
mod util;
// mod image;
// mod mask;
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
    transform: Affine,
    rctx: &mut RenderContext,
) {
    let target_size = tiny_skia_path::IntSize::from_wh(rctx.width() as u32, rctx.height() as u32).unwrap();
    let max_bbox = tiny_skia_path::IntRect::from_xywh(
        -(target_size.width() as i32) * 2,
        -(target_size.height() as i32) * 2,
        target_size.width() * 5,
        target_size.height() * 5,
    )
    .unwrap();

    let ctx = render::Context { max_bbox };
    render::render_nodes(tree.root(), &ctx, transform, rctx);
    
    rctx.flush();
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
    mut transform: Affine,
    rctx: &mut RenderContext,
) -> Option<()> {
    let bbox = node.abs_layer_bounding_box()?;

    let target_size = tiny_skia_path::IntSize::from_wh(rctx.width() as u32, rctx.height() as u32).unwrap();
    let max_bbox = tiny_skia_path::IntRect::from_xywh(
        -(target_size.width() as i32) * 2,
        -(target_size.height() as i32) * 2,
        target_size.width() * 5,
        target_size.height() * 5,
    )
    .unwrap();

    transform = transform.pre_translate(Vec2::new(-bbox.x() as f64, -bbox.y() as f64));

    let ctx = render::Context { max_bbox };
    render::render_node(node, &ctx, transform, rctx);

    rctx.flush();

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
