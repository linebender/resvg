// Copyright 2020 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::{ImageRefMut, f32_bound};
use rgb::RGBA8;
use usvg::ApproxEqUlps;
use usvg::filter::{ConvolveMatrix, EdgeMode};

#[inline]
fn pixel_at_f32(src: &ImageRefMut, fx: f32, fy: f32) -> (f32, f32, f32, f32) {
    let fx = fx.clamp(0.0, (src.width - 1) as f32);
    let fy = fy.clamp(0.0, (src.height - 1) as f32);
    let x0 = fx.floor() as u32;
    let y0 = fy.floor() as u32;
    let x1 = (x0 + 1).min(src.width - 1);
    let y1 = (y0 + 1).min(src.height - 1);
    let dx = fx - x0 as f32;
    let dy = fy - y0 as f32;

    let p00 = src.pixel_at(x0, y0);
    let p10 = src.pixel_at(x1, y0);
    let p01 = src.pixel_at(x0, y1);
    let p11 = src.pixel_at(x1, y1);

    let interp = |c00: u8, c10: u8, c01: u8, c11: u8| -> f32 {
        let top = c00 as f32 * (1.0 - dx) + c10 as f32 * dx;
        let bottom = c01 as f32 * (1.0 - dx) + c11 as f32 * dx;
        top * (1.0 - dy) + bottom * dy
    };

    (
        interp(p00.r, p10.r, p01.r, p11.r),
        interp(p00.g, p10.g, p01.g, p11.g),
        interp(p00.b, p10.b, p01.b, p11.b),
        interp(p00.a, p10.a, p01.a, p11.a),
    )
}

/// Applies a convolve matrix.
///
/// Input image pixels should have a **premultiplied alpha** when `preserve_alpha=false`.
///
/// # Allocations
///
/// This method will allocate a copy of the `src` image as a back buffer.
pub fn apply(matrix: &ConvolveMatrix, ts: usvg::Transform, src: ImageRefMut) {
    fn bound(min: i32, val: i32, max: i32) -> i32 {
        core::cmp::max(min, core::cmp::min(max, val))
    }

    let width_max = src.width as i32 - 1;
    let height_max = src.height as i32 - 1;

    let (step_x, step_y) = if let Some((kx, ky)) = matrix.kernel_unit_length() {
        let (sx, sy) = ts.get_scale();
        (kx.get() * sx, ky.get() * sy)
    } else {
        (1.0, 1.0)
    };

    let use_unit_step = step_x.approx_eq_ulps(&1.0, 4) && step_y.approx_eq_ulps(&1.0, 4);

    let mut buf = vec![RGBA8::default(); src.data.len()];
    let mut buf = ImageRefMut::new(src.width, src.height, &mut buf);
    let mut x = 0;
    let mut y = 0;
    for in_p in src.data.iter() {
        let mut new_r = 0.0;
        let mut new_g = 0.0;
        let mut new_b = 0.0;
        let mut new_a = 0.0;
        for oy in 0..matrix.matrix().rows() {
            for ox in 0..matrix.matrix().columns() {
                let k = matrix.matrix().get(
                    matrix.matrix().columns() - ox - 1,
                    matrix.matrix().rows() - oy - 1,
                );

                if use_unit_step {
                    let mut tx = x as i32 - matrix.matrix().target_x() as i32 + ox as i32;
                    let mut ty = y as i32 - matrix.matrix().target_y() as i32 + oy as i32;

                    match matrix.edge_mode() {
                        EdgeMode::None => {
                            if tx < 0 || tx > width_max || ty < 0 || ty > height_max {
                                continue;
                            }
                        }
                        EdgeMode::Duplicate => {
                            tx = bound(0, tx, width_max);
                            ty = bound(0, ty, height_max);
                        }
                        EdgeMode::Wrap => {
                            while tx < 0 {
                                tx += src.width as i32;
                            }
                            tx %= src.width as i32;

                            while ty < 0 {
                                ty += src.height as i32;
                            }
                            ty %= src.height as i32;
                        }
                    }

                    let p = src.pixel_at(tx as u32, ty as u32);
                    new_r += (p.r as f32) / 255.0 * k;
                    new_g += (p.g as f32) / 255.0 * k;
                    new_b += (p.b as f32) / 255.0 * k;

                    if !matrix.preserve_alpha() {
                        new_a += (p.a as f32) / 255.0 * k;
                    }
                } else {
                    let mut fx =
                        x as f32 - matrix.matrix().target_x() as f32 * step_x + ox as f32 * step_x;
                    let mut fy =
                        y as f32 - matrix.matrix().target_y() as f32 * step_y + oy as f32 * step_y;

                    match matrix.edge_mode() {
                        EdgeMode::None => {
                            if fx < 0.0
                                || fx > width_max as f32
                                || fy < 0.0
                                || fy > height_max as f32
                            {
                                continue;
                            }
                        }
                        EdgeMode::Duplicate => {
                            fx = fx.clamp(0.0, width_max as f32);
                            fy = fy.clamp(0.0, height_max as f32);
                        }
                        EdgeMode::Wrap => {
                            let w = src.width as f32;
                            let h = src.height as f32;
                            fx = fx.rem_euclid(w);
                            fy = fy.rem_euclid(h);
                        }
                    }

                    let (pr, pg, pb, pa) = pixel_at_f32(&src, fx, fy);
                    new_r += (pr / 255.0) * k;
                    new_g += (pg / 255.0) * k;
                    new_b += (pb / 255.0) * k;

                    if !matrix.preserve_alpha() {
                        new_a += (pa / 255.0) * k;
                    }
                }
            }
        }

        if matrix.preserve_alpha() {
            new_a = in_p.a as f32 / 255.0;
        } else {
            new_a = new_a / matrix.divisor().get() + matrix.bias();
        }

        let bounded_new_a = f32_bound(0.0, new_a, 1.0);

        let calc = |x| {
            let x = x / matrix.divisor().get() + matrix.bias() * new_a;

            let x = if matrix.preserve_alpha() {
                f32_bound(0.0, x, 1.0) * bounded_new_a
            } else {
                f32_bound(0.0, x, bounded_new_a)
            };

            (x * 255.0 + 0.5) as u8
        };

        let out_p = buf.pixel_at_mut(x, y);
        out_p.r = calc(new_r);
        out_p.g = calc(new_g);
        out_p.b = calc(new_b);
        out_p.a = (bounded_new_a * 255.0 + 0.5) as u8;

        x += 1;
        if x == src.width {
            x = 0;
            y += 1;
        }
    }

    // Do not use `mem::swap` because `data` referenced via FFI.
    src.data.copy_from_slice(buf.data);
}
