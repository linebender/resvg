// Copyright 2022 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::mem;
use std::sync::Arc;

use fontdb::{Database, ID};
use harfrust::Tag;
use skrifa::{
    FontRef, GlyphId, MetadataProvider,
    bitmap::{BitmapData, MaskData},
    instance::{LocationRef, Size as SkrifaSize},
    outline::{
        DrawSettings, Engine, HintingInstance, HintingOptions, OutlinePen, SmoothMode, Target,
        pen::ControlBoundsPen,
    },
    raw::TableProvider,
    setting::VariationSetting,
};
use tiny_skia_path::{NonZeroRect, Size, Transform};

use crate::*;

/// Encode raw image data as PNG.
fn encode_png(data: &[u8], width: u32, height: u32, color_type: png::ColorType) -> Option<Vec<u8>> {
    let mut png_data = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png_data, width, height);
        encoder.set_color(color_type);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().ok()?;
        writer.write_image_data(data).ok()?;
    }
    Some(png_data)
}

/// Extract a single pixel from bitmap mask data.
/// Returns grayscale value (0x00 = black, 0xFF = white).
fn extract_mask_pixel(data: &[u8], x: u32, y: u32, width: u32, bpp: u8, is_packed: bool) -> u8 {
    match bpp {
        1 => {
            let (byte_idx, bit_idx) = if is_packed {
                // Packed: bits flow continuously across row boundaries
                let bit_pos = (y as usize) * (width as usize) + (x as usize);
                (bit_pos / 8, 7 - (bit_pos % 8))
            } else {
                // Not packed: each row is byte-aligned
                let row_bytes = ((width + 7) / 8) as usize;
                let idx = (y as usize) * row_bytes + (x as usize) / 8;
                (idx, 7 - ((x as usize) % 8))
            };
            if byte_idx < data.len() {
                let bit = (data[byte_idx] >> bit_idx) & 1;
                // 1 = black (0x00), 0 = white (0xFF)
                if bit == 1 { 0x00 } else { 0xFF }
            } else {
                0xFF
            }
        }
        2 => {
            // 2 bpp: each byte contains 4 pixels
            let row_bytes = ((width + 3) / 4) as usize;
            let byte_idx = (y as usize) * row_bytes + (x as usize) / 4;
            let shift = 6 - (((x as usize) % 4) * 2);
            if byte_idx < data.len() {
                let val = (data[byte_idx] >> shift) & 0x03;
                // Scale 0-3 to 0-255 (inverted: 3 = black)
                255 - (val * 85)
            } else {
                0xFF
            }
        }
        4 => {
            // 4 bpp: each byte contains 2 pixels
            let row_bytes = ((width + 1) / 2) as usize;
            let byte_idx = (y as usize) * row_bytes + (x as usize) / 2;
            let shift = if x % 2 == 0 { 4 } else { 0 };
            if byte_idx < data.len() {
                let val = (data[byte_idx] >> shift) & 0x0F;
                // Scale 0-15 to 0-255 (inverted: 15 = black)
                255 - (val * 17)
            } else {
                0xFF
            }
        }
        8 => {
            // 8 bpp: one byte per pixel
            let idx = (y as usize) * (width as usize) + (x as usize);
            if idx < data.len() {
                // Invert: 255 = black, 0 = white
                255 - data[idx]
            } else {
                0xFF
            }
        }
        _ => 0xFF,
    }
}

/// Convert a monochrome/grayscale bitmap mask to PNG format.
/// Supports 1, 2, 4, and 8 bits per pixel.
fn mask_to_png(mask: &MaskData, width: u32, height: u32) -> Option<Vec<u8>> {
    if width == 0 || height == 0 {
        return None;
    }

    // Check for overflow: width * height must fit in usize
    let capacity = (width as usize).checked_mul(height as usize)?;

    // Decode mask data to 8-bit grayscale
    let mut grayscale = Vec::with_capacity(capacity);

    for y in 0..height {
        for x in 0..width {
            grayscale.push(extract_mask_pixel(
                mask.data,
                x,
                y,
                width,
                mask.bpp,
                mask.is_packed,
            ));
        }
    }

    encode_png(&grayscale, width, height, png::ColorType::Grayscale)
}

/// Convert BGRA bitmap data to PNG format.
fn bgra_to_png(data: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    if width == 0 || height == 0 {
        return None;
    }

    // Check for overflow: width * height * 4 must fit in usize
    let expected_len = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;
    if data.len() < expected_len {
        return None;
    }

    // Convert BGRA to RGBA
    let mut rgba = Vec::with_capacity(expected_len);
    for chunk in data[..expected_len].chunks(4) {
        rgba.push(chunk[2]); // R
        rgba.push(chunk[1]); // G
        rgba.push(chunk[0]); // B
        rgba.push(chunk[3]); // A
    }

    encode_png(&rgba, width, height, png::ColorType::Rgba)
}

fn resolve_rendering_mode(text: &Text) -> ShapeRendering {
    match text.rendering_mode {
        TextRendering::OptimizeSpeed => ShapeRendering::CrispEdges,
        TextRendering::OptimizeLegibility => ShapeRendering::GeometricPrecision,
        TextRendering::GeometricPrecision => ShapeRendering::GeometricPrecision,
    }
}

fn push_outline_paths(
    span: &layout::Span,
    builder: &mut tiny_skia_path::PathBuilder,
    new_children: &mut Vec<Node>,
    rendering_mode: ShapeRendering,
) {
    let builder = mem::replace(builder, tiny_skia_path::PathBuilder::new());

    if let Some(path) = builder.finish().and_then(|p| {
        Path::new(
            String::new(),
            span.visible,
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

/// Hinting context for controlling font hinting behavior.
///
/// Note: Hinting is applied at the SVG's source coordinate scale, not the final
/// output scale. For pixel-perfect mono rendering, the output should be rendered
/// at 1:1 scale (no zoom/fit), or use a zoom factor that's an integer multiple
/// (2x, 3x, etc.) to maintain pixel alignment.
#[derive(Clone, Copy, Debug)]
pub(crate) struct HintingContext {
    /// Whether hinting is enabled globally.
    pub(crate) enabled: bool,
}

impl HintingContext {
    /// Calculate pixels per em from font size.
    ///
    /// In SVG, font-size is specified in user units (pixels), not points.
    /// So ppem equals font_size directly when rendering at 1:1 scale.
    pub(crate) fn ppem(&self, font_size: f32) -> f32 {
        font_size
    }
}

/// Convert positioned glyphs to path outlines.
pub(crate) fn flatten(
    text: &mut Text,
    cache: &mut Cache,
    hinting_ctx: Option<HintingContext>,
) -> Option<(Group, NonZeroRect)> {
    flatten_impl(text, cache, hinting_ctx)
}

fn flatten_impl(
    text: &mut Text,
    cache: &mut Cache,
    hinting_ctx: Option<HintingContext>,
) -> Option<(Group, NonZeroRect)> {
    let mut new_children = vec![];

    let rendering_mode = resolve_rendering_mode(text);
    let hinting_mode = HintingMode::from_text_rendering(text.rendering_mode);

    // Determine if we should use hinting
    let use_hinting = hinting_ctx
        .map(|ctx| ctx.enabled && hinting_mode == HintingMode::Full)
        .unwrap_or(false);

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
        //
        // We also track rendering mode per-glyph: if a glyph uses Mono hinting target,
        // it needs CrispEdges rendering (no anti-aliasing), so we flush the builder
        // when the rendering mode changes.
        let mut span_builder = tiny_skia_path::PathBuilder::new();

        // Determine rendering mode for this span based on hinting target.
        // Mono target always disables anti-aliasing (CrispEdges), regardless of whether
        // hinting is enabled. This allows comparing hinted vs unhinted mono rendering.
        let span_rendering_mode = if span.hinting.target == crate::HintingTarget::Mono {
            ShapeRendering::CrispEdges // No anti-aliasing for mono target
        } else {
            rendering_mode
        };
        let mut current_glyph_rendering_mode = span_rendering_mode;

        // Check if we need variations for this span (uniform for all glyphs).
        // For variable fonts, we need to extract the outline with variations applied.
        // We can't use the cache here since the outline depends on variation values.
        let needs_variations = !span.variations.is_empty()
            || span.font_optical_sizing == crate::FontOpticalSizing::Auto;

        for glyph in &span.positioned_glyphs {
            // For mono hinting, all glyphs in the span use the same rendering mode
            let glyph_rendering_mode = span_rendering_mode;

            // If rendering mode changed, flush the current path segment
            if glyph_rendering_mode != current_glyph_rendering_mode {
                push_outline_paths(
                    span,
                    &mut span_builder,
                    &mut new_children,
                    current_glyph_rendering_mode,
                );
                current_glyph_rendering_mode = glyph_rendering_mode;
            }
            // A (best-effort conversion of a) COLR glyph.
            if let Some(tree) = cache.fontdb_colr(glyph.font, glyph.id) {
                let mut group = Group {
                    transform: glyph.colr_transform(),
                    ..Group::empty()
                };
                // TODO: Probably need to update abs_transform of children?
                group.children.push(Node::Group(Box::new(tree.root)));
                group.calculate_bounding_boxes();

                new_children.push(Node::Group(Box::new(group)));
            }
            // An SVG glyph. Will return the usvg node containing the glyph descriptions.
            else if let Some(node) = cache.fontdb_svg(glyph.font, glyph.id) {
                push_outline_paths(span, &mut span_builder, &mut new_children, rendering_mode);

                let mut group = Group {
                    transform: glyph.svg_transform(),
                    ..Group::empty()
                };
                // TODO: Probably need to update abs_transform of children?
                group.children.push(node);
                group.calculate_bounding_boxes();

                new_children.push(Node::Group(Box::new(group)));
            }
            // A bitmap glyph.
            else if let Some(img) = cache.fontdb_raster(glyph.font, glyph.id, glyph.font_size()) {
                push_outline_paths(span, &mut span_builder, &mut new_children, rendering_mode);

                let transform = if img.is_sbix {
                    glyph.sbix_transform(
                        img.x,
                        img.y,
                        img.glyph_bbox.map(|bbox| bbox.x_min as f32).unwrap_or(0.0),
                        img.glyph_bbox.map(|bbox| bbox.y_min as f32).unwrap_or(0.0),
                        img.pixels_per_em,
                        img.image.size.height(),
                    )
                } else {
                    glyph.cbdt_transform(img.x, img.y, img.pixels_per_em)
                };

                let mut group = Group {
                    transform,
                    ..Group::empty()
                };
                group.children.push(Node::Image(Box::new(img.image)));
                group.calculate_bounding_boxes();

                new_children.push(Node::Group(Box::new(group)));
            } else {
                // Use span-level variation settings (uniform for all glyphs in span).
                // Clone Arc<Database> before mutable borrow for the closure
                let fontdb = cache.fontdb.clone();

                // Compute ppem for hinting (if applicable)
                let ppem = if use_hinting {
                    hinting_ctx.map(|ctx| ctx.ppem(glyph.font_size()))
                } else {
                    None
                };

                // For cache key: when auto-opsz is enabled, assume font has opsz axis
                // This is a safe approximation that may create slightly more cache entries
                // for fonts without opsz, but avoids expensive axis lookup
                let has_opsz_axis = span.font_optical_sizing == crate::FontOpticalSizing::Auto;

                // Compute variation hash for cache key
                let variation_hash = crate::parser::compute_variation_hash(
                    &span.variations,
                    span.font_optical_sizing,
                    glyph.font_size(),
                    has_opsz_axis,
                );

                // Build cache key with all parameters affecting outline shape
                let cache_key = crate::parser::OutlineCacheKey {
                    font_id: glyph.font,
                    glyph_id: glyph.id,
                    ppem_bits: ppem.map(|p| p.to_bits()),
                    hinting_target: if use_hinting {
                        Some(span.hinting.target)
                    } else {
                        None
                    },
                    hinting_mode: if use_hinting {
                        Some(span.hinting.mode)
                    } else {
                        None
                    },
                    hinting_engine: if use_hinting {
                        Some(span.hinting.engine)
                    } else {
                        None
                    },
                    symmetric_rendering: span.hinting.symmetric_rendering,
                    preserve_linear_metrics: span.hinting.preserve_linear_metrics,
                    variation_hash,
                };

                // Capture values for closure
                let variations = &span.variations;
                let font_optical_sizing = span.font_optical_sizing;
                let hinting_settings = span.hinting;
                let font_id = glyph.font;
                let glyph_id = glyph.id;
                let font_size = glyph.font_size();

                // Get from cache or compute (unified for all outline types)
                let outline = cache.get_or_compute_outline(cache_key, || {
                    if use_hinting {
                        extract_outline_skrifa(
                            &fontdb,
                            font_id,
                            glyph_id,
                            variations,
                            font_size,
                            font_optical_sizing,
                            ppem,
                            hinting_settings,
                        )
                    } else if needs_variations {
                        fontdb.outline_with_variations(
                            font_id,
                            glyph_id,
                            variations,
                            font_size,
                            font_optical_sizing,
                        )
                    } else {
                        fontdb.outline(font_id, glyph_id)
                    }
                });

                if let Some(outline) = outline.and_then(|p| p.transform(glyph.outline_transform()))
                {
                    span_builder.push_path(&outline);
                }
            }
        }

        push_outline_paths(
            span,
            &mut span_builder,
            &mut new_children,
            current_glyph_rendering_mode,
        );

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

/// Extract glyph outline using skrifa with optional hinting.
fn extract_outline_skrifa(
    fontdb: &fontdb::Database,
    font_id: fontdb::ID,
    glyph_id: GlyphId,
    variations: &[crate::FontVariation],
    font_size: f32,
    font_optical_sizing: crate::FontOpticalSizing,
    ppem: Option<f32>,
    hinting_settings: crate::HintingSettings,
) -> Option<tiny_skia_path::Path> {
    fontdb.with_face_data(
        font_id,
        |data, face_index| -> Option<tiny_skia_path::Path> {
            let font = FontRef::from_index(data, face_index).ok()?;
            let outlines = font.outline_glyphs();
            let glyph = outlines.get(glyph_id)?;

            // Build variation coordinates if needed, using avar-aware normalization
            let needs_variations =
                !variations.is_empty() || font_optical_sizing == crate::FontOpticalSizing::Auto;

            let location = if needs_variations {
                let axes = font.axes();
                let mut coords: Vec<skrifa::instance::NormalizedCoord> =
                    vec![Default::default(); axes.len()];

                // Build variation settings including auto-opsz
                let mut settings: Vec<VariationSetting> = variations
                    .iter()
                    .map(|v| VariationSetting::new(Tag::new(&v.tag), v.value))
                    .collect();

                // Auto-set opsz if font-optical-sizing is auto and not explicitly set
                if font_optical_sizing == crate::FontOpticalSizing::Auto {
                    let has_explicit_opsz = variations.iter().any(|v| v.tag == *b"opsz");
                    if !has_explicit_opsz {
                        // Check if font has opsz axis
                        let has_opsz_axis = axes.iter().any(|a| a.tag() == Tag::new(b"opsz"));
                        if has_opsz_axis {
                            settings.push(VariationSetting::new(Tag::new(b"opsz"), font_size));
                        }
                    }
                }

                // Use location_to_slice which applies avar (axis variations) table remapping.
                // This differs from ttf-parser's set_variation() which used raw user-space values.
                // Avar remapping transforms user-space axis values to design-space coordinates,
                // which is required for correct variable font rendering (especially for fonts
                // like Roboto Flex that rely heavily on avar for intermediate axis values).
                axes.location_to_slice(&settings, &mut coords);

                Some(coords)
            } else {
                None
            };

            let location_ref = location
                .as_ref()
                .map(|c| LocationRef::new(c))
                .unwrap_or_default();

            // Choose drawing settings based on hinting
            // Hinted output is in pixel units (scaled by ppem), while unhinted is in font units.
            // We scale hinted output back to font units so outline_transform() can apply consistent scaling.
            if let Some(ppem_val) = ppem {
                let size = SkrifaSize::new(ppem_val);

                // Convert HintingSettings to skrifa's Target
                let hinting_target = match hinting_settings.target {
                    crate::HintingTarget::Mono => Target::Mono,
                    crate::HintingTarget::Smooth => {
                        // Convert HintingMode to skrifa's SmoothMode
                        let smooth_mode = match hinting_settings.mode {
                            crate::HintingMode::Normal => SmoothMode::Normal,
                            crate::HintingMode::Light => SmoothMode::Light,
                            crate::HintingMode::Lcd => SmoothMode::Lcd,
                            crate::HintingMode::VerticalLcd => SmoothMode::VerticalLcd,
                        };
                        Target::Smooth {
                            mode: smooth_mode,
                            symmetric_rendering: hinting_settings.symmetric_rendering,
                            preserve_linear_metrics: hinting_settings.preserve_linear_metrics,
                        }
                    }
                };

                // Convert HintingEngine to skrifa's Engine
                let engine = match hinting_settings.engine {
                    crate::HintingEngine::Auto => Engine::Auto(None),
                    crate::HintingEngine::Native => Engine::Interpreter,
                    crate::HintingEngine::AutoFallback => Engine::AutoFallback,
                };

                // Build HintingOptions with both engine and target
                let hinting_options = HintingOptions {
                    engine,
                    target: hinting_target,
                };

                // Create hinting instance with the configured options.
                // Note: HintingInstance is created per-glyph. For performance optimization,
                // consider caching instances keyed by (font_id, ppem, location, settings) if profiling
                // shows this is a bottleneck.
                if let Ok(hinting_instance) =
                    HintingInstance::new(&outlines, size, location_ref, hinting_options)
                {
                    // Use hinted drawing with the hinting instance
                    // Output is in pixel units at ppem scale, so we need to scale back to font units
                    let scale_back = font.head().unwrap().units_per_em() as f32 / ppem_val;
                    let mut pen = ScalingPen::new(scale_back);
                    let settings = DrawSettings::hinted(&hinting_instance, false);
                    glyph.draw(settings, &mut pen).ok()?;
                    return pen.finish();
                }
            }

            // Fallback to unhinted drawing (font units)
            let mut pen = SkrifaPen::new();
            let settings = DrawSettings::unhinted(SkrifaSize::unscaled(), location_ref);
            glyph.draw(settings, &mut pen).ok()?;
            pen.finish()
        },
    )?
}

/// Pen adapter for skrifa's OutlinePen trait -> tiny_skia_path::PathBuilder
struct SkrifaPen {
    builder: tiny_skia_path::PathBuilder,
}

impl SkrifaPen {
    fn new() -> Self {
        Self {
            builder: tiny_skia_path::PathBuilder::new(),
        }
    }

    fn finish(self) -> Option<tiny_skia_path::Path> {
        self.builder.finish()
    }
}

/// Pen that scales coordinates by a factor (used to convert hinted pixel coords back to font units)
struct ScalingPen {
    builder: tiny_skia_path::PathBuilder,
    scale: f32,
}

impl ScalingPen {
    fn new(scale: f32) -> Self {
        Self {
            builder: tiny_skia_path::PathBuilder::new(),
            scale,
        }
    }

    fn finish(self) -> Option<tiny_skia_path::Path> {
        self.builder.finish()
    }
}

impl OutlinePen for ScalingPen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.builder.move_to(x * self.scale, y * self.scale);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.builder.line_to(x * self.scale, y * self.scale);
    }

    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.builder.quad_to(
            cx * self.scale,
            cy * self.scale,
            x * self.scale,
            y * self.scale,
        );
    }

    fn curve_to(&mut self, cx1: f32, cy1: f32, cx2: f32, cy2: f32, x: f32, y: f32) {
        self.builder.cubic_to(
            cx1 * self.scale,
            cy1 * self.scale,
            cx2 * self.scale,
            cy2 * self.scale,
            x * self.scale,
            y * self.scale,
        );
    }

    fn close(&mut self) {
        self.builder.close();
    }
}

impl OutlinePen for SkrifaPen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.builder.move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.builder.line_to(x, y);
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.builder.quad_to(cx0, cy0, x, y);
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.builder.cubic_to(cx0, cy0, cx1, cy1, x, y);
    }

    fn close(&mut self) {
        self.builder.close();
    }
}

/// Hinting mode derived from CSS text-rendering property
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HintingMode {
    /// No hinting (text-rendering: geometricPrecision)
    None,
    /// Full hinting (text-rendering: optimizeLegibility)
    Full,
}

impl HintingMode {
    /// Convert CSS TextRendering to HintingMode
    pub fn from_text_rendering(text_rendering: TextRendering) -> Self {
        match text_rendering {
            TextRendering::OptimizeSpeed => HintingMode::Full,
            TextRendering::OptimizeLegibility => HintingMode::Full,
            TextRendering::GeometricPrecision => HintingMode::None,
        }
    }
}

pub(crate) trait DatabaseExt {
    fn outline(&self, id: ID, glyph_id: GlyphId) -> Option<tiny_skia_path::Path>;
    fn outline_with_variations(
        &self,
        id: ID,
        glyph_id: GlyphId,
        variations: &[crate::FontVariation],
        font_size: f32,
        font_optical_sizing: crate::FontOpticalSizing,
    ) -> Option<tiny_skia_path::Path>;
    fn raster(&self, id: ID, glyph_id: GlyphId, font_size: f32) -> Option<BitmapImage>;
    fn svg(&self, id: ID, glyph_id: GlyphId) -> Option<Node>;
    fn colr(&self, id: ID, glyph_id: GlyphId) -> Option<Tree>;
}

/// Bounding box for a glyph (x_min, y_min, x_max, y_max)
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub(crate) struct GlyphBbox {
    pub x_min: i16,
    pub y_min: i16,
    pub x_max: i16,
    pub y_max: i16,
}

#[derive(Clone)]
pub(crate) struct BitmapImage {
    image: Image,
    x: f32,
    y: f32,
    pixels_per_em: f32,
    glyph_bbox: Option<GlyphBbox>,
    is_sbix: bool,
}

impl DatabaseExt for Database {
    #[inline(never)]
    fn outline(&self, id: ID, glyph_id: GlyphId) -> Option<tiny_skia_path::Path> {
        self.with_face_data(id, |data, face_index| -> Option<tiny_skia_path::Path> {
            let font = FontRef::from_index(data, face_index).ok()?;
            let outlines = font.outline_glyphs();
            let glyph = outlines.get(glyph_id)?;

            let mut pen = SkrifaPen::new();
            let settings = DrawSettings::unhinted(SkrifaSize::unscaled(), LocationRef::default());
            glyph.draw(settings, &mut pen).ok()?;
            pen.finish()
        })?
    }

    #[inline(never)]
    fn outline_with_variations(
        &self,
        id: ID,
        glyph_id: GlyphId,
        variations: &[crate::FontVariation],
        font_size: f32,
        font_optical_sizing: crate::FontOpticalSizing,
    ) -> Option<tiny_skia_path::Path> {
        self.with_face_data(id, |data, face_index| -> Option<tiny_skia_path::Path> {
            let font = FontRef::from_index(data, face_index).ok()?;
            let outlines = font.outline_glyphs();
            let glyph = outlines.get(glyph_id)?;

            // Build variation coordinates using avar-aware normalization
            let axes = font.axes();
            let mut coords: Vec<skrifa::instance::NormalizedCoord> =
                vec![Default::default(); axes.len()];

            // Build variation settings including auto-opsz
            let mut settings: Vec<VariationSetting> = variations
                .iter()
                .map(|v| VariationSetting::new(Tag::new(&v.tag), v.value))
                .collect();

            // Auto-set opsz if font-optical-sizing is auto and not explicitly set
            if font_optical_sizing == crate::FontOpticalSizing::Auto {
                let has_explicit_opsz = variations.iter().any(|v| v.tag == *b"opsz");
                if !has_explicit_opsz {
                    let has_opsz_axis = axes.iter().any(|a| a.tag() == Tag::new(b"opsz"));
                    if has_opsz_axis {
                        settings.push(VariationSetting::new(Tag::new(b"opsz"), font_size));
                    }
                }
            }

            // Use location_to_slice which applies avar (axis variations) table remapping.
            // This differs from ttf-parser's set_variation() which used raw user-space values.
            // Avar remapping transforms user-space axis values to design-space coordinates,
            // which is required for correct variable font rendering (especially for fonts
            // like Roboto Flex that rely heavily on avar for intermediate axis values).
            axes.location_to_slice(&settings, &mut coords);

            let location = LocationRef::new(&coords);
            let mut pen = SkrifaPen::new();
            let settings = DrawSettings::unhinted(SkrifaSize::unscaled(), location);
            glyph.draw(settings, &mut pen).ok()?;
            pen.finish()
        })?
    }

    fn raster(&self, id: ID, glyph_id: GlyphId, font_size: f32) -> Option<BitmapImage> {
        self.with_face_data(id, |data, face_index| -> Option<BitmapImage> {
            let font = FontRef::from_index(data, face_index).ok()?;

            // Try to get bitmap strikes
            let strikes = font.bitmap_strikes();

            // Get the largest available strike first to check bitmap type
            let largest_strike = strikes
                .iter()
                .max_by(|a, b| a.ppem().partial_cmp(&b.ppem()).unwrap_or(std::cmp::Ordering::Equal))?;

            let bitmap_glyph = largest_strike.get(glyph_id)?;
            let bitmap_data = bitmap_glyph.data;

            // Check if this is a color bitmap (PNG/BGRA) or monochrome (Mask)
            let is_color_bitmap = matches!(bitmap_data, BitmapData::Png(_) | BitmapData::Bgra(_));

            // Strike selection strategy:
            // - Color bitmaps (PNG/BGRA): use largest strike (original behavior)
            // - Monochrome bitmaps (Mask): only use if exact size match OR no outline exists
            let strike = if is_color_bitmap {
                // Color bitmap: use largest strike (original behavior)
                largest_strike
            } else {
                // Monochrome bitmap: prefer exact match for pixel-perfect rendering
                let has_outline = font.outline_glyphs().get(glyph_id).is_some();
                let exact_match = strikes
                    .iter()
                    .find(|s| (s.ppem() - font_size).abs() < 0.01);

                if let Some(strike) = exact_match {
                    strike
                } else if !has_outline {
                    // No outline fallback, use best available strike
                    strikes
                        .iter()
                        .filter(|s| s.ppem() >= font_size)
                        .min_by(|a, b| a.ppem().partial_cmp(&b.ppem()).unwrap_or(std::cmp::Ordering::Equal))
                        .unwrap_or(largest_strike)
                } else {
                    // Has outline and no exact match - caller will use outline
                    return None;
                }
            };

            // Re-get bitmap_glyph for the selected strike (may differ for monochrome)
            let bitmap_glyph = strike.get(glyph_id)?;
            let bitmap_data = bitmap_glyph.data;

            // Handle different bitmap formats
            let (png_data, width, height): (Vec<u8>, u32, u32) = match bitmap_data {
                BitmapData::Png(data) => {
                    // Get PNG dimensions using imagesize
                    let (w, h) = if let Ok(size) = imagesize::blob_size(data) {
                        (size.width as u32, size.height as u32)
                    } else {
                        // Fallback: estimate from strike ppem
                        let ppem = strike.ppem();
                        (ppem as u32, ppem as u32)
                    };
                    (data.to_vec(), w, h)
                }
                BitmapData::Mask(mask) => {
                    // Convert monochrome/grayscale mask to PNG
                    let w = bitmap_glyph.width;
                    let h = bitmap_glyph.height;
                    match mask_to_png(&mask, w, h) {
                        Some(png) => (png, w, h),
                        None => return None,
                    }
                }
                BitmapData::Bgra(data) => {
                    // Convert BGRA to PNG
                    let w = bitmap_glyph.width;
                    let h = bitmap_glyph.height;
                    match bgra_to_png(data, w, h) {
                        Some(png) => (png, w, h),
                        None => return None,
                    }
                }
            };

            // Get the glyph outline bounding box for SBIX positioning.
            // SBIX requires the outline bbox for proper vertical alignment.
            let glyph_bbox = {
                let outlines = font.outline_glyphs();
                outlines.get(glyph_id).and_then(|glyph| {
                    let mut bounds_pen = ControlBoundsPen::new();
                    let settings = DrawSettings::unhinted(SkrifaSize::unscaled(), LocationRef::default());
                    glyph.draw(settings, &mut bounds_pen).ok()?;
                    bounds_pen.bounding_box().map(|bb| GlyphBbox {
                        // Clamp to i16 range to prevent truncation issues with large glyph bounds
                        x_min: bb.x_min.clamp(i16::MIN as f32, i16::MAX as f32) as i16,
                        y_min: bb.y_min.clamp(i16::MIN as f32, i16::MAX as f32) as i16,
                        x_max: bb.x_max.clamp(i16::MIN as f32, i16::MAX as f32) as i16,
                        y_max: bb.y_max.clamp(i16::MIN as f32, i16::MAX as f32) as i16,
                    })
                })
            };

            // Detect SBIX format by checking if the font has an sbix table.
            let is_sbix = font.table_data(Tag::new(b"sbix")).is_some();

            log::trace!(
                "Bitmap glyph: bearing=({}, {}), inner_bearing=({}, {}), ppem={}, bbox={:?}, is_sbix={}, size={}x{}",
                bitmap_glyph.bearing_x, bitmap_glyph.bearing_y,
                bitmap_glyph.inner_bearing_x, bitmap_glyph.inner_bearing_y,
                strike.ppem(), glyph_bbox, is_sbix, width, height
            );

            // Use skrifa's inner_bearing values directly for both SBIX and CBDT.
            // inner_bearing_x/y contain the glyph positioning offsets we need.
            let (x, y) = (bitmap_glyph.inner_bearing_x, bitmap_glyph.inner_bearing_y);

            // Choose rendering mode based on bitmap type:
            // - Color bitmaps: smooth scaling for better quality when resized
            // - Monochrome bitmaps: nearest-neighbor for pixel-perfect rendering
            let rendering_mode = if is_color_bitmap {
                ImageRendering::OptimizeQuality
            } else {
                ImageRendering::OptimizeSpeed
            };

            let bitmap_image = BitmapImage {
                image: Image {
                    id: String::new(),
                    visible: true,
                    size: Size::from_wh(width as f32, height as f32)?,
                    rendering_mode,
                    kind: ImageKind::PNG(Arc::new(png_data)),
                    abs_transform: Transform::default(),
                    abs_bounding_box: NonZeroRect::from_xywh(0.0, 0.0, width as f32, height as f32)?,
                },
                x,
                y,
                pixels_per_em: strike.ppem(),
                glyph_bbox,
                is_sbix,
            };

            Some(bitmap_image)
        })?
    }

    fn svg(&self, id: ID, glyph_id: GlyphId) -> Option<Node> {
        // Parse SVG table manually since skrifa doesn't expose SVG table access yet.
        // SVG table format (OpenType spec):
        // - Header: version (u16), svgDocListOffset (u32), reserved (u32)
        // - Document list at offset: numEntries (u16), entries[]
        // - Each entry: startGlyphID (u16), endGlyphID (u16), svgDocOffset (u32), svgDocLength (u32)
        self.with_face_data(id, |data, face_index| -> Option<Node> {
            let font = FontRef::from_index(data, face_index).ok()?;

            let svg_table = font.table_data(Tag::new(b"SVG "))?;
            let svg_data = svg_table.as_ref();

            // Need at least header (10 bytes)
            if svg_data.len() < 10 {
                return None;
            }

            // Parse header
            let _version = u16::from_be_bytes([svg_data[0], svg_data[1]]);
            let doc_list_offset =
                u32::from_be_bytes([svg_data[2], svg_data[3], svg_data[4], svg_data[5]]) as usize;

            // Navigate to document list
            if doc_list_offset + 2 > svg_data.len() {
                return None;
            }

            let doc_list = &svg_data[doc_list_offset..];
            let num_entries = u16::from_be_bytes([doc_list[0], doc_list[1]]) as usize;

            // Each entry is 12 bytes
            let entries_start = 2;
            let glyph_id_val = glyph_id.to_u32() as u16;

            // Find the entry for this glyph
            for i in 0..num_entries {
                let entry_offset = entries_start + i * 12;
                if entry_offset + 12 > doc_list.len() {
                    break;
                }

                let entry = &doc_list[entry_offset..entry_offset + 12];
                let start_glyph = u16::from_be_bytes([entry[0], entry[1]]);
                let end_glyph = u16::from_be_bytes([entry[2], entry[3]]);
                let svg_doc_offset =
                    u32::from_be_bytes([entry[4], entry[5], entry[6], entry[7]]) as usize;
                let svg_doc_length =
                    u32::from_be_bytes([entry[8], entry[9], entry[10], entry[11]]) as usize;

                if glyph_id_val >= start_glyph && glyph_id_val <= end_glyph {
                    // Found the entry - extract SVG document
                    // Offset is relative to start of SVG table
                    let abs_offset = doc_list_offset + svg_doc_offset;
                    if abs_offset + svg_doc_length > svg_data.len() {
                        return None;
                    }

                    let svg_doc_data = &svg_data[abs_offset..abs_offset + svg_doc_length];

                    // Handle gzip compression (SVG documents may be gzip compressed)
                    let svg_bytes: std::borrow::Cow<[u8]> =
                        if svg_doc_data.starts_with(&[0x1f, 0x8b]) {
                            // Gzip compressed
                            use std::io::Read;
                            let mut decoder = flate2::read::GzDecoder::new(svg_doc_data);
                            let mut decompressed = Vec::new();
                            if decoder.read_to_end(&mut decompressed).is_err() {
                                return None;
                            }
                            std::borrow::Cow::Owned(decompressed)
                        } else {
                            std::borrow::Cow::Borrowed(svg_doc_data)
                        };

                    // Parse the SVG document
                    let tree =
                        crate::Tree::from_data(&svg_bytes, &crate::Options::default()).ok()?;

                    // If this record covers a single glyph, return the whole tree
                    // Otherwise, look for the specific glyph by ID
                    let node = if start_glyph == end_glyph {
                        Node::Group(Box::new(tree.root))
                    } else {
                        // Multi-glyph record - find the specific glyph by ID
                        let glyph_node_id = format!("glyph{}", glyph_id_val);
                        tree.node_by_id(&glyph_node_id).cloned()?
                    };

                    return Some(node);
                }
            }

            None
        })?
    }

    fn colr(&self, id: ID, glyph_id: GlyphId) -> Option<Tree> {
        // Use skrifa-based COLR painting
        // This provides COLRv1 support (sweep gradients, advanced blend modes)
        let result = self.with_face_data(id, |data, face_index| {
            super::skrifa_colr::paint_colr_glyph(data, face_index, glyph_id)
        })?;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skrifa_variable_font() {
        // Test that skrifa properly applies variable font axes
        let font_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../crates/resvg/tests/fonts/RobotoFlex.subset.ttf"
        );
        let font_data = std::fs::read(font_path).expect("Font not found");

        let font = FontRef::new(&font_data).expect("Failed to parse font");
        let outlines = font.outline_glyphs();

        // Get glyph for 'N'
        let charmap = font.charmap();
        let glyph_id = charmap.map('N').expect("Glyph not found");
        let glyph = outlines.get(glyph_id).expect("Outline not found");

        // Get axes
        let axes = font.axes();

        // Find wdth axis
        let wdth_idx = axes
            .iter()
            .position(|a| a.tag() == Tag::new(b"wdth"))
            .expect("wdth axis not found");

        // Draw with default location
        let mut pen1 = SkrifaPen::new();
        let settings1 = DrawSettings::unhinted(SkrifaSize::unscaled(), LocationRef::default());
        glyph.draw(settings1, &mut pen1).expect("Draw failed");
        let path1 = pen1.finish().expect("Path failed");
        let bounds1 = path1.bounds();

        // Draw with wdth=25 (narrow)
        let mut coords = vec![skrifa::instance::NormalizedCoord::default(); axes.len()];
        coords[wdth_idx] = axes.get(wdth_idx).unwrap().normalize(25.0);

        let location = LocationRef::new(&coords);
        let mut pen2 = SkrifaPen::new();
        let settings2 = DrawSettings::unhinted(SkrifaSize::unscaled(), location);
        glyph.draw(settings2, &mut pen2).expect("Draw failed");
        let path2 = pen2.finish().expect("Path failed");
        let bounds2 = path2.bounds();

        // The narrow version should have a smaller width
        assert!(
            bounds2.width() < bounds1.width(),
            "wdth=25 should be narrower than default! default width: {}, wdth=25 width: {}",
            bounds1.width(),
            bounds2.width()
        );
    }
}
