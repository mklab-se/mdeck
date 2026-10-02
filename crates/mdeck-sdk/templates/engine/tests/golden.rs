//! A golden image of the engine's settled frame. The first run records
//! `tests/golden/{{name}}.png`; later runs compare against it. After a
//! deliberate change, accept the new look with
//! `MDECK_UPDATE_GOLDEN=1 cargo test`.

use mdeck_sdk::stage::{Frame, Moment, Stage};
use mdeck_sdk::testing::{GOLDEN_TOLERANCE, Headless, assert_golden};
use mdeck_sdk::tokens::{EngineSettings, Tokens};

#[test]
fn settled_frame() {
    let mut h = Headless::new(320, 180);
    let (tokens, settings) = (Tokens::default(), EngineSettings::new());
    let mut frame = Frame::new(h.rect(), &tokens, &settings);
    frame.still = true;
    let mut engine = ({{crate_name}}::DEF.create)(&settings);
    let img = h.render_engine(engine.as_mut(), &frame, &Stage::new(Moment::Slide));
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/{{name}}.png");
    assert_golden(path, &img, GOLDEN_TOLERANCE);
}
