mod actions;
mod ai;
mod countdown;
mod drawing;
mod end_slide;
mod frame;
mod grid;
mod helpers;
mod input;
pub mod keys;
mod launch;
mod navigation;
mod overlays;
mod overview;
mod reload;

use actions::{MonitorMove, ViewportSnapshot};
use countdown::{Countdown, CountdownPhase};
use grid::GridLayout;
use helpers::{
    find_matching_slide, hash_content, load_app_icon, print_incident_summary, resolve_setting,
    spawn_file_watcher,
};
use input::{ActiveDraw, ArrowAnnotation, PenStroke};
use keys::{Action, DoubleTap, MonitorMoveOutcome, evaluate_monitor_move};
pub use launch::run;

use eframe::egui;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use notify_debouncer_mini::{Debouncer, notify};

use crate::check::CheckReport;
use crate::config::{Config, DefaultsConfig};
use crate::deck::{self, Deck};
use crate::incident_log::IncidentLog;
use crate::parser::{self, Presentation};
use crate::render;
use crate::render::transition::{ActiveTransition, TransitionDirection, TransitionKind};
use crate::theme::{Countdown as ThemeCountdown, Theme, lookup};

const OVERVIEW_TRANSITION_DURATION: f32 = 0.4;
const DRAW_FADE_DURATION: f32 = 8.0;
const DRAG_THRESHOLD: f32 = 5.0;
/// Window for double-tap quit gestures (Esc, Q, Ctrl+C).
const DOUBLE_TAP_WINDOW: Duration = Duration::from_secs(1);
/// A reveal animation counts as "in flight" for this long after it started.
const REVEAL_IN_FLIGHT_WINDOW: Duration = Duration::from_secs(3);

/// A navigation request made while a transition was running; applied when
/// the transition completes so quick key presses are not dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingNav {
    Forward,
    Backward,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum RawOverlaySide {
    Off,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum AppMode {
    /// Presenting slides; `end` is the virtual "The End" slide after the last.
    Presentation {
        end: bool,
    },
    Grid {
        selected: usize,
    },
    OverviewTransition {
        selected: usize,
        entering: bool,
    },
}

/// Where one slide stands: how far it is revealed and how far scrolled.
#[derive(Debug, Clone, Copy, Default)]
struct SlideView {
    /// Reveal steps shown.
    reveal: usize,
    /// When the latest step was revealed (animates it in); `None` once
    /// stepping back, so what stays on screen does not rise in again.
    revealed_at: Option<Instant>,
    /// Current, animated scroll offset.
    scroll: f32,
    /// Where the scroll offset is heading.
    scroll_target: f32,
}

impl SlideView {
    fn reset_scroll(&mut self) {
        self.scroll = 0.0;
        self.scroll_target = 0.0;
    }
}

struct PresentationApp {
    /// The deck and everything resolved for it, shared with export.
    deck: Deck,
    current_slide: usize,
    watcher_rx: mpsc::Receiver<()>,
    _watcher: Option<Debouncer<notify::RecommendedWatcher>>,
    mode: AppMode,
    theme: Theme,
    /// The name the theme was looked up by (for `Shift+T` cycling).
    theme_key: String,
    /// Where themes are looked up for this deck.
    themes: lookup::Lookup,
    /// A theme waiting for its font faces to become drawable (one frame).
    pending_theme: Option<Theme>,
    /// `--engine` from the command line (wins over everything).
    cli_engine: Option<crate::engines::EngineKind>,
    /// The engine every theme runs on for this deck (`--engine`, then
    /// `@engine`); `None` keeps each theme's own.
    engine_override: Option<crate::engines::EngineKind>,
    /// Keeps the context's fonts in step with theme font files.
    font_sync: render::fonts::FontSync,
    default_transition: TransitionKind,
    transition: Option<ActiveTransition>,
    show_hud: bool,
    raw_overlay_side: RawOverlaySide,
    toast: Option<Toast>,
    ctrl_c_tap: DoubleTap,
    esc_tap: DoubleTap,
    quit_tap: DoubleTap,
    /// Reveal progress and scroll position, one per slide.
    views: Vec<SlideView>,
    frame_count: u32,
    fps: f32,
    fps_update: Instant,
    overview_transition_start: Option<Instant>,
    pen_strokes: Vec<PenStroke>,
    arrows: Vec<ArrowAnnotation>,
    active_draw: ActiveDraw,
    /// Cached slide rect from last frame, used for mouse coordinate conversion
    last_slide_rect: egui::Rect,
    /// Which grid cell the mouse is hovering over
    hover_slide: Option<usize>,
    /// Whether to show hover effect (false when keyboard took over)
    use_hover: bool,
    /// Last known hover position, used to detect actual mouse movement
    last_hover_pos: Option<egui::Pos2>,
    /// Current animated scroll position in grid
    grid_scroll_offset: f32,
    /// Target scroll position in grid
    grid_scroll_target: f32,
    /// Hash of last loaded file content (to skip spurious watcher events)
    last_content_hash: u64,
    /// Cancel flag for the background diagram route pre-caching thread.
    precache_cancel: Arc<AtomicBool>,
    /// Receives the check report from the background precache thread.
    precache_report_rx: Option<mpsc::Receiver<CheckReport>>,
    /// Whether the precache report has already been printed.
    precache_report_printed: bool,
    /// Suppress non-essential output.
    quiet: bool,
    /// Whether the screen is blacked out (toggled with `.` or `B`).
    blackout: bool,
    /// In-progress monitor hop (M key).
    monitor_move: Option<MonitorMove>,
    /// Navigation requested during a transition, applied when it completes.
    pending_nav: Option<PendingNav>,
    /// A reveal step was just added; scroll to show it once content is measured.
    pending_reveal_scroll: bool,
    /// Seed the grid scroll so the selected cell is visible before the zoom-out.
    grid_seed_scroll: bool,
    /// Cached texture for the embedded logo (loaded once on first draw).
    end_logo_texture: Option<egui::TextureHandle>,
    /// Shared slide position for recovery after display errors.
    shared_slide: Option<Arc<AtomicUsize>>,
    /// Incident log for recording recovered and fatal errors.
    incident_log: Arc<IncidentLog>,
    /// Timestamp of the previous frame, used to detect power-state time jumps.
    last_frame: Instant,
    /// Background `S` art generation in flight: receives (slide, result).
    art_rx: Option<mpsc::Receiver<(usize, Result<(), String>)>>,
    /// Background `S` generation in flight: receives (slide, result).
    story_rx: Option<mpsc::Receiver<(usize, Result<(), String>)>>,
    /// Opening countdown, while it runs.
    countdown: Option<Countdown>,
}

struct Toast {
    message: String,
    start: Instant,
}

impl Toast {
    fn new(message: String) -> Self {
        Self {
            message,
            start: Instant::now(),
        }
    }

    fn opacity(&self) -> f32 {
        let elapsed = self.start.elapsed().as_secs_f32();
        let duration = 1.5;
        let fade_start = 1.0;
        if elapsed < fade_start {
            1.0
        } else if elapsed < duration {
            1.0 - (elapsed - fade_start) / (duration - fade_start)
        } else {
            0.0
        }
    }

    fn is_expired(&self) -> bool {
        self.start.elapsed().as_secs_f32() >= 1.5
    }
}

/// The watcher that tells the window its deck file changed.
struct FileWatch {
    rx: mpsc::Receiver<()>,
    watcher: Option<Debouncer<notify::RecommendedWatcher>>,
    /// Hash of the content the deck was parsed from.
    content_hash: u64,
}

/// What the window starts with from the command line and the config.
struct Launch {
    quiet: bool,
    incident_log: Arc<IncidentLog>,
    defaults: DefaultsConfig,
    /// `--engine`.
    cli_engine: Option<crate::engines::EngineKind>,
}

impl PresentationApp {
    fn new(file: PathBuf, presentation: Presentation, watch: FileWatch, launch: Launch) -> Self {
        let Launch {
            quiet,
            incident_log,
            defaults,
            cli_engine,
        } = launch;
        let FileWatch {
            rx: watcher_rx,
            watcher,
            content_hash,
        } = watch;
        // Precedence: frontmatter > config defaults > built-in
        let themes = lookup::Lookup::for_deck(file.parent());
        let (resolved, theme_key) =
            deck::deck_theme(&themes, &presentation, defaults.theme.as_deref());
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

        let transition_name = resolve_setting(
            presentation.meta.transition.as_deref(),
            defaults.transition.as_deref(),
            "slide",
        );
        let default_transition = TransitionKind::from_name(&transition_name);

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
            art_rx: None,
            current_slide: 0,
            watcher_rx,
            _watcher: watcher,
            mode: AppMode::Presentation { end: false },
            theme,
            theme_key,
            themes,
            pending_theme,
            font_sync,
            default_transition,
            transition: None,
            show_hud: false,
            raw_overlay_side: RawOverlaySide::Off,
            toast: None,
            ctrl_c_tap: DoubleTap::new(DOUBLE_TAP_WINDOW),
            esc_tap: DoubleTap::new(DOUBLE_TAP_WINDOW),
            quit_tap: DoubleTap::new(DOUBLE_TAP_WINDOW),
            views: vec![SlideView::default(); slide_count],
            frame_count: 0,
            fps: 0.0,
            fps_update: now,
            overview_transition_start: None,
            pen_strokes: Vec::new(),
            arrows: Vec::new(),
            active_draw: ActiveDraw::None,
            last_slide_rect: egui::Rect::ZERO,
            hover_slide: None,
            use_hover: false,
            last_hover_pos: None,
            grid_scroll_offset: 0.0,
            grid_scroll_target: 0.0,
            last_content_hash: content_hash,
            precache_cancel: Arc::new(AtomicBool::new(false)),
            precache_report_rx: None,
            precache_report_printed: false,
            quiet,
            blackout: false,
            monitor_move: None,
            pending_nav: None,
            pending_reveal_scroll: false,
            grid_seed_scroll: false,
            end_logo_texture: None,
            shared_slide: None,
            incident_log,
            last_frame: now,
            cli_engine,
            engine_override,
            story_rx: None,
            countdown: None,
        }
    }

    fn on_end_slide(&self) -> bool {
        matches!(self.mode, AppMode::Presentation { end: true })
    }

    /// Back from the end slide to the last real one, if it is showing.
    fn leave_end_slide(&mut self) {
        if let AppMode::Presentation { end } = &mut self.mode {
            *end = false;
        }
    }

    fn countdown_running(&self) -> bool {
        self.countdown.is_some()
    }

    /// The story playing on slide `index`, if any.
    fn story(&self, index: usize) -> Option<&crate::render::story::Script> {
        self.deck.story(index)
    }

    fn slide_count(&self) -> usize {
        self.deck.slide_count()
    }

    /// Slide `index`'s reveal and scroll state (settled, at the top, when
    /// out of range).
    fn view(&self, index: usize) -> SlideView {
        self.views.get(index).copied().unwrap_or_default()
    }

    /// Keep every slide's reveal within its step count (after the counts
    /// changed: a theme switch, new stories).
    fn clamp_reveals(&mut self) {
        for (v, &max) in self.views.iter_mut().zip(&self.deck.max_steps) {
            v.reveal = v.reveal.min(max);
        }
    }

    fn display_title(&self) -> String {
        self.deck
            .presentation
            .meta
            .title
            .clone()
            .unwrap_or_else(|| {
                self.deck
                    .file
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()
            })
    }

    /// Start decoding images on the upcoming slides so they're ready to draw
    /// by the time the presenter reaches them.
    fn preload_upcoming_images(&self, ctx: &egui::Context) {
        for offset in 1..=2 {
            let Some(slide) = self
                .deck
                .presentation
                .slides
                .get(self.current_slide + offset)
            else {
                break;
            };
            for block in &slide.blocks {
                if let parser::Block::Image { path, .. } = block
                    && !path.is_empty()
                    && path != "image-generation"
                {
                    self.deck.image_cache.preload(ctx, path);
                }
            }
        }
    }

    /// Whether any time-based animation is currently running. Used to decide
    /// whether a long frame gap (sleep, occlusion) is worth an incident entry.
    fn animation_in_flight(&self, reference: Instant) -> bool {
        self.transition.is_some()
            || self.overview_transition_start.is_some()
            || self.toast.is_some()
            || !self.pen_strokes.is_empty()
            || !self.arrows.is_empty()
            || !matches!(self.active_draw, ActiveDraw::None)
            || self.monitor_move.is_some()
            || self
                .views
                .iter()
                .any(|v| keys::reveal_in_flight(v.revealed_at, reference, REVEAL_IN_FLIGHT_WINDOW))
    }

    /// Shift every animation timestamp forward by `jump` so animations resume
    /// smoothly after a frame gap instead of snapping to completion.
    fn shift_timestamps(&mut self, jump: Duration, now: Instant) {
        if let Some(ref mut t) = self.transition {
            t.start = (t.start + jump).min(now);
        }
        if let Some(ref mut t) = self.overview_transition_start {
            *t = (*t + jump).min(now);
        }
        for stroke in &mut self.pen_strokes {
            stroke.start = (stroke.start + jump).min(now);
        }
        for arrow in &mut self.arrows {
            arrow.start = (arrow.start + jump).min(now);
        }
        if let Some(ref mut t) = self.toast {
            t.start = (t.start + jump).min(now);
        }
        for t in self.views.iter_mut().filter_map(|v| v.revealed_at.as_mut()) {
            *t = (*t + jump).min(now);
        }
    }

    /// `Shift+T`: the next theme visible from this deck (built-ins, then
    /// user and deck themes by name).
    fn toggle_theme(&mut self) {
        let names: Vec<String> = self
            .themes
            .available()
            .into_iter()
            .map(|f| f.name)
            .collect();
        if names.is_empty() {
            return;
        }
        let next = names
            .iter()
            .position(|n| *n == self.theme_key)
            .map(|i| (i + 1) % names.len())
            .unwrap_or(0);
        let key = names[next].clone();
        match self.themes.load(&key) {
            Ok(built) => {
                deck::report_theme_problems(&built.warnings);
                self.theme_key = key;
                self.pending_theme = Some(built.theme);
            }
            Err(e) => {
                // Skip a broken theme rather than getting stuck on it.
                deck::report_theme_problems(&[e.to_string()]);
                self.theme_key = key;
                self.toast = Some(Toast::new(format!("Theme {}: {e}", names[next])));
            }
        }
    }

    /// Switch to `theme` now: step counts follow its engine (story beats are
    /// an ember feature; other engines step through the content's own reveals).
    fn apply_theme(&mut self, theme: Theme) {
        self.theme = crate::engines::with_engine(theme, self.engine_override);
        self.deck.retheme(&self.theme);
        self.clamp_reveals();
        self.toast = Some(Toast::new(format!("Theme: {}", self.theme.name)));
    }

    fn cycle_transition(&mut self) {
        self.default_transition = match self.default_transition {
            TransitionKind::SlideHorizontal => TransitionKind::Fade,
            TransitionKind::Fade => TransitionKind::Spatial,
            TransitionKind::Spatial => TransitionKind::None,
            TransitionKind::None => TransitionKind::SlideHorizontal,
        };
        let name = match self.default_transition {
            TransitionKind::SlideHorizontal => "Slide",
            TransitionKind::Fade => "Fade",
            TransitionKind::Spatial => "Spatial",
            TransitionKind::None => "None",
        };
        self.toast = Some(Toast::new(format!("Transition: {name}")));
    }

    fn update_fps(&mut self) {
        self.frame_count += 1;
        let elapsed = self.fps_update.elapsed().as_secs_f32();
        if elapsed >= 0.5 {
            self.fps = self.frame_count as f32 / elapsed;
            self.frame_count = 0;
            self.fps_update = Instant::now();
        }
    }

    /// The overview grid for this deck in `rect`.
    fn grid(&self, rect: egui::Rect, scale: f32) -> GridLayout {
        GridLayout::new(self.slide_count(), rect, scale)
    }

    fn compute_scale(rect: egui::Rect) -> f32 {
        let ref_w = 1920.0;
        let ref_h = 1080.0;
        (rect.width() / ref_w).min(rect.height() / ref_h)
    }

    /// Convert screen position to slide-local coordinates (accounting for scroll)
    fn screen_to_local(&self, screen_pos: egui::Pos2) -> egui::Pos2 {
        let rect = self.last_slide_rect;
        let scroll = self.views[self.current_slide].scroll;
        egui::pos2(
            screen_pos.x - rect.left(),
            screen_pos.y - rect.top() + scroll,
        )
    }

    /// Convert slide-local coordinates back to screen position
    fn local_to_screen(&self, local: egui::Pos2) -> egui::Pos2 {
        let rect = self.last_slide_rect;
        let scroll = self.views[self.current_slide].scroll;
        egui::pos2(local.x + rect.left(), local.y + rect.top() - scroll)
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
    if let Some(line) =
        crate::engines::unsupported_summary(engine, &deck.presentation, &deck.with_story())
    {
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
        "warning: art for the {} engine: {}; run `mdeck ai art {}` (or press S on a slide)",
        medium.name,
        parts.join(", "),
        deck.file.file_name().unwrap_or_default().to_string_lossy()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::parser::{Layout, Slide};

    fn slide(raw: &str) -> Slide {
        Slide {
            directives: vec![],
            blocks: vec![],
            layout: Layout::Content,
            raw_source: raw.to_string(),
            notes: None,
            story_hint: None,
            scene_script: None,
            illustration: None,
            logo: None,
            art: None,
        }
    }

    #[test]
    fn find_matching_slide_exact_match() {
        let _slides = [slide("a"), slide("b"), slide("c")];
        // Was at index 1 ("b"), new slides inserted "x" before it
        let new_slides = vec![slide("x"), slide("a"), slide("b"), slide("c")];
        assert_eq!(find_matching_slide(Some("b"), 1, &new_slides), 2);
    }

    #[test]
    fn find_matching_slide_edited_stays_at_index() {
        let old_raw = "old content";
        let new_slides = vec![slide("a"), slide("new content"), slide("c")];
        // Old raw doesn't match any new slide: clamp to old index
        assert_eq!(find_matching_slide(Some(old_raw), 1, &new_slides), 1);
    }

    #[test]
    fn find_matching_slide_clamps_when_out_of_bounds() {
        let new_slides = vec![slide("a"), slide("b")];
        // Was at index 5, only 2 slides now
        assert_eq!(find_matching_slide(Some("gone"), 5, &new_slides), 1);
    }

    #[test]
    fn find_matching_slide_no_old_raw_returns_zero() {
        let new_slides = vec![slide("a"), slide("b")];
        assert_eq!(find_matching_slide(None, 0, &new_slides), 0);
    }
}
