// Copyright 2020 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::{ImageRef, ImageRefMut, f32_bound};
use rgb::RGBA8;
use usvg::filter::{DiffuseLighting, LightSource, SpecularLighting};
use usvg::{ApproxEqUlps, ApproxZeroUlps, Color};

const FACTOR_1_2: f32 = 1.0 / 2.0;
const FACTOR_1_3: f32 = 1.0 / 3.0;
const FACTOR_1_4: f32 = 1.0 / 4.0;
const FACTOR_2_3: f32 = 2.0 / 3.0;

#[derive(Clone, Copy, Debug)]
struct Vector2 {
    x: f32,
    y: f32,
}

impl Vector2 {
    #[inline]
    fn new(x: f32, y: f32) -> Self {
        Vector2 { x, y }
    }

    #[inline]
    fn approx_zero(&self) -> bool {
        self.x.approx_zero_ulps(4) && self.y.approx_zero_ulps(4)
    }
}

impl core::ops::Mul<f32> for Vector2 {
    type Output = Self;

    #[inline]
    fn mul(self, c: f32) -> Self::Output {
        Vector2 {
            x: self.x * c,
            y: self.y * c,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Vector3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vector3 {
    #[inline]
    fn new(x: f32, y: f32, z: f32) -> Self {
        Vector3 { x, y, z }
    }

    #[inline]
    fn dot(&self, other: &Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    #[inline]
    fn length(&self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    #[inline]
    fn normalized(&self) -> Option<Self> {
        let length = self.length();
        if !length.approx_zero_ulps(4) {
            Some(Vector3 {
                x: self.x / length,
                y: self.y / length,
                z: self.z / length,
            })
        } else {
            None
        }
    }
}

impl core::ops::Add<Vector3> for Vector3 {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Vector3) -> Self::Output {
        Vector3 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl core::ops::Sub<Vector3> for Vector3 {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Vector3) -> Self::Output {
        Vector3 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Normal {
    factor: Vector2,
    normal: Vector2,
}

impl Normal {
    #[inline]
    fn new(factor_x: f32, factor_y: f32, nx: i16, ny: i16) -> Self {
        Normal {
            factor: Vector2::new(factor_x, factor_y),
            normal: Vector2::new(-nx as f32, -ny as f32),
        }
    }
}

#[inline]
fn alpha_at_f32(img: ImageRef, fx: f32, fy: f32) -> f32 {
    let fx = fx.clamp(0.0, (img.width - 1) as f32);
    let fy = fy.clamp(0.0, (img.height - 1) as f32);
    let x0 = fx.floor() as u32;
    let y0 = fy.floor() as u32;
    let x1 = (x0 + 1).min(img.width - 1);
    let y1 = (y0 + 1).min(img.height - 1);
    let dx = fx - x0 as f32;
    let dy = fy - y0 as f32;

    let a00 = img.alpha_at(x0, y0) as f32;
    let a10 = img.alpha_at(x1, y0) as f32;
    let a01 = img.alpha_at(x0, y1) as f32;
    let a11 = img.alpha_at(x1, y1) as f32;

    let top = a00 * (1.0 - dx) + a10 * dx;
    let bottom = a01 * (1.0 - dx) + a11 * dx;
    top * (1.0 - dy) + bottom * dy
}

#[inline]
fn normal_at(img: ImageRef, x: u32, y: u32, step_x: f32, step_y: f32) -> Normal {
    let fx = x as f32;
    let fy = y as f32;

    let a00 = alpha_at_f32(img, fx - step_x, fy - step_y);
    let a10 = alpha_at_f32(img, fx, fy - step_y);
    let a20 = alpha_at_f32(img, fx + step_x, fy - step_y);
    let a01 = alpha_at_f32(img, fx - step_x, fy);
    let a21 = alpha_at_f32(img, fx + step_x, fy);
    let a02 = alpha_at_f32(img, fx - step_x, fy + step_y);
    let a12 = alpha_at_f32(img, fx, fy + step_y);
    let a22 = alpha_at_f32(img, fx + step_x, fy + step_y);

    let nx = -a00 + a20 - 2.0 * a01 + 2.0 * a21 - a02 + a22;
    let ny = -a00 - 2.0 * a10 - a20 + a02 + 2.0 * a12 + a22;

    Normal {
        factor: Vector2::new(1.0 / (4.0 * step_x), 1.0 / (4.0 * step_y)),
        normal: Vector2::new(-nx, -ny),
    }
}

/// Renders a diffuse lighting.
///
/// - `src` pixels can have any alpha method, since only the alpha channel is used.
/// - `dest` will have an **unpremultiplied alpha**.
///
/// Does nothing when `src` is less than 3x3.
///
/// `src` and `dest` dimensions must match.
pub fn diffuse_lighting(
    fe: &DiffuseLighting,
    light_source: LightSource,
    ts: usvg::Transform,
    src: ImageRef,
    dest: ImageRefMut,
) {
    debug_assert!(src.width == dest.width && src.height == dest.height);

    let (surface_scale, step_x, step_y) = if let Some((kx, ky)) = fe.kernel_unit_length() {
        let (sx, sy) = ts.get_scale();
        let sz = (ts.sx * ts.sx + ts.sy * ts.sy).sqrt() / core::f32::consts::SQRT_2;
        (fe.surface_scale() * sz, kx.get() * sx, ky.get() * sy)
    } else {
        (fe.surface_scale(), 1.0, 1.0)
    };

    let light_factor = |normal: Normal, light_vector: Vector3| {
        let k = if normal.normal.approx_zero() {
            light_vector.z
        } else {
            let mut n = normal.normal * (surface_scale / 255.0);
            n.x *= normal.factor.x;
            n.y *= normal.factor.y;

            let normal = Vector3::new(n.x, n.y, 1.0);

            normal.dot(&light_vector) / normal.length()
        };

        fe.diffuse_constant() * k
    };

    apply(
        light_source,
        surface_scale,
        fe.lighting_color(),
        step_x,
        step_y,
        &light_factor,
        calc_diffuse_alpha,
        src,
        dest,
    );
}

/// Renders a specular lighting.
///
/// - `src` pixels can have any alpha method, since only the alpha channel is used.
/// - `dest` will have a **premultiplied alpha**.
///
/// Does nothing when `src` is less than 3x3.
///
/// `src` and `dest` dimensions must match.
pub fn specular_lighting(
    fe: &SpecularLighting,
    light_source: LightSource,
    ts: usvg::Transform,
    src: ImageRef,
    dest: ImageRefMut,
) {
    debug_assert!(src.width == dest.width && src.height == dest.height);

    let (surface_scale, step_x, step_y) = if let Some((kx, ky)) = fe.kernel_unit_length() {
        let (sx, sy) = ts.get_scale();
        let sz = (ts.sx * ts.sx + ts.sy * ts.sy).sqrt() / core::f32::consts::SQRT_2;
        (fe.surface_scale() * sz, kx.get() * sx, ky.get() * sy)
    } else {
        (fe.surface_scale(), 1.0, 1.0)
    };

    let light_factor = |normal: Normal, light_vector: Vector3| {
        let h = light_vector + Vector3::new(0.0, 0.0, 1.0);
        let h_length = h.length();

        if h_length.approx_zero_ulps(4) {
            return 0.0;
        }

        let k = if normal.normal.approx_zero() {
            let n_dot_h = h.z / h_length;
            if fe.specular_exponent().approx_eq_ulps(&1.0, 4) {
                n_dot_h
            } else {
                n_dot_h.powf(fe.specular_exponent())
            }
        } else {
            let mut n = normal.normal * (surface_scale / 255.0);
            n.x *= normal.factor.x;
            n.y *= normal.factor.y;

            let normal = Vector3::new(n.x, n.y, 1.0);

            let n_dot_h = normal.dot(&h) / normal.length() / h_length;
            if fe.specular_exponent().approx_eq_ulps(&1.0, 4) {
                n_dot_h
            } else {
                n_dot_h.powf(fe.specular_exponent())
            }
        };

        fe.specular_constant() * k
    };

    apply(
        light_source,
        surface_scale,
        fe.lighting_color(),
        step_x,
        step_y,
        &light_factor,
        calc_specular_alpha,
        src,
        dest,
    );
}

fn apply(
    light_source: LightSource,
    surface_scale: f32,
    lighting_color: Color,
    step_x: f32,
    step_y: f32,
    light_factor: &dyn Fn(Normal, Vector3) -> f32,
    calc_alpha: fn(u8, u8, u8) -> u8,
    src: ImageRef,
    mut dest: ImageRefMut,
) {
    if src.width < 3 || src.height < 3 {
        return;
    }

    let width = src.width;
    let height = src.height;

    // `feDistantLight` has a fixed vector, so calculate it beforehand.
    let mut light_vector = match light_source {
        LightSource::DistantLight(light) => {
            let azimuth = light.azimuth.to_radians();
            let elevation = light.elevation.to_radians();
            Vector3::new(
                azimuth.cos() * elevation.cos(),
                azimuth.sin() * elevation.cos(),
                elevation.sin(),
            )
        }
        _ => Vector3::new(1.0, 1.0, 1.0),
    };

    let mut calc = |nx, ny, normal: Normal| {
        match light_source {
            LightSource::DistantLight(_) => {}
            LightSource::PointLight(ref light) => {
                let nz = src.alpha_at(nx, ny) as f32 / 255.0 * surface_scale;
                let origin = Vector3::new(light.x, light.y, light.z);
                let v = origin - Vector3::new(nx as f32, ny as f32, nz);
                light_vector = v.normalized().unwrap_or(v);
            }
            LightSource::SpotLight(ref light) => {
                let nz = src.alpha_at(nx, ny) as f32 / 255.0 * surface_scale;
                let origin = Vector3::new(light.x, light.y, light.z);
                let v = origin - Vector3::new(nx as f32, ny as f32, nz);
                light_vector = v.normalized().unwrap_or(v);
            }
        }

        let light_color = light_color(&light_source, lighting_color, light_vector);
        let factor = light_factor(normal, light_vector);

        let compute = |x| (f32_bound(0.0, x as f32 * factor, 255.0) + 0.5) as u8;

        let r = compute(light_color.red);
        let g = compute(light_color.green);
        let b = compute(light_color.blue);
        let a = calc_alpha(r, g, b);

        *dest.pixel_at_mut(nx, ny) = RGBA8 { b, g, r, a };
    };

    if step_x.approx_eq_ulps(&1.0, 4) && step_y.approx_eq_ulps(&1.0, 4) {
        calc(0, 0, top_left_normal(src));
        calc(width - 1, 0, top_right_normal(src));
        calc(0, height - 1, bottom_left_normal(src));
        calc(width - 1, height - 1, bottom_right_normal(src));

        for x in 1..width - 1 {
            calc(x, 0, top_row_normal(src, x));
            calc(x, height - 1, bottom_row_normal(src, x));
        }

        for y in 1..height - 1 {
            calc(0, y, left_column_normal(src, y));
            calc(width - 1, y, right_column_normal(src, y));
        }

        for y in 1..height - 1 {
            for x in 1..width - 1 {
                calc(x, y, interior_normal(src, x, y));
            }
        }
    } else {
        for y in 0..height {
            for x in 0..width {
                calc(x, y, normal_at(src, x, y, step_x, step_y));
            }
        }
    }
}

fn light_color(light: &LightSource, lighting_color: Color, light_vector: Vector3) -> Color {
    match *light {
        LightSource::DistantLight(_) | LightSource::PointLight(_) => lighting_color,
        LightSource::SpotLight(ref light) => {
            let origin = Vector3::new(light.x, light.y, light.z);
            let direction = Vector3::new(light.points_at_x, light.points_at_y, light.points_at_z);
            let direction = direction - origin;
            let direction = direction.normalized().unwrap_or(direction);
            let minus_l_dot_s = -light_vector.dot(&direction);
            if minus_l_dot_s <= 0.0 {
                return Color::black();
            }

            if let Some(limiting_cone_angle) = light.limiting_cone_angle {
                if minus_l_dot_s < limiting_cone_angle.to_radians().cos() {
                    return Color::black();
                }
            }

            let factor = minus_l_dot_s.powf(light.specular_exponent.get());
            let compute = |x| (f32_bound(0.0, x as f32 * factor, 255.0) + 0.5) as u8;

            Color::new_rgb(
                compute(lighting_color.red),
                compute(lighting_color.green),
                compute(lighting_color.blue),
            )
        }
    }
}

fn top_left_normal(img: ImageRef) -> Normal {
    let center = img.alpha_at(0, 0);
    let right = img.alpha_at(1, 0);
    let bottom = img.alpha_at(0, 1);
    let bottom_right = img.alpha_at(1, 1);

    Normal::new(
        FACTOR_2_3,
        FACTOR_2_3,
        -2 * center + 2 * right - bottom + bottom_right,
        -2 * center - right + 2 * bottom + bottom_right,
    )
}

fn top_right_normal(img: ImageRef) -> Normal {
    let left = img.alpha_at(img.width - 2, 0);
    let center = img.alpha_at(img.width - 1, 0);
    let bottom_left = img.alpha_at(img.width - 2, 1);
    let bottom = img.alpha_at(img.width - 1, 1);

    Normal::new(
        FACTOR_2_3,
        FACTOR_2_3,
        -2 * left + 2 * center - bottom_left + bottom,
        -left - 2 * center + bottom_left + 2 * bottom,
    )
}

fn bottom_left_normal(img: ImageRef) -> Normal {
    let top = img.alpha_at(0, img.height - 2);
    let top_right = img.alpha_at(1, img.height - 2);
    let center = img.alpha_at(0, img.height - 1);
    let right = img.alpha_at(1, img.height - 1);

    Normal::new(
        FACTOR_2_3,
        FACTOR_2_3,
        -top + top_right - 2 * center + 2 * right,
        -2 * top - top_right + 2 * center + right,
    )
}

fn bottom_right_normal(img: ImageRef) -> Normal {
    let top_left = img.alpha_at(img.width - 2, img.height - 2);
    let top = img.alpha_at(img.width - 1, img.height - 2);
    let left = img.alpha_at(img.width - 2, img.height - 1);
    let center = img.alpha_at(img.width - 1, img.height - 1);

    Normal::new(
        FACTOR_2_3,
        FACTOR_2_3,
        -top_left + top - 2 * left + 2 * center,
        -top_left - 2 * top + left + 2 * center,
    )
}

fn top_row_normal(img: ImageRef, x: u32) -> Normal {
    let left = img.alpha_at(x - 1, 0);
    let center = img.alpha_at(x, 0);
    let right = img.alpha_at(x + 1, 0);
    let bottom_left = img.alpha_at(x - 1, 1);
    let bottom = img.alpha_at(x, 1);
    let bottom_right = img.alpha_at(x + 1, 1);

    Normal::new(
        FACTOR_1_3,
        FACTOR_1_2,
        -2 * left + 2 * right - bottom_left + bottom_right,
        -left - 2 * center - right + bottom_left + 2 * bottom + bottom_right,
    )
}

fn bottom_row_normal(img: ImageRef, x: u32) -> Normal {
    let top_left = img.alpha_at(x - 1, img.height - 2);
    let top = img.alpha_at(x, img.height - 2);
    let top_right = img.alpha_at(x + 1, img.height - 2);
    let left = img.alpha_at(x - 1, img.height - 1);
    let center = img.alpha_at(x, img.height - 1);
    let right = img.alpha_at(x + 1, img.height - 1);

    Normal::new(
        FACTOR_1_3,
        FACTOR_1_2,
        -top_left + top_right - 2 * left + 2 * right,
        -top_left - 2 * top - top_right + left + 2 * center + right,
    )
}

fn left_column_normal(img: ImageRef, y: u32) -> Normal {
    let top = img.alpha_at(0, y - 1);
    let top_right = img.alpha_at(1, y - 1);
    let center = img.alpha_at(0, y);
    let right = img.alpha_at(1, y);
    let bottom = img.alpha_at(0, y + 1);
    let bottom_right = img.alpha_at(1, y + 1);

    Normal::new(
        FACTOR_1_2,
        FACTOR_1_3,
        -top + top_right - 2 * center + 2 * right - bottom + bottom_right,
        -2 * top - top_right + 2 * bottom + bottom_right,
    )
}

fn right_column_normal(img: ImageRef, y: u32) -> Normal {
    let top_left = img.alpha_at(img.width - 2, y - 1);
    let top = img.alpha_at(img.width - 1, y - 1);
    let left = img.alpha_at(img.width - 2, y);
    let center = img.alpha_at(img.width - 1, y);
    let bottom_left = img.alpha_at(img.width - 2, y + 1);
    let bottom = img.alpha_at(img.width - 1, y + 1);

    Normal::new(
        FACTOR_1_2,
        FACTOR_1_3,
        -top_left + top - 2 * left + 2 * center - bottom_left + bottom,
        -top_left - 2 * top + bottom_left + 2 * bottom,
    )
}

fn interior_normal(img: ImageRef, x: u32, y: u32) -> Normal {
    let top_left = img.alpha_at(x - 1, y - 1);
    let top = img.alpha_at(x, y - 1);
    let top_right = img.alpha_at(x + 1, y - 1);
    let left = img.alpha_at(x - 1, y);
    let right = img.alpha_at(x + 1, y);
    let bottom_left = img.alpha_at(x - 1, y + 1);
    let bottom = img.alpha_at(x, y + 1);
    let bottom_right = img.alpha_at(x + 1, y + 1);

    Normal::new(
        FACTOR_1_4,
        FACTOR_1_4,
        -top_left + top_right - 2 * left + 2 * right - bottom_left + bottom_right,
        -top_left - 2 * top - top_right + bottom_left + 2 * bottom + bottom_right,
    )
}

fn calc_diffuse_alpha(_: u8, _: u8, _: u8) -> u8 {
    255
}

fn calc_specular_alpha(r: u8, g: u8, b: u8) -> u8 {
    use core::cmp::max;
    max(max(r, g), b)
}
