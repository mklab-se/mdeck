//! EXT-05 end to end: a code design set an extension registers, named by a
//! theme's `designs:`, draws the slide in export, exactly as a custom build
//! (`mdeck build --with`) runs it.
//!
//! The test set fills the slide with one colour, so the exported slide is
//! that colour wherever the core does not draw its chrome. The standard
//! designs would show the heading on the theme's background instead.
//!
//! No test harness: export opens a (hidden) window, which needs the main
//! thread. Without a display (headless CI) the export cannot run and the
//! test says so and passes.

use std::process::ExitCode;

use mdeck_sdk::content::Slide;
use mdeck_sdk::design::{DesignCx, DesignSet};
use mdeck_sdk::paint::{Color, Rect};

/// Fills every slide with magenta.
struct Magenta;

impl DesignSet for Magenta {
    fn name(&self) -> &str {
        "magenta"
    }
    fn render(&self, cx: &mut DesignCx, _slide: &Slide, rect: Rect) {
        cx.painter()
            .rect_filled(rect, 0.0, Color::from_rgb(255, 0, 255));
    }
}

fn main() -> ExitCode {
    let dir = std::env::temp_dir().join(format!("mdeck-ext-design-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let deck = dir.join("deck.md");
    std::fs::write(
        &deck,
        "---\ntheme: magenta-cards\n---\n\n# Cards\n\n- one\n- two\n",
    )
    .expect("deck");
    let out = dir.join("out");

    let mut registry = mdeck_sdk::registry::Registry::new();
    mdeck::builtins(&mut registry).expect("built-ins");
    registry.set_origin("magenta-pack");
    registry
        .design_set(Box::new(Magenta))
        .expect("the design set registers");
    registry
        .theme(
            "magenta-cards",
            "name: magenta-cards\nextends: dark\ndesigns: magenta\n",
        )
        .expect("the theme registers");

    let code = mdeck::run_with_args(
        registry,
        [
            "mdeck".as_ref(),
            "export".as_ref(),
            deck.as_os_str(),
            "--output-dir".as_ref(),
            out.as_os_str(),
            "--width".as_ref(),
            "640".as_ref(),
            "--height".as_ref(),
            "360".as_ref(),
        ],
    );
    let png = out.join("slide-01.png");
    if code != ExitCode::SUCCESS || !png.is_file() {
        if std::env::var_os("CI").is_some() && cfg!(target_os = "linux") {
            eprintln!("extension_design_set: no display to export with; skipped");
            return ExitCode::SUCCESS;
        }
        eprintln!("extension_design_set: the export failed");
        return ExitCode::FAILURE;
    }

    let img = image::open(&png).expect("png").to_rgba8();
    let total = img.width() as usize * img.height() as usize;
    let magenta = img
        .pixels()
        .filter(|p| p.0[0] > 230 && p.0[1] < 30 && p.0[2] > 230)
        .count();
    std::fs::remove_dir_all(&dir).ok();
    if magenta * 10 < total * 9 {
        eprintln!(
            "extension_design_set: the slide was not drawn by the design set \
             ({magenta} of {total} pixels magenta)"
        );
        return ExitCode::FAILURE;
    }
    println!("extension_design_set: ok ({magenta} of {total} pixels magenta)");
    ExitCode::SUCCESS
}
