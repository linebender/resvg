use std::time::Instant;
use tiny_skia::Transform;

fn is_16bpc() -> bool {
    std::env::var("RESVG_BENCH_16BPC").is_ok()
}

fn bench_svg(name: &str, svg_str: &str, width: u32, height: u32, iterations: usize) {
    let tree = usvg::Tree::from_str(svg_str, &usvg::Options::default()).unwrap();
    let is_16 = is_16bpc();

    #[cfg(feature = "16bpc")]
    if is_16 {
        let mut pixmap = tiny_skia::PixmapU16::new(width, height).unwrap();
        resvg::render_u16(&tree, Transform::identity(), &mut pixmap.as_mut());

        let start = Instant::now();
        let mut pixmap = tiny_skia::PixmapU16::new(width, height).unwrap();
        for _ in 0..iterations {
            pixmap.fill(tiny_skia::Color::TRANSPARENT);
            resvg::render_u16(&tree, Transform::identity(), &mut pixmap.as_mut());
        }
        let duration = start.elapsed();
        let per_iter = duration / iterations as u32;

        println!(
            "[16bpc] {} ({}x{}) -> total: {:?}, per iter: {:?}",
            name, width, height, duration, per_iter
        );
        return;
    }

    let mut pixmap = tiny_skia::Pixmap::new(width, height).unwrap();
    resvg::render(&tree, Transform::identity(), &mut pixmap.as_mut());

    let start = Instant::now();
    let mut pixmap = tiny_skia::Pixmap::new(width, height).unwrap();
    for _ in 0..iterations {
        pixmap.fill(tiny_skia::Color::TRANSPARENT);
        resvg::render(&tree, Transform::identity(), &mut pixmap.as_mut());
    }
    let duration = start.elapsed();
    let per_iter = duration / iterations as u32;

    println!(
        "[ 8bpc] {} ({}x{}) -> total: {:?}, per iter: {:?}",
        name, width, height, duration, per_iter
    );
}

fn main() {
    println!("=== Resvg Rendering Benchmarks (16bpc switch: RESVG_BENCH_16BPC) ===");
    let is_16 = is_16bpc();
    println!(
        "Pipeline Mode: {}\n",
        if is_16 {
            "16-Bit Per Channel (u16)"
        } else {
            "8-Bit Per Channel (u8)"
        }
    );

    // 1. Color Space & feColorMatrix Benchmark (exercises sRGB <-> LinearRGB + Color Matrix)
    let svg_color_matrix = r##"
    <svg xmlns="http://www.w3.org/2000/svg" width="500" height="500">
        <filter id="f1" color-interpolation-filters="linearRGB" x="0%" y="0%" width="100%" height="100%">
            <feColorMatrix type="matrix" values="
                0.5 0.5 0.0 0.0 0.1
                0.0 0.8 0.2 0.0 0.1
                0.2 0.2 0.6 0.0 0.1
                0.0 0.0 0.0 1.0 0.0" />
            <feColorMatrix type="saturate" values="1.5" />
        </filter>
        <rect x="20" y="20" width="460" height="460" fill="orange" filter="url(#f1)"/>
    </svg>
    "##;
    bench_svg(
        "filter_color_matrix_linear_rgb",
        svg_color_matrix,
        500,
        500,
        50,
    );

    // 2. Arithmetic Composite Filter Benchmark
    let svg_composite = r##"
    <svg xmlns="http://www.w3.org/2000/svg" width="500" height="500">
        <filter id="f2" x="0%" y="0%" width="100%" height="100%">
            <feFlood flood-color="cyan" flood-opacity="0.8" result="flood"/>
            <feComposite in2="flood" operator="arithmetic" k1="0.5" k2="0.3" k3="0.3" k4="0.1"/>
        </filter>
        <rect x="10" y="10" width="480" height="480" fill="purple" filter="url(#f2)"/>
    </svg>
    "##;
    bench_svg("filter_composite_arithmetic", svg_composite, 500, 500, 50);

    // 3. Gaussian Blur Filter Benchmark
    let svg_blur = r##"
    <svg xmlns="http://www.w3.org/2000/svg" width="500" height="500">
        <filter id="f3" x="-20%" y="-20%" width="140%" height="140%">
            <feGaussianBlur stdDeviation="6"/>
        </filter>
        <circle cx="250" cy="250" r="180" fill="crimson" filter="url(#f3)"/>
    </svg>
    "##;
    bench_svg("filter_gaussian_blur_sigma6", svg_blur, 500, 500, 50);

    // 4. Lighting Filter Benchmark
    let svg_lighting = r##"
    <svg xmlns="http://www.w3.org/2000/svg" width="400" height="400">
        <filter id="f4" x="0%" y="0%" width="100%" height="100%">
            <feDiffuseLighting lighting-color="white" surfaceScale="3" diffuseConstant="1.2">
                <feDistantLight azimuth="45" elevation="60"/>
            </feDiffuseLighting>
        </filter>
        <rect x="20" y="20" width="360" height="360" fill="green" filter="url(#f4)"/>
    </svg>
    "##;
    bench_svg("filter_diffuse_lighting", svg_lighting, 400, 400, 30);

    // 5. Layer Compositing & Blend Modes Benchmark
    let svg_layers = r##"
    <svg xmlns="http://www.w3.org/2000/svg" width="500" height="500">
        <rect x="0" y="0" width="500" height="500" fill="#202040"/>
        <g opacity="0.85" style="mix-blend-mode: multiply;">
            <circle cx="200" cy="200" r="150" fill="#ff4080"/>
        </g>
        <g opacity="0.85" style="mix-blend-mode: screen;">
            <circle cx="300" cy="200" r="150" fill="#40ff80"/>
        </g>
        <g opacity="0.85" style="mix-blend-mode: overlay;">
            <circle cx="250" cy="300" r="150" fill="#4080ff"/>
        </g>
    </svg>
    "##;
    bench_svg("layer_blend_modes_compositing", svg_layers, 500, 500, 50);

    // 6. Complex SVG Gradients & Vector Paths
    let svg_gradients = r##"
    <svg xmlns="http://www.w3.org/2000/svg" width="600" height="600">
        <defs>
            <linearGradient id="g1" x1="0%" y1="0%" x2="100%" y2="100%">
                <stop offset="0%" stop-color="#ff0080"/>
                <stop offset="50%" stop-color="#ff8c00"/>
                <stop offset="100%" stop-color="#40e0d0"/>
            </linearGradient>
            <radialGradient id="g2" cx="50%" cy="50%" r="50%">
                <stop offset="0%" stop-color="#ffff00" stop-opacity="1"/>
                <stop offset="100%" stop-color="#0000ff" stop-opacity="0"/>
            </radialGradient>
        </defs>
        <rect x="50" y="50" width="500" height="500" rx="30" fill="url(#g1)"/>
        <circle cx="300" cy="300" r="220" fill="url(#g2)"/>
    </svg>
    "##;
    bench_svg("complex_gradients_and_paths", svg_gradients, 600, 600, 50);
}

#[test]
fn run_benchmarks() {
    main();
}
