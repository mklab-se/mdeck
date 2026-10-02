//! Opening the presenting window: start position, warnings, the viewport,
//! and the event loop.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Instant;

use eframe::egui;
use notify_debouncer_mini::{Debouncer, notify};

use crate::config::{Config, DefaultsConfig};
use crate::deck::{self, Deck};
use crate::incident_log::IncidentLog;
use crate::parser::{self, Presentation};
use crate::render;
use crate::theme::{Theme, lookup};

use super::helpers::{hash_content, load_app_icon, print_incident_summary, spawn_file_watcher};
use super::{
    AppMode, Fps, GridState, Ink, Jobs, PresentationApp, QuitTaps, RawOverlaySide, SlideView,
};

/// The watcher that tells the window its deck file changed.
pub(super) struct FileWatch {
    pub(super) rx: mpsc::Receiver<()>,
    pub(super) watcher: Option<Debouncer<notify::RecommendedWatcher>>,
    /// Hash of the content the deck was parsed from.
    pub(super) content_hash: u64,
}

/// What the window starts with from the command line and the config.
pub(super) struct Launch {
    pub(super) quiet: bool,
    pub(super) incident_log: Arc<IncidentLog>,
    pub(super) defaults: DefaultsConfig,
    /// `--engine`.
    pub(super) cli_engine: Option<crate::engines::EngineKind>,
    /// `--reduced-motion` or `defaults.reduced_motion`.
    pub(super) reduced_motion: bool,
    /// `--theme`: present in this theme instead of the deck's.
    pub(super) cli_theme: Option<String>,
    /// `--presenter`: open the presenter view on the first frame.
    pub(super) presenter: bool,
}

impl PresentationApp {
    pub(super) fn new(
        file: PathBuf,
        presentation: Presentation,
        watch: FileWatch,
        launch: Launch,
    ) -> Self {
        let Launch {
            quiet,
            incident_log,
            defaults,
            cli_engine,
            reduced_motion,
            cli_theme,
            presenter,
        } = launch;
        let FileWatch {
            rx: watcher_rx,
            watcher,
            content_hash,
        } = watch;
        // Precedence: --theme > frontmatter > config defaults > built-in
        let themes = lookup::Lookup::for_deck(file.parent());
        let (resolved, theme_key) = match &cli_theme {
            Some(name) => {
                let (theme, problems) = lookup::resolve_or_default(&themes, name);
                deck::report_theme_problems(&problems);
                (theme, name.trim().to_ascii_lowercase())
            }
            None => deck::deck_theme(&themes, &presentation, defaults.theme.as_deref()),
        };
        let engine_override = deck::deck_engine(cli_engine, &presentation, quiet);
        let resolved = crate::engines::with_engine(resolved, engine_override);
        // `run` preloads every theme's fonts before installing them; should a
        // face still be new, start on the default theme for the frame it takes.
        let font_sync = render::fonts::FontSync::installed();
        let (theme, pending_theme) = if font_sync.ready() {
            (resolved, None)
        } else {
            (Theme::light(), Some(resolved))
        };

        let mut deck = Deck::open(file, presentation, &theme, true, quiet);
        // Art follows the theme the window is about to switch to.
        if let Some(pending) = &pending_theme {
            deck.art.sync(&deck.presentation, pending);
        }
        let engine_kind = resolved_engine(&theme, &pending_theme);
        if !quiet {
            report_deck_warnings(&deck, engine_kind);
        }
        deck.art.preload();
        let slide_count = deck.slide_count();
        let now = Instant::now();
        Self {
            deck,
            current_slide: 0,
            watcher_rx,
            _watcher: watcher,
            mode: AppMode::Presentation { end: false },
            theme,
            theme_key,
            themes,
            pending_theme,
            font_sync,
            config_transition: defaults.transition.clone(),
            cycled_transition: None,
            cli_theme,
            transition: None,
            jump: super::keys::SlideJump::default(),
            presenter: super::presenter::Presenter::new(presenter),
            show_hud: false,
            raw_overlay_side: RawOverlaySide::Off,
            toast: None,
            views: vec![SlideView::default(); slide_count],
            taps: QuitTaps::default(),
            fps: Fps::default(),
            ink: Ink::default(),
            grid: GridState::default(),
            jobs: Jobs::default(),
            overview_transition_start: None,
            last_slide_rect: egui::Rect::ZERO,
            last_content_hash: content_hash,
            quiet,
            blackout: false,
            monitor_move: None,
            pending_nav: None,
            pending_reveal_scroll: false,
            end_logo_texture: None,
            live_palette: None,
            reduced_motion,
            shared_slide: None,
            incident_log,
            last_frame: now,
            cli_engine,
            engine_override,
            countdown: None,
        }
    }
}

/// The engine the window will run on: the pending theme's when one waits
/// for its fonts, else the current theme's.
fn resolved_engine(theme: &Theme, pending: &Option<Theme>) -> crate::engines::EngineKind {
    pending.as_ref().unwrap_or(theme).engine
}

/// Startup warnings about what the deck asks of its engine: features it
/// does not support, and generated art that is missing or stale.
fn report_deck_warnings(deck: &Deck, engine: crate::engines::EngineKind) {
    if let Some(line) = crate::engines::unsupported_summary(engine, &deck.presentation) {
        eprintln!("warning: {line}");
    }
    for p in deck.art.problems() {
        eprintln!("warning: art: {p}");
    }
    let (Some(c), Some(medium)) = (deck.art.coverage(&deck.presentation), engine.medium()) else {
        return;
    };
    if c.missing == 0 && c.stale == 0 {
        return;
    }
    let mut parts = Vec::new();
    if c.missing > 0 {
        parts.push(format!(
            "{} of {} slides have no picture",
            c.missing, c.wanted
        ));
    }
    if c.stale > 0 {
        parts.push(format!("{} stale", c.stale));
    }
    eprintln!(
        "warning: art for the {} engine: {}; run `mdeck ai pictures {}` (or press S on a slide)",
        medium.name,
        parts.join(", "),
        deck.file.file_name().unwrap_or_default().to_string_lossy()
    );
}

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
        .filter(|b| matches!(b, parser::Block::Image { path, .. } if crate::assets::placeholders::is_image(path)))
        .count();
    if ungenerated > 0 {
        use colored::Colorize;
        eprintln!(
            "{} This presentation contains {} ungenerated image(s).",
            "Warning:".yellow().bold(),
            ungenerated
        );
        eprintln!(
            "  Run `mdeck ai images {}` to generate them first.\n",
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

/// How `mdeck <file>` presents.
pub struct RunOptions {
    pub file: PathBuf,
    /// A window instead of fullscreen.
    pub windowed: bool,
    /// Start on this slide (1-based).
    pub start_slide: Option<usize>,
    /// Start in the overview grid.
    pub start_overview: bool,
    pub quiet: bool,
    /// `--engine`: wins over `engine` and the theme's.
    pub engine: Option<String>,
    /// `--theme`: wins over `theme` and the config default.
    pub theme: Option<String>,
    pub reduced_motion: bool,
    /// `--presenter`: open the presenter view at once.
    pub presenter: bool,
}

pub fn run(opts: RunOptions) -> anyhow::Result<()> {
    let RunOptions {
        file,
        windowed,
        start_slide,
        start_overview,
        quiet,
        engine,
        theme: cli_theme,
        reduced_motion,
        presenter,
    } = opts;
    let file = file.canonicalize().unwrap_or(file);

    // Config defaults: start mode, theme/transition fallbacks, monitor position
    let config = Config::load_or_default();
    let defaults = config.defaults.clone().unwrap_or_default();
    let reduced_motion = reduced_motion || defaults.reduced_motion == Some(true);
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
    // An explicit --theme must exist, as for export.
    if let Some(name) = &cli_theme {
        let built = lookup::Lookup::for_deck(file.parent())
            .load(name)
            .map_err(|e| anyhow::anyhow!("--theme {name}: {e}"))?;
        if !quiet {
            deck::report_theme_problems(&built.warnings);
        }
    }

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
                reduced_motion,
                cli_theme,
                presenter,
            };
            let mut app = PresentationApp::new(file_clone, presentation, watch, launch);
            app.start_at(initial_slide, initial_overview, shared);
            Ok(Box::new(app))
        }),
    );
    report_exit(result, &incident_log, &shared_slide)
}

/// The window closed: summarise incidents, and log a display error together
/// with the slide it happened on.
fn report_exit(
    result: eframe::Result,
    incident_log: &IncidentLog,
    shared_slide: &AtomicUsize,
) -> anyhow::Result<()> {
    if let Err(e) = &result {
        let slide = shared_slide.load(Ordering::Relaxed);
        incident_log.record(
            "display_error",
            "eframe display error",
            &format!("{e}\nslide: {slide}"),
        );
    }
    print_incident_summary(incident_log);
    result.map_err(|e| anyhow::anyhow!("{e}"))
}

impl PresentationApp {
    /// Open on `slide` (in the grid with `overview`), publishing the position
    /// to `shared` for the incident log. Only the first slide gets the opening
    /// countdown: starting on a chosen slide (an agent checking its work, a
    /// presenter resuming) skips it.
    fn start_at(&mut self, slide: usize, overview: bool, shared: Arc<AtomicUsize>) {
        self.current_slide = slide;
        self.shared_slide = Some(shared);
        if overview {
            self.mode = AppMode::Grid { selected: slide };
        } else if slide == 0 {
            self.start_countdown();
        }
        self.spawn_diagram_precache();
    }
}
