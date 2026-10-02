//! A golden image of the visual. The first run records
//! `tests/golden/{{name}}.png`; later runs compare against it. After a
//! deliberate change, accept the new look with
//! `MDECK_UPDATE_GOLDEN=1 cargo test`.

use mdeck_sdk::geometry::Hint;
use mdeck_sdk::testing::{GOLDEN_TOLERANCE, Headless, assert_golden};
use mdeck_sdk::tokens::Tokens;

const SRC: &str = "title: Revenue\n- North: 42\n- South: 31 (color: 1)\n+ East: 55 (color: 2)\n";

#[test]
fn draws_and_publishes_bars() {
    let tokens = Tokens::default();
    let out = Headless::new(480, 270).render_visual(&{{crate_name}}::Bars, SRC, 1, &tokens);
    assert_eq!(
        out.hints
            .iter()
            .filter(|h| matches!(h, Hint::Bar(_)))
            .count(),
        3
    );
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/{{name}}.png");
    assert_golden(path, &out.image, GOLDEN_TOLERANCE);
}

#[test]
fn the_step_hides_later_items() {
    let tokens = Tokens::default();
    let out = Headless::new(480, 270).render_visual(&{{crate_name}}::Bars, SRC, 0, &tokens);
    assert_eq!(out.hints.len(), 2);
}
