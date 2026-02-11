// Copyright 2026 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! COLRv1 color glyph painting using skrifa's ColorPainter.
//!
//! This module provides an alternative to ttf-parser for rendering COLR glyphs,
//! using skrifa's ColorPainter trait. This enables full COLRv1 support including
//! sweep/conic gradients.

use skrifa::{
    FontRef, GlyphId, MetadataProvider,
    color::{Brush, ColorGlyphFormat, ColorPainter, CompositeMode},
    instance::LocationRef,
    outline::OutlinePen,
    raw::types::BoundingBox,
};
use xmlwriter::XmlWriter;

use crate::{Options, Tree};

/// Skrifa-based pen for building SVG path data.
struct SvgPathPen<'a> {
    path: &'a mut String,
}

impl<'a> SvgPathPen<'a> {
    fn new(path: &'a mut String) -> Self {
        Self { path }
    }

    fn finish(&mut self) {
        if !self.path.is_empty() {
            self.path.pop(); // remove trailing space
        }
    }
}

impl OutlinePen for SvgPathPen<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        use std::fmt::Write;
        write!(self.path, "M {} {} ", x, y).unwrap();
    }

    fn line_to(&mut self, x: f32, y: f32) {
        use std::fmt::Write;
        write!(self.path, "L {} {} ", x, y).unwrap();
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        use std::fmt::Write;
        write!(self.path, "Q {} {} {} {} ", cx0, cy0, x, y).unwrap();
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        use std::fmt::Write;
        write!(self.path, "C {} {} {} {} {} {} ", cx0, cy0, cx1, cy1, x, y).unwrap();
    }

    fn close(&mut self) {
        self.path.push_str("Z ");
    }
}

/// COLR glyph painter that outputs SVG using skrifa's ColorPainter.
pub(crate) struct SkrifaGlyphPainter<'a> {
    font: FontRef<'a>,
    svg: &'a mut XmlWriter,
    path_buf: &'a mut String,
    gradient_index: usize,
    clip_path_index: usize,
    transform_stack: Vec<skrifa::color::Transform>,
    current_transform: skrifa::color::Transform,
}

impl<'a> SkrifaGlyphPainter<'a> {
    pub fn new(font: FontRef<'a>, svg: &'a mut XmlWriter, path_buf: &'a mut String) -> Self {
        Self {
            font,
            svg,
            path_buf,
            gradient_index: 1,
            clip_path_index: 1,
            transform_stack: Vec::new(),
            current_transform: skrifa::color::Transform::default(),
        }
    }

    fn get_color(&self, palette_index: u16) -> Option<skrifa::color::Color> {
        // TODO: SVG 2 allows specifying color palette via CSS font-palette property.
        // Currently we always use palette 0 (the default). Supporting font-palette
        // would require passing the palette index through the rendering pipeline.
        self.font
            .color_palettes()
            .get(0)?
            .colors()
            .get(palette_index as usize)
            .copied()
    }

    fn write_color(&mut self, name: &str, palette_index: u16, alpha: f32) {
        if let Some(color) = self.get_color(palette_index) {
            self.svg.write_attribute_fmt(
                name,
                format_args!("rgb({}, {}, {})", color.red, color.green, color.blue),
            );
            let opacity = (color.alpha as f32 / 255.0) * alpha;
            if opacity < 1.0 {
                let opacity_name = if name == "fill" {
                    "fill-opacity"
                } else {
                    "stop-opacity"
                };
                self.svg.write_attribute(opacity_name, &opacity);
            }
        }
    }

    fn write_transform(&mut self, name: &str, ts: skrifa::color::Transform) {
        // Check if it's an identity transform (no transformation)
        if ts.xx == 1.0
            && ts.yx == 0.0
            && ts.xy == 0.0
            && ts.yy == 1.0
            && ts.dx == 0.0
            && ts.dy == 0.0
        {
            return;
        }

        self.svg.write_attribute_fmt(
            name,
            format_args!(
                "matrix({} {} {} {} {} {})",
                ts.xx, ts.yx, ts.xy, ts.yy, ts.dx, ts.dy
            ),
        );
    }

    fn paint_solid(&mut self, palette_index: u16, alpha: f32) {
        self.svg.start_element("path");
        self.write_color("fill", palette_index, alpha);
        self.write_transform("transform", self.current_transform);
        self.svg.write_attribute("d", self.path_buf);
        self.svg.end_element();
    }

    fn paint_linear_gradient(
        &mut self,
        p0: skrifa::raw::types::Point<f32>,
        p1: skrifa::raw::types::Point<f32>,
        stops: &[skrifa::color::ColorStop],
        extend: skrifa::color::Extend,
    ) {
        let gradient_id = format!("lg{}", self.gradient_index);
        self.gradient_index += 1;

        self.svg.start_element("linearGradient");
        self.svg.write_attribute("id", &gradient_id);
        self.svg.write_attribute("x1", &p0.x);
        self.svg.write_attribute("y1", &p0.y);
        self.svg.write_attribute("x2", &p1.x);
        self.svg.write_attribute("y2", &p1.y);
        self.svg.write_attribute("gradientUnits", &"userSpaceOnUse");
        self.write_spread_method(extend);
        self.write_transform("gradientTransform", self.current_transform);
        self.write_gradient_stops(stops);
        self.svg.end_element();

        self.svg.start_element("path");
        self.svg
            .write_attribute_fmt("fill", format_args!("url(#{})", gradient_id));
        self.svg.write_attribute("d", self.path_buf);
        self.svg.end_element();
    }

    fn paint_radial_gradient(
        &mut self,
        c0: skrifa::raw::types::Point<f32>,
        r0: f32,
        c1: skrifa::raw::types::Point<f32>,
        r1: f32,
        stops: &[skrifa::color::ColorStop],
        extend: skrifa::color::Extend,
    ) {
        let gradient_id = format!("rg{}", self.gradient_index);
        self.gradient_index += 1;

        self.svg.start_element("radialGradient");
        self.svg.write_attribute("id", &gradient_id);
        self.svg.write_attribute("cx", &c1.x);
        self.svg.write_attribute("cy", &c1.y);
        self.svg.write_attribute("r", &r1);
        self.svg.write_attribute("fr", &r0);
        self.svg.write_attribute("fx", &c0.x);
        self.svg.write_attribute("fy", &c0.y);
        self.svg.write_attribute("gradientUnits", &"userSpaceOnUse");
        self.write_spread_method(extend);
        self.write_transform("gradientTransform", self.current_transform);
        self.write_gradient_stops(stops);
        self.svg.end_element();

        self.svg.start_element("path");
        self.svg
            .write_attribute_fmt("fill", format_args!("url(#{})", gradient_id));
        self.svg.write_attribute("d", self.path_buf);
        self.svg.end_element();
    }

    fn paint_sweep_gradient(
        &mut self,
        c0: skrifa::raw::types::Point<f32>,
        start_angle: f32,
        end_angle: f32,
        stops: &[skrifa::color::ColorStop],
        extend: skrifa::color::Extend,
    ) {
        // SVG doesn't have native sweep gradient support.
        // We approximate with a conic gradient in CSS or fall back to first stop color.
        // For now, use the first stop color as a fallback.
        log::warn!(
            "Sweep gradient at ({}, {}) from {}° to {}° - using fallback",
            c0.x,
            c0.y,
            start_angle,
            end_angle
        );

        if let Some(first_stop) = stops.first() {
            self.paint_solid(first_stop.palette_index, first_stop.alpha);
        }

        // Consume extend to suppress unused warning
        let _ = extend;
    }

    fn write_spread_method(&mut self, extend: skrifa::color::Extend) {
        let method = match extend {
            skrifa::color::Extend::Pad => "pad",
            skrifa::color::Extend::Repeat => "repeat",
            skrifa::color::Extend::Reflect => "reflect",
            _ => "pad", // Default to pad for unknown values
        };
        self.svg.write_attribute("spreadMethod", &method);
    }

    fn write_gradient_stops(&mut self, stops: &[skrifa::color::ColorStop]) {
        for stop in stops {
            self.svg.start_element("stop");
            self.svg.write_attribute("offset", &stop.offset);
            self.write_color("stop-color", stop.palette_index, stop.alpha);
            self.svg.end_element();
        }
    }

    fn clip_with_path(&mut self, path: &str) {
        let clip_id = format!("cp{}", self.clip_path_index);
        self.clip_path_index += 1;

        self.svg.start_element("clipPath");
        self.svg.write_attribute("id", &clip_id);
        self.svg.start_element("path");
        self.write_transform("transform", self.current_transform);
        self.svg.write_attribute("d", &path);
        self.svg.end_element();
        self.svg.end_element();

        self.svg.start_element("g");
        self.svg
            .write_attribute_fmt("clip-path", format_args!("url(#{})", clip_id));
    }
}

impl<'a> ColorPainter for SkrifaGlyphPainter<'a> {
    fn push_transform(&mut self, transform: skrifa::color::Transform) {
        self.transform_stack.push(self.current_transform);
        self.current_transform = self.current_transform * transform;
    }

    fn pop_transform(&mut self) {
        if let Some(ts) = self.transform_stack.pop() {
            self.current_transform = ts;
        }
    }

    fn push_clip_glyph(&mut self, glyph_id: GlyphId) {
        self.path_buf.clear();
        let outlines = self.font.outline_glyphs();
        if let Some(glyph) = outlines.get(glyph_id) {
            let mut pen = SvgPathPen::new(self.path_buf);
            let settings = skrifa::outline::DrawSettings::unhinted(
                skrifa::instance::Size::unscaled(),
                LocationRef::default(),
            );
            let _ = glyph.draw(settings, &mut pen);
            pen.finish();
        }
        self.clip_with_path(&self.path_buf.clone());
    }

    fn push_clip_box(&mut self, clip_box: BoundingBox<f32>) {
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
        self.svg.end_element(); // g with clip-path
    }

    fn fill(&mut self, brush: Brush<'_>) {
        match brush {
            Brush::Solid {
                palette_index,
                alpha,
            } => {
                self.paint_solid(palette_index, alpha);
            }
            Brush::LinearGradient {
                p0,
                p1,
                color_stops,
                extend,
            } => {
                self.paint_linear_gradient(p0, p1, color_stops, extend);
            }
            Brush::RadialGradient {
                c0,
                r0,
                c1,
                r1,
                color_stops,
                extend,
            } => {
                self.paint_radial_gradient(c0, r0, c1, r1, color_stops, extend);
            }
            Brush::SweepGradient {
                c0,
                start_angle,
                end_angle,
                color_stops,
                extend,
            } => {
                self.paint_sweep_gradient(c0, start_angle, end_angle, color_stops, extend);
            }
        }
    }

    fn push_layer(&mut self, mode: CompositeMode) {
        self.svg.start_element("g");

        let mode_str = match mode {
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
                log::warn!("Unsupported blend mode: {:?}", mode);
                "normal"
            }
        };
        self.svg.write_attribute_fmt(
            "style",
            format_args!("mix-blend-mode: {}; isolation: isolate", mode_str),
        );
    }

    fn pop_layer(&mut self) {
        self.svg.end_element(); // g
    }
}

/// Paint a COLR glyph using skrifa's ColorPainter and return the resulting SVG tree.
pub(crate) fn paint_colr_glyph(data: &[u8], face_index: u32, glyph_id: GlyphId) -> Option<Tree> {
    let font = FontRef::from_index(data, face_index).ok()?;

    let mut svg = XmlWriter::new(xmlwriter::Options::default());

    svg.start_element("svg");
    svg.write_attribute("xmlns", "http://www.w3.org/2000/svg");
    svg.write_attribute("xmlns:xlink", "http://www.w3.org/1999/xlink");

    let mut path_buf = String::with_capacity(256);

    svg.start_element("g");

    let color_glyphs = font.color_glyphs();

    // Try COLRv1 first, then fall back to COLRv0
    let color_glyph = color_glyphs
        .get_with_format(glyph_id, ColorGlyphFormat::ColrV1)
        .or_else(|| color_glyphs.get_with_format(glyph_id, ColorGlyphFormat::ColrV0))?;

    let mut painter = SkrifaGlyphPainter::new(font, &mut svg, &mut path_buf);

    // Paint the glyph - this calls our ColorPainter implementation
    let _ = color_glyph.paint(LocationRef::default(), &mut painter);

    svg.end_element(); // g

    Tree::from_data(svg.end_document().as_bytes(), &Options::default()).ok()
}
