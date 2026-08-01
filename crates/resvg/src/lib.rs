// Copyright 2017 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

/*!
[resvg](https://github.com/linebender/resvg) is an SVG rendering library.
*/

#![forbid(unsafe_code)]
#![forbid(clippy::missing_panics_doc)]
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

/// A rendering error.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    /// The SVG contains a too large filter region.
    FilterTooLarge,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FilterTooLarge => write!(f, "filter region is too large"),
        }
    }
}

impl std::error::Error for Error {}

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
) -> Result<(), Error> {
    let max_bbox = max_filter_bbox(pixmap.width(), pixmap.height())?;

    let ctx = render::Context { max_bbox };
    render::render_nodes(tree.root(), &ctx, transform, pixmap);
    Ok(())
}

/// Renders a node onto the pixmap.
///
/// `transform` will be used as a root transform.
/// Can be used to position SVG inside the `pixmap`.
///
/// The expected pixmap size can be retrieved from `usvg::Node::abs_layer_bounding_box()`.
///
/// Returns `Ok(None)` when `node` has a zero size.
///
/// The produced content is in the sRGB color space.
pub fn render_node(
    node: &usvg::Node,
    mut transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) -> Result<Option<()>, Error> {
    let Some(bbox) = node.abs_layer_bounding_box() else {
        return Ok(None);
    };

    let max_bbox = max_filter_bbox(pixmap.width(), pixmap.height())?;

    transform = transform.pre_translate(-bbox.x(), -bbox.y());

    let ctx = render::Context { max_bbox };
    render::render_node(node, &ctx, transform, pixmap);

    Ok(Some(()))
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

fn max_filter_bbox(width: u32, height: u32) -> Result<tiny_skia::IntRect, Error> {
    (|| {
        tiny_skia::IntRect::from_xywh(
            i32::try_from(width).ok()?.checked_mul(-2)?,
            i32::try_from(height).ok()?.checked_mul(-2)?,
            width.checked_mul(5)?,
            height.checked_mul(5)?,
        )
    })()
    .ok_or(Error::FilterTooLarge)
}
