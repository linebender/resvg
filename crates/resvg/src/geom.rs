// Copyright 2023 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// Converts a filter region into pixels, rounding its edges outwards, but taking an edge within
/// a thousandth of a pixel of an integer as that integer, as Skia's image filters do
/// (`RoundOut`), so that the float error of a transform does not add a row or a column of pixels.
pub fn to_int_rect(r: tiny_skia::NonZeroRect) -> Option<tiny_skia::IntRect> {
    const ROUND_EPSILON: f32 = 1e-3;
    let r = r.to_rect();
    r.inset(ROUND_EPSILON, ROUND_EPSILON)
        .unwrap_or(r)
        .round_out()
}

/// Fits the current rect into the specified bounds.
pub fn fit_to_rect(
    r: tiny_skia::IntRect,
    bounds: tiny_skia::IntRect,
) -> Option<tiny_skia::IntRect> {
    let mut left = r.left();
    if left < bounds.left() {
        left = bounds.left();
    }

    let mut top = r.top();
    if top < bounds.top() {
        top = bounds.top();
    }

    let mut right = r.right();
    if right > bounds.right() {
        right = bounds.right();
    }

    let mut bottom = r.bottom();
    if bottom > bounds.bottom() {
        bottom = bounds.bottom();
    }

    tiny_skia::IntRect::from_ltrb(left, top, right, bottom)
}
