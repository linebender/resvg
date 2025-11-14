// Copyright 2019 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::render::Context;
use usvg::ApproxEqUlps;

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
        usvg::Paint::LinearGradient(ref lg) => {
            paint.shader = convert_linear_gradient(lg, fill.opacity())?;
        }
        usvg::Paint::RadialGradient(ref rg) => {
            paint.shader = convert_radial_gradient(rg, fill.opacity())?;
        }
        usvg::Paint::Pattern(ref pattern) => {
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
        usvg::Paint::LinearGradient(ref lg) => {
            paint.shader = convert_linear_gradient(lg, stroke.opacity())?;
        }
        usvg::Paint::RadialGradient(ref rg) => {
            paint.shader = convert_radial_gradient(rg, stroke.opacity())?;
        }
        usvg::Paint::Pattern(ref pattern) => {
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

    let mut points = Vec::with_capacity(gradient.stops().len());
    for stop in gradient.stops() {
        let alpha = stop.opacity() * opacity;
        let color = tiny_skia::Color::from_rgba8(
            stop.color().red,
            stop.color().green,
            stop.color().blue,
            alpha.to_u8(),
        );
        points.push(tiny_skia::GradientStop::new(stop.offset().get(), color));
    }

    Some((mode, points))
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

// Note: The following functions provide optional support for Rectangle, Ellipse, and Polygon nodes.
// These shapes are converted to paths for rendering, maintaining compatibility with the existing
// rendering pipeline while preserving primitive shape information in the usvg tree.

/// Extension trait for `tiny_skia_path::PathBuilder` to add SVG arc support.
///
/// This trait provides an `arc_to` method that converts SVG arc commands to cubic Bézier curves.
/// This implementation is duplicated from `usvg::parser::shapes::PathBuilderExt` because
/// `usvg::parser::shapes` is a private module and cannot be accessed from the `resvg` crate.
///
/// The implementation uses the `kurbo` library to convert SVG arcs to cubic Bézier curves
/// with a tolerance of 0.1.
trait PathBuilderExt {
    fn arc_to(
        &mut self,
        rx: f32,
        ry: f32,
        x_axis_rotation: f32,
        large_arc: bool,
        sweep: bool,
        x: f32,
        y: f32,
    );
}

impl PathBuilderExt for usvg::tiny_skia_path::PathBuilder {
    fn arc_to(
        &mut self,
        rx: f32,
        ry: f32,
        x_axis_rotation: f32,
        large_arc: bool,
        sweep: bool,
        x: f32,
        y: f32,
    ) {
        let prev = match self.last_point() {
            Some(v) => v,
            None => return,
        };

        let svg_arc = kurbo::SvgArc {
            from: kurbo::Point::new(prev.x as f64, prev.y as f64),
            to: kurbo::Point::new(x as f64, y as f64),
            radii: kurbo::Vec2::new(rx as f64, ry as f64),
            x_rotation: (x_axis_rotation as f64).to_radians(),
            large_arc,
            sweep,
        };

        match kurbo::Arc::from_svg_arc(&svg_arc) {
            Some(arc) => {
                arc.to_cubic_beziers(0.1, |p1, p2, p| {
                    self.cubic_to(
                        p1.x as f32,
                        p1.y as f32,
                        p2.x as f32,
                        p2.y as f32,
                        p.x as f32,
                        p.y as f32,
                    );
                });
            }
            None => {
                self.line_to(x, y);
            }
        }
    }
}

/// Converts a Rectangle node to a Path for rendering.
///
/// This function converts a `Rectangle` primitive node to a `Path` node so it can be rendered
/// using the existing path rendering pipeline. The conversion preserves all visual properties
/// including rounded corners (rx/ry) using proper arc conversion.
pub(crate) fn rect_to_path(rect: &usvg::Rectangle) -> Option<usvg::Path> {
    use std::sync::Arc;
    use usvg::tiny_skia_path::PathBuilder;

    let x = rect.x();
    let y = rect.y();
    let width = rect.width();
    let height = rect.height();
    let rx = rect.rx();
    let ry = rect.ry();

    // Convert rectangle to path with proper arcs for rounded corners
    let path_data = if rx.approx_eq_ulps(&0.0, 4) && ry.approx_eq_ulps(&0.0, 4) {
        match usvg::Rect::from_xywh(x, y, width, height) {
            Some(r) => PathBuilder::from_rect(r),
            None => return None,
        }
    } else {
        // For rounded rectangles, convert to path with proper arcs
        let mut builder = PathBuilder::new();
        builder.move_to(x + rx, y);
        builder.line_to(x + width - rx, y);
        builder.arc_to(rx, ry, 0.0, false, true, x + width, y + ry);
        builder.line_to(x + width, y + height - ry);
        builder.arc_to(rx, ry, 0.0, false, true, x + width - rx, y + height);
        builder.line_to(x + rx, y + height);
        builder.arc_to(rx, ry, 0.0, false, true, x, y + height - ry);
        builder.line_to(x, y + ry);
        builder.arc_to(rx, ry, 0.0, false, true, x + rx, y);
        builder.close();
        match builder.finish() {
            Some(p) => p,
            None => return None,
        }
    };

    usvg::Path::new(
        rect.id().to_string(),
        rect.is_visible(),
        rect.fill().cloned(),
        rect.stroke().cloned(),
        rect.paint_order(),
        rect.rendering_mode(),
        Arc::new(path_data),
        rect.abs_transform(),
    )
}

/// Converts an Ellipse node to a Path for rendering.
///
/// This function converts an `Ellipse` primitive node to a `Path` node so it can be rendered
/// using the existing path rendering pipeline. The ellipse is converted using four arc segments
/// to maintain visual accuracy.
pub(crate) fn ellipse_to_path(ellipse: &usvg::Ellipse) -> Option<usvg::Path> {
    use std::sync::Arc;
    use usvg::tiny_skia_path::PathBuilder;

    let cx = ellipse.cx();
    let cy = ellipse.cy();
    let rx = ellipse.rx();
    let ry = ellipse.ry();

    // Convert ellipse to path with proper arcs
    let mut builder = PathBuilder::new();
    builder.move_to(cx + rx, cy);
    builder.arc_to(rx, ry, 0.0, false, true, cx, cy + ry);
    builder.arc_to(rx, ry, 0.0, false, true, cx - rx, cy);
    builder.arc_to(rx, ry, 0.0, false, true, cx, cy - ry);
    builder.arc_to(rx, ry, 0.0, false, true, cx + rx, cy);
    builder.close();

    let path_data = match builder.finish() {
        Some(p) => p,
        None => return None,
    };

    usvg::Path::new(
        ellipse.id().to_string(),
        ellipse.is_visible(),
        ellipse.fill().cloned(),
        ellipse.stroke().cloned(),
        ellipse.paint_order(),
        ellipse.rendering_mode(),
        Arc::new(path_data),
        ellipse.abs_transform(),
    )
}

/// Converts a Polygon node to a Path for rendering.
///
/// This function converts a `Polygon` primitive node to a `Path` node so it can be rendered
/// using the existing path rendering pipeline. The polygon points are converted to a closed path.
pub(crate) fn polygon_to_path(polygon: &usvg::Polygon) -> Option<usvg::Path> {
    use std::sync::Arc;
    use usvg::tiny_skia_path::PathBuilder;

    let mut builder = PathBuilder::new();
    let points = polygon.points();
    if points.is_empty() {
        return None;
    }

    builder.move_to(points[0].0, points[0].1);
    for &(x, y) in &points[1..] {
        builder.line_to(x, y);
    }
    builder.close();

    let path_data = match builder.finish() {
        Some(p) => p,
        None => return None,
    };

    usvg::Path::new(
        polygon.id().to_string(),
        polygon.is_visible(),
        polygon.fill().cloned(),
        polygon.stroke().cloned(),
        polygon.paint_order(),
        polygon.rendering_mode(),
        Arc::new(path_data),
        polygon.abs_transform(),
    )
}
