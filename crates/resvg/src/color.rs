// Copyright 2026 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// Converts a normalized, unpremultiplied sRGB channel to linear RGB.
pub(crate) fn srgb_to_linear(value: f32) -> f32 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

/// Converts a normalized, unpremultiplied linear RGB channel to sRGB.
pub(crate) fn linear_to_srgb(value: f32) -> f32 {
    if value <= 0.0031308 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
}
