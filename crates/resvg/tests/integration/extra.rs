// Copyright 2023 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::{render_extra, render_extra_with_scale, render_node};

#[test]
fn group_with_only_transform() {
    assert_eq!(render_extra("extra/group-with-only-transform"), 0);
}

#[test]
fn subpixel_rect_position() {
    assert_eq!(render_extra("extra/subpixel-rect-position"), 0);
}

#[test]
fn transformed_rect() {
    assert_eq!(render_extra("extra/transformed-rect"), 0);
}

#[test]
fn hidden_element() {
    assert_eq!(render_extra("extra/hidden-element"), 0);
}

#[test]
fn simple_stroke() {
    assert_eq!(render_extra("extra/simple-stroke"), 0);
}

#[test]
fn fill_and_stroke() {
    assert_eq!(render_extra("extra/fill-and-stroke"), 0);
}

#[test]
fn paint_order_stroke() {
    assert_eq!(render_extra("extra/paint-order=stroke"), 0);
}

#[test]
fn stroke_linecap_square() {
    assert_eq!(render_extra("extra/stroke-linecap=square"), 0);
}

#[test]
fn miter_join_with_acute_angle() {
    assert_eq!(render_extra("extra/miter-join-with-acute-angle"), 0);
}

#[test]
fn horizontal_line() {
    assert_eq!(render_extra("extra/horizontal-line"), 0);
}

#[test]
fn horizontal_line_no_stroke() {
    assert_eq!(render_extra("extra/horizontal-line-no-stroke"), 0);
}

#[test]
fn filter_region_precision() {
    assert_eq!(
        render_extra_with_scale("extra/filter-region-precision", 10.0),
        0
    );
}

#[test]
fn translate_outside_viewbox() {
    assert_eq!(render_extra("extra/translate-outside-viewbox"), 0);
}

#[test]
fn render_node_filter_on_empty_group() {
    assert_eq!(render_node("extra/filter-on-empty-group", "g1"), 0);
}

#[test]
fn render_node_filter_with_transform_on_shape() {
    assert_eq!(render_node("extra/filter-with-transform-on-shape", "g1"), 0);
}

#[test]
fn kernel_unit_length_diffuse_lighting() {
    let svg = "
    <svg viewBox='0 0 100 100' xmlns='http://www.w3.org/2000/svg'>
        <filter id='light' filterUnits='userSpaceOnUse' x='0' y='0' width='100' height='100'>
            <feDiffuseLighting in='SourceGraphic' kernelUnitLength='2 2' surfaceScale='5' diffuseConstant='1'>
                <feDistantLight azimuth='45' elevation='60'/>
            </feDiffuseLighting>
        </filter>
        <circle cx='50' cy='50' r='30' fill='white' filter='url(#light)'/>
    </svg>
    ";

    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();
    let mut pixmap = tiny_skia::Pixmap::new(100, 100).unwrap();
    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );

    // Verify non-empty rendering and diffuse shading
    let non_zero_pixels = pixmap.data().iter().filter(|&&b| b != 0).count();
    assert!(non_zero_pixels > 0);
}

#[test]
fn kernel_unit_length_specular_lighting() {
    let svg = "
    <svg viewBox='0 0 100 100' xmlns='http://www.w3.org/2000/svg'>
        <filter id='spec' filterUnits='userSpaceOnUse' x='0' y='0' width='100' height='100'>
            <feSpecularLighting in='SourceGraphic' kernelUnitLength='2 2' surfaceScale='5' specularConstant='1' specularExponent='20'>
                <fePointLight x='50' y='50' z='30'/>
            </feSpecularLighting>
        </filter>
        <circle cx='50' cy='50' r='30' fill='white' filter='url(#spec)'/>
    </svg>
    ";

    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();
    let mut pixmap = tiny_skia::Pixmap::new(100, 100).unwrap();
    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );

    let non_zero_pixels = pixmap.data().iter().filter(|&&b| b != 0).count();
    assert!(non_zero_pixels > 0);
}

#[test]
fn kernel_unit_length_convolve_matrix() {
    let svg = "
    <svg viewBox='0 0 100 100' xmlns='http://www.w3.org/2000/svg'>
        <filter id='conv' filterUnits='userSpaceOnUse' x='0' y='0' width='100' height='100'>
            <feConvolveMatrix order='3' kernelMatrix='1 1 1 1 1 1 1 1 1' divisor='9' kernelUnitLength='2 2'/>
        </filter>
        <rect x='30' y='30' width='40' height='40' fill='black' filter='url(#conv)'/>
    </svg>
    ";

    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();
    let mut pixmap = tiny_skia::Pixmap::new(100, 100).unwrap();
    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );

    let non_zero_pixels = pixmap.data().iter().filter(|&&b| b != 0).count();
    assert!(non_zero_pixels > 0);
}
