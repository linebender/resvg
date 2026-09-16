// Copyright 2019 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::render::Context;
use tiny_skia::HighPixel;

pub fn apply<P: HighPixel>(
    mask: &usvg::Mask,
    ctx: &Context,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapGeneric<P>,
) {
    if mask.root().children().is_empty() {
        pixmap.fill(tiny_skia::Color::TRANSPARENT);
        return;
    }

    #[cfg(feature = "16bpc")]
    if P::BYTES_PER_PIXEL == 8 {
        let mut mask_pixmap = tiny_skia::PixmapU16::new(pixmap.width(), pixmap.height()).unwrap();

        {
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

        let mask_pixels: &[tiny_skia::PremultipliedColorU16] = mask_pixmap.pixels();
        let target_pixels: &mut [tiny_skia::PremultipliedColorU16] =
            bytemuck::cast_slice_mut(pixmap.data_mut());

        match mask.kind() {
            usvg::MaskType::Luminance => {
                for (target, mask_px) in target_pixels.iter_mut().zip(mask_pixels.iter()) {
                    let c = mask_px.demultiply();
                    let luma = (13933 * (c.red() as u32)
                        + 46871 * (c.green() as u32)
                        + 4731 * (c.blue() as u32))
                        >> 16;
                    let luma_a = ((luma * (c.alpha() as u32)) + 32768) >> 16;
                    let t = target.demultiply();
                    let new_a = ((t.alpha() as u32 * luma_a) + 32768) >> 16;
                    *target =
                        tiny_skia::ColorU16::from_rgba(t.red(), t.green(), t.blue(), new_a as u16)
                            .premultiply();
                }
            }
            usvg::MaskType::Alpha => {
                for (target, mask_px) in target_pixels.iter_mut().zip(mask_pixels.iter()) {
                    let mask_a = mask_px.alpha() as u32;
                    let t = target.demultiply();
                    let new_a = ((t.alpha() as u32 * mask_a) + 32768) >> 16;
                    *target =
                        tiny_skia::ColorU16::from_rgba(t.red(), t.green(), t.blue(), new_a as u16)
                            .premultiply();
                }
            }
        }
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

    let mask = tiny_skia::Mask::from_pixmap(mask_pixmap.as_ref(), mask_type);
    pixmap.apply_mask(&mask);
}
