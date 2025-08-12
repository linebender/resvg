// Copyright 2019 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use vello_cpu::kurbo::{Affine, Rect, Shape};
use vello_cpu::{Mask, Pixmap, RenderContext, RenderMode, RenderSettings};
use crate::render::Context;

pub fn get_mask(
    mask: &usvg::Mask,
    ctx: &Context,
    transform: Affine,
    width: u16,
    height: u16,
    render_settings: &RenderSettings
) -> Mask {
    let mut mask_ctx = RenderContext::new_with(width, height, *render_settings);
    let mut mask_pix = Pixmap::new(width, height);
    
    let clip_path = {
        let r = mask.rect();
        transform * Rect::new(r.left() as f64, r.top() as f64,  r.right() as f64, r.bottom() as f64).to_path(0.1)
    };
    
    let inner_mask = mask.mask().map(|inner_mask| get_mask(inner_mask, ctx, transform, width, height, render_settings));
    
    mask_ctx.push_layer(Some(&clip_path), None, None, inner_mask);
    crate::render::render_nodes(mask.root(), ctx, transform, &mut mask_ctx);
    mask_ctx.pop_layer();
    
    mask_ctx.flush();
    mask_ctx.render_to_pixmap(&mut mask_pix, RenderMode::OptimizeQuality);
    
    match mask.kind() {
        usvg::MaskType::Luminance => Mask::new_luminance(&mask_pix),
        usvg::MaskType::Alpha => Mask::new_alpha(&mask_pix),
    }
}
