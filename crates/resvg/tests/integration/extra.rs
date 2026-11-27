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
fn raw_image() {
    use std::sync::Arc;

    // A 2x1 image: opaque red and half-transparent blue.
    let pixels: Arc<Vec<u8>> = Arc::new(vec![255, 0, 0, 255, 0, 0, 255, 128]);
    let image = usvg::RawImage::from_rgba8(2, 1, pixels).unwrap();

    let mut opt = usvg::Options::default();
    opt.image_href_resolver.resolve_string =
        Box::new(move |href, _| (href == "pixels").then(|| usvg::ImageKind::RGBA(image.clone())));

    let svg = "<svg xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink' \
               width='4' height='2'>\
               <image xlink:href='pixels' width='4' height='2' style='image-rendering:pixelated'/>\
               </svg>";
    let tree = usvg::Tree::from_str(svg, &opt).unwrap();

    let mut pixmap = tiny_skia::Pixmap::new(4, 2).unwrap();
    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());

    let red = tiny_skia::ColorU8::from_rgba(255, 0, 0, 255).premultiply();
    let blue = tiny_skia::ColorU8::from_rgba(0, 0, 255, 128).premultiply();
    for y in 0..2 {
        assert_eq!(pixmap.pixel(0, y), Some(red));
        assert_eq!(pixmap.pixel(1, y), Some(red));
        assert_eq!(pixmap.pixel(2, y), Some(blue));
        assert_eq!(pixmap.pixel(3, y), Some(blue));
    }
}

#[test]
fn raw_image_invalid_size() {
    use std::sync::Arc;

    assert!(usvg::RawImage::from_rgba8(2, 2, Arc::new(vec![0u8; 12])).is_none());
    assert!(usvg::RawImage::from_rgba8(0, 1, Arc::new(Vec::<u8>::new())).is_none());
}
