// Copyright 2018 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use vello_cpu::kurbo::Affine;
use vello_cpu::peniko::{BlendMode, Compose, Mix};
use vello_cpu::RenderContext;
use usvg::tiny_skia_path;
use crate::clip::clip_mask;
use crate::util::{convert_transform, default_blend_mode};

pub struct Context {
    pub max_bbox: tiny_skia_path::IntRect,
}

pub fn render_nodes(
    parent: &usvg::Group,
    ctx: &Context,
    transform: Affine,
    pixmap: &mut RenderContext,
) {
    for node in parent.children() {
        render_node(node, ctx, transform, pixmap);
    }
}

pub fn render_node(
    node: &usvg::Node,
    ctx: &Context,
    transform: Affine,
    pixmap: &mut RenderContext,
) {
    match node {
        usvg::Node::Group(ref group) => {
            render_group(group, ctx, transform, pixmap);
        }
        usvg::Node::Path(ref path) => {
            crate::path::render(
                path,
                default_blend_mode(),
                ctx,
                transform,
                pixmap,
            );
        }
        usvg::Node::Image(ref image) => {
            crate::image::render(image, transform, pixmap);
        }
        usvg::Node::Text(ref text) => {
            render_group(text.flattened(), ctx, transform, pixmap);
        }
    }
}

fn render_group(
    group: &usvg::Group,
    ctx: &Context,
    transform: Affine,
    rctx: &mut RenderContext,
) -> Option<()> {
    let transform = transform * convert_transform(group.transform());

    if !group.should_isolate() {
        render_nodes(group, ctx, transform, rctx);
        return Some(());
    }
    
    let mask = group.clip_path().map(|clip| clip_mask(clip, transform, rctx.width(), rctx.height(), &rctx.render_settings()));

    rctx.push_layer(None, Some(convert_blend_mode(group.blend_mode())), Some(group.opacity().get()), mask);

    render_nodes(group, ctx, transform, rctx);
    
    rctx.pop_layer();

    if !group.filters().is_empty() {
        unimplemented!();
        // for filter in group.filters() {
        //     crate::filter::apply(filter, transform, &mut sub_pixmap);
        // }
    }

    if let Some(mask) = group.mask() {
        unimplemented!();
        // crate::mask::apply(mask, ctx, transform, &mut sub_pixmap);
    }

    Some(())
}

pub fn convert_blend_mode(mode: usvg::BlendMode) -> BlendMode {
    let mix = match mode {
        usvg::BlendMode::Normal => Mix::Normal,
        usvg::BlendMode::Multiply => Mix::Multiply,
        usvg::BlendMode::Screen => Mix::Screen,
        usvg::BlendMode::Overlay => Mix::Overlay,
        usvg::BlendMode::Darken => Mix::Darken,
        usvg::BlendMode::Lighten => Mix::Lighten,
        usvg::BlendMode::ColorDodge => Mix::ColorDodge,
        usvg::BlendMode::ColorBurn => Mix::ColorBurn,
        usvg::BlendMode::HardLight => Mix::HardLight,
        usvg::BlendMode::SoftLight => Mix::SoftLight,
        usvg::BlendMode::Difference => Mix::Difference,
        usvg::BlendMode::Exclusion => Mix::Exclusion,
        usvg::BlendMode::Hue => Mix::Hue,
        usvg::BlendMode::Saturation => Mix::Saturation,
        usvg::BlendMode::Color => Mix::Color,
        usvg::BlendMode::Luminosity => Mix::Luminosity,
    };
    
    BlendMode::new(mix, Compose::SrcOver)
}
