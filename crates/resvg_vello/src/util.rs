// Copyright 2023 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use vello_cpu::color::{AlphaColor, Srgb};
use vello_cpu::kurbo::{Affine, BezPath};
use vello_cpu::peniko::{BlendMode, Compose, Mix};
use vello_cpu::RenderSettings;
use usvg::{tiny_skia_path, Color};
use usvg::tiny_skia_path::PathSegment;

/// Fits the current rect into the specified bounds.
pub fn fit_to_rect(
    r: tiny_skia_path::IntRect,
    bounds: tiny_skia_path::IntRect,
) -> Option<tiny_skia_path::IntRect> {
    let mut left = r.left();
    if left < bounds.left() {
        left = bounds.left();
    }

    let mut top = r.top();
    if top < bounds.top() {
        top = bounds.top();
    }

    let mut right = r.right();
    if right > bounds.right() {
        right = bounds.right();
    }

    let mut bottom = r.bottom();
    if bottom > bounds.bottom() {
        bottom = bounds.bottom();
    }

    tiny_skia_path::IntRect::from_ltrb(left, top, right, bottom)
}

pub fn convert_transform(transform: usvg::Transform) -> Affine {
    Affine::new([
        transform.sx as f64, 
        transform.ky as f64,
        transform.kx as f64, 
        transform.sy as f64,
        transform.tx as f64,
        transform.ty as f64,
    ])
}

pub fn convert_path(path: &tiny_skia_path::Path) -> BezPath {
    // TODO: We should probably reuse the allocation
    let mut bez_path = BezPath::new();

    for e in path.segments() {
        match e {
            PathSegment::MoveTo(p) => {
                bez_path.move_to((p.x, p.y));
            }
            PathSegment::LineTo(p) => {
                bez_path.line_to((p.x, p.y));
            }
            PathSegment::QuadTo(p1, p2) => {
                bez_path.quad_to((p1.x, p1.y), (p2.x, p2.y));
            }
            PathSegment::CubicTo(p1, p2, p3) => {
                bez_path.curve_to((p1.x, p1.y), (p2.x, p2.y), (p3.x, p3.y));
            }
            PathSegment::Close => {
                bez_path.close_path();
            }
        }
    }

    bez_path
}

pub fn convert_affine(affine: Affine) -> usvg::Transform {
    let c = affine.as_coeffs();
    usvg::Transform::from_row(c[0] as f32, c[1] as f32, c[2] as f32, c[3] as f32, c[4] as f32, c[5] as f32)
}

pub(crate) fn convert_color(color: Color, opacity: u8) -> AlphaColor<Srgb> {
    AlphaColor::from_rgba8(color.red, color.green, color.blue, opacity)
}

pub(crate) fn default_blend_mode() -> BlendMode {
    BlendMode::new(Mix::Normal, Compose::SrcOver)
}

pub(crate) fn get_scale(affine: Affine) -> (f64, f64) {
    let c = affine.as_coeffs();
    let x_scale = (c[0] * c[0] + c[2] * c[2]).sqrt();
    let y_scale = (c[1] * c[1] + c[3] * c[3]).sqrt();

    (x_scale, y_scale)
}

pub(crate) fn settings(settings: &RenderSettings) -> RenderSettings {
    RenderSettings {
        num_threads: 0,
        ..*settings
    }
}