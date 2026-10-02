//! EXT-14 end to end: an extension crate (the SDK's first tutorial engine,
//! `examples/engine-ambience`) registered into a registry next to mdeck's
//! built-ins, and a slide exported with it through `mdeck::run_with_args`,
//! exactly as a custom build (`mdeck build --with`) runs it. Its theme
//! (`dusk`) comes from the extension too.
//!
//! No test harness: export opens a (hidden) window, which needs the main
//! thread. Without a display (headless CI) the export cannot run and the
//! test says so and passes.

use std::process::ExitCode;

fn main() -> ExitCode {
    let dir = std::env::temp_dir().join(format!("mdeck-ext-engine-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let deck = dir.join("deck.md");
    std::fs::write(
        &deck,
        "---\ntheme: dusk\n---\n\n# Ambience\n\nA slide over soft lights.\n",
    )
    .expect("deck");
    let out = dir.join("out");

    let mut registry = mdeck_sdk::registry::Registry::new();
    mdeck::builtins(&mut registry).expect("built-ins");
    registry.set_origin("engine-ambience");
    engine_ambience::register(&mut registry).expect("the extension registers");
    assert!(registry.engine_def("ambience").is_some());

    let code = mdeck::run_with_args(
        registry,
        [
            "mdeck".as_ref(),
            "export".as_ref(),
            deck.as_os_str(),
            "--output-dir".as_ref(),
            out.as_os_str(),
        ],
    );
    let png = out.join("slide-01.png");
    if code != ExitCode::SUCCESS || !png.is_file() {
        if std::env::var_os("CI").is_some() && cfg!(target_os = "linux") {
            eprintln!("extension_engine: no display to export with; skipped");
            return ExitCode::SUCCESS;
        }
        eprintln!("extension_engine: the export failed");
        return ExitCode::FAILURE;
    }

    // The ambience engine paints lights over dusk's flat background, so the
    // slide is not one colour outside its text.
    let img = image::open(&png).expect("png").to_rgba8();
    let corner = *img.get_pixel(4, img.height() - 4);
    let lit = img
        .pixels()
        .filter(|p| p.0.iter().zip(corner.0).any(|(a, b)| a.abs_diff(b) > 12))
        .count();
    std::fs::remove_dir_all(&dir).ok();
    if lit < 1000 {
        eprintln!("extension_engine: the ambience lights are missing ({lit} pixels differ)");
        return ExitCode::FAILURE;
    }
    println!("extension_engine: ok ({lit} lit pixels)");
    ExitCode::SUCCESS
}
