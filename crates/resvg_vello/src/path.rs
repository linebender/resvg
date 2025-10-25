// Copyright 2019 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::sync::Arc;
use smallvec::smallvec;
use vello_cpu::kurbo::{Affine, Cap, Dashes, Join, Point, Stroke};
use vello_cpu::peniko::{BlendMode, ColorStop, ColorStops, Fill, Gradient, GradientKind};
use vello_cpu::{peniko, Image, ImageSource, PaintType, Pixmap, RenderContext, RenderMode, RenderSettings};
use vello_cpu::color::{ColorSpaceTag, DynamicColor};
use usvg::{LineCap, LineJoin};
use crate::render::Context;
use crate::util::{convert_color, convert_path, convert_transform, default_blend_mode, get_scale, settings};

pub fn render(
    path: &usvg::Path,
    blend_mode: BlendMode,
    ctx: &Context,
    transform: Affine,
    rctx: &mut RenderContext,
) {
    if blend_mode != default_blend_mode() {
        unimplemented!();
    }
    
    if !path.is_visible() {
        return;
    }

    if path.paint_order() == usvg::PaintOrder::FillAndStroke {
        fill_path(path, None, blend_mode, ctx, transform, rctx);
        stroke_path(path, blend_mode, ctx, transform, rctx);
    } else {
        stroke_path(path, blend_mode, ctx, transform, rctx);
        fill_path(path, None, blend_mode, ctx, transform, rctx);
    }
}

pub fn fill_path(
    path: &usvg::Path,
    override_paint: Option<(PaintType, Affine)>,
    _: BlendMode,
    ctx: &Context,
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

    let (paint, paint_transform) = override_paint
        .unwrap_or(convert_paint(fill.paint(), ctx, transform, *rctx.render_settings(), fill.opacity())?);
    
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
    ctx: &Context,
    transform: Affine,
    rctx: &mut RenderContext,
) -> Option<()> {
    let stroke = path.stroke()?;
    let (paint, paint_transform) = convert_paint(stroke.paint(), ctx, transform, *rctx.render_settings(), stroke.opacity())?;

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

fn convert_paint(paint: &usvg::Paint, ctx: &Context, transform: Affine, render_settings: RenderSettings, opacity: usvg::Opacity) -> Option<(PaintType, Affine)> {
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
            let (pix, transform) = render_pattern_pixmap(pattern, ctx, transform, opacity.get(), &settings(&render_settings))?;
            (PaintType::Image(Image {
                source: ImageSource::Pixmap(Arc::new(pix)),
                x_extend: peniko::Extend::Repeat,
                y_extend: peniko::Extend::Repeat,
                quality: peniko::ImageQuality::High,
            }), transform)
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

fn render_pattern_pixmap(
    pattern: &usvg::Pattern,
    ctx: &Context,
    transform: Affine,
    opacity: f32,
    render_settings: &RenderSettings
) -> Option<(Pixmap, Affine)> {
    let (sx, sy) = {
        let ts2 = transform * convert_transform(pattern.transform());
        get_scale(ts2)
    };

    let rect = pattern.rect();
    let width = (rect.width() as f64 * sx).round() as u16;
    let height = (rect.height() as f64 * sy).round() as u16;

    let mut rctx = RenderContext::new_with(width, height, settings(&render_settings));
    let mut pixmap = Pixmap::new(width, height);

    let transform = Affine::scale_non_uniform(sx, sy);
    
    if opacity != 1.0 {
        rctx.push_layer(
            None, None, Some(opacity), None
        );
    }
    
    crate::render::render_nodes(pattern.root(), ctx, transform, &mut rctx);
    
    if opacity != 1.0 {
        rctx.pop_layer();
    }
    
    rctx.flush();
    // TODO: Make render mode configurable
    rctx.render_to_pixmap(&mut pixmap, RenderMode::OptimizeQuality);

    let mut ts = Affine::IDENTITY;
    ts = ts * convert_transform(pattern.transform());
    ts = ts * Affine::translate((rect.x() as f64, rect.y() as f64));
    ts = ts * Affine::scale_non_uniform(1.0 / sx, 1.0 / sy);

    Some((pixmap, ts))
}
