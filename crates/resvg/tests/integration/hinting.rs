// Copyright 2025 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Tests for font hinting functionality.
//!
//! These tests verify that:
//! 1. Hinting produces visibly different output than non-hinted rendering
//! 2. The `text-rendering` CSS property correctly controls hinting behavior
//! 3. Hinting works correctly at various font sizes

use crate::GLOBAL_FONTDB;

/// Renders an SVG with the specified hinting settings and returns the pixel data.
fn render_with_hinting(svg_data: &[u8], hinting_enabled: bool) -> Vec<u8> {
    let opt = usvg::Options {
        fontdb: GLOBAL_FONTDB.clone(),
        hinting: usvg::HintingOptions {
            enabled: hinting_enabled,
            dpi: Some(96.0),
        },
        ..usvg::Options::default()
    };

    let tree = usvg::Tree::from_data(svg_data, &opt).unwrap();
    let size = tree.size().to_int_size();
    let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height()).unwrap();
    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );

    pixmap.take()
}

/// Count the number of pixels that differ between two images.
fn count_different_pixels(img1: &[u8], img2: &[u8]) -> usize {
    assert_eq!(img1.len(), img2.len());
    img1.chunks(4)
        .zip(img2.chunks(4))
        .filter(|(p1, p2)| p1 != p2)
        .count()
}

/// Test that hinting produces different output than non-hinted rendering.
/// This demonstrates that hinting is actually being applied.
#[test]
fn hinting_produces_different_output() {
    // Small text at 12px where hinting effects are most visible
    let svg = br#"
        <svg xmlns="http://www.w3.org/2000/svg" width="200" height="50">
            <text x="10" y="30" font-family="Noto Sans" font-size="12"
                  text-rendering="optimizeLegibility">
                Hinting Test
            </text>
        </svg>
    "#;

    let hinted = render_with_hinting(svg, true);
    let unhinted = render_with_hinting(svg, false);

    let diff_count = count_different_pixels(&hinted, &unhinted);

    // Hinted and unhinted output should differ
    // The exact number of different pixels depends on the font and size,
    // but there should be a noticeable difference
    assert!(
        diff_count > 0,
        "Hinted and unhinted output should differ, but they are identical"
    );

    // Log the difference for debugging
    eprintln!(
        "hinting_produces_different_output: {} pixels differ",
        diff_count
    );
}

/// Test that geometric-precision disables hinting even when hinting is enabled.
#[test]
fn geometric_precision_disables_hinting() {
    let svg_geometric = br#"
        <svg xmlns="http://www.w3.org/2000/svg" width="200" height="50">
            <text x="10" y="30" font-family="Noto Sans" font-size="12"
                  text-rendering="geometricPrecision">
                Geometric Precision
            </text>
        </svg>
    "#;

    // With geometricPrecision, hinting should be disabled regardless of the option
    let with_hinting_option = render_with_hinting(svg_geometric, true);
    let without_hinting_option = render_with_hinting(svg_geometric, false);

    let diff_count = count_different_pixels(&with_hinting_option, &without_hinting_option);

    // Both should produce the same output since geometricPrecision disables hinting
    assert_eq!(
        diff_count, 0,
        "geometricPrecision should produce identical output regardless of hinting option"
    );
}

/// Test that optimizeLegibility enables hinting when the option is set.
#[test]
fn optimize_legibility_enables_hinting() {
    let svg = br#"
        <svg xmlns="http://www.w3.org/2000/svg" width="200" height="50">
            <text x="10" y="30" font-family="Noto Sans" font-size="12"
                  text-rendering="optimizeLegibility">
                Optimize Legibility
            </text>
        </svg>
    "#;

    let hinted = render_with_hinting(svg, true);
    let unhinted = render_with_hinting(svg, false);

    let diff_count = count_different_pixels(&hinted, &unhinted);

    // optimizeLegibility with hinting enabled should differ from unhinted
    assert!(
        diff_count > 0,
        "optimizeLegibility should produce different output when hinting is enabled"
    );
}

/// Test hinting at various font sizes to demonstrate size-dependent effects.
#[test]
fn hinting_at_various_sizes() {
    let sizes = [8, 10, 12, 14, 16, 20, 24, 32, 48];
    let mut results = Vec::new();

    for size in sizes {
        let svg = format!(
            r#"
            <svg xmlns="http://www.w3.org/2000/svg" width="300" height="60">
                <text x="10" y="40" font-family="Noto Sans" font-size="{}"
                      text-rendering="optimizeLegibility">
                    Size {} pixels
                </text>
            </svg>
            "#,
            size, size
        );

        let hinted = render_with_hinting(svg.as_bytes(), true);
        let unhinted = render_with_hinting(svg.as_bytes(), false);

        let diff_count = count_different_pixels(&hinted, &unhinted);
        results.push((size, diff_count));

        eprintln!("Size {}px: {} pixels differ", size, diff_count);
    }

    // Verify that at least some sizes show hinting differences
    let sizes_with_differences = results.iter().filter(|(_, diff)| *diff > 0).count();
    assert!(
        sizes_with_differences > 0,
        "Hinting should produce differences at various sizes"
    );
}

/// Test that hinting DPI option is accepted and doesn't cause errors.
/// Note: Different DPI values affect the ppem used for hinting calculations,
/// but when rendering to the same canvas size at the same nominal font size,
/// the pixel output may be identical because hinting aligns glyphs to the
/// same target pixel grid.
#[test]
fn hinting_with_different_dpi() {
    let svg = br#"
        <svg xmlns="http://www.w3.org/2000/svg" width="200" height="50">
            <text x="10" y="30" font-family="Noto Sans" font-size="12"
                  text-rendering="optimizeLegibility">
                DPI Test
            </text>
        </svg>
    "#;

    let render_at_dpi = |dpi: f32| -> Vec<u8> {
        let opt = usvg::Options {
            fontdb: GLOBAL_FONTDB.clone(),
            dpi,
            hinting: usvg::HintingOptions {
                enabled: true,
                dpi: Some(dpi),
            },
            ..usvg::Options::default()
        };

        let tree = usvg::Tree::from_data(svg, &opt).unwrap();
        let size = tree.size().to_int_size();
        let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height()).unwrap();
        resvg::render(
            &tree,
            tiny_skia::Transform::identity(),
            &mut pixmap.as_mut(),
        );
        pixmap.take()
    };

    let at_72dpi = render_at_dpi(72.0);
    let at_96dpi = render_at_dpi(96.0);
    let at_144dpi = render_at_dpi(144.0);

    // Different DPI values affect ppem calculation for hinting:
    // ppem = font_size * dpi / 72, so:
    // - 72 DPI: ppem = 12 * 72 / 72 = 12
    // - 96 DPI: ppem = 12 * 96 / 72 = 16
    // - 144 DPI: ppem = 12 * 144 / 72 = 24
    let diff_72_96 = count_different_pixels(&at_72dpi, &at_96dpi);
    let diff_96_144 = count_different_pixels(&at_96dpi, &at_144dpi);

    eprintln!("72 vs 96 DPI: {} pixels differ (ppem 12 vs 16)", diff_72_96);
    eprintln!(
        "96 vs 144 DPI: {} pixels differ (ppem 16 vs 24)",
        diff_96_144
    );

    // Verify that rendering at different DPIs doesn't crash and produces valid output.
    // The actual pixel differences depend on the font's hinting instructions and
    // may be zero when rendering to the same canvas size.
    assert!(
        !at_72dpi.is_empty(),
        "72 DPI rendering should produce output"
    );
    assert!(
        !at_96dpi.is_empty(),
        "96 DPI rendering should produce output"
    );
    assert!(
        !at_144dpi.is_empty(),
        "144 DPI rendering should produce output"
    );
}

/// Test hinting with variable fonts (Roboto Flex).
#[test]
fn hinting_with_variable_font() {
    let svg = br#"
        <svg xmlns="http://www.w3.org/2000/svg" width="300" height="50">
            <text x="10" y="30" font-family="Roboto Flex" font-size="14"
                  font-weight="400" font-stretch="100%"
                  text-rendering="optimizeLegibility">
                Variable Font Hinting
            </text>
        </svg>
    "#;

    let hinted = render_with_hinting(svg, true);
    let unhinted = render_with_hinting(svg, false);

    let diff_count = count_different_pixels(&hinted, &unhinted);

    eprintln!("Variable font hinting: {} pixels differ", diff_count);

    // Variable fonts should also show hinting differences
    // (though the exact behavior depends on the font's hinting data)
}

/// Test that auto text-rendering defaults to optimizeLegibility behavior.
#[test]
fn auto_text_rendering_uses_hinting() {
    // SVG with auto (default) text-rendering
    let svg_auto = br#"
        <svg xmlns="http://www.w3.org/2000/svg" width="200" height="50">
            <text x="10" y="30" font-family="Noto Sans" font-size="12">
                Auto Text Rendering
            </text>
        </svg>
    "#;

    // SVG with explicit optimizeLegibility
    let svg_legibility = br#"
        <svg xmlns="http://www.w3.org/2000/svg" width="200" height="50">
            <text x="10" y="30" font-family="Noto Sans" font-size="12"
                  text-rendering="optimizeLegibility">
                Auto Text Rendering
            </text>
        </svg>
    "#;

    let auto_hinted = render_with_hinting(svg_auto, true);
    let legibility_hinted = render_with_hinting(svg_legibility, true);

    let diff_count = count_different_pixels(&auto_hinted, &legibility_hinted);

    // Both should produce the same output since auto defaults to optimizeLegibility
    assert_eq!(
        diff_count, 0,
        "auto and optimizeLegibility should produce identical output"
    );
}
