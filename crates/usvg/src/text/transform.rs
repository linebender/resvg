pub(crate) fn skrifa_to_tsp_tranform(t: skrifa::color::Transform) -> tiny_skia_path::Transform {
    tiny_skia_path::Transform::from_row(t.xx, t.yx, t.xy, t.yy, t.dx, t.dy)
}

pub(crate) fn tsp_to_skrifa_tranform(t: tiny_skia_path::Transform) -> skrifa::color::Transform {
    skrifa::color::Transform {
        xx: t.sx,
        yx: t.ky,
        xy: t.kx,
        yy: t.sy,
        dx: t.tx,
        dy: t.ty,
    }
}
