// Copyright 2024 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::sync::Arc;

use once_cell::sync::Lazy;
use usvg::{DominantBaseline, Group, Node, Text};

static GLOBAL_FONTDB: Lazy<Arc<usvg::fontdb::Database>> = Lazy::new(|| {
    let mut fontdb = usvg::fontdb::Database::new();
    fontdb.load_fonts_dir("../resvg/tests/fonts");
    fontdb.set_serif_family("Noto Serif");
    fontdb.set_sans_serif_family("Noto Sans");
    fontdb.set_cursive_family("Yellowtail");
    fontdb.set_fantasy_family("Sedgwick Ave Display");
    fontdb.set_monospace_family("Noto Mono");
    Arc::new(fontdb)
});

fn parse(svg: &str) -> usvg::Tree {
    let opt = usvg::Options {
        fontdb: GLOBAL_FONTDB.clone(),
        ..Default::default()
    };
    usvg::Tree::from_str(svg, &opt).unwrap()
}

fn first_text(group: &Group) -> Option<&Text> {
    for node in group.children() {
        match node {
            Node::Text(t) => return Some(t),
            Node::Group(g) => {
                if let Some(t) = first_text(g) {
                    return Some(t);
                }
            }
            _ => {}
        }
    }
    None
}

// Regression test for https://github.com/linebender/resvg/issues/864
//
// `dominant-baseline` is an inherited property (CSS Inline Layout 3 / SVG 2),
// so a nested `<tspan>` that does not set it must inherit the value from an
// ancestor `<text>` — even when its direct parent `<tspan>` doesn't carry it.
// Previously the nested span fell back to `Auto`, placing it on the alphabetic
// baseline while its sibling text used `text-after-edge`, so the two were no
// longer on the same line.
#[test]
fn nested_tspan_inherits_dominant_baseline() {
    let svg = "<svg xmlns='http://www.w3.org/2000/svg' width='400' height='200'>
        <text x='20' y='150' font-family='Noto Sans' dominant-baseline='text-after-edge'>
            <tspan x='20' font-size='40'><tspan>hello</tspan> world</tspan>
        </text>
    </svg>";

    let tree = parse(svg);
    let text = first_text(tree.root()).expect("a text node");

    let chunk = text
        .chunks()
        .iter()
        .find(|c| c.text() == "hello world")
        .expect("the 'hello world' chunk");

    // Two spans: the nested `<tspan>hello` and the trailing ` world`.
    assert_eq!(chunk.spans().len(), 2);
    for span in chunk.spans() {
        assert_eq!(
            span.dominant_baseline(),
            DominantBaseline::TextAfterEdge,
            "span {:?} did not inherit dominant-baseline from <text>",
            &chunk.text()[span.start()..span.end()]
        );
    }
}

// A nested `<tspan>` setting its own `dominant-baseline` still overrides the
// inherited one, and deeper descendants inherit the nested value.
#[test]
fn nested_tspan_dominant_baseline_override() {
    let svg = "<svg xmlns='http://www.w3.org/2000/svg' width='400' height='200'>
        <text x='20' y='150' font-family='Noto Sans' dominant-baseline='text-after-edge'>
            <tspan x='20' font-size='40' dominant-baseline='hanging'><tspan>hi</tspan></tspan>
        </text>
    </svg>";

    let tree = parse(svg);
    let text = first_text(tree.root()).expect("a text node");
    let chunk = text
        .chunks()
        .iter()
        .find(|c| c.text() == "hi")
        .expect("the 'hi' chunk");

    let span = &chunk.spans()[0];
    assert_eq!(span.dominant_baseline(), DominantBaseline::Hanging);
}
