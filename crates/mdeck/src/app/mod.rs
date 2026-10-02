mod actions;
mod ai;
mod cockpit;
mod countdown;
mod drawing;
mod end_slide;
mod frame;
mod grid;
mod helpers;
mod input;
pub mod keys;
mod launch;
mod look;
mod navigation;
mod overlays;
mod overview;
mod placement;
pub mod presenter;
mod reload;
mod toast;

use actions::{MonitorMove, QuitTaps, ViewportSnapshot};
use countdown::{Countdown, CountdownPhase};
use grid::{GridLayout, GridState};
use input::Ink;
pub use launch::{RunOptions, run};
use toast::Toast;

use eframe::egui;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use notify_debouncer_mini::{Debouncer, notify};

use crate::check::CheckReport;
use crate::deck::Deck;
use crate::incident_log::IncidentLog;
use crate::parser::{self};
use crate::render;
use crate::render::transition::{ActiveTransition, TransitionKind};
use crate::theme::{Theme, lookup};

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

/// Work running off the frame loop.
#[derive(Default)]
struct Jobs {
    /// Cancels the background diagram route pre-caching thread.
    precache_cancel: Arc<AtomicBool>,
    /// The check report from the pre-caching thread.
    precache_report: Option<mpsc::Receiver<CheckReport>>,
    /// Whether that report has been printed.
    report_printed: bool,
    /// `S` art generation in flight: receives (slide, result).
    art: Option<mpsc::Receiver<(usize, Result<(), String>)>>,
}

/// Frames per second, measured over half-second windows (shown with the HUD).
struct Fps {
    frames: u32,
    per_second: f32,
    since: Instant,
}

impl Fps {
    /// Count a frame; the rate updates every half second.
    fn tick(&mut self) {
        self.frames += 1;
        let elapsed = self.since.elapsed().as_secs_f32();
        if elapsed >= 0.5 {
            self.per_second = self.frames as f32 / elapsed;
            self.frames = 0;
            self.since = Instant::now();
        }
    }
}

impl Default for Fps {
    fn default() -> Self {
        Self {
            frames: 0,
            per_second: 0.0,
            since: Instant::now(),
        }
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
    /// `engine`); `None` keeps each theme's own.
    engine_override: Option<crate::engines::EngineKind>,
    /// Keeps the context's fonts in step with theme font files.
    font_sync: render::fonts::FontSync,
    /// `defaults.transition` from the user config.
    config_transition: Option<String>,
    /// A transition picked live with `T`; wins over the resolved one.
    cycled_transition: Option<TransitionKind>,
    /// `--theme` from the command line: wins over `theme`, also on reload.
    cli_theme: Option<String>,
    transition: Option<ActiveTransition>,
    /// Digits typed for a slide jump.
    jump: keys::SlideJump,
    /// The presenter window and the notes overlay.
    presenter: presenter::Presenter,
    show_hud: bool,
    raw_overlay_side: RawOverlaySide,
    toast: Option<Toast>,
    /// Reveal progress and scroll position, one per slide.
    views: Vec<SlideView>,
    /// The double-tap quit keys (Esc, Q, Ctrl+C).
    taps: QuitTaps,
    fps: Fps,
    /// Pen strokes and arrows over the slides.
    ink: Ink,
    /// The overview grid's pointer and scroll.
    grid: GridState,
    /// Work running off the frame loop.
    jobs: Jobs,
    overview_transition_start: Option<Instant>,
    /// Cached slide rect from last frame, used for mouse coordinate conversion
    last_slide_rect: egui::Rect,
    /// Hash of last loaded file content (to skip spurious watcher events)
    last_content_hash: u64,
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
    /// Cached texture for the embedded logo (loaded once on first draw).
    end_logo_texture: Option<egui::TextureHandle>,
    /// The thermal palette picked live with `C`; `None` keeps each block's.
    pub(super) live_palette: Option<crate::render::thermal::Palette>,
    /// `--reduced-motion` (or the config default): every slide and step in
    /// its settled state, without transitions, entries or engine motion.
    pub(super) reduced_motion: bool,
    /// Shared slide position for recovery after display errors.
    shared_slide: Option<Arc<AtomicUsize>>,
    /// Incident log for recording recovered and fatal errors.
    incident_log: Arc<IncidentLog>,
    /// Timestamp of the previous frame, used to detect power-state time jumps.
    last_frame: Instant,
    /// Opening countdown, while it runs.
    countdown: Option<Countdown>,
}

impl PresentationApp {
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

    fn slide_count(&self) -> usize {
        self.deck.slide_count()
    }

    /// Slide `index`'s reveal and scroll state (settled, at the top, when
    /// out of range).
    fn view(&self, index: usize) -> SlideView {
        self.views.get(index).copied().unwrap_or_default()
    }

    /// Keep every slide's reveal within its step count (after the counts
    /// changed).
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
        self.deck
            .preload_backgrounds(ctx, self.current_slide..self.current_slide + 3);
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

    /// The overview grid for this deck in `rect`.
    fn grid_layout(&self, rect: egui::Rect, scale: f32) -> GridLayout {
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
