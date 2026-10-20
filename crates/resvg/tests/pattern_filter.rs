// Copyright 2026 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use resvg::tiny_skia::{FilterQuality, Pixmap, Transform};

const PATTERN: &str = r#"<pattern id="p" patternUnits="userSpaceOnUse" width="6.2" height="6.2"
    patternTransform="rotate(12)"><rect width="2" height="6"/></pattern>"#;

fn tree(content: &str) -> resvg::usvg::Tree {
    resvg::usvg::Tree::from_str(
        &format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24">
            <defs>{PATTERN}</defs>{content}</svg>"#
        ),
        &resvg::usvg::Options::default(),
    )
    .unwrap()
}

fn render(tree: &resvg::usvg::Tree, filter: FilterQuality) -> Pixmap {
    let mut pixmap = Pixmap::new(24, 24).unwrap();
    resvg::render_with_options(
        tree,
        Transform::identity(),
        &resvg::Options {
            pattern_filter: filter,
        },
        &mut pixmap.as_mut(),
    );
    pixmap
}

#[test]
fn default_preserves_bicubic_output() {
    let tree = tree(r#"<rect width="24" height="24" fill="url(#p)"/>"#);
    let mut default = Pixmap::new(24, 24).unwrap();
    resvg::render(&tree, Transform::identity(), &mut default.as_mut());
    assert_eq!(default.data(), render(&tree, FilterQuality::Bicubic).data());
    assert_eq!(
        resvg::Options::default().pattern_filter,
        FilterQuality::Bicubic
    );
}

#[test]
fn filter_changes_pattern_fills_and_strokes() {
    for content in [
        r#"<rect width="24" height="24" fill="url(#p)"/>"#,
        r#"<path d="M0 12H24" stroke="url(#p)" stroke-width="12"/>"#,
    ] {
        let tree = tree(content);
        let nearest = render(&tree, FilterQuality::Nearest);
        let bilinear = render(&tree, FilterQuality::Bilinear);
        let bicubic = render(&tree, FilterQuality::Bicubic);
        assert!(nearest.data() != bilinear.data());
        assert!(bilinear.data() != bicubic.data());
    }
}

#[test]
fn filter_reaches_isolated_groups_masks_and_filter_images() {
    for content in [
        r#"<g opacity=".5"><rect width="24" height="24" fill="url(#p)"/></g>"#,
        r#"<defs><mask id="m" mask-type="alpha" x="0" y="0" width="24" height="24"
            maskUnits="userSpaceOnUse"><rect width="24" height="24" fill="url(#p)"/>
            </mask></defs><rect width="24" height="24" mask="url(#m)"/>"#,
        r##"<defs><rect id="r" width="24" height="24" fill="url(#p)"/>
            <filter id="f" x="0" y="0" width="24" height="24" filterUnits="userSpaceOnUse">
            <feImage href="#r"/></filter></defs>
            <rect width="24" height="24" filter="url(#f)"/>"##,
    ] {
        let tree = tree(content);
        assert!(
            render(&tree, FilterQuality::Nearest).data()
                != render(&tree, FilterQuality::Bilinear).data()
        );
    }
}

#[test]
fn filter_reaches_embedded_svg_images() {
    let nested = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24">
        <defs>{PATTERN}</defs><rect width="24" height="24" fill="url(#p)"/></svg>"#
    );
    let encoded: String = nested.bytes().map(|byte| format!("%{byte:02X}")).collect();
    let tree = tree(&format!(
        r#"<image width="24" height="24" href="data:image/svg+xml,{encoded}"/>"#
    ));
    assert_ne!(
        render(&tree, FilterQuality::Nearest).data(),
        render(&tree, FilterQuality::Bilinear).data()
    );
}

#[test]
fn node_rendering_uses_filter_and_preserves_default() {
    let tree = tree(r#"<rect id="r" width="24" height="24" fill="url(#p)"/>"#);
    let node = tree.node_by_id("r").unwrap();
    let mut default = Pixmap::new(24, 24).unwrap();
    resvg::render_node(node, Transform::identity(), &mut default.as_mut()).unwrap();
    for filter in [
        FilterQuality::Nearest,
        FilterQuality::Bilinear,
        FilterQuality::Bicubic,
    ] {
        let mut actual = Pixmap::new(24, 24).unwrap();
        resvg::render_node_with_options(
            node,
            Transform::identity(),
            &resvg::Options {
                pattern_filter: filter,
            },
            &mut actual.as_mut(),
        )
        .unwrap();
        assert_eq!(actual.data(), render(&tree, filter).data());
        if filter == FilterQuality::Bicubic {
            assert_eq!(default.data(), actual.data());
        }
    }
}

#[cfg(all(
    feature = "svgz",
    feature = "text",
    feature = "system-fonts",
    feature = "memmap-fonts"
))]
#[test]
fn cli_documents_and_validates_pattern_filter() {
    let help = std::process::Command::new(env!("CARGO_BIN_EXE_resvg"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(help.status.success());
    assert!(
        String::from_utf8(help.stdout)
            .unwrap()
            .contains("--pattern-filter")
    );
    let invalid = std::process::Command::new(env!("CARGO_BIN_EXE_resvg"))
        .args(["--pattern-filter", "invalid"])
        .output()
        .unwrap();
    assert!(!invalid.status.success());
    assert!(
        String::from_utf8(invalid.stderr)
            .unwrap()
            .contains("expected nearest, bilinear or bicubic")
    );
}

#[test]
fn example_uses_all_three_filters() {
    let tree = resvg::usvg::Tree::from_str(
        include_str!("extra/pattern-filter.svg"),
        &resvg::usvg::Options::default(),
    )
    .unwrap();
    let render = |filter| {
        let mut pixmap = Pixmap::new(320, 96).unwrap();
        resvg::render_with_options(
            &tree,
            Transform::from_scale(2.0, 2.0),
            &resvg::Options {
                pattern_filter: filter,
            },
            &mut pixmap.as_mut(),
        );
        pixmap
    };
    assert!(render(FilterQuality::Nearest).data() != render(FilterQuality::Bilinear).data());
    assert!(render(FilterQuality::Bilinear).data() != render(FilterQuality::Bicubic).data());
}
