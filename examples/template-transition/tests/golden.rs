//! A golden image of the transition's overlay mid-way. The first run
//! records `tests/golden/template-transition.png`; later runs compare against it.
//! After a deliberate change, accept the new look with
//! `MDECK_UPDATE_GOLDEN=1 cargo test`.

use mdeck_sdk::testing::{GOLDEN_TOLERANCE, Headless, assert_golden};
use mdeck_sdk::tokens::Tokens;

#[test]
fn overlay_mid_way() {
    let img = Headless::new(320, 180).render_transition(
        &template_transition::Rise,
        0.5,
        true,
        &Tokens::default(),
    );
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/template-transition.png");
    assert_golden(path, &img, GOLDEN_TOLERANCE);
}
