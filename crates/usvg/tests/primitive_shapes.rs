// Copyright 2024 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[test]
fn rect_preserved_as_rectangle_node() {
    let svg = r#"
    <svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'>
        <rect id='rect1' x='10' y='20' width='30' height='40' rx='5' ry='5'/>
    </svg>
    "#;

    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    let root = tree.root();

    assert_eq!(root.children().len(), 1);

    match &root.children()[0] {
        usvg::Node::Rectangle(ref rect) => {
            assert_eq!(rect.id(), "rect1");
            assert_eq!(rect.x(), 10.0);
            assert_eq!(rect.y(), 20.0);
            assert_eq!(rect.width(), 30.0);
            assert_eq!(rect.height(), 40.0);
            assert_eq!(rect.rx(), 5.0);
            assert_eq!(rect.ry(), 5.0);
            assert!(rect.is_visible());
        }
        _ => panic!("Expected Rectangle node, got something else"),
    }
}

#[test]
fn ellipse_preserved_as_ellipse_node() {
    let svg = r#"
    <svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'>
        <ellipse id='ellipse1' cx='50' cy='50' rx='30' ry='20'/>
    </svg>
    "#;

    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    let root = tree.root();

    assert_eq!(root.children().len(), 1);

    match &root.children()[0] {
        usvg::Node::Ellipse(ref ellipse) => {
            assert_eq!(ellipse.id(), "ellipse1");
            assert_eq!(ellipse.cx(), 50.0);
            assert_eq!(ellipse.cy(), 50.0);
            assert_eq!(ellipse.rx(), 30.0);
            assert_eq!(ellipse.ry(), 20.0);
            assert!(ellipse.is_visible());
        }
        _ => panic!("Expected Ellipse node, got something else"),
    }
}

#[test]
fn polygon_preserved_as_polygon_node() {
    let svg = r#"
    <svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'>
        <polygon id='polygon1' points='10,10 50,10 50,50 10,50'/>
    </svg>
    "#;

    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    let root = tree.root();

    assert_eq!(root.children().len(), 1);

    match &root.children()[0] {
        usvg::Node::Polygon(ref polygon) => {
            assert_eq!(polygon.id(), "polygon1");
            let points = polygon.points();
            assert_eq!(points.len(), 4);
            assert_eq!(points[0], (10.0, 10.0));
            assert_eq!(points[1], (50.0, 10.0));
            assert_eq!(points[2], (50.0, 50.0));
            assert_eq!(points[3], (10.0, 50.0));
            assert!(polygon.is_visible());
        }
        _ => panic!("Expected Polygon node, got something else"),
    }
}

#[test]
fn multiple_primitive_shapes() {
    let svg = r#"
    <svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'>
        <rect id='rect1' x='10' y='10' width='20' height='20'/>
        <ellipse id='ellipse1' cx='50' cy='50' rx='15' ry='10'/>
        <polygon id='polygon1' points='70,10 90,10 90,30 70,30'/>
    </svg>
    "#;

    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    let root = tree.root();

    assert_eq!(root.children().len(), 3);

    // Check first child is Rectangle
    match &root.children()[0] {
        usvg::Node::Rectangle(ref rect) => {
            assert_eq!(rect.id(), "rect1");
        }
        _ => panic!("Expected Rectangle node"),
    }

    // Check second child is Ellipse
    match &root.children()[1] {
        usvg::Node::Ellipse(ref ellipse) => {
            assert_eq!(ellipse.id(), "ellipse1");
        }
        _ => panic!("Expected Ellipse node"),
    }

    // Check third child is Polygon
    match &root.children()[2] {
        usvg::Node::Polygon(ref polygon) => {
            assert_eq!(polygon.id(), "polygon1");
        }
        _ => panic!("Expected Polygon node"),
    }
}

#[test]
fn primitive_shape_bounding_boxes() {
    let svg = r#"
    <svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'>
        <rect id='rect1' x='10' y='20' width='30' height='40'/>
    </svg>
    "#;

    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    let root = tree.root();

    match &root.children()[0] {
        usvg::Node::Rectangle(ref rect) => {
            let bbox = rect.bounding_box();
            assert_eq!(bbox.x(), 10.0);
            assert_eq!(bbox.y(), 20.0);
            assert_eq!(bbox.width(), 30.0);
            assert_eq!(bbox.height(), 40.0);

            // Check absolute bounding box
            let abs_bbox = rect.abs_bounding_box();
            assert!(abs_bbox.width() > 0.0);
            assert!(abs_bbox.height() > 0.0);
        }
        _ => panic!("Expected Rectangle node"),
    }
}
