// Copyright 2020 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::{ImageRefMut, f32_bound};
use rgb::RGBA8;
use usvg::filter::ColorMatrixKind as ColorMatrix;

/// Applies a color matrix filter.
///
/// Input image pixels should have an **unpremultiplied alpha**.
pub fn apply(matrix: &ColorMatrix, src: ImageRefMut) {
    match matrix {
        ColorMatrix::Matrix(m) => {
            for pixel in src.data {
                let (r, g, b, a) = to_normalized_components(*pixel);

                let new_r = r * m[0] + g * m[1] + b * m[2] + a * m[3] + m[4];
                let new_g = r * m[5] + g * m[6] + b * m[7] + a * m[8] + m[9];
                let new_b = r * m[10] + g * m[11] + b * m[12] + a * m[13] + m[14];
                let new_a = r * m[15] + g * m[16] + b * m[17] + a * m[18] + m[19];

                pixel.r = from_normalized(new_r);
                pixel.g = from_normalized(new_g);
                pixel.b = from_normalized(new_b);
                pixel.a = from_normalized(new_a);
            }
        }
        ColorMatrix::Saturate(v) => {
            let v = v.get().max(0.0);
            let m = [
                0.213 + 0.787 * v,
                0.715 - 0.715 * v,
                0.072 - 0.072 * v,
                0.213 - 0.213 * v,
                0.715 + 0.285 * v,
                0.072 - 0.072 * v,
                0.213 - 0.213 * v,
                0.715 - 0.715 * v,
                0.072 + 0.928 * v,
            ];

            for pixel in src.data {
                let (r, g, b, _) = to_normalized_components(*pixel);

                let new_r = r * m[0] + g * m[1] + b * m[2];
                let new_g = r * m[3] + g * m[4] + b * m[5];
                let new_b = r * m[6] + g * m[7] + b * m[8];

                pixel.r = from_normalized(new_r);
                pixel.g = from_normalized(new_g);
                pixel.b = from_normalized(new_b);
            }
        }
        ColorMatrix::HueRotate(angle) => {
            let angle = angle.to_radians();
            let a1 = angle.cos();
            let a2 = angle.sin();
            let m = [
                0.213 + 0.787 * a1 - 0.213 * a2,
                0.715 - 0.715 * a1 - 0.715 * a2,
                0.072 - 0.072 * a1 + 0.928 * a2,
                0.213 - 0.213 * a1 + 0.143 * a2,
                0.715 + 0.285 * a1 + 0.140 * a2,
                0.072 - 0.072 * a1 - 0.283 * a2,
                0.213 - 0.213 * a1 - 0.787 * a2,
                0.715 - 0.715 * a1 + 0.715 * a2,
                0.072 + 0.928 * a1 + 0.072 * a2,
            ];

            for pixel in src.data {
                let (r, g, b, _) = to_normalized_components(*pixel);

                let new_r = r * m[0] + g * m[1] + b * m[2];
                let new_g = r * m[3] + g * m[4] + b * m[5];
                let new_b = r * m[6] + g * m[7] + b * m[8];

                pixel.r = from_normalized(new_r);
                pixel.g = from_normalized(new_g);
                pixel.b = from_normalized(new_b);
            }
        }
        ColorMatrix::LuminanceToAlpha => {
            for pixel in src.data {
                let (r, g, b, _) = to_normalized_components(*pixel);

                let new_a = r * 0.2125 + g * 0.7154 + b * 0.0721;

                pixel.r = 0;
                pixel.g = 0;
                pixel.b = 0;
                pixel.a = from_normalized(new_a);
            }
        }
    }
}

#[inline]
fn to_normalized_components(pixel: RGBA8) -> (f32, f32, f32, f32) {
    (
        pixel.r as f32 / 255.0,
        pixel.g as f32 / 255.0,
        pixel.b as f32 / 255.0,
        pixel.a as f32 / 255.0,
    )
}

#[inline]
fn from_normalized(c: f32) -> u8 {
    (f32_bound(0.0, c, 1.0) * 255.0) as u8
}

#[cfg(feature = "16bpc")]
pub use u16_impl::apply_u16;

#[cfg(feature = "16bpc")]
mod u16_impl {
    use super::*;

    /// Applies a color matrix filter to a 16-bit pixel buffer.
    pub fn apply_u16(matrix: &ColorMatrix, data: &mut [tiny_skia::PremultipliedColorU16]) {
        match matrix {
            ColorMatrix::Matrix(m) => {
                let m0 = m[0];
                let m1 = m[1];
                let m2 = m[2];
                let m3 = m[3];
                let m4 = m[4] * 65535.0;

                let m5 = m[5];
                let m6 = m[6];
                let m7 = m[7];
                let m8 = m[8];
                let m9 = m[9] * 65535.0;

                let m10 = m[10];
                let m11 = m[11];
                let m12 = m[12];
                let m13 = m[13];
                let m14 = m[14] * 65535.0;

                let m15 = m[15];
                let m16 = m[16];
                let m17 = m[17];
                let m18 = m[18];
                let m19 = m[19] * 65535.0;

                for pixel in data {
                    let c = pixel.demultiply();
                    let r = c.red() as f32;
                    let g = c.green() as f32;
                    let b = c.blue() as f32;
                    let a = c.alpha() as f32;

                    let new_r = (r * m0 + g * m1 + b * m2 + a * m3 + m4).clamp(0.0, 65535.0) + 0.5;
                    let new_g = (r * m5 + g * m6 + b * m7 + a * m8 + m9).clamp(0.0, 65535.0) + 0.5;
                    let new_b =
                        (r * m10 + g * m11 + b * m12 + a * m13 + m14).clamp(0.0, 65535.0) + 0.5;
                    let new_a =
                        (r * m15 + g * m16 + b * m17 + a * m18 + m19).clamp(0.0, 65535.0) + 0.5;

                    let nr = new_r as u16;
                    let ng = new_g as u16;
                    let nb = new_b as u16;
                    let na = new_a as u16;
                    *pixel = tiny_skia::ColorU16::from_rgba(nr, ng, nb, na).premultiply();
                }
            }
            ColorMatrix::Saturate(v) => {
                let v = v.get().max(0.0);
                let m0 = 0.213 + 0.787 * v;
                let m1 = 0.715 - 0.715 * v;
                let m2 = 0.072 - 0.072 * v;
                let m3 = 0.213 - 0.213 * v;
                let m4 = 0.715 + 0.285 * v;
                let m5 = 0.072 - 0.072 * v;
                let m6 = 0.213 - 0.213 * v;
                let m7 = 0.715 - 0.715 * v;
                let m8 = 0.072 + 0.928 * v;

                for pixel in data {
                    let c = pixel.demultiply();
                    let r = c.red() as f32;
                    let g = c.green() as f32;
                    let b = c.blue() as f32;
                    let a = c.alpha();

                    let new_r = (r * m0 + g * m1 + b * m2).clamp(0.0, 65535.0) + 0.5;
                    let new_g = (r * m3 + g * m4 + b * m5).clamp(0.0, 65535.0) + 0.5;
                    let new_b = (r * m6 + g * m7 + b * m8).clamp(0.0, 65535.0) + 0.5;

                    let nr = new_r as u16;
                    let ng = new_g as u16;
                    let nb = new_b as u16;
                    *pixel = tiny_skia::ColorU16::from_rgba(nr, ng, nb, a).premultiply();
                }
            }
            ColorMatrix::HueRotate(angle) => {
                let angle = angle.to_radians();
                let a1 = angle.cos();
                let a2 = angle.sin();
                let m0 = 0.213 + 0.787 * a1 - 0.213 * a2;
                let m1 = 0.715 - 0.715 * a1 - 0.715 * a2;
                let m2 = 0.072 - 0.072 * a1 + 0.928 * a2;
                let m3 = 0.213 - 0.213 * a1 + 0.143 * a2;
                let m4 = 0.715 + 0.285 * a1 + 0.140 * a2;
                let m5 = 0.072 - 0.072 * a1 - 0.283 * a2;
                let m6 = 0.213 - 0.213 * a1 - 0.787 * a2;
                let m7 = 0.715 - 0.715 * a1 + 0.715 * a2;
                let m8 = 0.072 + 0.928 * a1 + 0.072 * a2;

                for pixel in data {
                    let c = pixel.demultiply();
                    let r = c.red() as f32;
                    let g = c.green() as f32;
                    let b = c.blue() as f32;
                    let a = c.alpha();

                    let new_r = (r * m0 + g * m1 + b * m2).clamp(0.0, 65535.0) + 0.5;
                    let new_g = (r * m3 + g * m4 + b * m5).clamp(0.0, 65535.0) + 0.5;
                    let new_b = (r * m6 + g * m7 + b * m8).clamp(0.0, 65535.0) + 0.5;

                    let nr = new_r as u16;
                    let ng = new_g as u16;
                    let nb = new_b as u16;
                    *pixel = tiny_skia::ColorU16::from_rgba(nr, ng, nb, a).premultiply();
                }
            }
            ColorMatrix::LuminanceToAlpha => {
                for pixel in data {
                    let c = pixel.demultiply();
                    let r = c.red() as f32;
                    let g = c.green() as f32;
                    let b = c.blue() as f32;

                    let new_a = (r * 0.2125 + g * 0.7154 + b * 0.0721).clamp(0.0, 65535.0) + 0.5;
                    let na = new_a as u16;
                    *pixel = tiny_skia::PremultipliedColorU16::from_rgba_unchecked(0, 0, 0, na);
                }
            }
        }
    }
}
