// Copyright 2018 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::OptionLog;

pub struct Context {
    pub max_bbox: tiny_skia::IntRect,
}

pub fn render_nodes(
    parent: &usvg::Group,
    ctx: &Context,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) {
    for node in parent.children() {
        render_node(node, ctx, transform, pixmap);
    }
}

pub fn render_node(
    node: &usvg::Node,
    ctx: &Context,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) {
    match node {
        usvg::Node::Group(group) => {
            render_group(group, ctx, transform, pixmap);
        }
        usvg::Node::Path(path) => {
            crate::path::render(
                path,
                tiny_skia::BlendMode::SourceOver,
                ctx,
                transform,
                pixmap,
            );
        }
        usvg::Node::Image(image) => {
            crate::image::render(image, transform, pixmap);
        }
        usvg::Node::Text(text) => {
            render_group(text.flattened(), ctx, transform, pixmap);
        }
    }
}

fn render_group(
    group: &usvg::Group,
    ctx: &Context,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) -> Option<()> {
    let transform = transform.pre_concat(group.transform());

    if !group.should_isolate() {
        render_nodes(group, ctx, transform, pixmap);
        return Some(());
    }

    let mut rest = tiny_skia::Transform::identity();
    let mut layer_ts = transform;
    if !group.filters().is_empty() {
        let sx = transform.sx.hypot(transform.ky);
        let sy = transform.kx.hypot(transform.sy);
        rest = tiny_skia::Transform::from_row(
            transform.sx / sx,
            transform.ky / sx,
            transform.kx / sy,
            transform.sy / sy,
            0.0,
            0.0,
        );
        if rest.has_skew() {
            // A rotated or skewed layer takes the translation along and is scaled exactly,
            // so that its pixels stay aligned with the group's user space, as in browsers.
            rest = rest.post_translate(transform.tx, transform.ty);
            layer_ts = tiny_skia::Transform::from_scale(sx, sy);
        } else {
            layer_ts = rest.invert()?.pre_concat(transform);
        }
    }
    let inv_rest = rest.invert()?;
    let mut max_bbox = ctx.max_bbox.to_rect().transform(inv_rest)?.round_out()?;

    if rest.has_skew() {
        // A skew can make the layer arbitrarily large. As in Skia, no side may be longer than
        // the bounds a filter may cover (or 2048), at a lower resolution if need be.
        let limit = ctx.max_bbox.width().max(ctx.max_bbox.height()).max(2048) as f32;
        let bounds = crate::geom::to_int_rect(group.layer_bounding_box().transform(layer_ts)?)?;
        let bounds = bounds.intersect(&max_bbox)?;
        let kx = (limit / bounds.width() as f32).min(1.0);
        let ky = (limit / bounds.height() as f32).min(1.0);
        if kx < 1.0 || ky < 1.0 {
            layer_ts = layer_ts.post_scale(kx, ky);
            rest = rest.pre_scale(1.0 / kx, 1.0 / ky);
            let scale = tiny_skia::Transform::from_scale(kx, ky);
            max_bbox = bounds.to_rect().transform(scale)?.round_out()?;
        }
    }
    let (layer, ibbox) = render_layer(group, &Context { max_bbox }, layer_ts)?;

    let paint = tiny_skia::Paint {
        shader: tiny_skia::Pattern::new(
            layer.as_ref(),
            tiny_skia::SpreadMode::Pad,
            tiny_skia::FilterQuality::Bilinear,
            group.opacity().get(),
            tiny_skia::Transform::from_translate(ibbox.x() as f32, ibbox.y() as f32),
        ),
        blend_mode: convert_blend_mode(group.blend_mode()),
        anti_alias: true,
        ..tiny_skia::Paint::default()
    };
    let path = tiny_skia::PathBuilder::from_rect(ibbox.to_rect());
    pixmap.fill_path(&path, &paint, tiny_skia::FillRule::Winding, rest, None);

    Some(())
}

/// Renders an isolated group into a new layer, returning it and its position.
fn render_layer(
    group: &usvg::Group,
    ctx: &Context,
    transform: tiny_skia::Transform,
) -> Option<(tiny_skia::Pixmap, tiny_skia::IntRect)> {
    let bbox = group.layer_bounding_box().transform(transform)?;

    let mut ibbox = if group.filters().is_empty() {
        // Convert group bbox into an integer one, expanding each side outwards by 2px
        // to make sure that anti-aliased pixels would not be clipped.
        tiny_skia::IntRect::from_xywh(
            (bbox.x().floor() as i32).checked_sub(2)?,
            (bbox.y().floor() as i32).checked_sub(2)?,
            (bbox.width().ceil() as u32).checked_add(4)?,
            (bbox.height().ceil() as u32).checked_add(4)?,
        )?
    } else {
        // The bounding box for groups with filters is special and should not be expanded by 2px,
        // because it's already acting as a clipping region.
        let bbox = crate::geom::to_int_rect(bbox)?;
        // Make sure our filter region is not bigger than 4x the canvas size.
        // This is required mainly to prevent huge filter regions that would tank the performance.
        // It should not affect the final result in any way.
        crate::geom::fit_to_rect(bbox, ctx.max_bbox)?
    };

    // Make sure our layer is not bigger than 4x the canvas size.
    // This is required to prevent huge layers.
    if group.filters().is_empty() {
        ibbox = crate::geom::fit_to_rect(ibbox, ctx.max_bbox)?;
    }

    let shift_ts = {
        // Original shift.
        let mut dx = bbox.x();
        let mut dy = bbox.y();

        // Account for subpixel positioned layers.
        dx -= bbox.x() - ibbox.x() as f32;
        dy -= bbox.y() - ibbox.y() as f32;

        tiny_skia::Transform::from_translate(-dx, -dy)
    };

    let transform = shift_ts.pre_concat(transform);

    let mut sub_pixmap = tiny_skia::Pixmap::new(ibbox.width(), ibbox.height())
        .log_none(|| log::warn!("Failed to allocate a group layer for: {:?}.", ibbox))?;

    render_nodes(group, ctx, transform, &mut sub_pixmap.as_mut());

    if !group.filters().is_empty() {
        for filter in group.filters() {
            crate::filter::apply(filter, transform, &mut sub_pixmap);
        }
    }

    if let Some(clip_path) = group.clip_path() {
        crate::clip::apply(clip_path, transform, &mut sub_pixmap);
    }

    if let Some(mask) = group.mask() {
        crate::mask::apply(mask, ctx, transform, &mut sub_pixmap);
    }

    Some((sub_pixmap, ibbox))
}

pub fn convert_blend_mode(mode: usvg::BlendMode) -> tiny_skia::BlendMode {
    match mode {
        usvg::BlendMode::Normal => tiny_skia::BlendMode::SourceOver,
        usvg::BlendMode::Multiply => tiny_skia::BlendMode::Multiply,
        usvg::BlendMode::Screen => tiny_skia::BlendMode::Screen,
        usvg::BlendMode::Overlay => tiny_skia::BlendMode::Overlay,
        usvg::BlendMode::Darken => tiny_skia::BlendMode::Darken,
        usvg::BlendMode::Lighten => tiny_skia::BlendMode::Lighten,
        usvg::BlendMode::ColorDodge => tiny_skia::BlendMode::ColorDodge,
        usvg::BlendMode::ColorBurn => tiny_skia::BlendMode::ColorBurn,
        usvg::BlendMode::HardLight => tiny_skia::BlendMode::HardLight,
        usvg::BlendMode::SoftLight => tiny_skia::BlendMode::SoftLight,
        usvg::BlendMode::Difference => tiny_skia::BlendMode::Difference,
        usvg::BlendMode::Exclusion => tiny_skia::BlendMode::Exclusion,
        usvg::BlendMode::Hue => tiny_skia::BlendMode::Hue,
        usvg::BlendMode::Saturation => tiny_skia::BlendMode::Saturation,
        usvg::BlendMode::Color => tiny_skia::BlendMode::Color,
        usvg::BlendMode::Luminosity => tiny_skia::BlendMode::Luminosity,
        usvg::BlendMode::PlusLighter => tiny_skia::BlendMode::Plus,
    }
}

#[cfg(test)]
mod tests {

    // Derived from https://github.com/servo/servo/issues/42258.
    #[test]
    fn filter_bbox_outside_int_rect() {
        let svg = r#"<svg filter="url(#f)"><filter id="f" x="2em"><feFlood/></filter><path d="M0 0H1e8V1"/></svg>"#;
        let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();
        let mut pixmap = tiny_skia::Pixmap::new(1, 1).unwrap();

        // Just make sure we don't panic.
        crate::render(
            &tree,
            tiny_skia::Transform::identity(),
            &mut pixmap.as_mut(),
        );
    }

    // A nearly 90° skew makes the scaled layer of a filter millions of pixels tall.
    #[test]
    fn filter_on_nearly_singular_skew() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200">
            <filter id="f"><feGaussianBlur stdDeviation="2"/></filter>
            <g transform="translate(100 100) skewX(89.9999) translate(-100 -100)">
                <rect x="60" y="60" width="80" height="80" filter="url(#f)"/>
            </g>
        </svg>"#;
        let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();
        let mut pixmap = tiny_skia::Pixmap::new(200, 200).unwrap();

        // Just make sure the layer is not allocated at that size.
        crate::render(
            &tree,
            tiny_skia::Transform::identity(),
            &mut pixmap.as_mut(),
        );
    }
}
