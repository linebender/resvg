// Copyright 2020 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use once_cell::sync::Lazy;
use png::{BitDepth, ColorType, Encoder};
use rgb::{FromSlice, RGBA8, Rgba};
use std::cmp::max;
use std::fs::File;
use std::io::{BufWriter, Cursor};
use std::process::Command;
use std::sync::Arc;
use usvg::fontdb;

/// Save a tiny_skia::Pixmap as PNG with 96 DPI metadata.
fn save_pixmap_png_with_dpi(
    pixmap: &tiny_skia::Pixmap,
    path: &str,
) -> Result<(), png::EncodingError> {
    resvg::save_png_with_dpi(pixmap, std::path::Path::new(path), 96)
}

#[rustfmt::skip]
mod render;

#[rustfmt::skip]
mod render_hinted;

mod extra;

mod hinting;

const IMAGE_SIZE: u32 = 300;

static GLOBAL_FONTDB: Lazy<Arc<fontdb::Database>> = Lazy::new(|| {
    if let Ok(()) = log::set_logger(&LOGGER) {
        log::set_max_level(log::LevelFilter::Warn);
    }

    let mut fontdb = fontdb::Database::new();
    fontdb.load_fonts_dir("tests/fonts");
    fontdb.set_serif_family("Noto Serif");
    fontdb.set_sans_serif_family("Noto Sans");
    fontdb.set_cursive_family("Yellowtail");
    fontdb.set_fantasy_family("Sedgwick Ave Display");
    fontdb.set_monospace_family("Noto Mono");
    Arc::new(fontdb)
});

pub fn render(name: &str) -> usize {
    render_inner(name, TestMode::Normal, HintingMode::Disabled)
}

pub fn render_hinted(name: &str) -> usize {
    render_inner(name, TestMode::Normal, HintingMode::Enabled)
}

pub fn render_extra_with_scale(name: &str, scale: f32) -> usize {
    render_inner(name, TestMode::Extra(scale), HintingMode::Disabled)
}

pub fn render_extra(name: &str) -> usize {
    render_extra_with_scale(name, 1.0)
}

pub fn render_node(name: &str, id: &str) -> usize {
    render_inner(name, TestMode::Node(id), HintingMode::Disabled)
}

#[derive(Clone, Copy)]
pub enum HintingMode {
    Disabled,
    Enabled,
}

pub fn render_inner(name: &str, test_mode: TestMode, hinting_mode: HintingMode) -> usize {
    let (svg_path, png_path, diff_dir) = match hinting_mode {
        HintingMode::Disabled => (
            format!("tests/{}.svg", name),
            format!("tests/{}.png", name),
            "tests/diffs",
        ),
        HintingMode::Enabled => (
            format!("tests/{}.svg", name),
            format!("tests-hinted/{}.png", name),
            "tests/diffs-hinted",
        ),
    };
    let make_ref = std::env::var("MAKE_REF").is_ok();

    let hinting_options = match hinting_mode {
        HintingMode::Disabled => usvg::HintingOptions {
            enabled: false,
            dpi: None,
        },
        HintingMode::Enabled => usvg::HintingOptions {
            enabled: true,
            dpi: Some(96.0),
        },
    };

    let opt = usvg::Options {
        fontdb: GLOBAL_FONTDB.clone(),
        resources_dir: Some(
            std::path::PathBuf::from(&svg_path)
                .parent()
                .unwrap()
                .to_owned(),
        ),
        #[cfg(feature = "text")]
        hinting: hinting_options,
        ..usvg::Options::default()
    };

    let tree = {
        let svg_data = std::fs::read(&svg_path).unwrap();
        usvg::Tree::from_data(&svg_data, &opt).unwrap()
    };

    let size;
    let mut pixmap;

    match test_mode {
        TestMode::Normal => {
            size = tree
                .size()
                .to_int_size()
                .scale_to_width(IMAGE_SIZE)
                .unwrap();
            pixmap = tiny_skia::Pixmap::new(size.width(), size.height()).unwrap();
            let render_ts = tiny_skia::Transform::from_scale(
                size.width() as f32 / tree.size().width() as f32,
                size.height() as f32 / tree.size().height() as f32,
            );
            resvg::render(&tree, render_ts, &mut pixmap.as_mut());
        }
        TestMode::Node(id) => {
            let node = tree.node_by_id(id).unwrap();
            size = node.abs_layer_bounding_box().unwrap().size().to_int_size();
            pixmap = tiny_skia::Pixmap::new(size.width(), size.height()).unwrap();
            resvg::render_node(node, tiny_skia::Transform::identity(), &mut pixmap.as_mut());
        }
        TestMode::Extra(scale) => {
            size = tree.size().to_int_size().scale_by(scale).unwrap();
            pixmap = tiny_skia::Pixmap::new(size.width(), size.height()).unwrap();
            let render_ts = tiny_skia::Transform::from_scale(scale, scale);
            resvg::render(&tree, render_ts, &mut pixmap.as_mut());
        }
    }

    let actual_image = {
        let (width, height) = (pixmap.width(), pixmap.height());
        let mut data = pixmap.clone().take();
        demultiply_alpha(data.as_mut_slice().as_rgba_mut());

        TestImage::new_with(data, width, height)
    };

    let make_ref_fn = || -> ! {
        // Create parent directory if needed (for tests-hinted/)
        if let Some(parent) = std::path::Path::new(&png_path).parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("Warning: failed to create directory {:?}: {}", parent, e);
            }
        }
        save_pixmap_png_with_dpi(&pixmap, &png_path).unwrap();
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

    let reference_image = if let Ok(image_data) = std::fs::read(&png_path) {
        load_png(image_data)
    } else {
        if make_ref {
            make_ref_fn();
        } else {
            panic!("missing reference image: {}", png_path);
        }
    };

    if let Some((diff_image, pixel_diff)) = get_diff(&reference_image, &actual_image) {
        if make_ref {
            make_ref_fn();
        } else {
            let _ = std::fs::create_dir_all(diff_dir);
            diff_image.save_png(&format!("{}/{}.png", diff_dir, name.replace("/", "_")));

            pixel_diff
        }
    } else {
        0
    }
}

/// Returns `Some` if there is at least one different pixel, and `None` if the images match.
fn get_diff(expected_image: &TestImage, actual_image: &TestImage) -> Option<(TestImage, usize)> {
    /// Pixel difference threshold for image comparison.
    /// Value of 1 means any channel difference > 1 is considered a mismatch.
    /// This is strict but necessary for detecting subtle font rendering changes.
    /// Note: May need platform-specific adjustments if tests become flaky.
    const DIFF_THRESHOLD: u8 = 1;

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
                    if is_pix_diff(&expected, &actual, DIFF_THRESHOLD) {
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

/// Demultiplies provided pixels alpha.
fn demultiply_alpha(data: &mut [RGBA8]) {
    for p in data {
        let a = p.a as f64 / 255.0;
        p.b = (p.b as f64 / a + 0.5) as u8;
        p.g = (p.g as f64 / a + 0.5) as u8;
        p.r = (p.r as f64 / a + 0.5) as u8;
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

fn load_png(data: Vec<u8>) -> TestImage {
    let mut decoder = png::Decoder::new(Cursor::new(data.as_slice()));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().unwrap();
    let mut img_data = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut img_data).unwrap();

    let data = match info.color_type {
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
        let ref mut w = BufWriter::new(file);

        let mut encoder = Encoder::new(w, self.width, self.height);
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
