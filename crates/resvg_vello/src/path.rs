// Copyright 2019 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use smallvec::smallvec;
use vello_cpu::kurbo::{Affine, Cap, Dashes, Join, Point, Stroke};
use vello_cpu::peniko::{BlendMode, ColorStop, ColorStops, Compose, Fill, Gradient, GradientKind, Mix};
use vello_cpu::{peniko, PaintType, RenderContext};
use vello_cpu::color::{ColorSpaceTag, DynamicColor};
use usvg::{LineCap, LineJoin};
use crate::render::Context;
use crate::util::{convert_color, convert_path, convert_transform};

pub fn render(
    path: &usvg::Path,
    blend_mode: BlendMode,
    ctx: &Context,
    transform: Affine,
    rctx: &mut RenderContext,
) {
    if blend_mode != BlendMode::new(Mix::Normal, Compose::SrcOver) {
        unimplemented!();
    }
    
    if !path.is_visible() {
        return;
    }

    if path.paint_order() == usvg::PaintOrder::FillAndStroke {
        fill_path(path, blend_mode, ctx, transform, rctx);
        stroke_path(path, blend_mode, ctx, transform, rctx);
    } else {
        stroke_path(path, blend_mode, ctx, transform, rctx);
        fill_path(path, blend_mode, ctx, transform, rctx);
    }
}

pub fn fill_path(
    path: &usvg::Path,
    _: BlendMode,
    _: &Context,
    transform: Affine,
    rctx: &mut RenderContext,
) -> Option<()> {
    let fill = path.fill()?;

    // Horizontal and vertical lines cannot be filled. Skip.
    if path.data().bounds().width() == 0.0 || path.data().bounds().height() == 0.0 {
        return None;
    }

    let rule = match fill.rule() {
        usvg::FillRule::NonZero => Fill::NonZero,
        usvg::FillRule::EvenOdd => Fill::EvenOdd,
    };

    let (paint, paint_transform) = convert_paint(fill.paint(), fill.opacity())?;
    
    rctx.set_paint(paint);
    rctx.set_anti_aliasing(path.rendering_mode().use_shape_antialiasing());
    rctx.set_paint_transform(paint_transform);
    rctx.set_transform(transform);
    rctx.set_fill_rule(rule);
    rctx.fill_path(&convert_path(path.data()));

    Some(())
}

fn stroke_path(
    path: &usvg::Path,
    _: BlendMode,
    _: &Context,
    transform: Affine,
    rctx: &mut RenderContext,
) -> Option<()> {
    let stroke = path.stroke()?;
    let (paint, paint_transform) = convert_paint(stroke.paint(), stroke.opacity())?;

    rctx.set_paint(paint);
    rctx.set_anti_aliasing(path.rendering_mode().use_shape_antialiasing());
    rctx.set_transform(transform);
    rctx.set_paint_transform(paint_transform);
    rctx.set_stroke(convert_stroke(stroke));
    rctx.stroke_path(&convert_path(path.data()));

    Some(())
}

fn convert_stroke(stroke: &usvg::Stroke) -> Stroke {
    Stroke {
        width: stroke.width().get() as f64,
        join: convert_join(stroke.linejoin()),
        miter_limit: stroke.miterlimit().get() as f64,
        start_cap: convert_cap(stroke.linecap()),
        end_cap: convert_cap(stroke.linecap()),
        dash_pattern: stroke.dasharray().map(|d| d.iter().map(|v| *v as f64).collect::<Dashes>())
            .unwrap_or_default(),
        dash_offset: stroke.dashoffset() as f64,
    }
}

fn convert_join(line_join: LineJoin) -> Join {
    match line_join {
        LineJoin::Miter => Join::Miter,
        LineJoin::MiterClip => unimplemented!(),
        LineJoin::Round => Join::Round,
        LineJoin::Bevel => Join::Bevel,
    }
}

fn convert_cap(cap: LineCap) -> Cap {
    match cap {
        LineCap::Butt => Cap::Butt,
        LineCap::Round => Cap::Round,
        LineCap::Square => Cap::Square,
    }
}

fn convert_paint(paint: &usvg::Paint, opacity: usvg::Opacity) -> Option<(PaintType, Affine)> {
    let paint = match paint {
        usvg::Paint::Color(c) => {
            (PaintType::Solid(convert_color(*c, opacity.to_u8())), Affine::IDENTITY)
        }
        usvg::Paint::LinearGradient(ref lg) => {
            (PaintType::Gradient(convert_linear_gradient(lg, opacity)?), convert_transform(lg.transform()))
        }
        usvg::Paint::RadialGradient(ref rg) => {
            (PaintType::Gradient(convert_radial_gradient(rg, opacity)?), convert_transform(rg.transform()))
        }
        usvg::Paint::Pattern(ref pattern) => {
            unimplemented!()
        }
    };
    
    Some(paint)
}

fn convert_linear_gradient(
    gradient: &usvg::LinearGradient,
    opacity: usvg::Opacity,
) -> Option<Gradient> {
    let kind = {
        GradientKind::Linear {
            start: Point::new(gradient.x1() as f64, gradient.y1() as f64),
            end: Point::new(gradient.x2() as f64, gradient.y2() as f64),
        }
    };
    
    convert_base_gradient(gradient, kind, opacity)
}

fn convert_radial_gradient(
    gradient: &usvg::RadialGradient,
    opacity: usvg::Opacity,
) -> Option<Gradient> {
    let kind = {
        GradientKind::Radial {
            start_center: Point::new(gradient.fx() as f64, gradient.fy() as f64),
            start_radius: 0.0,
            end_center: Point::new(gradient.cx() as f64, gradient.cy() as f64),
            end_radius: gradient.r().get()
        }
    };

    convert_base_gradient(gradient, kind, opacity)
}

fn convert_base_gradient(
    gradient: &usvg::BaseGradient,
    gradient_kind: GradientKind,
    opacity: usvg::Opacity,
) -> Option<Gradient> {
    let mode = match gradient.spread_method() {
        usvg::SpreadMethod::Pad => peniko::Extend::Pad,
        usvg::SpreadMethod::Reflect => peniko::Extend::Reflect,
        usvg::SpreadMethod::Repeat => peniko::Extend::Repeat,
    };

    let mut stops = smallvec![];
    for stop in gradient.stops() {
        let alpha = stop.opacity() * opacity;
        let color = convert_color(stop.color(), alpha.to_u8());
        
        stops.push(ColorStop {
            offset: stop.offset().get(),
            color: DynamicColor::from_alpha_color(color),
        });
    }
    
    let gradient = Gradient {
        kind: gradient_kind,
        extend: mode,
        stops: ColorStops(stops),
        interpolation_cs: ColorSpaceTag::Srgb,
        hue_direction: Default::default(),
    };

    Some(gradient)
}

// fn render_pattern_pixmap(
//     pattern: &usvg::Pattern,
//     ctx: &Context,
//     transform: tiny_skia::Transform,
// ) -> Option<(tiny_skia::Pixmap, tiny_skia::Transform)> {
//     let (sx, sy) = {
//         let ts2 = transform.pre_concat(pattern.transform());
//         ts2.get_scale()
//     };
// 
//     let rect = pattern.rect();
//     let img_size = tiny_skia::IntSize::from_wh(
//         (rect.width() * sx).round() as u32,
//         (rect.height() * sy).round() as u32,
//     )?;
//     let mut pixmap = tiny_skia::Pixmap::new(img_size.width(), img_size.height())?;
// 
//     let transform = tiny_skia::Transform::from_scale(sx, sy);
//     crate::render::render_nodes(pattern.root(), ctx, transform, &mut pixmap.as_mut());
// 
//     let mut ts = tiny_skia::Transform::default();
//     ts = ts.pre_concat(pattern.transform());
//     ts = ts.pre_translate(rect.x(), rect.y());
//     ts = ts.pre_scale(1.0 / sx, 1.0 / sy);
// 
//     Some((pixmap, ts))
// }
