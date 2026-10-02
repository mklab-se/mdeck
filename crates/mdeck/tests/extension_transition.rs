//! EXT-05 end to end: a transition an extension registers, named by a
//! deck's `transition:`, drives the change between slides. Exported with
//! `--moment transition` (the change into the second slide, halfway), as a
//! custom build (`mdeck build --with`) runs it.
//!
//! The test transition hides both slides (its `look`) and paints the left
//! half of the slide green (its `paint_over`), so the still is green on the
//! left and the bare background on the right. A built-in transition would
//! show the slides' text halfway through.
//!
//! No test harness: export opens a (hidden) window, which needs the main
//! thread. Without a display (headless CI) the export cannot run and the
//! test says so and passes.

use std::process::ExitCode;

use mdeck_sdk::paint::{Color, Rect};
use mdeck_sdk::transition::{SideLook, Transition, TransitionCx};

/// Hides both slides and paints a green curtain over the left half.
struct Curtain;

impl Transition for Curtain {
    fn name(&self) -> &str {
        "curtain"
    }
    fn summary(&self) -> &str {
        "A green curtain over the left half."
    }
    fn duration(&self) -> f32 {
        1.0
    }
    fn look(&self, _t: f32, _forward: bool, _rect: Rect) -> (SideLook, SideLook) {
        (SideLook::HIDDEN, SideLook::HIDDEN)
    }
    fn paint_over(&self, cx: &mut TransitionCx, _t: f32) {
        let half = cx.rect().sub_rect(0.0, 0.0, 0.5, 1.0);
        cx.painter()
            .rect_filled(half, 0.0, Color::from_rgb(0, 255, 0));
    }
}

fn main() -> ExitCode {
    let dir = std::env::temp_dir().join(format!("mdeck-ext-transition-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let deck = dir.join("deck.md");
    std::fs::write(
        &deck,
        "---\ntheme: dark\ntransition: curtain\n---\n\n# LEAVING\n\nThe slide we leave, in large type across the whole slide\n\n# ARRIVING\n\nThe slide we arrive at, in large type across the whole slide\n",
    )
    .expect("deck");
    let out = dir.join("out");

    let mut registry = mdeck_sdk::registry::Registry::new();
    mdeck::builtins(&mut registry).expect("built-ins");
    registry.set_origin("curtain-pack");
    registry
        .transition(Box::new(Curtain))
        .expect("the transition registers");

    let code = mdeck::run_with_args(
        registry,
        [
            "mdeck".as_ref(),
            "export".as_ref(),
            deck.as_os_str(),
            "--moment".as_ref(),
            "transition".as_ref(),
            "--output-dir".as_ref(),
            out.as_os_str(),
            "--width".as_ref(),
            "640".as_ref(),
            "--height".as_ref(),
            "360".as_ref(),
        ],
    );
    let png = out.join("transition.png");
    if code != ExitCode::SUCCESS || !png.is_file() {
        if std::env::var_os("CI").is_some() && cfg!(target_os = "linux") {
            eprintln!("extension_transition: no display to export with; skipped");
            return ExitCode::SUCCESS;
        }
        eprintln!("extension_transition: the export failed");
        return ExitCode::FAILURE;
    }

    let img = image::open(&png).expect("png").to_rgba8();
    let (w, h) = (img.width(), img.height());
    // above the footer and counter, which the core draws over the change
    let body = h * 85 / 100;
    let green = |p: &image::Rgba<u8>| p.0[0] < 30 && p.0[1] > 225 && p.0[2] < 30;
    let background = *img.get_pixel(w - 2, 2);
    let (mut left, mut left_green, mut right, mut right_ink) = (0, 0, 0, 0);
    for y in 0..body {
        for x in 0..w {
            let p = img.get_pixel(x, y);
            if x < w / 2 - 1 {
                left += 1;
                left_green += usize::from(green(p));
            } else if x > w / 2 + 1 {
                right += 1;
                let differs =
                    p.0.iter()
                        .zip(background.0)
                        .any(|(a, b)| a.abs_diff(b) > 24);
                right_ink += usize::from(differs);
            }
        }
    }
    std::fs::remove_dir_all(&dir).ok();
    if left_green * 100 < left * 98 {
        eprintln!(
            "extension_transition: paint_over did not draw ({left_green} of {left} pixels green)"
        );
        return ExitCode::FAILURE;
    }
    if right_ink * 100 > right {
        eprintln!(
            "extension_transition: look did not hide the slides ({right_ink} of {right} pixels drawn)"
        );
        return ExitCode::FAILURE;
    }
    println!("extension_transition: ok ({left_green} green, {right_ink} drawn on the right)");
    ExitCode::SUCCESS
}
