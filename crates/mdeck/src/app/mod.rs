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
mod look;
mod navigation;
mod overlays;
mod overview;
mod reload;
mod toast;

use actions::{MonitorMove, ViewportSnapshot};
use countdown::{Countdown, CountdownPhase};
use grid::GridLayout;
use input::{ActiveDraw, ArrowAnnotation, PenStroke};
use keys::DoubleTap;
pub use launch::run;
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
