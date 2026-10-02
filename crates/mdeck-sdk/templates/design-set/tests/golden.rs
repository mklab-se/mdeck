//! A golden image of one slide in the design set. The first run records
//! `tests/golden/{{name}}.png`; later runs compare against it. After a
//! deliberate change, accept the new look with
//! `MDECK_UPDATE_GOLDEN=1 cargo test`.

use mdeck_sdk::content::{Block, Inline, ListItem, ListMarker, Slide};
use mdeck_sdk::testing::{GOLDEN_TOLERANCE, Headless, assert_golden};
use mdeck_sdk::tokens::Tokens;

fn text(s: &str) -> Vec<Inline> {
    vec![Inline::Text(s.into())]
}

#[test]
fn a_poster_slide() {
    let item = |s: &str| ListItem {
        marker: ListMarker::Static,
        inlines: text(s),
        children: vec![],
    };
    let slide = Slide {
        blocks: vec![
            Block::Heading {
                level: 2,
                inlines: text("Why posters"),
            },
            Block::List {
                ordered: false,
                items: vec![item("One idea per slide"), item("Big type")],
            },
        ],
        ..Default::default()
    };
    let out = Headless::new(480, 270).render_design(
        &{{crate_name}}::Poster,
        &slide,
        0,
        &Tokens::default(),
    );
    assert_eq!(out.hints.len(), 1, "the copy is published as a frame");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/{{name}}.png");
    assert_golden(path, &out.image, GOLDEN_TOLERANCE);
}
