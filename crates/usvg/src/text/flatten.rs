// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use fontdb::{Database, ID};
use rustybuzz::ttf_parser;
use rustybuzz::ttf_parser::{GlyphId, RasterImageFormat};
use tiny_skia_path::{NonZeroRect, Size, Transform};

use std::mem;
use std::sync::Arc;

use crate::layout::Span;
use crate::{
    Color, Fill, FillRule, Group, Image, ImageKind, ImageRendering, Node, Opacity, Options, Paint,
    PaintOrder, Path, ShapeRendering, Text, TextRendering, Tree, Visibility,
};

fn resolve_rendering_mode(text: &Text) -> ShapeRendering {
    match text.rendering_mode {
        TextRendering::OptimizeSpeed => ShapeRendering::CrispEdges,
        TextRendering::OptimizeLegibility => ShapeRendering::GeometricPrecision,
        TextRendering::GeometricPrecision => ShapeRendering::GeometricPrecision,
    }
}

fn push_outline_paths(
    span: &Span,
    builder: &mut tiny_skia_path::PathBuilder,
    new_children: &mut Vec<Node>,
    rendering_mode: ShapeRendering,
) {
    let builder = mem::replace(builder, tiny_skia_path::PathBuilder::new());

    if let Some(path) = builder.finish().and_then(|p| {
        Path::new(
            String::new(),
            span.visibility,
            span.fill.clone(),
            span.stroke.clone(),
            span.paint_order,
            rendering_mode,
            Arc::new(p),
            Transform::default(),
        )
    }) {
        new_children.push(Node::Path(Box::new(path)));
    }
}

pub(crate) fn flatten(text: &mut Text, fontdb: &fontdb::Database) -> Option<(Group, NonZeroRect)> {
    let mut new_children = vec![];

    let rendering_mode = resolve_rendering_mode(text);

    for span in &text.layouted {
        if let Some(path) = span.overline.as_ref() {
            let mut path = path.clone();
            path.rendering_mode = rendering_mode;
            new_children.push(Node::Path(Box::new(path)));
        }

        if let Some(path) = span.underline.as_ref() {
            let mut path = path.clone();
            path.rendering_mode = rendering_mode;
            new_children.push(Node::Path(Box::new(path)));
        }

        // Instead of always processing each glyph separately, we always collect
        // as many outline glyphs as possible by pushing them into the span_builder
        // and only if we encounter a different glyph, or we reach the very end of the
        // span to we push the actual outline paths into new_children. This way, we don't need
        // to create a new path for every glyph if we have many consecutive glyphs
        // with just outlines (which is the most common case).
        let mut span_builder = tiny_skia_path::PathBuilder::new();

        for glyph in &span.positioned_glyphs {
            // A COLRv0 glyph. Will return a vector of paths that make up the glyph description.
            // TODO: Don't use black for foreground color? But not sure whether to use fill or stroke
            // color.
            if let Some(layers) = fontdb.colr(glyph.font, glyph.id, Color::black()) {
                push_outline_paths(span, &mut span_builder, &mut new_children, rendering_mode);

                let mut group = Group {
                    transform: glyph.colr_transform(),
                    ..Group::empty()
                };

                for path in layers {
                    // TODO: Probably need to update abs_transform of children?
                    group.children.push(Node::Path(Box::new(path)));
                }
                group.calculate_bounding_boxes();

                new_children.push(Node::Group(Box::new(group)));
            }
            // An SVG glyph. Will return the usvg tree containing the glyph descriptions.
            else if let Some(tree) = fontdb.svg(glyph.font, glyph.id) {
                push_outline_paths(span, &mut span_builder, &mut new_children, rendering_mode);

                let mut group = Group {
                    transform: glyph.svg_transform(),
                    ..Group::empty()
                };
                // TODO: Probably need to update abs_transform of children?
                group.children.push(Node::Group(Box::new(tree.root)));
                group.calculate_bounding_boxes();

                new_children.push(Node::Group(Box::new(group)));
            }
            // A bitmap glyph.
            else if let Some(img) = fontdb.raster(glyph.font, glyph.id) {
                push_outline_paths(span, &mut span_builder, &mut new_children, rendering_mode);

                let transform = if img.is_sbix {
                    glyph.sbix_transform(
                        img.x as f32,
                        img.y as f32,
                        img.glyph_bbox.map(|bbox| bbox.x_min).unwrap_or(0) as f32,
                        img.glyph_bbox.map(|bbox| bbox.y_min).unwrap_or(0) as f32,
                        img.pixels_per_em as f32,
                        img.image.size.height(),
                    )
                } else {
                    glyph.cbdt_transform(
                        img.x as f32,
                        img.y as f32,
                        img.pixels_per_em as f32,
                        img.image.size.height(),
                    )
                };

                let mut group = Group {
                    transform,
                    ..Group::empty()
                };
                group.children.push(Node::Image(Box::new(img.image)));
                group.calculate_bounding_boxes();

                new_children.push(Node::Group(Box::new(group)));
            } else if let Some(outline) = fontdb
                .outline(glyph.font, glyph.id)
                .and_then(|p| p.transform(glyph.outline_transform()))
            {
                span_builder.push_path(&outline);
            }
        }

        push_outline_paths(span, &mut span_builder, &mut new_children, rendering_mode);

        if let Some(path) = span.line_through.as_ref() {
            let mut path = path.clone();
            path.rendering_mode = rendering_mode;
            new_children.push(Node::Path(Box::new(path)));
        }
    }

    let mut group = Group {
        id: text.id.clone(),
        ..Group::empty()
    };

    for child in new_children {
        group.children.push(child);
    }

    group.calculate_bounding_boxes();
    let stroke_bbox = group.stroke_bounding_box().to_non_zero_rect()?;
    Some((group, stroke_bbox))
}

struct PathBuilder {
    builder: tiny_skia_path::PathBuilder,
}

impl ttf_parser::OutlineBuilder for PathBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.builder.move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.builder.line_to(x, y);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.builder.quad_to(x1, y1, x, y);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.builder.cubic_to(x1, y1, x2, y2, x, y);
    }

    fn close(&mut self) {
        self.builder.close();
    }
}

pub(crate) trait DatabaseExt {
    fn outline(&self, id: ID, glyph_id: GlyphId) -> Option<tiny_skia_path::Path>;
    fn raster(&self, id: ID, glyph_id: GlyphId) -> Option<BitmapImage>;
    fn svg(&self, id: ID, glyph_id: GlyphId) -> Option<Tree>;
    fn colr(&self, id: ID, glyph_id: GlyphId, text_color: Color) -> Option<Vec<Path>>;
}

pub(crate) struct BitmapImage {
    image: Image,
    x: i16,
    y: i16,
    pixels_per_em: u16,
    glyph_bbox: Option<ttf_parser::Rect>,
    is_sbix: bool,
}

impl DatabaseExt for Database {
    #[inline(never)]
    fn outline(&self, id: ID, glyph_id: GlyphId) -> Option<tiny_skia_path::Path> {
        self.with_face_data(id, |data, face_index| -> Option<tiny_skia_path::Path> {
            let font = ttf_parser::Face::parse(data, face_index).ok()?;

            let mut builder = PathBuilder {
                builder: tiny_skia_path::PathBuilder::new(),
            };

            font.outline_glyph(glyph_id, &mut builder)?;
            builder.builder.finish()
        })?
    }

    fn raster(&self, id: ID, glyph_id: GlyphId) -> Option<BitmapImage> {
        self.with_face_data(id, |data, face_index| -> Option<BitmapImage> {
            let font = ttf_parser::Face::parse(data, face_index).ok()?;
            let image = font.glyph_raster_image(glyph_id, u16::MAX)?;

            if image.format == RasterImageFormat::PNG {
                let bitmap_image = BitmapImage {
                    image: Image {
                        id: String::new(),
                        visibility: Visibility::Visible,
                        size: Size::from_wh(image.width as f32, image.height as f32)?,
                        rendering_mode: ImageRendering::OptimizeQuality,
                        kind: ImageKind::PNG(Arc::new(image.data.into())),
                        abs_transform: Transform::default(),
                        // TODO: What to change to?
                        abs_bounding_box: NonZeroRect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap(),
                    },
                    x: image.x,
                    y: image.y,
                    pixels_per_em: image.pixels_per_em,
                    glyph_bbox: font.glyph_bounding_box(glyph_id),
                    // ttf-parser always checks sbix first, so if this table exists, it was used.
                    is_sbix: font.tables().sbix.is_some(),
                };

                return Some(bitmap_image);
            }

            None
        })?
    }

    fn svg(&self, id: ID, glyph_id: GlyphId) -> Option<Tree> {
        // TODO: Technically not 100% accurate because the SVG format in a OTF font
        // is actually a subset/superset of a normal SVG, but it seems to work fine
        // for Twitter Color Emoji, so might as well use what we already have.
        self.with_face_data(id, |data, face_index| -> Option<Tree> {
            let font = ttf_parser::Face::parse(data, face_index).ok()?;
            let image = font.glyph_svg_image(glyph_id)?;
            Tree::from_data(image.data, &Options::default(), &fontdb::Database::new()).ok()
        })?
    }

    fn colr(&self, id: ID, glyph_id: GlyphId, text_color: Color) -> Option<Vec<Path>> {
        self.with_face_data(id, |data, face_index| -> Option<Vec<Path>> {
            let font = ttf_parser::Face::parse(data, face_index).ok()?;

            let mut paths = vec![];
            let mut glyph_painter = GlyphPainter {
                face: &font,
                paths: &mut paths,
                builder: PathBuilder {
                    builder: tiny_skia_path::PathBuilder::new(),
                },
                foreground: text_color,
            };

            font.paint_color_glyph(glyph_id, 0, &mut glyph_painter)?;

            Some(paths)
        })?
    }
}

struct GlyphPainter<'a> {
    face: &'a ttf_parser::Face<'a>,
    paths: &'a mut Vec<Path>,
    builder: PathBuilder,
    foreground: Color,
}

impl ttf_parser::colr::Painter for GlyphPainter<'_> {
    fn outline(&mut self, glyph_id: ttf_parser::GlyphId) {
        let builder = &mut self.builder;
        match self.face.outline_glyph(glyph_id, builder) {
            Some(v) => v,
            None => return,
        };
    }

    fn paint_foreground(&mut self) {
        self.paint_color(ttf_parser::RgbaColor::new(
            self.foreground.red,
            self.foreground.green,
            self.foreground.blue,
            255,
        ));
    }

    fn paint_color(&mut self, color: ttf_parser::RgbaColor) {
        let builder = mem::replace(
            &mut self.builder,
            PathBuilder {
                builder: tiny_skia_path::PathBuilder::new(),
            },
        );

        if let Some(path) = builder.builder.finish().and_then(|p| {
            let fill = Fill {
                paint: Paint::Color(Color::new_rgb(color.red, color.green, color.blue)),
                opacity: Opacity::new(f32::from(color.alpha) / 255.0).unwrap(),
                rule: FillRule::NonZero,
                context_element: None,
            };

            Path::new(
                String::new(),
                Visibility::Visible,
                Some(fill),
                None,
                PaintOrder::FillAndStroke,
                ShapeRendering::GeometricPrecision,
                Arc::new(p),
                Transform::default(),
            )
        }) {
            self.paths.push(path)
        }
    }
}
