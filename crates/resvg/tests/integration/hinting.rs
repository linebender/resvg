// Copyright 2026 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Hinting is an `Options` setting rather than something an SVG can ask for, so
// these render the same file the auto-generated `text_hinting_sizes` test uses,
// once per configuration.

use crate::render_hinted;

#[test]
fn smooth() {
    let hinting = usvg::HintingOptions::default();
    assert_eq!(
        render_hinted("tests/text/hinting/sizes", "smooth", hinting),
        0
    );
}

#[test]
fn autohinter() {
    let hinting = usvg::HintingOptions {
        engine: usvg::HintingEngine::Auto,
        ..usvg::HintingOptions::default()
    };
    assert_eq!(
        render_hinted("tests/text/hinting/sizes", "auto", hinting),
        0
    );
}

#[test]
fn autohinter_mono() {
    // The TrueType interpreter barely distinguishes a mono target from a smooth
    // one, so this pairs it with the engine that does.
    let hinting = usvg::HintingOptions {
        engine: usvg::HintingEngine::Auto,
        target: usvg::HintingTarget::Mono,
    };
    assert_eq!(
        render_hinted("tests/text/hinting/sizes", "mono", hinting),
        0
    );
}
