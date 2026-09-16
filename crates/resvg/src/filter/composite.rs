// Copyright 2020 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::{ImageRef, ImageRefMut, f32_bound};
use rgb::RGBA8;
use usvg::ApproxZeroUlps;

/// Performs an arithmetic composition.
///
/// - `src1` and `src2` image pixels should have a **premultiplied alpha**.
/// - `dest` image pixels will have a **premultiplied alpha**.
///
/// `src1`, `src2` and `dest` dimensions must match.
pub fn arithmetic(
    k1: f32,
    k2: f32,
    k3: f32,
    k4: f32,
    src1: ImageRef,
    src2: ImageRef,
    dest: ImageRefMut,
) {
    debug_assert!(src1.width == src2.width && src1.width == dest.width);
    debug_assert!(src1.height == src2.height && src1.height == dest.height);

    let calc = |i1, i2, max| {
        let i1 = i1 as f32 / 255.0;
        let i2 = i2 as f32 / 255.0;
        let result = k1 * i1 * i2 + k2 * i1 + k3 * i2 + k4;
        f32_bound(0.0, result, max)
    };

    let mut i = 0;
    for (c1, c2) in src1.data.iter().zip(src2.data.iter()) {
        let a = calc(c1.a, c2.a, 1.0);
        if a.approx_zero_ulps(4) {
            i += 1;
            continue;
        }

        let r = (calc(c1.r, c2.r, a) * 255.0) as u8;
        let g = (calc(c1.g, c2.g, a) * 255.0) as u8;
        let b = (calc(c1.b, c2.b, a) * 255.0) as u8;
        let a = (a * 255.0) as u8;

        dest.data[i] = RGBA8 { r, g, b, a };

        i += 1;
    }
}

#[cfg(feature = "16bpc")]
pub use u16_impl::arithmetic_u16;

#[cfg(feature = "16bpc")]
mod u16_impl {
    use super::*;

    /// Performs an arithmetic composition on 16-bit premultiplied pixel buffers.
    pub fn arithmetic_u16(
        k1: f32,
        k2: f32,
        k3: f32,
        k4: f32,
        src1: &[tiny_skia::PremultipliedColorU16],
        src2: &[tiny_skia::PremultipliedColorU16],
        dest: &mut [tiny_skia::PremultipliedColorU16],
    ) {
        debug_assert!(src1.len() == src2.len() && src1.len() == dest.len());

        let c1 = k1 * (1.0 / 65535.0);
        let c2 = k2;
        let c3 = k3;
        let c4 = k4 * 65535.0;

        for (i, (s1, s2)) in src1.iter().zip(src2.iter()).enumerate() {
            let a1 = s1.alpha() as f32;
            let a2 = s2.alpha() as f32;
            let a_res = c1 * a1 * a2 + c2 * a1 + c3 * a2 + c4;
            let a_clamped = f32_bound(0.0, a_res, 65535.0);

            if a_clamped.approx_zero_ulps(4) {
                dest[i] = tiny_skia::PremultipliedColorU16::TRANSPARENT;
                continue;
            }

            let r1 = s1.red() as f32;
            let r2 = s2.red() as f32;
            let r = f32_bound(0.0, c1 * r1 * r2 + c2 * r1 + c3 * r2 + c4, a_clamped);

            let g1 = s1.green() as f32;
            let g2 = s2.green() as f32;
            let g = f32_bound(0.0, c1 * g1 * g2 + c2 * g1 + c3 * g2 + c4, a_clamped);

            let b1 = s1.blue() as f32;
            let b2 = s2.blue() as f32;
            let b = f32_bound(0.0, c1 * b1 * b2 + c2 * b1 + c3 * b2 + c4, a_clamped);

            dest[i] = tiny_skia::PremultipliedColorU16::from_rgba_unchecked(
                (r + 0.5) as u16,
                (g + 0.5) as u16,
                (b + 0.5) as u16,
                (a_clamped + 0.5) as u16,
            );
        }
    }
}
