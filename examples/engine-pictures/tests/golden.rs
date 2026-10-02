//! Golden images: a picture slide, a countdown digit and the end words.
//! Regenerate with `MDECK_UPDATE_GOLDEN=1 cargo test -p engine-pictures`.

use std::sync::Arc;

use engine_pictures::Motes;
use mdeck_sdk::cloud::{Cloud, Mask};
use mdeck_sdk::paint::{Font, ImageData};
use mdeck_sdk::stage::{Frame, Moment, Picture, PictureSource, Place, Stage};
use mdeck_sdk::testing::{GOLDEN_TOLERANCE, Headless, assert_golden};
use mdeck_sdk::tokens::{EngineSettings, Tokens};

fn golden(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(format!("{name}.png"))
}

/// A still of `stage`, the way `mdeck export` renders one.
fn still(h: &mut Headless, stage: &Stage) -> ImageData {
    let (tokens, settings) = (Tokens::default(), EngineSettings::new());
    let mut frame = Frame::new(h.rect(), &tokens, &settings);
    frame.still = true;
    h.render_engine(&mut Motes::new(), &frame, stage)
}

/// A heart, as a point cloud would hold it: points in the unit square.
fn heart() -> Arc<Cloud> {
    let pts = (0..600)
        .map(|i| {
            let t = i as f32 / 600.0 * std::f32::consts::TAU;
            let k = 0.6 + 0.4 * ((i * 7919) % 100) as f32 / 100.0;
            let x = 16.0 * t.sin().powi(3);
            let y =
                13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos();
            [0.5 + k * x / 34.0, 0.45 - k * y / 34.0]
        })
        .collect();
    Arc::new(Cloud::new("heart", pts, 1.0))
}

fn mask(h: &mut Headless, text: &str) -> Mask {
    let mut out = None;
    h.paint(|p| out = Some(p.glyph_points(text, Font::display(200.0), 1500)));
    out.unwrap()
}

#[test]
fn picture_beside_the_copy() {
    let mut stage = Stage::new(Moment::Slide);
    stage.picture = Some(Picture::new(
        PictureSource::Cloud(heart()),
        Place {
            u: 0.55,
            v: 0.1,
            w: 0.4,
            h: 0.8,
        },
    ));
    let img = still(&mut Headless::new(320, 180), &stage);
    assert_golden(golden("picture"), &img, GOLDEN_TOLERANCE);
}

#[test]
fn countdown_digit() {
    let mut h = Headless::new(320, 180);
    let mask = mask(&mut h, "3");
    let stage = Stage::new(Moment::countdown(3, mask, 0.5));
    assert_golden(
        golden("countdown"),
        &still(&mut h, &stage),
        GOLDEN_TOLERANCE,
    );
}

#[test]
fn end_words() {
    let mut h = Headless::new(320, 180);
    let words = mask(&mut h, "THE END");
    let stage = Stage::new(Moment::end(1.0, words));
    assert_golden(golden("end"), &still(&mut h, &stage), GOLDEN_TOLERANCE);
}
