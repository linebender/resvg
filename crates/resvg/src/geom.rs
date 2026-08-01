// Copyright 2023 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

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

/// Converts a `NonZeroRect` into an `IntRect` while clamping it to the specified bounds.
///
/// Unlike `NonZeroRect::to_int_rect`, doesn't panic when the rect
/// has coordinates outside the `i32` range.
/// Returns `None` when the rect doesn't overlap `bounds`.
pub fn clamped_int_rect(
    r: tiny_skia::NonZeroRect,
    bounds: tiny_skia::IntRect,
) -> Option<tiny_skia::IntRect> {
    // An f64 can represent any i32 exactly.
    let left = f64::from(r.left());
    let top = f64::from(r.top());
    let right = f64::from(r.right());
    let bottom = f64::from(r.bottom());

    let bounds_left = f64::from(bounds.left());
    let bounds_top = f64::from(bounds.top());
    let bounds_right = f64::from(bounds.right());
    let bounds_bottom = f64::from(bounds.bottom());

    if left >= bounds_right || right <= bounds_left || top >= bounds_bottom || bottom <= bounds_top
    {
        return None;
    }

    let left = left.max(bounds_left);
    let top = top.max(bounds_top);
    let right = right.min(bounds_right);
    let bottom = bottom.min(bounds_bottom);

    // The rounding matches `NonZeroRect::to_int_rect`
    tiny_skia::IntRect::from_xywh(
        left.floor() as i32,
        top.floor() as i32,
        ((right - left).ceil() as u32).max(1),
        ((bottom - top).ceil() as u32).max(1),
    )
}
