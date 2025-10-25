// Copyright 2019 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use vello_cpu::kurbo::Affine;
use vello_cpu::{Mask, PaintType, Pixmap, RenderContext, RenderMode, RenderSettings};
use vello_cpu::color::palette::css::BLACK;
use usvg::tiny_skia_path;
use crate::render::Context;
use crate::util::{convert_transform, default_blend_mode, settings};

pub fn clip_mask(
    clip: &usvg::ClipPath,
    transform: Affine,
    width: u16,
    height: u16,
    render_settings: &RenderSettings
) -> Mask {
    let mut clip_ctx = RenderContext::new_with(width, height, settings(render_settings)); 
    let mut clip_pixmap = Pixmap::new(width, height);

    let mask = clip.clip_path().map(|clip| clip_mask(clip, transform, width, height, render_settings));
    let has_mask = mask.is_some();
    
    if has_mask {
        clip_ctx.push_layer(None, None, None, mask);
    }
    
    draw_children(
        clip.root(),
        transform * convert_transform(clip.transform()),
        &mut clip_ctx,
    );
    
    if has_mask {
        clip_ctx.pop_layer();       
    }
    
    clip_ctx.flush();
    clip_ctx.render_to_pixmap(&mut clip_pixmap, RenderMode::OptimizeSpeed);

    Mask::new_alpha(&clip_pixmap)
}

fn draw_children(
    parent: &usvg::Group,
    transform: Affine,
    rctx: &mut RenderContext,
) {
    for child in parent.children() {
        match child {
            usvg::Node::Path(ref path) => {
                if !path.is_visible() {
                    continue;
                }
                
                let ctx = Context {
                    max_bbox: tiny_skia_path::IntRect::from_xywh(0, 0, 1, 1).unwrap(),
                };

                crate::path::fill_path(path, Some((PaintType::Solid(BLACK), Affine::IDENTITY)), default_blend_mode(), &ctx, transform, rctx);
            }
            usvg::Node::Text(ref text) => {
                draw_children(text.flattened(), transform, rctx);
            }
            usvg::Node::Group(ref group) => {
                let transform = transform * convert_transform(group.transform());

                if let Some(clip) = group.clip_path() {
                    // If a `clipPath` child also has a `clip-path`
                    // then we should render this child on a new canvas,
                    // clip it, and only then draw it to the `clipPath`.
                    clip_group(group, clip, transform, rctx);
                } else {
                    draw_children(group, transform, rctx);
                }
            }
            _ => {}
        }
    }
}

fn clip_group(
    children: &usvg::Group,
    clip: &usvg::ClipPath,
    transform: Affine,
    rctx: &mut RenderContext,
) -> Option<()> {
    let mask = clip_mask(clip, transform, rctx.width(), rctx.height(), &rctx.render_settings());

    rctx.push_layer(None, None, None, Some(mask));

    draw_children(
        children,
        transform,
        rctx,
    );
    
    rctx.pop_layer();

    Some(())
}
