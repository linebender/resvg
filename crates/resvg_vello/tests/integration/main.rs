// Copyright 2020 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::cmp::max;
use std::path::PathBuf;
use once_cell::sync::Lazy;
use rgb::{FromSlice, RGBA8};
use std::process::Command;
use std::sync::Arc;
use image::{Rgba, RgbaImage};
use vello_cpu::{Pixmap, RenderContext, RenderMode};
use vello_cpu::kurbo::Affine;
use usvg::fontdb;

#[rustfmt::skip]
mod render;

mod extra;

const IMAGE_SIZE: u32 = 300;

static DIFFS_PATH: std::sync::LazyLock<PathBuf> = std::sync::LazyLock::new(|| {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/diffs");
    let _ = std::fs::create_dir_all(&path);
    
    path
});

static GLOBAL_FONTDB: Lazy<Arc<fontdb::Database>> = Lazy::new(|| {
    if let Ok(()) = log::set_logger(&LOGGER) {
        log::set_max_level(log::LevelFilter::Warn);
    }

    let mut fontdb = fontdb::Database::new();
    fontdb.load_fonts_dir("../resvg/tests/fonts");
    fontdb.set_serif_family("Noto Serif");
    fontdb.set_sans_serif_family("Noto Sans");
    fontdb.set_cursive_family("Yellowtail");
    fontdb.set_fantasy_family("Sedgwick Ave Display");
    fontdb.set_monospace_family("Noto Mono");
    Arc::new(fontdb)
});

pub fn render(name: &str) -> usize {
    let svg_path = format!("../resvg/tests/{}.svg", name);
    let png_path = format!("../resvg/tests/{}.png", name);

    let opt = usvg::Options {
        resources_dir: Some(
            std::path::PathBuf::from(&svg_path)
                .parent()
                .unwrap()
                .to_owned(),
        ),
        fontdb: GLOBAL_FONTDB.clone(),
        ..usvg::Options::default()
    };

    let tree = {
        let svg_data = std::fs::read(&svg_path).unwrap();
        usvg::Tree::from_data(&svg_data, &opt).unwrap()
    };

    let size = tree
        .size()
        .to_int_size()
        .scale_to_width(IMAGE_SIZE)
        .unwrap();
    let mut ctx = RenderContext::new(size.width() as u16, size.height() as u16);
    let mut pixmap = Pixmap::new(size.width() as u16, size.height() as u16);
    
    let render_ts = Affine::scale_non_uniform(
        size.width() as f64 / tree.size().width() as f64,
        size.height() as f64 / tree.size().height() as f64,
    );
    resvg_vello::render(&tree, render_ts, &mut ctx);
    ctx.flush();
    ctx.render_to_pixmap(&mut pixmap, RenderMode::OptimizeQuality);
    
    let pix_png = pixmap.into_png().unwrap();

    let actual_image  = image::load_from_memory(&pix_png).unwrap().to_rgba8();
    let expected_image = image::load_from_memory(&std::fs::read(&png_path).unwrap()).unwrap().to_rgba8();

    if let Some((diff_image, diff_pixels)) = get_diff(&expected_image, &actual_image, 0) {
        if diff_pixels > 0 {
            diff_image.save(DIFFS_PATH.clone().join(format!("{}.png", diff_name(name)))).unwrap();
            
            if option_env!("REPLACE").is_some() {
                std::fs::write(&png_path, pix_png).unwrap();
            }
        }

        diff_pixels as usize
    }   else {
        0
    }
}

pub fn diff_name(name: &str) -> String {
    // From the Python script
    name.replace("tests/", "")
        .replace('/', "_")
        .replace('-', "_")
        .replace('=', "_eq_")
        .replace('.', "_")
        .replace('#', "")
}

pub fn render_extra_with_scale(name: &str, scale: f32) -> usize {
    let svg_path = format!("../resvg/tests/{}.svg", name);
    let png_path = format!("../resvg/tests/{}.png", name);

    let opt = usvg::Options {
        fontdb: GLOBAL_FONTDB.clone(),
        ..usvg::Options::default()
    };

    let tree = {
        let svg_data = std::fs::read(&svg_path).unwrap();
        usvg::Tree::from_data(&svg_data, &opt).unwrap()
    };

    let size = tree.size().to_int_size().scale_by(scale).unwrap();

    let mut ctx = RenderContext::new(size.width() as u16, size.height() as u16);
    let mut pixmap = Pixmap::new(size.width() as u16, size.height() as u16);

    let render_ts = Affine::scale(scale as f64);
    resvg_vello::render(&tree, render_ts, &mut ctx);

    ctx.flush();
    ctx.render_to_pixmap(&mut pixmap, RenderMode::OptimizeQuality);

    let pix_png = pixmap.into_png().unwrap();


    let actual_image  = image::load_from_memory(&pix_png).unwrap().to_rgba8();
    let expected_image = image::load_from_memory(&std::fs::read(&png_path).unwrap()).unwrap().to_rgba8();

    if let Some((diff_image, diff_pixels)) = get_diff(&expected_image, &actual_image, 0) {
        if diff_pixels > 0 {
            diff_image.save(DIFFS_PATH.clone().join(format!("{}.png", diff_name(name)))).unwrap();

            if option_env!("REPLACE").is_some() {
                std::fs::write(&png_path, pix_png).unwrap();
            }
        }

        diff_pixels as usize
    }   else {
        0
    }
}

pub fn render_extra(name: &str) -> usize {
    render_extra_with_scale(name, 1.0)
}

pub fn render_node(name: &str, id: &str) -> usize {
    let svg_path = format!("../resvg/tests/{}.svg", name);
    let png_path = format!("../resvg/tests/{}.png", name);

    let opt = usvg::Options {
        fontdb: GLOBAL_FONTDB.clone(),
        ..usvg::Options::default()
    };

    let tree = {
        let svg_data = std::fs::read(&svg_path).unwrap();
        usvg::Tree::from_data(&svg_data, &opt).unwrap()
    };

    let node = tree.node_by_id(id).unwrap();
    let size = node.abs_layer_bounding_box().unwrap().size().to_int_size();
    
    let mut ctx = RenderContext::new(size.width() as u16, size.height() as u16);
    let mut pixmap = Pixmap::new(size.width() as u16, size.height() as u16);
    
    resvg_vello::render_node(node, Affine::IDENTITY, &mut ctx);

    ctx.flush();
    ctx.render_to_pixmap(&mut pixmap, RenderMode::OptimizeQuality);

    let pix_png = pixmap.into_png().unwrap();

    let actual_image  = image::load_from_memory(&pix_png).unwrap().to_rgba8();
    let expected_image = image::load_from_memory(&std::fs::read(&png_path).unwrap()).unwrap().to_rgba8();
    
    if let Some((diff_image, diff_pixels)) = get_diff(&expected_image, &actual_image, 0) {
        diff_image.save(DIFFS_PATH.clone().join(format!("{}.png", diff_name(name)))).unwrap();
        
        diff_pixels as usize
    }   else {
        0
    }
}

fn get_diff(
    expected_image: &RgbaImage,
    actual_image: &RgbaImage,
    diff_pixels: u32,
) -> Option<(RgbaImage, u32)> {
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
                    if is_pix_diff(expected, actual, 0) {
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

    if pixel_diff > diff_pixels {
        Some((diff_image, pixel_diff))
    } else {
        None
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
