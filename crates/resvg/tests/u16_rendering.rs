#![cfg(feature = "16bpc")]

use resvg::tiny_skia::*;
use resvg::usvg;

#[test]
fn test_resvg_u16_solid_and_opacity() {
    let svg = r##"
    <svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
        <rect x="10" y="10" width="80" height="80" fill="rgb(255, 0, 0)" fill-opacity="0.5"/>
    </svg>
    "##;

    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();
    let mut pixmap = PixmapU16::new(100, 100).unwrap();

    resvg::render_u16(&tree, Transform::identity(), &mut pixmap.as_mut());

    // Center pixel: semi-transparent red (alpha=32768, red=32768, green=0, blue=0)
    let p = pixmap.pixel(50, 50).unwrap();
    assert_eq!(p.alpha(), 32768);
    assert_eq!(p.red(), 32768);
    assert_eq!(p.green(), 0);
    assert_eq!(p.blue(), 0);

    // Outside pixel: transparent
    let out = pixmap.pixel(5, 5).unwrap();
    assert_eq!(out, PremultipliedColorU16::TRANSPARENT);
}

#[test]
fn test_resvg_u16_gradient_sub_8bit_steps() {
    let svg = r##"
    <svg xmlns="http://www.w3.org/2000/svg" width="512" height="10">
        <defs>
            <linearGradient id="subtle" x1="0" y1="0" x2="512" y2="0" gradientUnits="userSpaceOnUse">
                <stop offset="0" stop-color="#000000"/>
                <stop offset="1" stop-color="#010101"/>
            </linearGradient>
        </defs>
        <rect x="0" y="0" width="512" height="10" fill="url(#subtle)"/>
    </svg>
    "##;

    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();
    let mut pixmap = PixmapU16::new(512, 10).unwrap();

    resvg::render_u16(&tree, Transform::identity(), &mut pixmap.as_mut());

    // Assert intermediate values between 0 and 257 (0x0101 in 16-bit)
    let mut intermediate_values = Vec::new();
    for x in 0..512 {
        let p = pixmap.pixel(x, 5).unwrap();
        let r = p.red();
        if r > 0 && r < 257 {
            intermediate_values.push(r);
        }
    }

    assert!(
        !intermediate_values.is_empty(),
        "16-bit pipeline must produce true 16-bit intermediate steps between 0 and 257"
    );
    intermediate_values.dedup();
    assert!(
        intermediate_values.len() > 10,
        "Expected multiple distinct sub-8-bit gradient steps, got {}",
        intermediate_values.len()
    );
}

#[test]
fn test_resvg_u16_layers_clipping_and_masking() {
    let svg = r##"
    <svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
        <clipPath id="clip">
            <circle cx="50" cy="50" r="30"/>
        </clipPath>
        <g clip-path="url(#clip)" opacity="0.8">
            <rect x="0" y="0" width="100" height="100" fill="blue"/>
        </g>
    </svg>
    "##;

    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();
    let mut pixmap = PixmapU16::new(100, 100).unwrap();

    resvg::render_u16(&tree, Transform::identity(), &mut pixmap.as_mut());

    // Center pixel inside circle: blue with 0.8 opacity
    let p_center = pixmap.pixel(50, 50).unwrap();
    assert!(p_center.blue() > 50000);
    assert_eq!(p_center.red(), 0);
    assert_eq!(p_center.green(), 0);

    // Corner pixel outside circle: transparent (clipped out)
    let p_corner = pixmap.pixel(5, 5).unwrap();
    assert_eq!(p_corner, PremultipliedColorU16::TRANSPARENT);
}

#[test]
fn test_resvg_dynamic_dispatch() {
    let svg = r##"
    <svg xmlns="http://www.w3.org/2000/svg" width="64" height="64">
        <rect x="0" y="0" width="64" height="64" fill="#00ff00"/>
    </svg>
    "##;

    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();

    // 8-bit dynamic pixmap
    let mut dyn_u8 = DynamicPixmap::U8(Pixmap::new(64, 64).unwrap());
    resvg::render_dynamic(&tree, Transform::identity(), &mut dyn_u8.as_mut());
    if let DynamicPixmap::U8(ref p) = dyn_u8 {
        let pixel = p.pixel(32, 32).unwrap();
        assert_eq!(pixel.green(), 255);
        assert_eq!(pixel.red(), 0);
    } else {
        panic!("Expected U8 variant");
    }

    // 16-bit dynamic pixmap
    let mut dyn_u16 = DynamicPixmap::U16(PixmapU16::new(64, 64).unwrap());
    resvg::render_dynamic(&tree, Transform::identity(), &mut dyn_u16.as_mut());
    if let DynamicPixmap::U16(ref p) = dyn_u16 {
        let pixel = p.pixel(32, 32).unwrap();
        assert_eq!(pixel.green(), 65535);
        assert_eq!(pixel.red(), 0);
    } else {
        panic!("Expected U16 variant");
    }
}

#[test]
fn test_resvg_u16_png_export() {
    let svg = r##"
    <svg xmlns="http://www.w3.org/2000/svg" width="32" height="32">
        <rect width="32" height="32" fill="#123456"/>
    </svg>
    "##;

    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();
    let mut pixmap = PixmapU16::new(32, 32).unwrap();
    resvg::render_u16(&tree, Transform::identity(), &mut pixmap.as_mut());

    let png_bytes = pixmap.encode_png().expect("PNG encoding must succeed");
    // Verify PNG signature (89 50 4E 47 0D 0A 1A 0A)
    assert_eq!(
        &png_bytes[0..8],
        &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
    );
    // Verify IHDR bit depth is 16 (offset 24 in standard PNG)
    assert_eq!(png_bytes[24], 16, "PNG bit depth must be 16-bit");
    // Verify IHDR color type is 6 (RGBA)
    assert_eq!(png_bytes[25], 6, "PNG color type must be RGBA (6)");
}

#[test]
fn test_resvg_u16_png_full_precision_decode() {
    let svg = r##"
    <svg xmlns="http://www.w3.org/2000/svg" width="512" height="10">
        <defs>
            <linearGradient id="subtle" x1="0" y1="0" x2="512" y2="0" gradientUnits="userSpaceOnUse">
                <stop offset="0" stop-color="#000000"/>
                <stop offset="1" stop-color="#010101"/>
            </linearGradient>
        </defs>
        <rect x="0" y="0" width="512" height="10" fill="url(#subtle)"/>
    </svg>
    "##;

    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();
    let mut pixmap = PixmapU16::new(512, 10).unwrap();
    resvg::render_u16(&tree, Transform::identity(), &mut pixmap.as_mut());

    let png_bytes = pixmap.encode_png().expect("PNG encoding must succeed");

    // Decode with standard png::Decoder
    let cursor = std::io::Cursor::new(png_bytes);
    let decoder = png::Decoder::new(cursor);
    let mut reader = decoder.read_info().expect("PNG decoder read_info failed");
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader
        .next_frame(&mut buf)
        .expect("PNG decoder next_frame failed");

    assert_eq!(info.bit_depth, png::BitDepth::Sixteen);
    assert_eq!(info.color_type, png::ColorType::Rgba);

    // Read 16-bit big-endian words
    let mut u16_red_samples = Vec::new();
    for chunk in buf[..info.buffer_size()].chunks_exact(8) {
        // RGBA64: each channel is 2 bytes big-endian
        let r = u16::from_be_bytes([chunk[0], chunk[1]]);
        u16_red_samples.push(r);
    }

    // Verify presence of intermediate values strictly between 0 and 257 (0x0101)
    let non_quantized: Vec<u16> = u16_red_samples
        .iter()
        .copied()
        .filter(|&v| v > 0 && v < 257)
        .collect();

    assert!(
        !non_quantized.is_empty(),
        "Decoded 16-bit PNG must contain true sub-8-bit intermediate values strictly between 0 and 257"
    );

    let mut distinct = non_quantized.clone();
    distinct.sort_unstable();
    distinct.dedup();

    assert!(
        distinct.len() > 10,
        "Decoded 16-bit PNG contains {} distinct sub-8-bit discrete levels between 0 and 257",
        distinct.len()
    );

    // Verify non-multiples of 257 (proves absence of 8-bit discrete step quantization)
    let non_multiples_of_257 = u16_red_samples
        .iter()
        .filter(|&&v| v > 0 && v % 257 != 0)
        .count();
    assert!(
        non_multiples_of_257 > 0,
        "Decoded 16-bit PNG contains values not divisible by 257"
    );
}

#[test]
fn test_resvg_test_svg_16bit_png_decode() {
    let svg_content = r##"<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="256"><defs><linearGradient id="g" x1="0%" y1="0%" x2="100%" y2="0%"><stop offset="0%" stop-color="#000000"/><stop offset="100%" stop-color="#ffffff"/></linearGradient></defs><rect width="1024" height="256" fill="url(#g)"/></svg>"##;
    let tree = usvg::Tree::from_str(svg_content, &usvg::Options::default()).unwrap();
    let mut pixmap = PixmapU16::new(1024, 256).unwrap();
    resvg::render_u16(&tree, Transform::identity(), &mut pixmap.as_mut());

    let png_bytes = pixmap.encode_png().expect("PNG encoding must succeed");

    let cursor = std::io::Cursor::new(png_bytes);
    let decoder = png::Decoder::new(cursor);
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();

    assert_eq!(info.bit_depth, png::BitDepth::Sixteen);
    assert_eq!(info.color_type, png::ColorType::Rgba);

    // Collect distinct 16-bit channel values and (r, g, b) tuples from decoded PNG
    let mut distinct_reds = std::collections::BTreeSet::new();
    let mut distinct_greens = std::collections::BTreeSet::new();
    let mut distinct_blues = std::collections::BTreeSet::new();
    let mut distinct_colors = std::collections::BTreeSet::new();

    for chunk in buf[..info.buffer_size()].chunks_exact(8) {
        let r = u16::from_be_bytes([chunk[0], chunk[1]]);
        let g = u16::from_be_bytes([chunk[2], chunk[3]]);
        let b = u16::from_be_bytes([chunk[4], chunk[5]]);
        distinct_reds.insert(r);
        distinct_greens.insert(g);
        distinct_blues.insert(b);
        distinct_colors.insert((r, g, b));
    }

    // Also render in 8-bit to verify strict precision improvement
    let mut u8_pixmap = tiny_skia::Pixmap::new(512, 256).unwrap();
    resvg::render(&tree, Transform::identity(), &mut u8_pixmap.as_mut());
    let mut u8_distinct_reds = std::collections::BTreeSet::new();
    let mut u8_distinct_colors = std::collections::BTreeSet::new();
    for pixel in u8_pixmap.pixels() {
        u8_distinct_reds.insert(pixel.red());
        u8_distinct_colors.insert((pixel.red(), pixel.green(), pixel.blue()));
    }

    assert!(
        distinct_reds.len() > 256,
        "Decoded 16-bit PNG contains {} distinct red values (> 256 proves sub-8-bit precision)",
        distinct_reds.len()
    );
    assert!(
        distinct_reds.len() > u8_distinct_reds.len() * 3,
        "16-bit PNG has {} distinct red values vs {} in 8-bit (> 3x more gradations)",
        distinct_reds.len(),
        u8_distinct_reds.len()
    );
    assert!(
        distinct_colors.len() > u8_distinct_colors.len(),
        "16-bit PNG has {} unique colors vs {} in 8-bit",
        distinct_colors.len(),
        u8_distinct_colors.len()
    );
}

#[test]
fn test_resvg_u16_component_transfer_gamma_precision() {
    let svg = r##"
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 64" width="512" height="64">
      <defs>
        <linearGradient id="grad" x1="0" y1="0" x2="512" y2="0" gradientUnits="userSpaceOnUse">
          <stop offset="0" stop-color="#000000" />
          <stop offset="1" stop-color="#808080" />
        </linearGradient>
        <filter id="gamma">
          <feComponentTransfer>
            <feFuncR type="gamma" amplitude="1" exponent="1.5" offset="0"/>
            <feFuncG type="gamma" amplitude="1" exponent="1.5" offset="0"/>
            <feFuncB type="gamma" amplitude="1" exponent="1.5" offset="0"/>
          </feComponentTransfer>
        </filter>
      </defs>
      <rect width="512" height="64" fill="url(#grad)" filter="url(#gamma)"/>
    </svg>
    "##;

    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();
    let mut pixmap = tiny_skia::PixmapU16::new(512, 64).unwrap();
    resvg::render_u16(&tree, Transform::identity(), &mut pixmap.as_mut());

    let mut distinct_reds = std::collections::BTreeSet::new();
    for pixel in pixmap.pixels() {
        let c = pixel.demultiply();
        distinct_reds.insert(c.red());
    }

    // In 8-bit, #00 to #20 (32 levels) has at most 33 discrete values.
    // In 16-bit, it generates hundreds of unique intermediate steps!
    assert!(
        distinct_reds.len() > 100,
        "Must have > 100 distinct red levels, got {}",
        distinct_reds.len()
    );
    let sub_8bit_steps = distinct_reds.iter().filter(|&&v| v % 257 != 0).count();
    assert!(
        sub_8bit_steps > 0,
        "Must contain true 16-bit sub-8-bit intermediate steps"
    );
}

#[test]
fn test_resvg_u16_color_matrix_precision() {
    let svg = r##"
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 64" width="512" height="64">
      <defs>
        <linearGradient id="grad" x1="0" y1="0" x2="512" y2="0" gradientUnits="userSpaceOnUse">
          <stop offset="0" stop-color="#000000" />
          <stop offset="1" stop-color="#808080" />
        </linearGradient>
        <filter id="matrix">
          <feColorMatrix type="matrix" values="
            0.5 0.5 0 0 0
            0 0.5 0.5 0 0
            0.5 0 0.5 0 0
            0 0 0 1 0" />
        </filter>
      </defs>
      <rect width="512" height="64" fill="url(#grad)" filter="url(#matrix)"/>
    </svg>
    "##;

    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();
    let mut pixmap = tiny_skia::PixmapU16::new(512, 64).unwrap();
    resvg::render_u16(&tree, Transform::identity(), &mut pixmap.as_mut());

    let mut distinct_reds = std::collections::BTreeSet::new();
    for pixel in pixmap.pixels() {
        let c = pixel.demultiply();
        distinct_reds.insert(c.red());
    }

    assert!(
        distinct_reds.len() > 256,
        "Must exceed 256 discrete levels in 16bpc, got {}",
        distinct_reds.len()
    );
}

#[test]
fn test_resvg_u16_turbulence_precision() {
    let svg = r##"
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" width="256" height="256">
      <defs>
        <filter id="noise">
          <feTurbulence type="fractalNoise" baseFrequency="0.05" numOctaves="2" result="turb"/>
        </filter>
      </defs>
      <rect width="256" height="256" filter="url(#noise)"/>
    </svg>
    "##;

    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();
    let mut pixmap = tiny_skia::PixmapU16::new(256, 256).unwrap();
    resvg::render_u16(&tree, Transform::identity(), &mut pixmap.as_mut());

    let mut distinct_reds = std::collections::BTreeSet::new();
    for pixel in pixmap.pixels() {
        let c = pixel.demultiply();
        distinct_reds.insert(c.red());
    }

    assert!(
        distinct_reds.len() > 1000,
        "16bpc Turbulence must generate > 1000 discrete levels, got {}",
        distinct_reds.len()
    );
    let sub_8bit_steps = distinct_reds.iter().filter(|&&v| v % 257 != 0).count();
    assert!(
        sub_8bit_steps > 0,
        "Must contain true 16-bit sub-8-bit intermediate steps"
    );
}

#[test]
fn test_resvg_u16_luminance_masking_precision() {
    let svg = r##"
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 64" width="512" height="64">
      <defs>
        <linearGradient id="maskGrad" x1="0" y1="0" x2="512" y2="0" gradientUnits="userSpaceOnUse">
          <stop offset="0" stop-color="#000000" />
          <stop offset="1" stop-color="#ffffff" />
        </linearGradient>
        <mask id="lumaMask" mask-type="luminance">
          <rect width="512" height="64" fill="url(#maskGrad)"/>
        </mask>
      </defs>
      <rect width="512" height="64" fill="#ff0000" mask="url(#lumaMask)"/>
    </svg>
    "##;

    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();
    let mut pixmap = tiny_skia::PixmapU16::new(512, 64).unwrap();
    resvg::render_u16(&tree, Transform::identity(), &mut pixmap.as_mut());

    let mut distinct_alphas = std::collections::BTreeSet::new();
    for pixel in pixmap.pixels() {
        distinct_alphas.insert(pixel.alpha());
    }

    assert!(
        distinct_alphas.len() > 400,
        "16bpc Luminance mask must preserve > 400 discrete alpha steps, got {}",
        distinct_alphas.len()
    );
    let sub_8bit_steps = distinct_alphas.iter().filter(|&&v| v % 257 != 0).count();
    assert!(
        sub_8bit_steps > 0,
        "Must contain true 16-bit intermediate steps"
    );
}

#[test]
fn test_blue_noise_dither_smooth_gradient() {
    let svg = r##"
    <svg xmlns="http://www.w3.org/2000/svg" width="512" height="64">
        <defs>
            <linearGradient id="shallow" x1="0" y1="0" x2="512" y2="0" gradientUnits="userSpaceOnUse">
                <stop offset="0" stop-color="#000000"/>
                <stop offset="1" stop-color="#010101"/>
            </linearGradient>
        </defs>
        <rect x="0" y="0" width="512" height="64" fill="url(#shallow)"/>
    </svg>
    "##;

    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();
    let mut u16_pixmap = PixmapU16::new(512, 64).unwrap();
    resvg::render_u16(&tree, Transform::identity(), &mut u16_pixmap.as_mut());

    // Compare raw truncated downsample vs blue-noise dithered
    let raw_u8 = resvg::dither::downsample_u16_to_u8(&u16_pixmap);
    let dithered_u8 = resvg::dither::dither_u16_to_u8(&u16_pixmap);

    // In raw truncation, all columns below 256 are 0, and >= 256 are 1
    assert_eq!(raw_u8.pixel(100, 32).unwrap().red(), 0);
    assert_eq!(raw_u8.pixel(400, 32).unwrap().red(), 1);

    // In blue noise dithered, the ratio of 1s to 0s increases smoothly with x
    let mut col_densities = Vec::new();
    for x in (0..512).step_by(64) {
        let mut count_ones = 0;
        for y in 0..64 {
            if dithered_u8.pixel(x, y).unwrap().red() == 1 {
                count_ones += 1;
            }
        }
        col_densities.push(count_ones as f32 / 64.0);
    }

    // Densities should be strictly increasing across the gradient
    for window in col_densities.windows(2) {
        assert!(
            window[1] >= window[0],
            "Dither density must monotonically increase from left to right: {:?} -> {:?}",
            window[0],
            window[1]
        );
    }
    assert!(col_densities.first().unwrap() < &0.1);
    assert!(col_densities.last().unwrap() > &0.8);
}

#[test]
fn test_blue_noise_channel_decorrelation() {
    let svg = r##"
    <svg xmlns="http://www.w3.org/2000/svg" width="256" height="64">
        <defs>
            <linearGradient id="shallow_gray" x1="0" y1="0" x2="256" y2="0" gradientUnits="userSpaceOnUse">
                <stop offset="0" stop-color="#000000"/>
                <stop offset="1" stop-color="#010101"/>
            </linearGradient>
        </defs>
        <rect x="0" y="0" width="256" height="64" fill="url(#shallow_gray)"/>
    </svg>
    "##;

    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();
    let mut u16_pixmap = PixmapU16::new(256, 64).unwrap();
    resvg::render_u16(&tree, Transform::identity(), &mut u16_pixmap.as_mut());

    let dithered = resvg::dither::dither_u16_to_u8(&u16_pixmap);

    // Check that R, G, B channels do not all switch on identical pixels (decorrelation)
    let mut different_channels_count = 0;
    for y in 0..64 {
        for x in 0..256 {
            let p = dithered.pixel(x, y).unwrap();
            if p.red() != p.green() || p.green() != p.blue() {
                different_channels_count += 1;
            }
        }
    }

    assert!(
        different_channels_count > 1000,
        "Decorrelated blue noise should produce independent channel dithering, got {} different pixels",
        different_channels_count
    );
}

#[test]
fn test_render_u16_dithered_public_api() {
    let svg = r##"
    <svg xmlns="http://www.w3.org/2000/svg" width="128" height="64">
        <rect width="128" height="64" fill="#123456"/>
    </svg>
    "##;

    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();
    let mut pixmap = Pixmap::new(128, 64).unwrap();
    resvg::render_u16_dithered(&tree, Transform::identity(), &mut pixmap.as_mut());

    let p = pixmap.pixel(64, 32).unwrap();
    assert_eq!(p.red(), 0x12);
    assert_eq!(p.green(), 0x34);
    assert_eq!(p.blue(), 0x56);
    assert_eq!(p.alpha(), 255);
}

#[test]
fn test_cli_output_modes() {
    let cargo_bin = env!("CARGO_BIN_EXE_resvg");
    let test_dir = std::env::temp_dir().join("resvg_dither_cli_test");
    std::fs::create_dir_all(&test_dir).unwrap();

    let in_svg = test_dir.join("test.svg");
    let svg_content = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><rect width="64" height="64" fill="#123456"/></svg>"##;
    std::fs::write(&in_svg, svg_content).unwrap();

    // 1. Test --16bpc (defaults to 16bpc output)
    let out_16 = test_dir.join("out_16.png");
    let status = std::process::Command::new(cargo_bin)
        .args([
            in_svg.to_str().unwrap(),
            out_16.to_str().unwrap(),
            "--16bpc",
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let png_bytes = std::fs::read(&out_16).unwrap();
    assert_eq!(png_bytes[24], 16, "Must be 16-bit PNG");

    // 2. Test --bit-depth 16 --output-bit-depth 8 --dither
    let out_dither = test_dir.join("out_dither.png");
    let status = std::process::Command::new(cargo_bin)
        .args([
            in_svg.to_str().unwrap(),
            out_dither.to_str().unwrap(),
            "--bit-depth",
            "16",
            "--output-bit-depth",
            "8",
            "--dither",
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let png_bytes = std::fs::read(&out_dither).unwrap();
    assert_eq!(png_bytes[24], 8, "Must be 8-bit PNG");

    // 3. Test --dither shortcut flag
    let out_dither_flag = test_dir.join("out_dither_flag.png");
    let status = std::process::Command::new(cargo_bin)
        .args([
            in_svg.to_str().unwrap(),
            out_dither_flag.to_str().unwrap(),
            "--dither",
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let png_bytes = std::fs::read(&out_dither_flag).unwrap();
    assert_eq!(png_bytes[24], 8, "Must be 8-bit PNG");

    // 4. Test --bit-depth 16 --output-bit-depth 8 (non-dithered downsample)
    let out_8 = test_dir.join("out_8.png");
    let status = std::process::Command::new(cargo_bin)
        .args([
            in_svg.to_str().unwrap(),
            out_8.to_str().unwrap(),
            "--bit-depth",
            "16",
            "--output-bit-depth",
            "8",
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let png_bytes = std::fs::read(&out_8).unwrap();
    assert_eq!(png_bytes[24], 8, "Must be 8-bit PNG");

    // 5. Test default 8bpc rendering
    let out_default = test_dir.join("out_default.png");
    let status = std::process::Command::new(cargo_bin)
        .args([in_svg.to_str().unwrap(), out_default.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(status.success());
    let png_bytes = std::fs::read(&out_default).unwrap();
    assert_eq!(png_bytes[24], 8, "Must be 8-bit PNG");

    let _ = std::fs::remove_dir_all(&test_dir);
}
