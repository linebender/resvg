// Copyright 2018 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use filtra::cpu;
use filtra::types as ft;

pub fn apply(
    filter: &usvg::filter::Filter,
    ts: tiny_skia::Transform,
    source: &mut tiny_skia::Pixmap,
) {
    let (sx, sy) = ts.get_scale();
    let transform = filtra::Transform::new(sx, sy, ts.tx, ts.ty);

    let (width, height) = (source.width(), source.height());
    let pixmap = cpu::PixmapMut::from_data(source.data_mut(), width, height);
    let mut pool = cpu::BufferPool::new(0);
    filtra::apply(
        filter,
        pixmap,
        &mut pool,
        transform,
        |root, subregion, rect, data| {
            render_image(root, subregion, transform, rect, data);
        },
    );
}

fn render_image(
    root: &usvg::Group,
    subregion: ft::Rect,
    transform: filtra::Transform,
    rect: filtra::IntRect,
    data: &mut [u8],
) {
    let (width, height) = (rect.width(), rect.height());
    let Some(mut target) = tiny_skia::PixmapMut::from_bytes(data, width, height) else {
        return;
    };

    // usvg places the content relative to the top-left corner of the subregion.
    let ts = tiny_skia::Transform::from_row(
        transform.sx,
        0.0,
        0.0,
        transform.sy,
        subregion.x * transform.sx + transform.tx - rect.x0 as f32,
        subregion.y * transform.sy + transform.ty - rect.y0 as f32,
    );
    let bbox = tiny_skia::IntRect::from_xywh(0, 0, width, height).unwrap();
    let ctx = crate::render::Context {
        max_bbox: bbox,
        needed_bbox: bbox,
    };

    crate::render::render_nodes(root, &ctx, ts, &mut target);
}
