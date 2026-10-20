// Copyright 2019 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::render::Context;

pub fn render(
    path: &usvg::Path,
    blend_mode: tiny_skia::BlendMode,
    ctx: &Context,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) {
    if !path.is_visible() {
        return;
    }

    if path.paint_order() == usvg::PaintOrder::FillAndStroke {
        fill_path(path, blend_mode, ctx, transform, pixmap);
        stroke_path(path, blend_mode, ctx, transform, pixmap);
    } else {
        stroke_path(path, blend_mode, ctx, transform, pixmap);
        fill_path(path, blend_mode, ctx, transform, pixmap);
    }
}

pub fn fill_path(
    path: &usvg::Path,
    blend_mode: tiny_skia::BlendMode,
    ctx: &Context,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) -> Option<()> {
    let fill = path.fill()?;

    // Horizontal and vertical lines cannot be filled. Skip.
    if path.data().bounds().width() == 0.0 || path.data().bounds().height() == 0.0 {
        return None;
    }

    let rule = match fill.rule() {
        usvg::FillRule::NonZero => tiny_skia::FillRule::Winding,
        usvg::FillRule::EvenOdd => tiny_skia::FillRule::EvenOdd,
    };

    let pattern_pixmap;
    let mut paint = tiny_skia::Paint::default();
    match fill.paint() {
        usvg::Paint::Color(c) => {
            paint.set_color_rgba8(c.red, c.green, c.blue, fill.opacity().to_u8());
        }
        usvg::Paint::LinearGradient(lg) => {
            paint.shader = convert_linear_gradient(lg, fill.opacity())?;
            paint.force_hq_pipeline =
                lg.color_interpolation() == usvg::ColorInterpolation::LinearRGB;
        }
        usvg::Paint::RadialGradient(rg) => {
            paint.shader = convert_radial_gradient(rg, fill.opacity())?;
            paint.force_hq_pipeline =
                rg.color_interpolation() == usvg::ColorInterpolation::LinearRGB;
        }
        usvg::Paint::Pattern(pattern) => {
            let (patt_pix, patt_ts) = render_pattern_pixmap(pattern, ctx, transform)?;

            pattern_pixmap = patt_pix;
            paint.shader = tiny_skia::Pattern::new(
                pattern_pixmap.as_ref(),
                tiny_skia::SpreadMode::Repeat,
                tiny_skia::FilterQuality::Bicubic,
                fill.opacity().get(),
                patt_ts,
            );
        }
    }
    paint.anti_alias = path.rendering_mode().use_shape_antialiasing();
    paint.blend_mode = blend_mode;

    pixmap.fill_path(path.data(), &paint, rule, transform, None);
    Some(())
}

fn stroke_path(
    path: &usvg::Path,
    blend_mode: tiny_skia::BlendMode,
    ctx: &Context,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) -> Option<()> {
    let stroke = path.stroke()?;
    let pattern_pixmap;
    let mut paint = tiny_skia::Paint::default();
    match stroke.paint() {
        usvg::Paint::Color(c) => {
            paint.set_color_rgba8(c.red, c.green, c.blue, stroke.opacity().to_u8());
        }
        usvg::Paint::LinearGradient(lg) => {
            paint.shader = convert_linear_gradient(lg, stroke.opacity())?;
            paint.force_hq_pipeline =
                lg.color_interpolation() == usvg::ColorInterpolation::LinearRGB;
        }
        usvg::Paint::RadialGradient(rg) => {
            paint.shader = convert_radial_gradient(rg, stroke.opacity())?;
            paint.force_hq_pipeline =
                rg.color_interpolation() == usvg::ColorInterpolation::LinearRGB;
        }
        usvg::Paint::Pattern(pattern) => {
            let (patt_pix, patt_ts) = render_pattern_pixmap(pattern, ctx, transform)?;

            pattern_pixmap = patt_pix;
            paint.shader = tiny_skia::Pattern::new(
                pattern_pixmap.as_ref(),
                tiny_skia::SpreadMode::Repeat,
                tiny_skia::FilterQuality::Bicubic,
                stroke.opacity().get(),
                patt_ts,
            );
        }
    }
    paint.anti_alias = path.rendering_mode().use_shape_antialiasing();
    paint.blend_mode = blend_mode;

    pixmap.stroke_path(path.data(), &paint, &stroke.to_tiny_skia(), transform, None);

    Some(())
}

fn convert_linear_gradient(
    gradient: &usvg::LinearGradient,
    opacity: usvg::Opacity,
) -> Option<tiny_skia::Shader<'_>> {
    let (mode, points) = convert_base_gradient(gradient, opacity)?;

    let shader = tiny_skia::LinearGradient::new(
        (gradient.x1(), gradient.y1()).into(),
        (gradient.x2(), gradient.y2()).into(),
        points,
        mode,
        gradient.transform(),
    )?;

    Some(shader)
}

fn convert_radial_gradient(
    gradient: &usvg::RadialGradient,
    opacity: usvg::Opacity,
) -> Option<tiny_skia::Shader<'_>> {
    let (mode, points) = convert_base_gradient(gradient, opacity)?;

    let shader = tiny_skia::RadialGradient::new(
        (gradient.fx(), gradient.fy()).into(),
        gradient.fr().get(),
        (gradient.cx(), gradient.cy()).into(),
        gradient.r().get(),
        points,
        mode,
        gradient.transform(),
    )?;

    Some(shader)
}

fn convert_base_gradient(
    gradient: &usvg::BaseGradient,
    opacity: usvg::Opacity,
) -> Option<(tiny_skia::SpreadMode, Vec<tiny_skia::GradientStop>)> {
    let mode = match gradient.spread_method() {
        usvg::SpreadMethod::Pad => tiny_skia::SpreadMode::Pad,
        usvg::SpreadMethod::Reflect => tiny_skia::SpreadMode::Reflect,
        usvg::SpreadMethod::Repeat => tiny_skia::SpreadMode::Repeat,
    };

    let linear_rgb = gradient.color_interpolation() == usvg::ColorInterpolation::LinearRGB;
    let mut points = Vec::with_capacity(gradient.stops().len());
    let mut previous = None;
    for stop in gradient.stops() {
        let alpha = stop.opacity() * opacity;
        let mut color = tiny_skia::Color::from_rgba8(
            stop.color().red,
            stop.color().green,
            stop.color().blue,
            alpha.to_u8(),
        );
        if linear_rgb {
            // Preserve opacity precision when adding intermediate stops.
            color.set_alpha(alpha.get());
        }
        let current = (stop.offset().get(), color);
        if linear_rgb {
            if let Some(previous) = previous {
                append_linear_rgb_stops(previous, current, &mut points);
            } else {
                points.push(tiny_skia::GradientStop::new(current.0, current.1));
            }
            previous = Some(current);
        } else {
            points.push(tiny_skia::GradientStop::new(current.0, current.1));
        }
    }

    Some((mode, points))
}

// Approximate linear-light interpolation with sRGB stops, leaving tiny-skia's
// existing alpha compositing and antialiasing unchanged.
fn append_linear_rgb_stops(
    start: (f32, tiny_skia::Color),
    end: (f32, tiny_skia::Color),
    output: &mut Vec<tiny_skia::GradientStop>,
) {
    use crate::color::{linear_to_srgb, srgb_to_linear};

    let offset = (start.0 + end.0) * 0.5;
    let channels = |c: tiny_skia::Color| [c.red(), c.green(), c.blue()];
    let a = channels(start.1);
    let b = channels(end.1);
    let middle = std::array::from_fn::<_, 3, _>(|i| {
        linear_to_srgb((srgb_to_linear(a[i]) + srgb_to_linear(b[i])) * 0.5)
    });

    // The encoding curve is concave. Its maximum distance from a chord is at
    // most twice its midpoint distance. Keep it below one eighth of an 8-bit
    // channel step to leave room for rounding during rasterization.
    let subdivide = (0..3).any(|i| (middle[i] - (a[i] + b[i]) * 0.5).abs() > 0.0625 / 255.0);
    if subdivide && offset > start.0 && offset < end.0 {
        let color = tiny_skia::Color::from_rgba(
            middle[0],
            middle[1],
            middle[2],
            (start.1.alpha() + end.1.alpha()) * 0.5,
        )
        .unwrap();
        let midpoint = (offset, color);
        append_linear_rgb_stops(start, midpoint, output);
        append_linear_rgb_stops(midpoint, end, output);
    } else {
        // Keep coincident stops, which represent hard transitions.
        output.push(tiny_skia::GradientStop::new(end.0, end.1));
    }
}

fn render_pattern_pixmap(
    pattern: &usvg::Pattern,
    ctx: &Context,
    transform: tiny_skia::Transform,
) -> Option<(tiny_skia::Pixmap, tiny_skia::Transform)> {
    let (sx, sy) = {
        let ts2 = transform.pre_concat(pattern.transform());
        ts2.get_scale()
    };

    let rect = pattern.rect();
    let img_size = tiny_skia::IntSize::from_wh(
        (rect.width() * sx).round() as u32,
        (rect.height() * sy).round() as u32,
    )?;
    let mut pixmap = tiny_skia::Pixmap::new(img_size.width(), img_size.height())?;

    let transform = tiny_skia::Transform::from_scale(sx, sy);
    crate::render::render_nodes(pattern.root(), ctx, transform, &mut pixmap.as_mut());

    let mut ts = tiny_skia::Transform::default();
    ts = ts.pre_concat(pattern.transform());
    ts = ts.pre_translate(rect.x(), rect.y());
    ts = ts.pre_scale(1.0 / sx, 1.0 / sy);

    Some((pixmap, ts))
}
