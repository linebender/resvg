// Copyright 2018 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::sync::Arc;

use strict_num::NonZeroPositiveF32;
use svgtypes::Length;
use tiny_skia_path::Point;

use super::converter;
use super::svgtree::{AId, EId, SvgNode};
use crate::{
    ApproxEqUlps, ApproxZeroUlps, ClipPath, Fill, Group, Node, NonZeroRect, Path, Transform,
    ViewBox,
};

// Similar to `tiny_skia_path::PathSegment`, but without the `QuadTo`.
#[derive(Copy, Clone, Debug)]
enum Segment {
    MoveTo(Point),
    LineTo(Point),
    CubicTo(Point, Point, Point),
    Close,
}

fn same_point(a: Point, b: Point) -> bool {
    a.x.approx_eq_ulps(&b.x, 4) && a.y.approx_eq_ulps(&b.y, 4)
}

pub(crate) fn is_valid(node: SvgNode) -> bool {
    // `marker-*` attributes cannot be set on shapes inside a `clipPath`.
    if node
        .ancestors()
        .any(|n| n.tag_name() == Some(EId::ClipPath))
    {
        return false;
    }

    let start = node.find_attribute::<SvgNode>(AId::MarkerStart);
    let mid = node.find_attribute::<SvgNode>(AId::MarkerMid);
    let end = node.find_attribute::<SvgNode>(AId::MarkerEnd);
    start.is_some() || mid.is_some() || end.is_some()
}

pub(crate) fn convert(
    node: SvgNode,
    path: &tiny_skia_path::Path,
    state: &converter::State,
    cache: &mut converter::Cache,
    parent: &mut Group,
) {
    let list = [
        (AId::MarkerStart, MarkerKind::Start),
        (AId::MarkerMid, MarkerKind::Middle),
        (AId::MarkerEnd, MarkerKind::End),
    ];

    for (aid, kind) in &list {
        let mut marker = None;
        if let Some(link) = node.find_attribute::<SvgNode>(*aid) {
            if link.tag_name() == Some(EId::Marker) {
                marker = Some(link);
            }
        }

        if let Some(marker) = marker {
            resolve(node, path, marker, *kind, state, cache, parent);
        }
    }
}

#[derive(Clone, Copy)]
enum MarkerKind {
    Start,
    Middle,
    End,
}

#[derive(Clone, Copy)]
enum MarkerOrientation {
    Auto,
    AutoStartReverse,
    Angle(f32),
}

fn resolve(
    shape_node: SvgNode,
    path: &tiny_skia_path::Path,
    marker_node: SvgNode,
    marker_kind: MarkerKind,
    state: &converter::State,
    cache: &mut converter::Cache,
    parent: &mut Group,
) -> Option<()> {
    let stroke_scale = stroke_scale(shape_node, marker_node, state)?.get();

    let r = convert_rect(marker_node, state)?;

    let view_box_ts = marker_node
        .parse_viewbox()
        .map(|vb| {
            ViewBox {
                rect: vb,
                aspect: marker_node
                    .attribute(AId::PreserveAspectRatio)
                    .unwrap_or_default(),
            }
            .to_transform(r.size())
        })
        .unwrap_or_default();

    let has_overflow = {
        let overflow = marker_node.attribute(AId::Overflow);
        // `overflow` is `hidden` by default.
        overflow.is_none() || overflow == Some("hidden") || overflow == Some("scroll")
    };

    let clip_path = if has_overflow {
        let clip_rect = r
            .size()
            .to_non_zero_rect(0.0, 0.0)
            .transform(view_box_ts.invert()?)?;

        let mut clip_path = ClipPath::empty(cache.gen_clip_path_id());

        let mut path = Path::new_simple(Arc::new(tiny_skia_path::PathBuilder::from_rect(
            clip_rect.to_rect(),
        )));
        path.fill = Some(Fill::default());

        clip_path.root.children.push(Node::Path(Box::new(path)));

        Some(Arc::new(clip_path))
    } else {
        None
    };

    // TODO: avoid allocation
    let mut segments: Vec<Segment> = Vec::with_capacity(path.len());
    let mut prev = Point::zero();
    let mut prev_move = Point::zero();
    for seg in path.segments() {
        match seg {
            tiny_skia_path::PathSegment::MoveTo(p) => {
                segments.push(Segment::MoveTo(p));
                prev = p;
                prev_move = p;
            }
            tiny_skia_path::PathSegment::LineTo(p) => {
                segments.push(Segment::LineTo(p));
                prev = p;
            }
            tiny_skia_path::PathSegment::QuadTo(p1, p) => {
                let (p1, p2, p) = quad_to_curve(prev, p1, p);
                segments.push(Segment::CubicTo(p1, p2, p));
                prev = p;
            }
            tiny_skia_path::PathSegment::CubicTo(p1, p2, p) => {
                segments.push(Segment::CubicTo(p1, p2, p));
                prev = p;
            }
            tiny_skia_path::PathSegment::Close => {
                segments.push(Segment::Close);
                prev = prev_move;
            }
        }
    }

    let orientation = convert_orientation(marker_node);
    let vertex_angles = match orientation {
        MarkerOrientation::Auto | MarkerOrientation::AutoStartReverse => {
            calc_vertex_angles(&segments)
        }
        MarkerOrientation::Angle(_) => Vec::new(),
    };

    let draw_marker = |p: tiny_skia_path::Point, idx: usize| {
        let mut ts = Transform::from_translate(p.x, p.y);

        let angle = match orientation {
            MarkerOrientation::AutoStartReverse if idx == 0 => (vertex_angles[idx] + 180.0) % 360.0,
            MarkerOrientation::Auto | MarkerOrientation::AutoStartReverse => vertex_angles[idx],
            MarkerOrientation::Angle(angle) => angle,
        };

        if !angle.approx_zero_ulps(4) {
            ts = ts.pre_rotate(angle);
        }

        let (sx, sy) = view_box_ts.get_scale();
        ts = ts.pre_scale(sx * stroke_scale, sy * stroke_scale);
        ts = ts.pre_translate(-r.x(), -r.y());

        // TODO: do not create a group when no clipPath
        let mut g = Group {
            transform: ts,
            abs_transform: parent.abs_transform.pre_concat(ts),
            clip_path: clip_path.clone(),
            ..Group::empty()
        };

        let mut marker_state = state.clone();
        marker_state.inside_marker = true;
        converter::convert_children(marker_node, &marker_state, cache, &mut g);

        if g.has_children() {
            parent.children.push(Node::Group(Box::new(g)));
        }
    };

    draw_markers(&segments, marker_kind, draw_marker);

    Some(())
}

fn stroke_scale(
    path_node: SvgNode,
    marker_node: SvgNode,
    state: &converter::State,
) -> Option<NonZeroPositiveF32> {
    match marker_node.attribute(AId::MarkerUnits) {
        Some("userSpaceOnUse") => NonZeroPositiveF32::new(1.0),
        _ => path_node.resolve_valid_length(AId::StrokeWidth, state, 1.0),
    }
}

fn draw_markers<P>(path: &[Segment], kind: MarkerKind, mut draw_marker: P)
where
    P: FnMut(tiny_skia_path::Point, usize),
{
    match kind {
        MarkerKind::Start => {
            if let Some(Segment::MoveTo(p)) = path.first().cloned() {
                draw_marker(p, 0);
            }
        }
        MarkerKind::Middle => {
            let total = path.len() - 1;
            let mut i = 1;
            while i < total {
                let p = match path[i] {
                    Segment::MoveTo(p) => p,
                    Segment::LineTo(p) => p,
                    Segment::CubicTo(_, _, p) => p,
                    _ => {
                        i += 1;
                        continue;
                    }
                };

                draw_marker(p, i);

                i += 1;
            }
        }
        MarkerKind::End => {
            let idx = path.len() - 1;
            match path.last().cloned() {
                Some(Segment::LineTo(p)) => {
                    draw_marker(p, idx);
                }
                Some(Segment::CubicTo(_, _, p)) => {
                    draw_marker(p, idx);
                }
                Some(Segment::Close) => {
                    let p = get_subpath_start(path, idx);
                    draw_marker(p, idx);
                }
                _ => {}
            }
        }
    }
}

fn calc_vertex_angles(path: &[Segment]) -> Vec<f32> {
    let mut preceding_end = vec![None; path.len()];
    let mut last = None;
    for idx in 0..path.len() {
        if let Segment::MoveTo(_) = path[idx] {
            last = None;
            continue;
        }
        preceding_end[idx] = last;
        last = end_tangent(path, idx).or(last);
    }

    let mut following_start = vec![None; path.len()];
    let mut next = None;
    for idx in (0..path.len()).rev() {
        if let Segment::MoveTo(_) = path[idx] {
            next = None;
            continue;
        }
        following_start[idx] = next;
        next = start_tangent(path, idx).or(next);
    }

    let start_direction = |idx: usize| {
        start_tangent(path, idx)
            .or(preceding_end[idx])
            .or(following_start[idx])
    };
    let end_direction = |idx: usize| {
        end_tangent(path, idx)
            .or(following_start[idx])
            .or(preceding_end[idx])
    };

    let mut angles = vec![0.0; path.len()];
    let mut start = 0;
    while start < path.len() {
        let end = (start + 1..path.len())
            .find(|&idx| matches!(path[idx], Segment::MoveTo(_)))
            .unwrap_or(path.len());
        let closed = matches!(path[end - 1], Segment::Close);

        for idx in start..end {
            let (preceding, following) =
                if closed && matches!(path[idx], Segment::MoveTo(_) | Segment::Close) {
                    (Some(end - 1), Some(start + 1))
                } else {
                    (
                        (idx > start).then_some(idx),
                        (idx + 1 < end).then_some(idx + 1),
                    )
                };

            let incoming = preceding.and_then(end_direction);
            let outgoing = following.and_then(start_direction);
            angles[idx] = match (incoming, outgoing) {
                (Some(incoming), Some(outgoing)) => calc_angle(incoming, outgoing),
                (Some(direction), None) | (None, Some(direction)) => {
                    calc_angle(direction, direction)
                }
                (None, None) => 0.0,
            };
        }

        start = end;
    }

    angles
}

fn start_tangent(path: &[Segment], idx: usize) -> Option<(Point, Point)> {
    let towards = match path[idx] {
        Segment::MoveTo(_) => return None,
        Segment::LineTo(p) => [p; 3],
        Segment::CubicTo(p1, p2, p) => [p1, p2, p],
        Segment::Close => [get_subpath_start(path, idx); 3],
    };
    let from = get_prev_vertex(path, idx);
    towards
        .into_iter()
        .find(|&p| !same_point(from, p))
        .map(|p| (from, p))
}

fn end_tangent(path: &[Segment], idx: usize) -> Option<(Point, Point)> {
    let prev = || get_prev_vertex(path, idx);
    let (to, from) = match path[idx] {
        Segment::MoveTo(_) => return None,
        Segment::LineTo(p) => (p, [prev(); 3]),
        Segment::CubicTo(p1, p2, p) => (p, [p2, p1, prev()]),
        Segment::Close => (get_subpath_start(path, idx), [prev(); 3]),
    };
    from.into_iter()
        .find(|&p| !same_point(p, to))
        .map(|p| (p, to))
}

fn calc_angle((a, b): (Point, Point), (c, d): (Point, Point)) -> f32 {
    use std::f32::consts::*;

    fn normalize(rad: f32) -> f32 {
        let v = rad % (PI * 2.0);
        if v < 0.0 { v + PI * 2.0 } else { v }
    }

    fn vector_angle(vx: f32, vy: f32) -> f32 {
        let rad = vy.atan2(vx);
        if rad.is_nan() { 0.0 } else { normalize(rad) }
    }

    let in_a = vector_angle(b.x - a.x, b.y - a.y);
    let out_a = vector_angle(d.x - c.x, d.y - c.y);
    let d = (out_a - in_a) * 0.5;

    let mut angle = in_a + d;
    if FRAC_PI_2 < d.abs() {
        angle -= PI;
    }

    normalize(angle).to_degrees()
}

fn get_subpath_start(segments: &[Segment], idx: usize) -> tiny_skia_path::Point {
    let offset = segments.len() - idx;
    for seg in segments.iter().rev().skip(offset) {
        if let Segment::MoveTo(p) = *seg {
            return p;
        }
    }

    tiny_skia_path::Point::zero()
}

fn get_prev_vertex(segments: &[Segment], idx: usize) -> tiny_skia_path::Point {
    match segments[idx - 1] {
        Segment::MoveTo(p) => p,
        Segment::LineTo(p) => p,
        Segment::CubicTo(_, _, p) => p,
        Segment::Close => get_subpath_start(segments, idx),
    }
}

fn convert_rect(node: SvgNode, state: &converter::State) -> Option<NonZeroRect> {
    NonZeroRect::from_xywh(
        node.convert_user_length(AId::RefX, state, Length::zero()),
        node.convert_user_length(AId::RefY, state, Length::zero()),
        node.convert_user_length(AId::MarkerWidth, state, Length::new_number(3.0)),
        node.convert_user_length(AId::MarkerHeight, state, Length::new_number(3.0)),
    )
}

fn convert_orientation(node: SvgNode) -> MarkerOrientation {
    match node.attribute(AId::Orient) {
        Some("auto") => MarkerOrientation::Auto,
        Some("auto-start-reverse") => MarkerOrientation::AutoStartReverse,
        _ => match node.attribute::<svgtypes::Angle>(AId::Orient) {
            Some(angle) => MarkerOrientation::Angle(angle.to_degrees() as f32),
            None => MarkerOrientation::Angle(0.0),
        },
    }
}

fn quad_to_curve(prev: Point, p1: Point, p: Point) -> (Point, Point, Point) {
    #[inline]
    fn calc(n1: f32, n2: f32) -> f32 {
        (n1 + n2 * 2.0) / 3.0
    }

    (
        Point::from_xy(calc(prev.x, p1.x), calc(prev.y, p1.y)),
        Point::from_xy(calc(p.x, p1.x), calc(p.y, p1.y)),
        p,
    )
}
