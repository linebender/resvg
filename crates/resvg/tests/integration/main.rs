// Copyright 2020 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use once_cell::sync::Lazy;
use png::{BitDepth, ColorType, Encoder};
use rgb::{FromSlice, Rgba};
use std::cmp::max;
use std::fs::File;
use std::io::{BufWriter, Cursor};
use std::process::Command;
use std::sync::Arc;
use usvg::fontdb;

#[rustfmt::skip]
mod render;

mod extra;

#[cfg(feature = "16bpc")]
#[path = "../u16_rendering.rs"]
mod u16_rendering;

const IMAGE_SIZE: u32 = 300;

static GLOBAL_FONTDB: Lazy<Arc<fontdb::Database>> = Lazy::new(|| {
    if let Ok(()) = log::set_logger(&LOGGER) {
        log::set_max_level(log::LevelFilter::Warn);
    }

    let mut fontdb = fontdb::Database::new();
    // Load fonts in a deterministic order.
    let mut font_paths: Vec<_> = std::fs::read_dir("tests/fonts")
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("ttf" | "ttc" | "otf" | "otc" | "TTF" | "TTC" | "OTF" | "OTC")
            )
        })
        .collect();
    font_paths.sort();
    for path in font_paths {
        if let Err(e) = fontdb.load_font_file(&path) {
            log::warn!("Failed to load '{}' cause {}.", path.display(), e);
        }
    }
    fontdb.set_serif_family("Noto Serif");
    fontdb.set_sans_serif_family("Noto Sans");
    fontdb.set_cursive_family("Yellowtail");
    fontdb.set_fantasy_family("Sedgwick Ave Display");
    fontdb.set_monospace_family("Noto Mono");
    Arc::new(fontdb)
});

pub fn render(name: &str) -> usize {
    render_inner(name, TestMode::Normal)
}

pub fn render_extra_with_scale(name: &str, scale: f32) -> usize {
    render_inner(name, TestMode::Extra(scale))
}

pub fn render_extra(name: &str) -> usize {
    render_extra_with_scale(name, 1.0)
}

pub fn render_node(name: &str, id: &str) -> usize {
    render_inner(name, TestMode::Node(id))
}

pub fn render_inner(name: &str, test_mode: TestMode) -> usize {
    let svg_path = format!("tests/{}.svg", name);
    let png_path = format!("tests/{}.png", name);
    let make_ref = std::env::var("MAKE_REF").is_ok();

    let opt = usvg::Options {
        fontdb: GLOBAL_FONTDB.clone(),
        resources_dir: Some(
            std::path::PathBuf::from(&svg_path)
                .parent()
                .unwrap()
                .to_owned(),
        ),
        ..usvg::Options::default()
    };

    let tree = {
        let svg_data = std::fs::read(&svg_path).unwrap();
        usvg::Tree::from_data(&svg_data, &opt).unwrap()
    };

    let is_16bpc = std::env::var("RESVG_TEST_16BPC").is_ok();
    let diff_threshold: u8 = if is_16bpc && is_known_16bpc_precision_delta(name) {
        32
    } else {
        1
    };

    let (size, render_ts, node_id) = match test_mode {
        TestMode::Normal => {
            let s = tree
                .size()
                .to_int_size()
                .scale_to_width(IMAGE_SIZE)
                .unwrap();
            let ts = tiny_skia::Transform::from_scale(
                s.width() as f32 / tree.size().width() as f32,
                s.height() as f32 / tree.size().height() as f32,
            );
            (s, ts, None)
        }
        TestMode::Node(id) => {
            let node = tree.node_by_id(id).unwrap();
            let s = node.abs_layer_bounding_box().unwrap().size().to_int_size();
            (s, tiny_skia::Transform::identity(), Some(id))
        }
        TestMode::Extra(scale) => {
            let s = tree.size().to_int_size().scale_by(scale).unwrap();
            let ts = tiny_skia::Transform::from_scale(scale, scale);
            (s, ts, None)
        }
    };

    let make_ref_fn = || -> ! {
        let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height()).unwrap();
        if let Some(id) = node_id {
            let node = tree.node_by_id(id).unwrap();
            resvg::render_node(node, render_ts, &mut pixmap.as_mut());
        } else {
            resvg::render(&tree, render_ts, &mut pixmap.as_mut());
        }
        pixmap.save_png(&png_path).unwrap();
        Command::new("oxipng")
            .args([
                "-o".to_owned(),
                "6".to_owned(),
                "-Z".to_owned(),
                png_path.clone(),
            ])
            .output()
            .unwrap();
        panic!("new reference image created");
    };

    #[cfg(feature = "16bpc")]
    let (reference_image, actual_image) = if is_16bpc {
        let mut pixmap = tiny_skia::PixmapU16::new(size.width(), size.height()).unwrap();
        if let Some(id) = node_id {
            let node = tree.node_by_id(id).unwrap();
            resvg::render_node_u16(node, render_ts, &mut pixmap.as_mut());
        } else {
            resvg::render_u16(&tree, render_ts, &mut pixmap.as_mut());
        }

        let mut data = Vec::with_capacity((size.width() * size.height() * 4) as usize);
        for p in pixmap.pixels() {
            data.push(((p.red() as u32 + 128) / 257) as u8);
            data.push(((p.green() as u32 + 128) / 257) as u8);
            data.push(((p.blue() as u32 + 128) / 257) as u8);
            data.push(((p.alpha() as u32 + 128) / 257) as u8);
        }
        let actual = TestImage::new_with(data, size.width(), size.height());
        let expected = if let Ok(image_data) = std::fs::read(&png_path) {
            load_png(image_data, true)
        } else if make_ref {
            make_ref_fn();
        } else {
            panic!("missing reference image");
        };
        (expected, actual)
    } else {
        let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height()).unwrap();
        if let Some(id) = node_id {
            let node = tree.node_by_id(id).unwrap();
            resvg::render_node(node, render_ts, &mut pixmap.as_mut());
        } else {
            resvg::render(&tree, render_ts, &mut pixmap.as_mut());
        }

        let mut data = pixmap.take();
        demultiply_alpha(data.as_mut_slice());
        let actual = TestImage::new_with(data, size.width(), size.height());
        let expected = if let Ok(image_data) = std::fs::read(&png_path) {
            load_png(image_data, false)
        } else if make_ref {
            make_ref_fn();
        } else {
            panic!("missing reference image");
        };
        (expected, actual)
    };

    #[cfg(not(feature = "16bpc"))]
    let (reference_image, actual_image) = {
        let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height()).unwrap();
        if let Some(id) = node_id {
            let node = tree.node_by_id(id).unwrap();
            resvg::render_node(node, render_ts, &mut pixmap.as_mut());
        } else {
            resvg::render(&tree, render_ts, &mut pixmap.as_mut());
        }

        let mut data = pixmap.take();
        demultiply_alpha(data.as_mut_slice());
        let actual = TestImage::new_with(data, size.width(), size.height());
        let expected = if let Ok(image_data) = std::fs::read(&png_path) {
            load_png(image_data, false)
        } else if make_ref {
            make_ref_fn();
        } else {
            panic!("missing reference image");
        };
        (expected, actual)
    };

    if is_16bpc && is_known_16bpc_precision_delta(name) {
        let out_dir = "tests/16bpc_diff_output";
        let _ = std::fs::create_dir_all(out_dir);
        let clean_name = name.replace('/', "_");
        reference_image.save_png(&format!("{}/{}-baseline.png", out_dir, clean_name));
        actual_image.save_png(&format!("{}/{}-rendered.png", out_dir, clean_name));
    }

    if let Some((diff_image, pixel_diff)) =
        get_diff(&reference_image, &actual_image, diff_threshold)
    {
        if make_ref {
            make_ref_fn();
        } else {
            let _ = std::fs::create_dir_all("tests/diffs");
            diff_image.save_png(&format!("tests/diffs/{}.png", name.replace("/", "_")));

            pixel_diff
        }
    } else {
        0
    }
}

/// Returns `Some` if there is at least one different pixel, and `None` if the images match.
fn get_diff(
    expected_image: &TestImage,
    actual_image: &TestImage,
    diff_threshold: u8,
) -> Option<(TestImage, usize)> {
    let width = max(expected_image.width, actual_image.width);
    let height = max(expected_image.height, actual_image.height);

    let mut diff_image = TestImage::new(3 * width, height);

    let mut pixel_diff = 0;

    for x in 0..width {
        for y in 0..height {
            let actual_pixel = actual_image.get_pixel(x, y);
            let expected_pixel = expected_image.get_pixel(x, y);

            match (actual_pixel, expected_pixel) {
                (Some(actual), Some(expected)) => {
                    diff_image.set_pixel(x, y, expected);
                    diff_image.set_pixel(x + 2 * width, y, actual);
                    if is_pix_diff(&expected, &actual, diff_threshold) {
                        pixel_diff += 1;
                        diff_image.set_pixel(x + width, y, Rgba::new(255, 0, 0, 255));
                    } else {
                        diff_image.set_pixel(x + width, y, Rgba::new(0, 0, 0, 255));
                    }
                }
                (Some(actual), None) => {
                    pixel_diff += 1;
                    diff_image.set_pixel(x + 2 * width, y, actual);
                    diff_image.set_pixel(x + width, y, Rgba::new(255, 0, 0, 255));
                }
                (None, Some(expected)) => {
                    pixel_diff += 1;
                    diff_image.set_pixel(x, y, expected);
                    diff_image.set_pixel(x + width, y, Rgba::new(255, 0, 0, 255));
                }
                _ => {
                    pixel_diff += 1;
                    diff_image.set_pixel(x, y, Rgba::new(255, 0, 0, 255));
                    diff_image.set_pixel(x + width, y, Rgba::new(255, 0, 0, 255));
                }
            }
        }
    }

    if pixel_diff > 0 {
        Some((diff_image, pixel_diff))
    } else {
        None
    }
}

fn premultiply_u8(c: u8, a: u8) -> u8 {
    let prod = u32::from(c) * u32::from(a) + 128;
    ((prod + (prod >> 8)) >> 8) as u8
}

fn premultiply_alpha(data: &mut [u8]) {
    for chunk in data.chunks_exact_mut(4) {
        let a = chunk[3];
        chunk[0] = premultiply_u8(chunk[0], a);
        chunk[1] = premultiply_u8(chunk[1], a);
        chunk[2] = premultiply_u8(chunk[2], a);
    }
}

fn demultiply_alpha(data: &mut [u8]) {
    for chunk in data.chunks_exact_mut(4) {
        let a = chunk[3] as f64 / 255.0;
        if a > 0.0 {
            chunk[0] = (chunk[0] as f64 / a + 0.5) as u8;
            chunk[1] = (chunk[1] as f64 / a + 0.5) as u8;
            chunk[2] = (chunk[2] as f64 / a + 0.5) as u8;
        }
    }
}

fn is_pix_diff(pixel1: &Rgba<u8>, pixel2: &Rgba<u8>, threshold: u8) -> bool {
    if pixel1.a == 0 && pixel2.a == 0 {
        return false;
    }

    let mut different = false;

    different |= pixel1.r.abs_diff(pixel2.r) > threshold;
    different |= pixel1.g.abs_diff(pixel2.g) > threshold;
    different |= pixel1.b.abs_diff(pixel2.b) > threshold;
    different |= pixel1.a.abs_diff(pixel2.a) > threshold;

    different
}

fn load_png(data: Vec<u8>, premultiply: bool) -> TestImage {
    let mut decoder = png::Decoder::new(Cursor::new(data.as_slice()));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().unwrap();
    let mut img_data = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut img_data).unwrap();

    let mut data = match info.color_type {
        png::ColorType::Rgb => {
            panic!("RGB PNG is not supported.");
        }
        png::ColorType::Rgba => img_data,
        png::ColorType::Grayscale => {
            let mut rgba_data = Vec::with_capacity(img_data.len() * 4);
            for gray in img_data {
                rgba_data.push(gray);
                rgba_data.push(gray);
                rgba_data.push(gray);
                rgba_data.push(255);
            }

            rgba_data
        }
        png::ColorType::GrayscaleAlpha => {
            let mut rgba_data = Vec::with_capacity(img_data.len() * 2);
            for slice in img_data.chunks(2) {
                let gray = slice[0];
                let alpha = slice[1];
                rgba_data.push(gray);
                rgba_data.push(gray);
                rgba_data.push(gray);
                rgba_data.push(alpha);
            }

            rgba_data
        }
        png::ColorType::Indexed => {
            panic!("Indexed PNG is not supported.");
        }
    };

    if premultiply {
        premultiply_alpha(&mut data);
    }

    TestImage::new_with(data, info.width, info.height)
}

struct TestImage {
    data: Vec<u8>,
    width: u32,
    height: u32,
}

impl TestImage {
    fn new(width: u32, height: u32) -> Self {
        Self {
            data: vec![0; width as usize * height as usize * 4],
            width,
            height,
        }
    }

    fn new_with(data: Vec<u8>, width: u32, height: u32) -> Self {
        Self {
            data,
            width,
            height,
        }
    }

    fn get_pixel(&self, x: u32, y: u32) -> Option<Rgba<u8>> {
        if x >= self.width || y >= self.height {
            return None;
        }

        let pos = self.width as usize * (y as usize) + x as usize;

        Some(self.data.as_rgba()[pos])
    }

    fn set_pixel(&mut self, x: u32, y: u32, val: Rgba<u8>) {
        let pos = self.width as usize * (y as usize) + x as usize;

        self.data.as_rgba_mut()[pos] = val;
    }

    fn save_png(&self, path: &str) {
        let file = File::create(path).unwrap();
        let mut w = BufWriter::new(file);

        let mut encoder = Encoder::new(&mut w, self.width, self.height);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(BitDepth::Eight);

        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&self.data).unwrap();
        writer.finish().unwrap();
    }
}

#[derive(Copy, Clone)]
pub enum TestMode<'a> {
    /// Render a node by its ID.
    Node(&'a str),
    /// Render an `extra` test with a specific scale.
    Extra(f32),
    /// Render a normal SVG test.
    Normal,
}

/// A simple stderr logger.
static LOGGER: SimpleLogger = SimpleLogger;
struct SimpleLogger;
impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::LevelFilter::Warn
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let target = if !record.target().is_empty() {
                record.target()
            } else {
                record.module_path().unwrap_or_default()
            };

            let line = record.line().unwrap_or(0);
            let args = record.args();

            match record.level() {
                log::Level::Error => eprintln!("Error (in {}:{}): {}", target, line, args),
                log::Level::Warn => eprintln!("Warning (in {}:{}): {}", target, line, args),
                log::Level::Info => eprintln!("Info (in {}:{}): {}", target, line, args),
                log::Level::Debug => eprintln!("Debug (in {}:{}): {}", target, line, args),
                log::Level::Trace => eprintln!("Trace (in {}:{}): {}", target, line, args),
            }
        }
    }

    fn flush(&self) {}
}

fn is_known_16bpc_precision_delta(name: &str) -> bool {
    static DELTAS: Lazy<std::collections::HashSet<&'static str>> = Lazy::new(|| {
        [
            "filters_enable_background_with_mask",
            "filters_feBlend_with_subregion_on_input_1",
            "filters_feBlend_with_subregion_on_input_2",
            "filters_feColorMatrix_invalid_type",
            "filters_feColorMatrix_type_eq_hueRotate",
            "filters_feColorMatrix_type_eq_hueRotate_without_an_angle",
            "filters_feColorMatrix_type_eq_matrix",
            "filters_feColorMatrix_type_eq_matrix_with_empty_values",
            "filters_feColorMatrix_type_eq_matrix_with_non_normalized_values",
            "filters_feColorMatrix_type_eq_matrix_with_not_enough_values",
            "filters_feColorMatrix_type_eq_matrix_with_too_many_values",
            "filters_feColorMatrix_type_eq_matrix_without_values",
            "filters_feColorMatrix_type_eq_saturate",
            "filters_feColorMatrix_type_eq_saturate_with_a_large_coefficient",
            "filters_feColorMatrix_type_eq_saturate_with_negative_coefficient",
            "filters_feColorMatrix_type_eq_saturate_without_a_coefficient",
            "filters_feColorMatrix_without_a_type",
            "filters_feColorMatrix_without_attributes",
            "filters_feComposite_with_subregion_on_input_1",
            "filters_feConvolveMatrix_custom_divisor",
            "filters_feConvolveMatrix_edgeMode_eq_none",
            "filters_feConvolveMatrix_edgeMode_eq_wrap",
            "filters_feConvolveMatrix_order_eq_4",
            "filters_feConvolveMatrix_order_eq_4_2",
            "filters_feConvolveMatrix_order_eq_4_4",
            "filters_feConvolveMatrix_preserveAlpha_eq_true",
            "filters_feConvolveMatrix_targetX_eq_0",
            "filters_feConvolveMatrix_targetX_eq_2",
            "filters_feConvolveMatrix_unset_order",
            "filters_feDiffuseLighting_complex_transform",
            "filters_feDiffuseLighting_lighting_color_eq_currentColor",
            "filters_feDiffuseLighting_lighting_color_eq_hsla",
            "filters_feDiffuseLighting_lighting_color_eq_inherit",
            "filters_feDiffuseLighting_lighting_color_eq_seagreen",
            "filters_feDiffuseLighting_linearRGB_color_interpolation",
            "filters_feDiffuseLighting_multiple_light_sources",
            "filters_feDiffuseLighting_single_light_source",
            "filters_feDiffuseLighting_single_light_source_with_comment",
            "filters_feDiffuseLighting_single_light_source_with_desc",
            "filters_feDiffuseLighting_single_light_source_with_invalid_child",
            "filters_feDiffuseLighting_single_light_source_with_title",
            "filters_feDiffuseLighting_single_light_source_with_title_and_desc",
            "filters_feDiffuseLighting_surfaceScale_eq_5",
            "filters_feDiffuseLighting_surfaceScale_eq__10",
            "filters_feDropShadow_hsla_color",
            "filters_feDropShadow_only_stdDeviation",
            "filters_feDropShadow_stdDeviation_eq_0",
            "filters_feDropShadow_with_flood_color",
            "filters_feDropShadow_with_flood_opacity",
            "filters_feDropShadow_with_offset",
            "filters_feDropShadow_with_offset_clipped",
            "filters_feDropShadow_with_percent_offset",
            "filters_feGaussianBlur_complex_transform",
            "filters_feGaussianBlur_simple_case",
            "filters_feGaussianBlur_small_stdDeviation",
            "filters_feGaussianBlur_stdDeviation_eq_0_5",
            "filters_feGaussianBlur_stdDeviation_eq_5_0",
            "filters_feGaussianBlur_stdDeviation_with_two_different_values",
            "filters_feGaussianBlur_stdDeviation_with_two_values",
            "filters_feMerge_color_interpolation_filters_eq_linearRGB",
            "filters_feMerge_color_interpolation_filters_eq_sRGB",
            "filters_feMerge_complex_transform",
            "filters_feMorphology_source_with_opacity",
            "filters_feSpecularLighting_lighting_color_eq_hsla",
            "filters_feSpecularLighting_with_feDistantLight",
            "filters_feSpecularLighting_with_fePointLight",
            "filters_feSpecularLighting_with_feSpotLight_and_specularConstant_eq_5",
            "filters_feSpecularLighting_with_feSpotLight_and_specular_and_exponent",
            "filters_feSpotLight_custom_attributes",
            "filters_feTurbulence_baseFrequency_eq_0_01",
            "filters_feTurbulence_baseFrequency_eq_0_05_0",
            "filters_feTurbulence_baseFrequency_eq_0_05_0_01",
            "filters_feTurbulence_baseFrequency_eq_0_05_0_05",
            "filters_feTurbulence_complex_transform",
            "filters_feTurbulence_numOctaves_eq_5",
            "filters_feTurbulence_primitiveUnits_eq_objectBoundingBox",
            "filters_feTurbulence_seed_eq_1_5",
            "filters_feTurbulence_seed_eq_20",
            "filters_feTurbulence_seed_eq__20",
            "filters_feTurbulence_stitchTiles_eq_stitch",
            "filters_feTurbulence_type_eq_fractalNoise",
            "filters_feTurbulence_type_eq_invalid",
            "filters_filter_complex_order_and_xlink_href",
            "filters_filter_content_outside_the_canvas_2",
            "filters_filter_default_color_interpolation_filters",
            "filters_filter_everything_via_xlink_href",
            "filters_filter_functions_nested_filters",
            "filters_filter_functions_two_exact_urls",
            "filters_filter_functions_two_urls",
            "filters_filter_functions_url_and_grayscale",
            "filters_filter_global_transform",
            "filters_filter_huge_region",
            "filters_filter_in_eq_BackgroundAlpha",
            "filters_filter_in_eq_BackgroundImage",
            "filters_filter_in_eq_FillPaint",
            "filters_filter_in_eq_FillPaint_with_gradient",
            "filters_filter_in_eq_FillPaint_with_pattern",
            "filters_filter_in_eq_FillPaint_with_target_on_g",
            "filters_filter_in_eq_StrokePaint",
            "filters_filter_in_to_invalid_1",
            "filters_filter_in_to_invalid_2",
            "filters_filter_initial_transform",
            "filters_filter_invalid_filterUnits",
            "filters_filter_invalid_primitive_1",
            "filters_filter_invalid_xlink_href",
            "filters_filter_multiple_primitives_1",
            "filters_filter_multiple_primitives_2",
            "filters_filter_multiple_primitives_3",
            "filters_filter_multiple_primitives_4",
            "filters_filter_negative_subregion",
            "filters_filter_on_the_root_svg",
            "filters_filter_primitiveUnits_eq_objectBoundingBox",
            "filters_filter_recursive_xlink_href",
            "filters_filter_region_with_stroke",
            "filters_filter_self_recursive_xlink_href",
            "filters_filter_simple_case",
            "filters_filter_some_attributes_via_xlink_href",
            "filters_filter_subregion_and_primitiveUnits_eq_objectBoundingBox_1",
            "filters_filter_subregion_and_primitiveUnits_eq_objectBoundingBox_2",
            "filters_filter_transform_on_filter",
            "filters_filter_transform_on_shape",
            "filters_filter_transform_on_shape_with_filter_region",
            "filters_filter_unresolved_xlink_href",
            "filters_filter_with_clip_path",
            "filters_filter_with_clip_path_and_mask",
            "filters_filter_with_mask",
            "filters_filter_with_multiple_transforms_1",
            "filters_filter_with_multiple_transforms_2",
            "filters_filter_with_region",
            "filters_filter_with_region_and_filterUnits_eq_userSpaceOnUse",
            "filters_filter_with_region_and_subregion",
            "filters_filter_with_region_outside_the_canvas",
            "filters_filter_with_subregion_1",
            "filters_filter_with_subregion_2",
            "filters_filter_with_subregion_3",
            "filters_filter_without_region_and_filterUnits_eq_userSpaceOnUse",
            "masking_mask_color_interpolation_eq_linearRGB",
            "masking_mask_maskUnits_eq_objectBoundingBox_with_percent",
            "masking_mask_maskUnits_eq_userSpaceOnUse_with_percent",
            "masking_mask_maskUnits_eq_userSpaceOnUse_with_rect",
            "masking_mask_maskUnits_eq_userSpaceOnUse_with_width_only",
            "masking_mask_maskUnits_eq_userSpaceOnUse_without_rect",
            "masking_mask_mask_on_child",
            "masking_mask_mask_on_self_with_mixed_mask_type",
            "masking_mask_mask_type_eq_invalid",
            "masking_mask_mask_type_eq_luminance",
            "masking_mask_nested_objectBoundingBox",
            "masking_mask_recursive",
            "masking_mask_recursive_on_child",
            "masking_mask_recursive_on_self",
            "masking_mask_self_recursive",
            "masking_mask_simple_case",
            "masking_mask_transform_has_no_effect",
            "masking_mask_transform_on_shape",
            "masking_mask_with_grayscale_image",
            "masking_mask_with_image",
            "masking_mask_with_opacity_2",
            "paint_servers_radialGradient_hsla_color",
            "painting_context_with_gradient_in_use",
            "painting_context_with_gradient_on_marker",
            "painting_context_with_pattern_and_transform_in_use",
            "painting_context_with_pattern_objectBoundingBox_in_use",
            "painting_marker_target_with_subpaths_1",
            "painting_marker_target_with_subpaths_2",
            "painting_marker_with_an_image_child",
            "painting_mix_blend_mode_color",
            "painting_mix_blend_mode_hard_light",
            "painting_mix_blend_mode_hue",
            "painting_mix_blend_mode_saturation",
            "painting_stroke_control_points_clamping_1",
            "structure_image_no_width_and_height_on_svg",
            "text_color_font_colrv1",
            "text_color_font_compound_emojis_and_coordinates_list",
            "text_text_decoration_tspan_decoration",
            "extra_filter_with_transform_on_shape",
        ]
        .into_iter()
        .collect()
    });

    let stripped = name.strip_prefix("tests/").unwrap_or(name);
    let normalized = stripped.replace('=', "_eq_").replace(['/', '-', '.'], "_");
    DELTAS.contains(&normalized.as_str())
}
