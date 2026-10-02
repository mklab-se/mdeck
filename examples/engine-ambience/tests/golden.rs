//! Golden images: the settled ground on a dark and a light theme.
//! Regenerate with `MDECK_UPDATE_GOLDEN=1 cargo test -p engine-ambience`.

use engine_ambience::Ambience;
use mdeck_sdk::paint::{Color, ImageData};
use mdeck_sdk::stage::{Frame, Moment, Stage};
use mdeck_sdk::testing::{GOLDEN_TOLERANCE, Headless, assert_golden};
use mdeck_sdk::tokens::{EngineSettings, Tokens};

fn golden(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(format!("{name}.png"))
}

fn render(tokens: &Tokens, index: usize) -> ImageData {
    let mut h = Headless::new(320, 180);
    let settings = EngineSettings::new();
    let mut frame = Frame::new(h.rect(), tokens, &settings);
    frame.still = true;
    let mut stage = Stage::new(Moment::Slide);
    stage.index = index;
    h.render_engine(&mut Ambience::new(), &frame, &stage)
}

#[test]
fn dark_ground() {
    let img = render(&Tokens::default(), 0);
    assert_golden(golden("dark"), &img, GOLDEN_TOLERANCE);
}

#[test]
fn light_ground() {
    let tokens = Tokens {
        background: Color::from_rgb(246, 243, 236),
        light: true,
        ..Tokens::default()
    };
    assert_golden(golden("light"), &render(&tokens, 2), GOLDEN_TOLERANCE);
}
