// Copyright 2019 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::render::Context;

pub fn apply(
    mask: &usvg::Mask,
    ctx: &Context,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::Pixmap,
) {
    if mask.root().children().is_empty() {
        pixmap.fill(tiny_skia::Color::TRANSPARENT);
        return;
    }

    let mut mask_pixmap = tiny_skia::Pixmap::new(pixmap.width(), pixmap.height()).unwrap();

    {
        // TODO: only when needed
        // Mask has to be clipped by mask.region
        let mut alpha_mask = tiny_skia::Mask::new(pixmap.width(), pixmap.height()).unwrap();
        alpha_mask.fill_path(
            &tiny_skia::PathBuilder::from_rect(mask.rect().to_rect()),
            tiny_skia::FillRule::Winding,
            true,
            transform,
        );

        crate::render::render_nodes(mask.root(), ctx, transform, &mut mask_pixmap.as_mut());

        mask_pixmap.apply_mask(&alpha_mask);
    }

    if let Some(mask) = mask.mask() {
        self::apply(mask, ctx, transform, pixmap);
    }

    let mask_type = match mask.kind() {
        usvg::MaskType::Luminance => tiny_skia::MaskType::Luminance,
        usvg::MaskType::Alpha => tiny_skia::MaskType::Alpha,
    };

    let mask = if mask_type == tiny_skia::MaskType::Luminance
        && mask.color_interpolation() == usvg::ColorInterpolation::LinearRGB
    {
        let mut result = tiny_skia::Mask::new(mask_pixmap.width(), mask_pixmap.height()).unwrap();
        for (pixel, alpha) in mask_pixmap.pixels().iter().zip(result.data_mut()) {
            if pixel.alpha() == 0 {
                continue;
            }
            // Decode unpremultiplied channels before calculating luminance.
            let a = f32::from(pixel.alpha());
            let r = crate::color::srgb_to_linear(f32::from(pixel.red()) / a);
            let g = crate::color::srgb_to_linear(f32::from(pixel.green()) / a);
            let b = crate::color::srgb_to_linear(f32::from(pixel.blue()) / a);
            *alpha = ((r * 0.2126 + g * 0.7152 + b * 0.0722) * a).round() as u8;
        }
        result
    } else {
        tiny_skia::Mask::from_pixmap(mask_pixmap.as_ref(), mask_type)
    };
    pixmap.apply_mask(&mask);
}
