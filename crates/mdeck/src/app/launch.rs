//! Opening the presenting window: start position, warnings, the viewport,
//! and the event loop.

use super::*;

/// Resolve the initial slide (0-indexed) and overview flag from CLI flags and
/// the configured `defaults.start_mode`. CLI flags win.
fn resolve_start(
    start_slide: Option<usize>,
    start_overview: bool,
    config_start: Option<&str>,
) -> (usize, bool) {
    if start_overview {
        return (start_slide.map(|s| s.saturating_sub(1)).unwrap_or(0), true);
    }
    if let Some(s) = start_slide {
        return (s.saturating_sub(1), false);
    }
    match config_start {
        Some("overview") => (0, true),
        Some("first") | None => (0, false),
        Some(n) => match n.parse::<usize>() {
            Ok(num) => (num.saturating_sub(1), false),
            Err(_) => (0, false),
        },
    }
}

/// Warnings worth seeing before the window opens: CJK text without a CJK
/// font, and AI images that were never generated.
fn warn_before_presenting(presentation: &Presentation, file: &std::path::Path) {
    crate::commands::check::warn_missing_cjk_font(presentation);
    let ungenerated = presentation
        .slides
        .iter()
        .flat_map(|s| s.blocks.iter())
        .filter(|b| matches!(b, parser::Block::Image { path, .. } if path == "image-generation"))
        .count();
    if ungenerated > 0 {
        use colored::Colorize;
        eprintln!(
            "{} This presentation contains {} ungenerated image(s).",
            "Warning:".yellow().bold(),
            ungenerated
        );
        eprintln!(
            "  Run `mdeck ai generate {}` to generate them first.\n",
            file.display()
        );
    }
}

/// The window: 1280x720, or fullscreen on the remembered monitor.
fn viewport(
    title: &str,
    windowed: bool,
    monitor_position: Option<[f32; 2]>,
) -> egui::ViewportBuilder {
    let viewport = egui::ViewportBuilder::default().with_title(title);
    let viewport = if windowed {
        viewport.with_inner_size([1280.0, 720.0])
    } else {
        let viewport = viewport.with_fullscreen(true);
        // A saved monitor position opens fullscreen on that monitor.
        match monitor_position {
            Some([x, y]) => viewport.with_position(egui::pos2(x, y)),
            None => viewport,
        }
    };
    match load_app_icon() {
        Some(icon) => viewport.with_icon(Arc::new(icon)),
        None => viewport,
    }
}

pub fn run(
    file: PathBuf,
    windowed: bool,
    start_slide: Option<usize>,
    start_overview: bool,
    quiet: bool,
    engine: Option<String>,
) -> anyhow::Result<()> {
    let file = file.canonicalize().unwrap_or(file);

    // Config defaults: start mode, theme/transition fallbacks, monitor position
    let config = Config::load_or_default();
    let defaults = config.defaults.clone().unwrap_or_default();
    let (cli_initial_slide, cli_initial_overview) =
        resolve_start(start_slide, start_overview, defaults.start_mode.as_deref());

    let incident_log = Arc::new(IncidentLog::new(&file.display().to_string()));

    let content = std::fs::read_to_string(&file)?;
    let presentation = parser::parse(&content);

    if presentation.slides.is_empty() {
        anyhow::bail!("No slides found in {}", file.display());
    }
    let (cli_engine, _) =
        crate::engines::choose(engine.as_deref(), None).map_err(|e| anyhow::anyhow!("{e}"))?;

    if !quiet {
        warn_before_presenting(&presentation, &file);
    }

    let title = presentation.meta.title.clone().unwrap_or_else(|| {
        format!(
            "mdeck: {}",
            file.file_name().unwrap_or_default().to_string_lossy()
        )
    });

    let slide_count = presentation.slides.len();
    let initial_slide = cli_initial_slide.min(slide_count.saturating_sub(1));
    let initial_overview = cli_initial_overview;

    // The slide position is shared with the window so a display error can be
    // logged together with where the presentation was.
    let shared_slide = Arc::new(AtomicUsize::new(initial_slide));

    let viewport = viewport(&title, windowed, defaults.monitor_position);
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    let shared = shared_slide.clone();
    let file_clone = file.clone();
    let log_clone = incident_log.clone();
    // winit allows exactly one event loop per process, so there is no point
    // retrying `run_native` after a display error: run once, log, and bail.
    // Theme font files must be registered before the window installs fonts.
    lookup::preload(&lookup::Lookup::for_deck(file.parent()));

    let result = eframe::run_native(
        &title,
        options,
        Box::new(move |cc| {
            render::fonts::install(&cc.egui_ctx);
            let (rx, watcher) =
                spawn_file_watcher(&file_clone, cc.egui_ctx.clone(), log_clone.clone())?;
            let watch = FileWatch {
                rx,
                watcher: Some(watcher),
                content_hash: hash_content(&content),
            };
            let launch = Launch {
                quiet,
                incident_log: log_clone,
                defaults,
                cli_engine,
            };
            let mut app = PresentationApp::new(file_clone, presentation, watch, launch);
            app.current_slide = initial_slide;
            app.shared_slide = Some(shared);
            if initial_overview {
                app.mode = AppMode::Grid {
                    selected: initial_slide,
                };
            } else if initial_slide == 0 {
                // Starting on a chosen slide (an agent checking its work, a
                // presenter resuming) skips the opener.
                app.start_countdown();
            }
            app.spawn_diagram_precache();
            Ok(Box::new(app))
        }),
    );

    match result {
        Ok(()) => {
            print_incident_summary(&incident_log);
            Ok(())
        }
        Err(e) => {
            let slide = shared_slide.load(Ordering::Relaxed);
            incident_log.record(
                "display_error",
                "eframe display error",
                &format!("{e}\nslide: {slide}"),
            );
            print_incident_summary(&incident_log);
            Err(anyhow::anyhow!("{e}"))
        }
    }
}
