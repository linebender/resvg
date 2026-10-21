// Copyright 2018 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use tiny_skia_path::Rect;
use usvg::Color;

#[test]
fn gradient_stop_offset_overflowing_f32() {
    // `4e38` overflows f32 to infinity; parsing must not panic.
    let svg = "<svg xmlns='http://www.w3.org/2000/svg'>\
        <defs><linearGradient id='g'><stop offset='4e38'/></linearGradient></defs>\
        <rect width='1' height='1' fill='url(#g)'/>\
    </svg>";
    assert!(usvg::Tree::from_str(svg, &usvg::Options::default()).is_ok());
}

#[test]
fn clippath_with_invalid_child() {
    let svg = "
    <svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 1 1'>
        <clipPath id='clip1'>
            <rect/>
        </clipPath>
        <rect clip-path='url(#clip1)' width='10' height='10'/>
    </svg>
    ";

    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    // clipPath is invalid and should be removed together with rect.
    assert!(!tree.root().has_children());
}

#[test]
fn stylesheet_injection() {
    let svg = "<svg id='svg1' viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'>
    <style>
        #rect4 {
            fill: green
        }
    </style>
    <rect id='rect1' x='20' y='20' width='60' height='60'/>
    <rect id='rect2' x='120' y='20' width='60' height='60' fill='green'/>
    <rect id='rect3' x='20' y='120' width='60' height='60' style='fill: green'/>
    <rect id='rect4' x='120' y='120' width='60' height='60'/>
    <rect id='rect5' x='70' y='70' width='60' height='60' style='fill: green !important'/>
</svg>
";

    let stylesheet = "rect { fill: red }".to_string();

    let options = usvg::Options {
        style_sheet: Some(stylesheet),
        ..usvg::Options::default()
    };

    let tree = usvg::Tree::from_str(&svg, &options).unwrap();

    let usvg::Node::Path(first) = &tree.root().children()[0] else {
        unreachable!()
    };

    // Only the rects with no CSS attributes should be overridden.
    assert_eq!(
        first.fill().unwrap().paint(),
        &usvg::Paint::Color(Color::new_rgb(255, 0, 0))
    );

    let usvg::Node::Path(second) = &tree.root().children()[1] else {
        unreachable!()
    };
    assert_eq!(
        second.fill().unwrap().paint(),
        &usvg::Paint::Color(Color::new_rgb(255, 0, 0))
    );

    let usvg::Node::Path(third) = &tree.root().children()[2] else {
        unreachable!()
    };
    assert_eq!(
        third.fill().unwrap().paint(),
        &usvg::Paint::Color(Color::new_rgb(0, 128, 0))
    );

    let usvg::Node::Path(fourth) = &tree.root().children()[3] else {
        unreachable!()
    };
    assert_eq!(
        fourth.fill().unwrap().paint(),
        &usvg::Paint::Color(Color::new_rgb(0, 128, 0))
    );

    let usvg::Node::Path(fifth) = &tree.root().children()[4] else {
        unreachable!()
    };
    assert_eq!(
        fifth.fill().unwrap().paint(),
        &usvg::Paint::Color(Color::new_rgb(0, 128, 0))
    );
}

#[test]
fn stylesheet_injection_with_important() {
    let svg = "<svg id='svg1' viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'>
    <style>
        #rect4 {
            fill: green
        }
    </style>
    <rect id='rect1' x='20' y='20' width='60' height='60'/>
    <rect id='rect2' x='120' y='20' width='60' height='60' fill='green'/>
    <rect id='rect3' x='20' y='120' width='60' height='60' style='fill: green'/>
    <rect id='rect4' x='120' y='120' width='60' height='60'/>
    <rect id='rect5' x='70' y='70' width='60' height='60' style='fill: green !important'/>
</svg>
";

    let stylesheet = "rect { fill: red !important }".to_string();

    let options = usvg::Options {
        style_sheet: Some(stylesheet),
        ..usvg::Options::default()
    };

    let tree = usvg::Tree::from_str(&svg, &options).unwrap();

    let usvg::Node::Path(first) = &tree.root().children()[0] else {
        unreachable!()
    };

    // All rects should be overridden, since we use `important`.
    assert_eq!(
        first.fill().unwrap().paint(),
        &usvg::Paint::Color(Color::new_rgb(255, 0, 0))
    );

    let usvg::Node::Path(second) = &tree.root().children()[1] else {
        unreachable!()
    };
    assert_eq!(
        second.fill().unwrap().paint(),
        &usvg::Paint::Color(Color::new_rgb(255, 0, 0))
    );

    let usvg::Node::Path(third) = &tree.root().children()[2] else {
        unreachable!()
    };
    assert_eq!(
        third.fill().unwrap().paint(),
        &usvg::Paint::Color(Color::new_rgb(255, 0, 0))
    );

    let usvg::Node::Path(third) = &tree.root().children()[3] else {
        unreachable!()
    };
    assert_eq!(
        third.fill().unwrap().paint(),
        &usvg::Paint::Color(Color::new_rgb(255, 0, 0))
    );

    let usvg::Node::Path(third) = &tree.root().children()[4] else {
        unreachable!()
    };
    assert_eq!(
        third.fill().unwrap().paint(),
        &usvg::Paint::Color(Color::new_rgb(255, 0, 0))
    );
}

#[test]
fn simplify_paths() {
    let svg = "
    <svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 1 1'>
        <path d='M 10 20 L 10 30 Z Z Z'/>
    </svg>
    ";

    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    let path = &tree.root().children()[0];
    match path {
        usvg::Node::Path(path) => {
            // Make sure we have MLZ and not MLZZZ
            assert_eq!(path.data().verbs().len(), 3);
        }
        _ => unreachable!(),
    };
}

#[test]
fn size_detection_1() {
    let svg = "<svg viewBox='0 0 10 20' xmlns='http://www.w3.org/2000/svg'/>";
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    assert_eq!(tree.size(), usvg::Size::from_wh(10.0, 20.0).unwrap());
}

#[test]
fn size_detection_2() {
    let svg =
        "<svg width='30' height='40' viewBox='0 0 10 20' xmlns='http://www.w3.org/2000/svg'/>";
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    assert_eq!(tree.size(), usvg::Size::from_wh(30.0, 40.0).unwrap());
}

#[test]
fn size_detection_3() {
    let svg =
        "<svg width='50%' height='100%' viewBox='0 0 10 20' xmlns='http://www.w3.org/2000/svg'/>";
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    assert_eq!(tree.size(), usvg::Size::from_wh(5.0, 20.0).unwrap());
}

#[test]
fn size_detection_4() {
    let svg = "
    <svg xmlns='http://www.w3.org/2000/svg'>
        <circle cx='18' cy='18' r='18'/>
    </svg>
    ";
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    assert_eq!(tree.size(), usvg::Size::from_wh(36.0, 36.0).unwrap());
    assert_eq!(tree.size(), usvg::Size::from_wh(36.0, 36.0).unwrap());
}

#[test]
fn size_detection_5() {
    let svg = "<svg xmlns='http://www.w3.org/2000/svg'/>";
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    assert_eq!(tree.size(), usvg::Size::from_wh(100.0, 100.0).unwrap());
}

#[test]
fn size_detection_6() {
    let svg = "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 10 10' width='30'/>";
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    assert_eq!(tree.size(), usvg::Size::from_wh(30.0, 30.0).unwrap());
}

#[test]
fn size_detection_7() {
    let svg = "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 10 20' width='30'/>";
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    assert_eq!(tree.size(), usvg::Size::from_wh(30.0, 60.0).unwrap());
}

#[test]
fn size_detection_8() {
    let svg = "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 10 10' height='30'/>";
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    assert_eq!(tree.size(), usvg::Size::from_wh(30.0, 30.0).unwrap());
}

#[test]
fn size_detection_9() {
    let svg = "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 10 20' height='30'/>";
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    assert_eq!(tree.size(), usvg::Size::from_wh(15.0, 30.0).unwrap());
}

#[test]
fn invalid_size_1() {
    let svg = "<svg width='0' height='0' viewBox='0 0 10 20' xmlns='http://www.w3.org/2000/svg'/>";
    let result = usvg::Tree::from_str(&svg, &usvg::Options::default());
    assert!(result.is_err());
}

#[test]
fn tree_is_send_and_sync() {
    fn ensure_send_and_sync<T: Send + Sync>() {}
    ensure_send_and_sync::<usvg::Tree>();
}

#[test]
fn path_transform() {
    let svg = "
    <svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'>
        <path transform='translate(10)' d='M 0 0 L 10 10'/>
    </svg>
    ";

    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    assert_eq!(tree.root().children().len(), 1);

    let group_node = &tree.root().children()[0];
    assert!(matches!(group_node, usvg::Node::Group(_)));
    assert_eq!(
        group_node.abs_transform(),
        usvg::Transform::from_translate(10.0, 0.0)
    );

    let group = match group_node {
        usvg::Node::Group(g) => g,
        _ => unreachable!(),
    };

    let path = &group.children()[0];
    assert!(matches!(path, usvg::Node::Path(_)));
    assert_eq!(
        path.abs_transform(),
        usvg::Transform::from_translate(10.0, 0.0)
    );
}

#[test]
fn path_transform_nested() {
    let svg = "
    <svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'>
        <g transform='translate(20)'>
            <path transform='translate(10)' d='M 0 0 L 10 10'/>
        </g>
    </svg>
    ";

    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    assert_eq!(tree.root().children().len(), 1);

    let group_node1 = &tree.root().children()[0];
    assert!(matches!(group_node1, usvg::Node::Group(_)));
    assert_eq!(
        group_node1.abs_transform(),
        usvg::Transform::from_translate(20.0, 0.0)
    );

    let group1 = match group_node1 {
        usvg::Node::Group(g) => g,
        _ => unreachable!(),
    };

    let group_node2 = &group1.children()[0];
    assert!(matches!(group_node2, usvg::Node::Group(_)));
    assert_eq!(
        group_node2.abs_transform(),
        usvg::Transform::from_translate(30.0, 0.0)
    );

    let group2 = match group_node2 {
        usvg::Node::Group(g) => g,
        _ => unreachable!(),
    };

    let path = &group2.children()[0];
    assert!(matches!(path, usvg::Node::Path(_)));
    assert_eq!(
        path.abs_transform(),
        usvg::Transform::from_translate(30.0, 0.0)
    );
}

#[test]
fn path_transform_in_symbol_no_clip() {
    let svg = "
    <svg viewBox='0 0 100 100' xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink'>
        <defs>
            <symbol id='symbol1' overflow='visible'>
                <rect id='rect1' x='0' y='0' width='10' height='10'/>
            </symbol>
        </defs>
        <use id='use1' xlink:href='#symbol1' x='20'/>
    </svg>
    ";

    // Will be parsed as:
    // <svg width="100" height="100" viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
    //     <g id="use1">
    //         <g transform="matrix(1 0 0 1 20 0)">
    //             <path fill="#000000" stroke="none" d="M 0 0 L 10 0 L 10 10 L 0 10 Z"/>
    //         </g>
    //     </g>
    // </svg>

    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();

    let group_node1 = &tree.root().children()[0];
    assert!(matches!(group_node1, usvg::Node::Group(_)));
    assert_eq!(group_node1.id(), "use1");
    assert_eq!(group_node1.abs_transform(), usvg::Transform::default());

    let group1 = match group_node1 {
        usvg::Node::Group(g) => g,
        _ => unreachable!(),
    };

    let group_node2 = &group1.children()[0];
    assert!(matches!(group_node2, usvg::Node::Group(_)));
    assert_eq!(
        group_node2.abs_transform(),
        usvg::Transform::from_translate(20.0, 0.0)
    );

    let group2 = match group_node2 {
        usvg::Node::Group(g) => g,
        _ => unreachable!(),
    };

    let path = &group2.children()[0];
    assert!(matches!(path, usvg::Node::Path(_)));
    assert_eq!(
        path.abs_transform(),
        usvg::Transform::from_translate(20.0, 0.0)
    );
}

#[test]
fn path_transform_in_symbol_with_clip() {
    let svg = "
    <svg viewBox='0 0 100 100' xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink'>
        <defs>
            <symbol id='symbol1' overflow='hidden'>
                <rect id='rect1' x='0' y='0' width='10' height='10'/>
            </symbol>
        </defs>
        <use id='use1' xlink:href='#symbol1' x='20'/>
    </svg>
    ";

    // Will be parsed as:
    // <svg width="100" height="100" viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
    //     <defs>
    //         <clipPath id="clipPath1">
    //             <path fill="#000000" stroke="none" d="M 20 0 L 120 0 L 120 100 L 20 100 Z"/>
    //         </clipPath>
    //     </defs>
    //     <g id="use1" clip-path="url(#clipPath1)">
    //         <g>
    //             <g transform="matrix(1 0 0 1 20 0)">
    //                 <path fill="#000000" stroke="none" d="M 0 0 L 10 0 L 10 10 L 0 10 Z"/>
    //             </g>
    //         </g>
    //     </g>
    // </svg>

    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();

    let group_node1 = &tree.root().children()[0];
    assert!(matches!(group_node1, usvg::Node::Group(_)));
    assert_eq!(group_node1.id(), "use1");
    assert_eq!(group_node1.abs_transform(), usvg::Transform::default());

    let group1 = match group_node1 {
        usvg::Node::Group(g) => g,
        _ => unreachable!(),
    };

    let group_node2 = &group1.children()[0];
    assert!(matches!(group_node2, usvg::Node::Group(_)));
    assert_eq!(group_node2.abs_transform(), usvg::Transform::default());

    let group2 = match group_node2 {
        usvg::Node::Group(g) => g,
        _ => unreachable!(),
    };

    let group_node3 = &group2.children()[0];
    assert!(matches!(group_node3, usvg::Node::Group(_)));
    assert_eq!(
        group_node3.abs_transform(),
        usvg::Transform::from_translate(20.0, 0.0)
    );

    let group3 = match group_node3 {
        usvg::Node::Group(g) => g,
        _ => unreachable!(),
    };

    let path = &group3.children()[0];
    assert!(matches!(path, usvg::Node::Path(_)));
    assert_eq!(
        path.abs_transform(),
        usvg::Transform::from_translate(20.0, 0.0)
    );
}

#[test]
fn path_transform_in_svg() {
    let svg = "
    <svg viewBox='0 0 100 100' xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink'>
        <g id='g1' transform='translate(100 150)'>
            <svg id='svg1' width='100' height='50'>
                <rect id='rect1' width='10' height='10'/>
            </svg>
        </g>
    </svg>
    ";

    // Will be parsed as:
    // <svg width="100" height="100" viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
    //     <defs>
    //         <clipPath id="clipPath1">
    //             <path fill="#000000" stroke="none" d="M 0 0 L 100 0 L 100 50 L 0 50 Z"/>
    //         </clipPath>
    //     </defs>
    //     <g id="g1" transform="matrix(1 0 0 1 100 150)">
    //         <g id="svg1" clip-path="url(#clipPath1)">
    //             <path id="rect1" fill="#000000" stroke="none" d="M 0 0 L 10 0 L 10 10 L 0 10 Z"/>
    //         </g>
    //     </g>
    // </svg>

    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();

    let group_node1 = &tree.root().children()[0];
    assert!(matches!(group_node1, usvg::Node::Group(_)));
    assert_eq!(group_node1.id(), "g1");
    assert_eq!(
        group_node1.abs_transform(),
        usvg::Transform::from_translate(100.0, 150.0)
    );

    let group1 = match group_node1 {
        usvg::Node::Group(g) => g,
        _ => unreachable!(),
    };

    let group_node2 = &group1.children()[0];
    assert!(matches!(group_node2, usvg::Node::Group(_)));
    assert_eq!(group_node2.id(), "svg1");
    assert_eq!(
        group_node2.abs_transform(),
        usvg::Transform::from_translate(100.0, 150.0)
    );

    let group2 = match group_node2 {
        usvg::Node::Group(g) => g,
        _ => unreachable!(),
    };

    let path = &group2.children()[0];
    assert!(matches!(path, usvg::Node::Path(_)));
    assert_eq!(
        path.abs_transform(),
        usvg::Transform::from_translate(100.0, 150.0)
    );
}

#[test]
fn svg_without_xmlns() {
    let svg = "
    <svg viewBox='0 0 100 100'>
        <rect x='0' y='0' width='10' height='10'/>
    </svg>
    ";

    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    assert_eq!(tree.size(), usvg::Size::from_wh(100.0, 100.0).unwrap());
}

#[test]
fn image_bbox_with_parent_transform() {
    let svg = "
    <svg viewBox='0 0 200 200' 
         xmlns='http://www.w3.org/2000/svg'
         xmlns:xlink='http://www.w3.org/1999/xlink'>
        <g transform='translate(25 25)'>
            <image id='image1' x='10' y='10' width='50' height='50' xlink:href='data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAGQAAABkCAYAAABw4pVUAAABb0lEQVR4Xu3VUQ0AIAzEUOZfA87wAgkq+vGmoGlz2exz73IZAyNIpsUHEaTVQ5BYD0EEqRmI8fghgsQMxHAsRJCYgRiOhQgSMxDDsRBBYgZiOBYiSMxADMdCBIkZiOFYiCAxAzEcCxEkZiCGYyGCxAzEcCxEkJiBGI6FCBIzEMOxEEFiBmI4FiJIzEAMx0IEiRmI4ViIIDEDMRwLESRmIIZjIYLEDMRwLESQmIEYjoUIEjMQw7EQQWIGYjgWIkjMQAzHQgSJGYjhWIggMQMxHAsRJGYghmMhgsQMxHAsRJCYgRiOhQgSMxDDsRBBYgZiOBYiSMxADMdCBIkZiOFYiCAxAzEcCxEkZiCGYyGCxAzEcCxEkJiBGI6FCBIzEMOxEEFiBmI4FiJIzEAMx0IEiRmI4ViIIDEDMRwLESRmIIZjIYLEDMRwLESQmIEYjoUIEjMQw7EQQWIGYjgWIkjMQAzHQgSJGYjhWIggMQMxnAdKSlrwlejIDgAAAABJRU5ErkJggg=='/>
        </g>
    </svg>
    ";

    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();

    let usvg::Node::Group(group_node1) = &tree.root().children()[0] else {
        unreachable!()
    };
    let usvg::Node::Group(group_node2) = &group_node1.children()[0] else {
        unreachable!()
    };
    let usvg::Node::Image(image_node) = &group_node2.children()[0] else {
        unreachable!()
    };

    assert_eq!(
        image_node.abs_bounding_box(),
        Rect::from_xywh(35.0, 35.0, 50.0, 50.0).unwrap()
    );
}

#[test]
fn no_text_nodes() {
    let svg = "
    <svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'>
        <g transform='translate(20)'>
            <path transform='translate(10)' d='M 0 0 L 10 10'/>
        </g>
    </svg>
    ";

    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    assert!(!tree.has_text_nodes());
}

#[test]
fn flattened_text_should_inherit_absolute_transform() {
    let svg = "
    <svg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'>
        <g transform='translate(20 20)'>
            <g transform='translate(20 20)'>
                <text x='32' y='100'>Text</text>
            </g>
        </g>
    </svg>
    ";

    let mut opts = usvg::Options::default();
    opts.fontdb_mut()
        .load_fonts_dir(env!("CARGO_MANIFEST_DIR").to_string() + "/../resvg/tests/fonts");
    opts.font_family = "Noto Sans".to_string();

    let tree = usvg::Tree::from_str(&svg, &opts).unwrap();

    let usvg::Node::Group(group0) = &tree.root().children()[0] else {
        unreachable!()
    };
    let usvg::Node::Group(group1) = &group0.children()[0] else {
        unreachable!()
    };
    let usvg::Node::Text(text) = &group1.children()[0] else {
        unreachable!()
    };
    let usvg::Node::Path(path) = &text.flattened().children()[0] else {
        unreachable!()
    };

    let t = path.abs_transform();

    assert_eq!(t.tx, 40.0);
    assert_eq!(t.ty, 40.0);

    assert_ne!(path.bounding_box(), path.abs_bounding_box());
    assert_eq!(
        path.bounding_box().transform(t).unwrap(),
        path.abs_bounding_box()
    );
}

#[test]
fn use_node_abs_transform() {
    let svg = "
    <svg viewBox='0 0 200 200'
         xmlns='http://www.w3.org/2000/svg'
         xmlns:xlink='http://www.w3.org/1999/xlink'>
        <defs>
            <rect id='rect1' x='0' y='0' width='100' height='100'/>
        </defs>
        <use xlink:href='#rect1' transform='matrix(0.5, 0, 0, 0.5, 20, 30)' />
    </svg>
    ";

    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();

    let usvg::Node::Group(group_node) = &tree.root().children()[0] else {
        unreachable!()
    };
    assert_eq!(group_node.abs_transform().get_scale(), (0.5, 0.5));

    let usvg::Node::Path(path_node) = &group_node.children()[0] else {
        unreachable!()
    };
    assert_eq!(path_node.abs_transform().get_scale(), (0.5, 0.5));
    assert_eq!(
        path_node.abs_bounding_box(),
        Rect::from_xywh(20.0, 30.0, 50.0, 50.0).unwrap()
    );
}

#[test]
fn nested_svg_abs_bounding_box() {
    let svg = "
    <svg xmlns='http://www.w3.org/2000/svg' width='100' height='100'>
        <svg transform='translate(10, 20)' width='50' height='50'>
            <path id='path' d='M 0 0 L 10 10'/>
        </svg>
    </svg>
    ";
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    assert_eq!(
        tree.node_by_id("path").unwrap().abs_bounding_box(),
        Rect::from_xywh(10.0, 20.0, 10.0, 10.0).unwrap()
    );
}

#[test]
fn resolve_fr_from_referenced_radial_gradient() {
    let svg = "
    <svg viewBox='0 0 100 100' xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink'>
        <defs>
            <radialGradient id='base' gradientUnits='userSpaceOnUse' cx='50' cy='50' r='50' fx='50' fy='50' fr='25'>
                <stop offset='0%' stop-color='red'/>
                <stop offset='100%' stop-color='blue'/>
            </radialGradient>
            <radialGradient id='ref' href='#base' xlink:href='#base'/>
        </defs>
        <rect width='100' height='100' fill='url(#ref)'/>
    </svg>
    ";

    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();

    let usvg::Node::Path(path) = &tree.root().children()[0] else {
        unreachable!()
    };
    let usvg::Paint::RadialGradient(rg) = path.fill().unwrap().paint() else {
        unreachable!()
    };

    assert_eq!(rg.fr().get(), 25.0);
}

fn text_options() -> usvg::Options<'static> {
    let mut opts = usvg::Options::default();
    opts.fontdb_mut()
        .load_fonts_dir(env!("CARGO_MANIFEST_DIR").to_string() + "/../resvg/tests/fonts");
    opts.font_family = "Noto Sans".to_string();
    opts
}

/// The text and x offset of all glyphs of the first text element, in visual order.
fn glyphs(tree: &usvg::Tree) -> Vec<(String, f32)> {
    let usvg::Node::Text(text) = &tree.root().children()[0] else {
        panic!("missing text")
    };
    text.layouted()
        .iter()
        .flat_map(|span| &span.positioned_glyphs)
        .map(|glyph| (glyph.text.clone(), glyph.transform().tx))
        .collect()
}

fn glyph_xs(tree: &usvg::Tree) -> Vec<f32> {
    glyphs(tree).into_iter().map(|(_, x)| x).collect()
}

#[test]
fn text_direction_is_inherited_by_text_and_chunks() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200">
        <g direction="rtl">
            <text x="100" y="50">Text<tspan x="100" y="80" direction="ltr">Tspan</tspan></text>
        </g>
        <text x="100" y="110">Text</text>
    </svg>"##;
    let tree = usvg::Tree::from_str(svg, &text_options()).unwrap();

    let usvg::Node::Group(group) = &tree.root().children()[0] else {
        panic!("missing group")
    };
    let usvg::Node::Text(text) = &group.children()[0] else {
        panic!("missing text")
    };
    assert_eq!(text.direction(), usvg::Direction::RightToLeft);
    let directions: Vec<_> = text.chunks().iter().map(|c| c.direction()).collect();
    assert_eq!(
        directions,
        [usvg::Direction::RightToLeft, usvg::Direction::LeftToRight]
    );

    let usvg::Node::Text(text) = &tree.root().children()[1] else {
        panic!("missing text")
    };
    assert_eq!(text.direction(), usvg::Direction::LeftToRight);
}

#[test]
fn text_anchor_follows_direction() {
    // The glyphs of `Text` at x = 100: left of the point for a right-to-left start,
    // right of it for a right-to-left end, and the other way around for left-to-right.
    let extent = |direction: &str, anchor: &str| {
        let svg = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200">
                <text x="100" y="50" font-size="20" direction="{direction}" text-anchor="{anchor}">Text</text>
            </svg>"##
        );
        let tree = usvg::Tree::from_str(&svg, &text_options()).unwrap();
        let xs = glyph_xs(&tree);
        (
            xs.iter().cloned().fold(f32::INFINITY, f32::min),
            xs.iter().cloned().fold(f32::NEG_INFINITY, f32::max),
        )
    };

    let (left, right) = extent("ltr", "start");
    assert!(
        (left - 100.0).abs() < 0.01 && right > 100.0,
        "{left} {right}"
    );
    let (left, right) = extent("rtl", "start");
    assert!(left < 100.0 && right < 100.0, "{left} {right}");
    let (left, right) = extent("rtl", "end");
    assert!(
        (left - 100.0).abs() < 0.01 && right > 100.0,
        "{left} {right}"
    );
    let (left, right) = extent("ltr", "end");
    assert!(left < 100.0 && right < 100.0, "{left} {right}");
}

#[test]
fn text_direction_is_ignored_in_vertical_writing_mode() {
    let xs = |direction: &str| {
        let svg = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200">
                <text x="100" y="50" font-size="20" writing-mode="tb" direction="{direction}"
                    unicode-bidi="bidi-override">Text</text>
            </svg>"##
        );
        glyph_xs(&usvg::Tree::from_str(&svg, &text_options()).unwrap())
    };
    assert_eq!(xs("ltr"), xs("rtl"));
}

#[test]
fn text_unicode_bidi_override_reverses_the_order_of_glyphs() {
    let glyphs = |attributes: &str| {
        let svg = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200">
                <text x="20" y="50" font-size="20" {attributes}>ABC</text>
            </svg>"##
        );
        glyphs(&usvg::Tree::from_str(&svg, &text_options()).unwrap())
    };

    // Glyphs are in visual order, left to right.
    let texts = |glyphs: &[(String, f32)]| glyphs.iter().map(|g| g.0.clone()).collect::<Vec<_>>();
    let normal = glyphs("");
    assert_eq!(texts(&normal), ["A", "B", "C"]);

    let overridden = glyphs(r#"direction="rtl" unicode-bidi="bidi-override""#);
    assert_eq!(texts(&overridden), ["C", "B", "A"]);
    assert!(overridden[0].1 < overridden[1].1 && overridden[1].1 < overridden[2].1);

    // Without `unicode-bidi`, `direction` only sets the base direction of the text.
    let rtl = glyphs(r#"direction="rtl""#);
    assert_eq!(texts(&rtl), ["A", "B", "C"]);
}

#[test]
fn text_direction_survives_writing_the_tree() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200">
        <text x="150" y="50" font-size="20" direction="rtl">1 <tspan unicode-bidi="embed"
            direction="ltr">ABC <tspan fill="red">DEF</tspan></tspan> 2</text>
        <text x="150" y="80" font-size="20"><tspan x="150" direction="rtl" text-anchor="end">ABC</tspan></text>
        <text y="110" font-size="20"><tspan unicode-bidi="embed" direction="rtl"><tspan direction="ltr"
            x="100" text-anchor="start">ABC DEF</tspan></tspan></text>
    </svg>"##;
    let opts = text_options();
    let tree = usvg::Tree::from_str(svg, &opts).unwrap();
    let written = tree.to_string(&usvg::WriteOptions {
        preserve_text: true,
        ..Default::default()
    });
    let tree2 = usvg::Tree::from_str(&written, &opts).unwrap();

    for (a, b) in tree.root().children().iter().zip(tree2.root().children()) {
        let (usvg::Node::Text(a), usvg::Node::Text(b)) = (a, b) else {
            panic!("missing text")
        };
        assert_eq!(a.direction(), b.direction());
        let (a, b): (Vec<_>, Vec<_>) = (
            a.layouted()
                .iter()
                .flat_map(|s| &s.positioned_glyphs)
                .map(|g| (g.text.clone(), g.transform()))
                .collect(),
            b.layouted()
                .iter()
                .flat_map(|s| &s.positioned_glyphs)
                .map(|g| (g.text.clone(), g.transform()))
                .collect(),
        );
        assert_eq!(a.len(), b.len());
        for ((ta, ma), (tb, mb)) in a.iter().zip(&b) {
            assert_eq!(ta, tb);
            assert!((ma.tx - mb.tx).abs() < 1e-3 && (ma.ty - mb.ty).abs() < 1e-3);
        }
    }
}

#[test]
fn text_anchor_follows_direction_on_a_path() {
    // The first glyph of `Text` on a path, 200 units after its start at x = 20.
    let left = |direction: &str, anchor: &str| {
        let svg = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="200">
                <defs><path id="p" d="M 20 100 H 380"/></defs>
                <text font-size="20" direction="{direction}" text-anchor="{anchor}">
                    <textPath href="#p" startOffset="200">Text</textPath>
                </text>
            </svg>"##
        );
        let tree = usvg::Tree::from_str(&svg, &text_options()).unwrap();
        glyph_xs(&tree).into_iter().fold(f32::INFINITY, f32::min)
    };

    // Right-to-left text ends at a `start` anchor and starts at an `end` one.
    assert!((left("rtl", "start") - left("ltr", "end")).abs() < 1e-3);
    assert!((left("rtl", "end") - left("ltr", "start")).abs() < 1e-3);
    assert!((left("rtl", "middle") - left("ltr", "middle")).abs() < 1e-3);
    assert!(left("rtl", "start") < 220.0 && left("ltr", "start") > 219.0);
}

#[test]
fn text_chunk_continues_from_the_unanchored_end_of_a_right_to_left_chunk() {
    // The chunk after a `tspan` with `direction="rtl"` starts where the same text with
    // `direction="ltr"` would end, because the anchor does not move the text position.
    let second = |direction: &str| {
        let svg = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="200">
                <text y="50" font-size="20"><tspan x="100" direction="{direction}">abc</tspan><tspan y="80">def</tspan></text>
            </svg>"##
        );
        let tree = usvg::Tree::from_str(&svg, &text_options()).unwrap();
        // The first glyph of the second chunk is the fourth one in visual order.
        glyph_xs(&tree)[3]
    };

    // `abc` ends at 100 for the right-to-left chunk and starts there for the other one.
    assert!((second("rtl") - second("ltr")).abs() < 1e-3);
    assert!(second("rtl") > 100.0);
}
