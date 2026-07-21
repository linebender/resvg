// Copyright 2024 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::text::flatten::PathBuilder;
use skrifa::color::{
    Brush, ColorPainter, ColorPalettes, ColorStop, CompositeMode, Extend, Transform,
};
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{DrawSettings, OutlineGlyphCollection};
use skrifa::raw::types::BoundingBox;
use skrifa::{FontRef, GlyphId, MetadataProvider};
use std::fmt::Write;
use tiny_skia_path::{NonZeroRect, PathSegment};

/// Serializes a path into an SVG path (`d` attribute) string.
fn path_to_svg(path: &tiny_skia_path::Path) -> String {
    let mut d = String::new();
    for segment in path.segments() {
        match segment {
            PathSegment::MoveTo(p) => write!(d, "M {} {} ", p.x, p.y),
            PathSegment::LineTo(p) => write!(d, "L {} {} ", p.x, p.y),
            PathSegment::QuadTo(p1, p) => write!(d, "Q {} {} {} {} ", p1.x, p1.y, p.x, p.y),
            PathSegment::CubicTo(p1, p2, p) => {
                write!(d, "C {} {} {} {} {} {} ", p1.x, p1.y, p2.x, p2.y, p.x, p.y)
            }
            PathSegment::Close => write!(d, "Z "),
        }
        .unwrap();
    }
    if d.ends_with(' ') {
        d.pop();
    }
    d
}

#[derive(Clone, Copy)]
struct Color {
    red: u8,
    green: u8,
    blue: u8,
    /// Final opacity in the `0.0..=1.0` range:
    /// Folds the palette entry's alpha with brush or gradient-stop alpha.
    opacity: f32,
}

trait XmlWriterExt {
    fn write_color_attribute(&mut self, name: &str, color: Color);
    fn write_transform_attribute(&mut self, name: &str, ts: Transform);
    fn write_spread_method_attribute(&mut self, extend: Extend);
}

impl XmlWriterExt for xmlwriter::XmlWriter {
    fn write_color_attribute(&mut self, name: &str, color: Color) {
        self.write_attribute_fmt(
            name,
            format_args!("rgb({}, {}, {})", color.red, color.green, color.blue),
        );
    }

    fn write_transform_attribute(&mut self, name: &str, ts: Transform) {
        if ts.xx == 1.0
            && ts.yx == 0.0
            && ts.xy == 0.0
            && ts.yy == 1.0
            && ts.dx == 0.0
            && ts.dy == 0.0
        {
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
                Extend::Pad => &"pad",
                Extend::Repeat => &"repeat",
                Extend::Reflect => &"reflect",
                _ => &"pad",
            },
        );
    }
}

// NOTE: This is only a best-effort translation of COLR into SVG.
pub(crate) struct GlyphPainter<'a> {
    pub(crate) svg: &'a mut xmlwriter::XmlWriter,
    pub(crate) gradient_index: usize,
    pub(crate) clip_path_index: usize,
    pub(crate) clip_stack: Vec<Option<NonZeroRect>>,
    pub(crate) outlines: OutlineGlyphCollection<'a>,
    pub(crate) palettes: ColorPalettes<'a>,
}

impl<'a> GlyphPainter<'a> {
    pub(crate) fn new(font: &'a FontRef<'a>, svg: &'a mut xmlwriter::XmlWriter) -> Self {
        GlyphPainter {
            outlines: font.outline_glyphs(),
            palettes: font.color_palettes(),
            svg,
            gradient_index: 1,
            clip_path_index: 1,
            clip_stack: Vec::new(),
        }
    }

    /// Resolves a `CPAL` palette index (and additional alpha) into a [`Color`].
    fn resolve_color(&self, palette_index: u16, alpha: f32) -> Color {
        // 0xFFFF means "use the current foreground/text color", which is black
        // for our purposes.
        if palette_index != 0xFFFF {
            if let Some(palette) = self.palettes.get(0) {
                if let Some(c) = palette.colors().get(palette_index as usize) {
                    return Color {
                        red: c.red,
                        green: c.green,
                        blue: c.blue,
                        opacity: (c.alpha as f32 / 255.0) * alpha,
                    };
                }
            }
        }

        Color {
            red: 0,
            green: 0,
            blue: 0,
            opacity: alpha,
        }
    }

    fn outline_to_path(&self, glyph_id: GlyphId) -> (String, Option<NonZeroRect>) {
        let Some(glyph) = self.outlines.get(glyph_id) else {
            return (String::new(), None);
        };

        let mut builder = PathBuilder::new();
        if glyph
            .draw(
                DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
                &mut builder,
            )
            .is_err()
        {
            return (String::new(), None);
        }

        let Some(path) = builder.finish() else {
            return (String::new(), None);
        };

        (path_to_svg(&path), path.bounds().to_non_zero_rect())
    }

    fn write_gradient_stops(&mut self, stops: &[ColorStop]) {
        for stop in stops {
            let color = self.resolve_color(stop.palette_index, stop.alpha);
            self.svg.start_element("stop");
            self.svg.write_attribute("offset", &stop.offset);
            self.svg.write_color_attribute("stop-color", color);
            self.svg.write_attribute("stop-opacity", &color.opacity);
            self.svg.end_element();
        }
    }

    fn clip_with_path(&mut self, path: &str, bounds: Option<NonZeroRect>) {
        let clip_id = format!("cp{}", self.clip_path_index);
        self.clip_path_index += 1;

        self.svg.start_element("clipPath");
        self.svg.write_attribute("id", &clip_id);
        self.svg.start_element("path");
        self.svg.write_attribute("d", &path);
        self.svg.end_element();
        self.svg.end_element();

        self.svg.start_element("g");
        self.svg
            .write_attribute_fmt("clip-path", format_args!("url(#{})", clip_id));
        self.clip_stack.push(bounds);
    }

    fn fill_rect_with_gradient(&mut self, gradient_id: &str, rect: &str) {
        self.svg.start_element("path");
        self.svg
            .write_attribute_fmt("fill", format_args!("url(#{})", gradient_id));
        self.svg.write_attribute("d", &rect);
        self.svg.end_element();
    }
}

impl ColorPainter for GlyphPainter<'_> {
    fn push_transform(&mut self, transform: Transform) {
        self.svg.start_element("g");
        self.svg.write_transform_attribute("transform", transform);
    }

    fn pop_transform(&mut self) {
        self.svg.end_element();
    }

    fn push_clip_glyph(&mut self, glyph_id: GlyphId) {
        let (path, bounds) = self.outline_to_path(glyph_id);
        self.clip_with_path(&path, bounds);
    }

    fn push_clip_box(&mut self, clip_box: BoundingBox<f32>) {
        let clip_path = format!(
            "M {} {} L {} {} L {} {} L {} {} Z",
            clip_box.x_min,
            clip_box.y_min,
            clip_box.x_max,
            clip_box.y_min,
            clip_box.x_max,
            clip_box.y_max,
            clip_box.x_min,
            clip_box.y_max
        );

        self.clip_with_path(
            &clip_path,
            NonZeroRect::from_ltrb(
                clip_box.x_min,
                clip_box.y_min,
                clip_box.x_max,
                clip_box.y_max,
            ),
        );
    }

    fn pop_clip(&mut self) {
        self.svg.end_element();
        self.clip_stack.pop();
    }

    fn fill(&mut self, brush: Brush<'_>) {
        // `fill` paints the current clip region. We paint a rectangle covering
        // the innermost clip's bounds and let the enclosing `clip-path` shape it
        // (the previous painter filled the glyph outline directly). Without an
        // active clip there is nothing to fill.
        //
        // TODO: The rectangle is sized in the clip's coordinate space, but it is
        // emitted inside any brush/paint transform pushed between the clip and
        // this fill (`push_transform`). A large translation or a small scale in
        // that transform can move the rectangle off the clip, leaving the glyph
        // partly or entirely unpainted. This does not occur for gradients
        // authored in glyph space (the common case), but can for synthetic
        // COLRv1 fonts with extreme `PaintTransform`s. To fix, track the
        // accumulated transform and size this rectangle in the current space by
        // mapping the clip's root-space bounds back through its inverse (or
        // apply the transform only to the paint server, not the fill geometry).
        let Some(bounds) = self.clip_stack.last().copied().flatten() else {
            return;
        };
        let margin = bounds.width().max(bounds.height());
        let x0 = bounds.left() - margin;
        let y0 = bounds.top() - margin;
        let x1 = bounds.right() + margin;
        let y1 = bounds.bottom() + margin;
        let rect = format!(
            "M {} {} L {} {} L {} {} L {} {} Z",
            x0, y0, x1, y0, x1, y1, x0, y1
        );

        match brush {
            Brush::Solid {
                palette_index,
                alpha,
            } => {
                let color = self.resolve_color(palette_index, alpha);
                self.svg.start_element("path");
                self.svg.write_color_attribute("fill", color);
                self.svg.write_attribute("fill-opacity", &color.opacity);
                self.svg.write_attribute("d", &rect);
                self.svg.end_element();
            }
            Brush::LinearGradient {
                p0,
                p1,
                color_stops,
                extend,
            } => {
                let gradient_id = format!("lg{}", self.gradient_index);
                self.gradient_index += 1;

                self.svg.start_element("linearGradient");
                self.svg.write_attribute("id", &gradient_id);
                self.svg.write_attribute("x1", &p0.x);
                self.svg.write_attribute("y1", &p0.y);
                self.svg.write_attribute("x2", &p1.x);
                self.svg.write_attribute("y2", &p1.y);
                self.svg.write_attribute("gradientUnits", &"userSpaceOnUse");
                self.svg.write_spread_method_attribute(extend);
                self.write_gradient_stops(color_stops);
                self.svg.end_element();

                self.fill_rect_with_gradient(&gradient_id, &rect);
            }
            Brush::RadialGradient {
                c0,
                r0,
                c1,
                r1,
                color_stops,
                extend,
            } => {
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
                self.svg.write_spread_method_attribute(extend);
                self.write_gradient_stops(color_stops);
                self.svg.end_element();

                self.fill_rect_with_gradient(&gradient_id, &rect);
            }
            Brush::SweepGradient { .. } => {
                println!("Warning: sweep gradients are not supported.");
            }
        }
    }

    fn push_layer(&mut self, mode: CompositeMode) {
        self.svg.start_element("g");

        // TODO: Need to figure out how to represent the other blend modes
        // in SVG.
        let mode = match mode {
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
                println!("Warning: unsupported blend mode: {:?}", mode);
                "normal"
            }
        };
        self.svg.write_attribute_fmt(
            "style",
            format_args!("mix-blend-mode: {}; isolation: isolate", mode),
        );
    }

    fn pop_layer(&mut self) {
        self.svg.end_element(); // g
    }
}
