// Copyright 2026 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use resvg::tiny_skia::{self, Pixmap, Transform};

fn render(content: &str, transform: Transform) -> Pixmap {
    let tree = resvg::usvg::Tree::from_str(
        &format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24">{content}</svg>"#
        ),
        &resvg::usvg::Options::default(),
    )
    .unwrap();
    let mut pixmap = Pixmap::new(24, 24).unwrap();
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    pixmap
}

#[test]
fn fractional_edges_use_area_coverage() {
    let horizontal = render(
        r#"<rect width="8.130081" height="20"/>"#,
        Transform::identity(),
    );
    let vertical = render(
        r#"<rect width="20" height="8.130081"/>"#,
        Transform::identity(),
    );
    // About 13% of the boundary pixel is covered, not the 25% produced
    // by the general path rasterizer's four-by-four coverage grid.
    assert_eq!(horizontal.pixel(8, 10).unwrap().alpha(), 33);
    assert_eq!(vertical.pixel(10, 8).unwrap().alpha(), 33);
    assert_eq!(horizontal.pixel(7, 10).unwrap().alpha(), 255);
    assert_eq!(horizontal.pixel(9, 10).unwrap().alpha(), 0);
}

#[test]
fn scaled_and_reflected_rectangles_keep_precise_coverage() {
    let scaled = render(
        r#"<rect width="4" height="8.3"/>"#,
        Transform::from_scale(25.0 / 12.3, 17.0 / 8.3),
    );
    assert_eq!(scaled.pixel(8, 10).unwrap().alpha(), 33);
    let reflected = render(
        r#"<rect width="8.130081" height="20"/>"#,
        Transform::from_row(-1.0, 0.0, 0.0, 1.0, 16.0, 0.0),
    );
    assert_eq!(reflected.pixel(7, 10).unwrap().alpha(), 33);
}

#[test]
fn rectangle_paths_accept_both_winding_directions() {
    for path in ["M0 0H8.130081V20H0Z", "M0 0V20H8.130081V0Z"] {
        let pixmap = render(&format!(r#"<path d="{path}"/>"#), Transform::identity());
        assert_eq!(pixmap.pixel(8, 10).unwrap().alpha(), 33);
    }
}

#[test]
fn gradient_coordinates_follow_the_transformed_rectangle() {
    let transform = Transform::from_row(2.0, 0.0, 0.0, 1.0, 1.25, 0.5);
    let actual = render(
        r#"<defs><linearGradient id="g" gradientUnits="userSpaceOnUse" x2="4">
        <stop stop-color="red"/><stop offset="1" stop-color="blue"/></linearGradient></defs>
        <rect width="4" height="8" fill="url(#g)"/>"#,
        transform,
    );
    let mut expected = Pixmap::new(24, 24).unwrap();
    let mut paint = tiny_skia::Paint::default();
    paint.shader = tiny_skia::LinearGradient::new(
        (0.0, 0.0).into(),
        (4.0, 0.0).into(),
        vec![
            tiny_skia::GradientStop::new(0.0, tiny_skia::Color::from_rgba8(255, 0, 0, 255)),
            tiny_skia::GradientStop::new(1.0, tiny_skia::Color::from_rgba8(0, 0, 255, 255)),
        ],
        tiny_skia::SpreadMode::Pad,
        transform,
    )
    .unwrap();
    expected.fill_rect(
        tiny_skia::Rect::from_xywh(1.25, 0.5, 8.0, 8.0).unwrap(),
        &paint,
        Transform::identity(),
        None,
    );
    assert_eq!(actual.data(), expected.data());
}

#[test]
fn clipping_and_group_opacity_preserve_fractional_edges() {
    let clipped = render(
        r#"<defs><clipPath id="c"><rect width="8.130081" height="20"/></clipPath></defs>
        <rect width="20" height="20" clip-path="url(#c)"/>"#,
        Transform::identity(),
    );
    assert_eq!(clipped.pixel(8, 10).unwrap().alpha(), 33);
    let opacity = render(
        r#"<g opacity=".5"><rect width="8.130081" height="20"/></g>"#,
        Transform::identity(),
    );
    assert!((16..=17).contains(&opacity.pixel(8, 10).unwrap().alpha()));
}

#[test]
fn other_paths_and_skewed_rectangles_keep_the_general_rasterizer() {
    for (data, transform, rule) in [
        (
            "M0 0L8.130081 0L0 20Z",
            Transform::identity(),
            tiny_skia::FillRule::Winding,
        ),
        (
            "M0 0H8.130081V20H0Z M2 2H6V18H2Z",
            Transform::identity(),
            tiny_skia::FillRule::EvenOdd,
        ),
        (
            "M0 0H8.130081V20H0Z",
            Transform::from_skew(0.2, 0.0),
            tiny_skia::FillRule::Winding,
        ),
        (
            "M0 0Q8 0 8.130081 10L0 20Z",
            Transform::identity(),
            tiny_skia::FillRule::Winding,
        ),
        (
            "M0 0L8 20L0 20L8 0Z",
            Transform::identity(),
            tiny_skia::FillRule::Winding,
        ),
    ] {
        let content = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24">
            <path id="p" d="{data}" fill-rule="evenodd"/></svg>"#
        );
        let tree = resvg::usvg::Tree::from_str(&content, &resvg::usvg::Options::default()).unwrap();
        let resvg::usvg::Node::Path(path) = tree.node_by_id("p").unwrap() else {
            panic!()
        };
        let mut actual = Pixmap::new(24, 24).unwrap();
        resvg::render(&tree, transform, &mut actual.as_mut());
        let mut expected = Pixmap::new(24, 24).unwrap();
        expected.fill_path(
            path.data(),
            &tiny_skia::Paint::default(),
            rule,
            transform,
            None,
        );
        assert_eq!(actual.data(), expected.data(), "{data}");
    }
}

#[test]
fn crisp_edges_and_large_coordinates_do_not_panic() {
    let crisp = render(
        r#"<rect width="8.130081" height="20" shape-rendering="crispEdges"/>"#,
        Transform::identity(),
    );
    assert_eq!(crisp.pixel(8, 10).unwrap().alpha(), 0);
    let huge = render(
        r#"<rect x="-1e8" y="-1e8" width="2e8" height="2e8"/>"#,
        Transform::identity(),
    );
    assert!(huge.pixels().iter().all(|pixel| pixel.alpha() == 255));
}
