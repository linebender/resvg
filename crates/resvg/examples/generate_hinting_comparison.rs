//! Run with: cargo run --example generate_hinting_comparison --features text,hinting

use std::sync::Arc;
use usvg::fontdb;

fn main() {
    // Load fonts
    let mut fontdb = fontdb::Database::new();
    fontdb.load_fonts_dir("crates/resvg/tests/fonts");
    fontdb.set_sans_serif_family("Noto Sans");
    let fontdb = Arc::new(fontdb);

    // SVG with small text where hinting is most visible
    let svg = br#"
        <svg xmlns="http://www.w3.org/2000/svg" width="400" height="200" style="background: white">
            <text x="10" y="30" font-family="Noto Sans" font-size="12" text-rendering="optimizeLegibility">
                The quick brown fox jumps over the lazy dog. (12px)
            </text>
            <text x="10" y="60" font-family="Noto Sans" font-size="14" text-rendering="optimizeLegibility">
                The quick brown fox jumps over the lazy dog. (14px)
            </text>
            <text x="10" y="95" font-family="Noto Sans" font-size="16" text-rendering="optimizeLegibility">
                The quick brown fox jumps over the lazy dog. (16px)
            </text>
            <text x="10" y="135" font-family="Noto Sans" font-size="20" text-rendering="optimizeLegibility">
                The quick brown fox jumps over. (20px)
            </text>
            <text x="10" y="180" font-family="Noto Sans" font-size="24" text-rendering="optimizeLegibility">
                The quick brown fox. (24px)
            </text>
        </svg>
    "#;

    // Render with hinting
    let opt_hinted = usvg::Options {
        fontdb: fontdb.clone(),
        hinting: usvg::HintingOptions {
            enabled: true,
            dpi: Some(96.0),
        },
        ..usvg::Options::default()
    };

    let tree = usvg::Tree::from_data(svg, &opt_hinted).unwrap();
    let size = tree.size().to_int_size();
    let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height()).unwrap();
    pixmap.fill(tiny_skia::Color::WHITE);
    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    pixmap.save_png("hinted.png").unwrap();
    println!("Saved hinted.png");

    // Render without hinting
    let opt_unhinted = usvg::Options {
        fontdb: fontdb.clone(),
        hinting: usvg::HintingOptions {
            enabled: false,
            dpi: Some(96.0),
        },
        ..usvg::Options::default()
    };

    let tree = usvg::Tree::from_data(svg, &opt_unhinted).unwrap();
    let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height()).unwrap();
    pixmap.fill(tiny_skia::Color::WHITE);
    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    pixmap.save_png("unhinted.png").unwrap();
    println!("Saved unhinted.png");

    println!("Done! Compare hinted.png and unhinted.png");
}
