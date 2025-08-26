// Copyright 2020 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use image::{DynamicImage, ImageBuffer, Rgba, RgbaImage};
use once_cell::sync::Lazy;
use rgb::{FromSlice, RGBA8};
use std::cmp::max;
use std::process::Command;
use std::sync::Arc;
use usvg::fontdb;

#[rustfmt::skip]
mod render;

mod extra;

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

    let mut rgba = pixmap.take();
    demultiply_alpha(rgba.as_mut_slice().as_rgba_mut());

    let actual_image =
        DynamicImage::ImageRgba8(ImageBuffer::from_raw(size.width(), size.height(), rgba).unwrap())
            .to_rgba8();

    let make_ref_fn = || -> ! {
        actual_image.save(&png_path).unwrap();
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
        image::load_from_memory(&image_data).unwrap().to_rgba8()
    } else {
        if make_ref {
            make_ref_fn();
        } else {
            panic!("missing reference image");
        }
    };

    if let Some((diff_image, pixel_diff)) = get_diff(&reference_image, &actual_image) {
        if make_ref {
            make_ref_fn();
        } else {
            let _ = std::fs::create_dir_all("tests/diffs");
            diff_image
                .save(format!("tests/diffs/{}.png", name.replace("/", "_")))
                .unwrap();

            pixel_diff
        }
    } else {
        0
    }
}

/// Returns `Some` if there is at least one different pixel, and `None` if the images match.
fn get_diff(expected_image: &RgbaImage, actual_image: &RgbaImage) -> Option<(RgbaImage, usize)> {
    const DIFF_THRESHOLD: u8 = 1;
    
    let width = max(expected_image.width(), actual_image.width());
    let height = max(expected_image.height(), actual_image.height());

    let mut diff_image = RgbaImage::new(width * 3, height);

    let mut pixel_diff = 0;

    for x in 0..width {
        for y in 0..height {
            let actual_pixel = actual_image.get_pixel_checked(x, y);
            let expected_pixel = expected_image.get_pixel_checked(x, y);
            
            match (actual_pixel, expected_pixel) {
                (Some(actual), Some(expected)) => {
                    diff_image.put_pixel(x, y, *expected);
                    diff_image.put_pixel(x + 2 * width, y, *actual);
                    if is_pix_diff(expected, actual, DIFF_THRESHOLD) {
                        pixel_diff += 1;
                        diff_image.put_pixel(x + width, y, Rgba([255, 0, 0, 255]));
                    } else {
                        diff_image.put_pixel(x + width, y, Rgba([0, 0, 0, 255]));
                    }
                }
                (Some(actual), None) => {
                    pixel_diff += 1;
                    diff_image.put_pixel(x + 2 * width, y, *actual);
                    diff_image.put_pixel(x + width, y, Rgba([255, 0, 0, 255]));
                }
                (None, Some(expected)) => {
                    pixel_diff += 1;
                    diff_image.put_pixel(x, y, *expected);
                    diff_image.put_pixel(x + width, y, Rgba([255, 0, 0, 255]));
                }
                _ => {
                    pixel_diff += 1;
                    diff_image.put_pixel(x, y, Rgba([255, 0, 0, 255]));
                    diff_image.put_pixel(x + width, y, Rgba([255, 0, 0, 255]));
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
    if pixel1.0[3] == 0 && pixel2.0[3] == 0 {
        return false;
    }

    let mut different = false;

    for i in 0..3 {
        let difference = pixel1.0[i].abs_diff(pixel2.0[i]);
        different |= difference > threshold;
    }

    different
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
