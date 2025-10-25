// Copyright 2024 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::parser::OptionLog;
use skrifa::instance::Location;
use skrifa::prelude::Size;
use skrifa::raw::types::Point;
use skrifa::{
    color::{Brush, ColorStop, Extend, Transform},
    outline::DrawSettings,
    raw::TableProvider as _,
    MetadataProvider,
};
use std::fmt::Write as _;
use svgtypes::Color;

use super::transform::{skrifa_to_tsp_transform, tsp_to_skrifa_transform};

struct Builder<'a>(&'a mut String);

impl Builder<'_> {
    fn finish(&mut self) {
        if !self.0.is_empty() {
            self.0.pop(); // remove trailing space
        }
    }
}

impl skrifa::outline::OutlinePen for Builder<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        write!(self.0, "M {} {} ", x, y).unwrap();
    }

    fn line_to(&mut self, x: f32, y: f32) {
        write!(self.0, "L {} {} ", x, y).unwrap();
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        write!(self.0, "Q {} {} {} {} ", cx0, cy0, x, y).unwrap();
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        write!(self.0, "C {} {} {} {} {} {} ", cx0, cy0, cx1, cy1, x, y).unwrap();
    }

    fn close(&mut self) {
        self.0.push_str("Z ");
    }
}

trait XmlWriterExt {
    fn write_color_attribute(&mut self, name: &str, ts: Color);
    fn write_transform_attribute(&mut self, name: &str, ts: Transform);
    fn write_spread_method_attribute(&mut self, method: Extend);
}

impl XmlWriterExt for xmlwriter::XmlWriter {
    fn write_color_attribute(&mut self, name: &str, color: Color) {
        self.write_attribute_fmt(
            name,
            format_args!("rgb({}, {}, {})", color.red, color.green, color.blue),
        );
    }

    fn write_transform_attribute(&mut self, name: &str, ts: Transform) {
        if ts == Transform::default() {
            return;
        }

        self.write_attribute_fmt(
            name,
            format_args!(
                "matrix({} {} {} {} {} {})",
                ts.xx, ts.yx, ts.xy, ts.yy, ts.dx, ts.dy
            ),
        );
    }

    fn write_spread_method_attribute(&mut self, extend: Extend) {
        self.write_attribute(
            "spreadMethod",
            match extend {
                Extend::Pad => "pad",
                Extend::Repeat => "repeat",
                Extend::Reflect => "reflect",
                Extend::Unknown => return,
            },
        );
    }
}

// NOTE: This is only a best-effort translation of COLR into SVG.
pub(crate) struct GlyphPainter<'a> {
    pub(crate) font: &'a skrifa::FontRef<'a>,
    pub(crate) svg: &'a mut xmlwriter::XmlWriter,
    pub(crate) path_buf: &'a mut String,
    pub(crate) gradient_index: usize,
    pub(crate) clip_path_index: usize,
    pub(crate) foreground_color: Color,
    pub(crate) transform: Transform,
    pub(crate) outline_transform: Transform,
    pub(crate) transforms_stack: Vec<Transform>,
}

impl<'a> GlyphPainter<'a> {
    fn write_gradient_stops(&mut self, stops: &[ColorStop]) {
        for stop in stops {
            let color = self
                .palette_index_to_color(stop.palette_index, stop.alpha)
                .unwrap();
            self.svg.start_element("stop");
            self.svg.write_attribute("offset", &stop.offset);
            self.svg.write_color_attribute("stop-color", color);
            let opacity = f32::from(color.alpha) / 255.0;
            self.svg.write_attribute("stop-opacity", &opacity);
            self.svg.end_element();
        }
    }

    fn paint_solid(&mut self, color: Color) {
        self.svg.start_element("path");
        self.svg.write_color_attribute("fill", color);
        let opacity = f32::from(color.alpha) / 255.0;
        self.svg.write_attribute("fill-opacity", &opacity);
        self.svg
            .write_transform_attribute("transform", self.outline_transform);
        self.svg.write_attribute("d", self.path_buf);
        self.svg.end_element();
    }

    fn paint_linear_gradient(
        &mut self,
        p0: Point<f32>,
        p1: Point<f32>,
        color_stops: &[ColorStop],
        extend: Extend,
    ) {
        let gradient_id = format!("lg{}", self.gradient_index);
        self.gradient_index += 1;

        let gradient_transform = paint_transform(self.outline_transform, self.transform);

        // TODO: We ignore x2, y2. Have to apply them somehow.
        // TODO: The way spreadMode works in ttf and svg is a bit different. In SVG, the spreadMode
        // will always be applied based on x1/y1 and x2/y2. However, in TTF the spreadMode will
        // be applied from the first/last stop. So if we have a gradient with x1=0 x2=1, and
        // a stop at x=0.4 and x=0.6, then in SVG we will always see a padding, while in ttf
        // we will see the actual spreadMode. We need to account for that somehow.
        self.svg.start_element("linearGradient");
        self.svg.write_attribute("id", &gradient_id);
        self.svg.write_attribute("x1", &p0.x);
        self.svg.write_attribute("y1", &p0.y);
        self.svg.write_attribute("x2", &p1.x);
        self.svg.write_attribute("y2", &p1.y);
        self.svg.write_attribute("gradientUnits", &"userSpaceOnUse");
        self.svg.write_spread_method_attribute(extend);
        self.svg
            .write_transform_attribute("gradientTransform", gradient_transform);
        self.write_gradient_stops(color_stops);
        self.svg.end_element();

        self.svg.start_element("path");
        self.svg
            .write_attribute_fmt("fill", format_args!("url(#{})", gradient_id));
        self.svg
            .write_transform_attribute("transform", self.outline_transform);
        self.svg.write_attribute("d", self.path_buf);
        self.svg.end_element();
    }

    fn paint_radial_gradient(
        &mut self,
        c0: Point<f32>,
        r0: f32,
        c1: Point<f32>,
        r1: f32,
        color_stops: &[ColorStop],
        extend: Extend,
    ) {
        let gradient_id = format!("rg{}", self.gradient_index);
        self.gradient_index += 1;

        let gradient_transform = paint_transform(self.outline_transform, self.transform);

        self.svg.start_element("radialGradient");
        self.svg.write_attribute("id", &gradient_id);
        self.svg.write_attribute("cx", &c1.x);
        self.svg.write_attribute("cy", &c1.y);
        self.svg.write_attribute("r", &r1);
        self.svg.write_attribute("fr", &r0);
        self.svg.write_attribute("fx", &c0.x);
        self.svg.write_attribute("fy", &c0.y);
        self.svg.write_attribute("gradientUnits", &"userSpaceOnUse");
        self.svg.write_spread_method_attribute(extend);
        self.svg
            .write_transform_attribute("gradientTransform", gradient_transform);
        self.write_gradient_stops(color_stops);
        self.svg.end_element();

        self.svg.start_element("path");
        self.svg
            .write_attribute_fmt("fill", format_args!("url(#{})", gradient_id));
        self.svg
            .write_transform_attribute("transform", self.outline_transform);
        self.svg.write_attribute("d", self.path_buf);
        self.svg.end_element();
    }

    fn paint_sweep_gradient(
        &mut self,
        _c0: Point<f32>,
        _start_angle: f32,
        _end_angle: f32,
        _color_stops: &[ColorStop],
        _extend: Extend,
    ) {
        println!("Warning: sweep gradients are not supported.");
    }
}

fn paint_transform(outline_transform: Transform, transform: Transform) -> Transform {
    let outline_transform = skrifa_to_tsp_transform(outline_transform);
    let gradient_transform = skrifa_to_tsp_transform(transform);

    let gradient_transform = outline_transform
        .invert()
        .log_none(|| log::warn!("Failed to calculate transform for gradient in glyph."))
        .unwrap_or_default()
        .pre_concat(gradient_transform);

    tsp_to_skrifa_transform(gradient_transform)
}

impl GlyphPainter<'_> {
    fn clip_with_path(&mut self, path: &str) {
        let clip_id = format!("cp{}", self.clip_path_index);
        self.clip_path_index += 1;

        self.svg.start_element("clipPath");
        self.svg.write_attribute("id", &clip_id);
        self.svg.start_element("path");
        self.svg
            .write_transform_attribute("transform", self.outline_transform);
        self.svg.write_attribute("d", &path);
        self.svg.end_element();
        self.svg.end_element();

        self.svg.start_element("g");
        self.svg
            .write_attribute_fmt("clip-path", format_args!("url(#{})", clip_id));
    }

    fn palette_index_to_color(&self, palette_index: u16, alpha: f32) -> Option<Color> {
        let mut color = if palette_index == u16::MAX {
            self.foreground_color
        } else {
            let cpal = self.font.cpal().ok()?;
            let color = cpal.color_records_array()?.ok()?[palette_index as usize];
            Color {
                red: color.red,
                blue: color.blue,
                green: color.green,
                alpha: color.alpha,
            }
        };

        // Multiply alpha
        color.alpha = ((color.alpha as f32) * alpha) as u8;

        Some(color)
    }
}

impl<'a> skrifa::color::ColorPainter for GlyphPainter<'a> {
    fn push_transform(&mut self, transform: Transform) {
        self.transforms_stack.push(self.transform);
        self.transform = transform * self.transform;
    }

    fn pop_transform(&mut self) {
        if let Some(ts) = self.transforms_stack.pop() {
            self.transform = ts;
        }
    }

    fn push_clip_glyph(&mut self, glyph_id: skrifa::GlyphId) {
        self.path_buf.clear();
        let mut builder = Builder(&mut self.path_buf);

        match self.font.outline_glyphs().get(glyph_id) {
            Some(outliner) => {
                let size = Size::unscaled();
                let location = Location::default();
                outliner
                    .draw(DrawSettings::unhinted(size, &location), &mut builder)
                    .unwrap();
            }
            None => return,
        };
        builder.finish();

        // We have to write outline using the current transform.
        self.outline_transform = self.transform;
    }

    fn push_clip_box(&mut self, clip_box: skrifa::raw::types::BoundingBox<f32>) {
        let x_min = clip_box.x_min;
        let x_max = clip_box.x_max;
        let y_min = clip_box.y_min;
        let y_max = clip_box.y_max;

        let clip_path = format!(
            "M {} {} L {} {} L {} {} L {} {} Z",
            x_min, y_min, x_max, y_min, x_max, y_max, x_min, y_max
        );

        self.clip_with_path(&clip_path);
    }

    fn pop_clip(&mut self) {
        self.svg.end_element();
    }

    fn fill(&mut self, brush: Brush<'_>) {
        match brush {
            Brush::Solid {
                palette_index,
                alpha,
            } => {
                let color = self.palette_index_to_color(palette_index, alpha).unwrap();
                self.paint_solid(color);
            }
            Brush::LinearGradient {
                p0,
                p1,
                color_stops,
                extend,
            } => self.paint_linear_gradient(p0, p1, color_stops, extend),
            Brush::RadialGradient {
                c0,
                r0,
                c1,
                r1,
                color_stops,
                extend,
            } => self.paint_radial_gradient(c0, r0, c1, r1, color_stops, extend),

            Brush::SweepGradient {
                c0,
                start_angle,
                end_angle,
                color_stops,
                extend,
            } => self.paint_sweep_gradient(c0, start_angle, end_angle, color_stops, extend),
        }
    }

    fn push_layer(&mut self, composite_mode: skrifa::color::CompositeMode) {
        use skrifa::color::CompositeMode;
        // TODO: Need to figure out how to represent the other blend modes in SVG.
        let composite_mode = match composite_mode {
            CompositeMode::SrcOver => "normal",
            CompositeMode::Screen => "screen",
            CompositeMode::Overlay => "overlay",
            CompositeMode::Darken => "darken",
            CompositeMode::Lighten => "lighten",
            CompositeMode::ColorDodge => "color-dodge",
            CompositeMode::ColorBurn => "color-burn",
            CompositeMode::HardLight => "hard-light",
            CompositeMode::SoftLight => "soft-light",
            CompositeMode::Difference => "difference",
            CompositeMode::Exclusion => "exclusion",
            CompositeMode::Multiply => "multiply",
            CompositeMode::HslHue => "hue",
            CompositeMode::HslSaturation => "saturation",
            CompositeMode::HslColor => "color",
            CompositeMode::HslLuminosity => "luminosity",
            _ => {
                println!("Warning: unsupported blend mode: {:?}", composite_mode);
                "normal"
            }
        };

        self.svg.start_element("g");
        self.svg.write_attribute_fmt(
            "style",
            format_args!("mix-blend-mode: {}; isolation: isolate", composite_mode),
        );
    }
}
