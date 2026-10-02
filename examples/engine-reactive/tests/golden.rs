//! Golden image: a bar chart, a line series and an image frame, as a still.
//! Regenerate with `MDECK_UPDATE_GOLDEN=1 cargo test -p engine-reactive`.

use engine_reactive::{Reactive, Settings};
use mdeck_sdk::engine::Engine;
use mdeck_sdk::geometry::{Hint, fingerprint};
use mdeck_sdk::paint::{Color, Pos2, Rect, Stroke, Vec2};
use mdeck_sdk::stage::{Frame, Moment, Stage};
use mdeck_sdk::testing::{GOLDEN_TOLERANCE, Headless, assert_golden};
use mdeck_sdk::tokens::{EngineSettings, Tokens};

fn golden(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(format!("{name}.png"))
}

/// The geometry a slide with a bar chart, a line and an image would publish,
/// on a 640 by 360 canvas (drawn at 320 by 180 and doubled).
fn hints() -> Vec<Hint> {
    let mut h: Vec<Hint> = [40.0, 70.0, 55.0, 95.0]
        .iter()
        .enumerate()
        .map(|(i, &height)| {
            let x = 24.0 + i as f32 * 26.0;
            Hint::Bar(Rect::from_min_max(
                Pos2::new(x, 150.0 - height),
                Pos2::new(x + 16.0, 150.0),
            ))
        })
        .collect();
    h.push(Hint::Path(vec![
        Pos2::new(150.0, 120.0),
        Pos2::new(190.0, 90.0),
        Pos2::new(220.0, 100.0),
        Pos2::new(300.0, 40.0),
    ]));
    h.push(Hint::Frame(Rect::from_min_size(
        Pos2::new(230.0, 110.0),
        Vec2::new(70.0, 50.0),
    )));
    h.into_iter().map(double).collect()
}

fn double(h: Hint) -> Hint {
    let p = |p: Pos2| Pos2::new(p.x * 2.0, p.y * 2.0);
    let r = |r: Rect| Rect::from_min_max(p(r.min), p(r.max));
    match h {
        Hint::Bar(b) => Hint::Bar(r(b)),
        Hint::Frame(f) => Hint::Frame(r(f)),
        Hint::Path(pts) => Hint::Path(pts.into_iter().map(p).collect()),
        other => other,
    }
}

#[test]
fn chart_line_and_image() {
    let mut h = Headless::new(640, 360);
    let (tokens, settings) = (Tokens::default(), EngineSettings::new());
    let mut frame = Frame::new(h.rect(), &tokens, &settings);
    frame.still = true;
    let hints = hints();
    let mut stage = Stage::new(Moment::Slide);
    stage.geometry = &hints;
    stage.geometry_key = fingerprint(&hints);
    let mut engine = Reactive::new(Settings::default());
    // Draw the content the hints stand for, the way the slide would, so the
    // golden image shows the reaction in context.
    let img = h.paint(|p| {
        p.rect_filled(frame.rect, 0.0, tokens.background);
        engine.update(&frame, &stage);
        engine.paint(p, &frame, &stage);
        for hint in &hints {
            match hint {
                Hint::Bar(r) => p.rect_filled(*r, 4.0, tokens.series[0]),
                Hint::Path(pts) => p.line(pts.clone(), Stroke::new(4.0, tokens.series[2])),
                Hint::Frame(r) => p.rect_filled(*r, 8.0, Color::from_gray(60)),
                _ => {}
            }
        }
    });
    assert_golden(golden("reactive"), &img, GOLDEN_TOLERANCE);
}
