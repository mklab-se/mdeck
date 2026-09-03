mod drawing;
mod helpers;
mod input;
pub mod keys;

use drawing::{draw_hud, draw_raw_markdown_overlay};
use helpers::{
    find_matching_slide, hash_content, load_app_icon, print_incident_summary, resolve_setting,
    spawn_file_watcher,
};
use keys::{Action, DoubleTap, KeyMode, MonitorMoveOutcome, evaluate_monitor_move, map_key};

use eframe::egui;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use notify_debouncer_mini::{Debouncer, notify};

use crate::check::CheckReport;
use crate::config::{Config, DefaultsConfig};
use crate::incident_log::IncidentLog;
use crate::parser::{self, Presentation};
use crate::render;
use crate::render::image_cache::ImageCache;
use crate::render::transition::{ActiveTransition, TransitionDirection, TransitionKind};
use crate::theme::Theme;

const OVERVIEW_TRANSITION_DURATION: f32 = 0.4;
const DRAW_FADE_DURATION: f32 = 8.0;
const DRAG_THRESHOLD: f32 = 5.0;
/// Window for double-tap quit gestures (Esc, Q, Ctrl+C).
const DOUBLE_TAP_WINDOW: Duration = Duration::from_secs(1);
/// A reveal animation counts as "in flight" for this long after it started.
const REVEAL_IN_FLIGHT_WINDOW: Duration = Duration::from_secs(3);
/// How long to wait for the window to settle after a monitor move.
const MONITOR_MOVE_SETTLE: Duration = Duration::from_millis(1000);

/// A navigation request made while a transition was running; applied when
/// the transition completes so quick key presses are not dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingNav {
    Forward,
    Backward,
}

/// State machine for hopping a fullscreen window to the next monitor.
struct MonitorMove {
    /// Requested window position.
    target: egui::Pos2,
    monitor_width: f32,
    /// Whether we already wrapped around to the origin.
    wrapped: bool,
    phase: MonitorMovePhase,
}

enum MonitorMovePhase {
    /// Fullscreen was dropped and the window repositioned; re-enter fullscreen next frame.
    Reposition,
    /// Fullscreen re-entered at this instant; verify where the window landed after settling.
    Verify(Instant),
}

/// Viewport facts captured inside `ctx.input` for use outside the closure.
#[derive(Debug, Clone, Copy, Default)]
struct ViewportSnapshot {
    fullscreen: bool,
    monitor_size: Option<egui::Vec2>,
    outer_pos: Option<egui::Pos2>,
}

/// A freehand pen stroke (left-drag)
struct PenStroke {
    points: Vec<egui::Pos2>,
    start: Instant,
    slide_index: usize,
}

/// An arrow annotation (right-drag)
struct ArrowAnnotation {
    from: egui::Pos2,
    to: egui::Pos2,
    start: Instant,
    slide_index: usize,
}

/// Tracks an in-progress mouse interaction
enum ActiveDraw {
    None,
    /// Left button held: collecting points, might still be a click
    PenPending {
        origin: egui::Pos2,
        points: Vec<egui::Pos2>,
    },
    /// Left button held: drag threshold exceeded, definitely drawing
    PenDrawing {
        points: Vec<egui::Pos2>,
    },
    /// Right button held: collecting start/end, might still be a click
    ArrowPending {
        origin: egui::Pos2,
        current: egui::Pos2,
    },
    /// Right button held: drag threshold exceeded, definitely an arrow
    ArrowDrawing {
        from: egui::Pos2,
        current: egui::Pos2,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum RawOverlaySide {
    Off,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum AppMode {
    Presentation,
    Grid { selected: usize },
    OverviewTransition { selected: usize, entering: bool },
}

struct PresentationApp {
    presentation: Presentation,
    file_path: PathBuf,
    current_slide: usize,
    watcher_rx: mpsc::Receiver<()>,
    _watcher: Option<Debouncer<notify::RecommendedWatcher>>,
    mode: AppMode,
    theme: Theme,
    default_transition: TransitionKind,
    transition: Option<ActiveTransition>,
    image_cache: ImageCache,
    show_hud: bool,
    raw_overlay_side: RawOverlaySide,
    toast: Option<Toast>,
    ctrl_c_tap: DoubleTap,
    esc_tap: DoubleTap,
    quit_tap: DoubleTap,
    reveal_steps: Vec<usize>,
    max_steps: Vec<usize>,
    /// Timestamp of when each slide's reveal_step was last incremented (for animation)
    reveal_timestamps: Vec<Option<Instant>>,
    scroll_offsets: Vec<f32>,
    scroll_targets: Vec<f32>,
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
    /// Whether the virtual "The End" slide is being displayed.
    on_end_slide: bool,
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

impl PresentationApp {
    #[allow(clippy::too_many_arguments)]
    fn new(
        file: PathBuf,
        presentation: Presentation,
        watcher_rx: mpsc::Receiver<()>,
        watcher: Option<Debouncer<notify::RecommendedWatcher>>,
        content_hash: u64,
        quiet: bool,
        incident_log: Arc<IncidentLog>,
        defaults: &DefaultsConfig,
    ) -> Self {
        // Precedence: frontmatter > config defaults > built-in
        let theme_name = resolve_setting(
            presentation.meta.theme.as_deref(),
            defaults.theme.as_deref(),
            "light",
        );
        let theme = Theme::from_name(&theme_name);

        let transition_name = resolve_setting(
            presentation.meta.transition.as_deref(),
            defaults.transition.as_deref(),
            "slide",
        );
        let default_transition = TransitionKind::from_name(&transition_name);

        let base_path = file
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .to_path_buf();
        let image_cache = ImageCache::new(base_path);

        let max_steps: Vec<usize> = presentation
            .slides
            .iter()
            .map(|s| parser::compute_max_steps(&s.blocks))
            .collect();
        let slide_count = presentation.slides.len();
        let reveal_steps = vec![0; slide_count];
        let reveal_timestamps = vec![None; slide_count];
        let scroll_offsets = vec![0.0; slide_count];
        let scroll_targets = vec![0.0; slide_count];

        let now = Instant::now();
        Self {
            presentation,
            file_path: file,
            current_slide: 0,
            watcher_rx,
            _watcher: watcher,
            mode: AppMode::Presentation,
            theme,
            default_transition,
            transition: None,
            image_cache,
            show_hud: false,
            raw_overlay_side: RawOverlaySide::Off,
            toast: None,
            ctrl_c_tap: DoubleTap::new(DOUBLE_TAP_WINDOW),
            esc_tap: DoubleTap::new(DOUBLE_TAP_WINDOW),
            quit_tap: DoubleTap::new(DOUBLE_TAP_WINDOW),
            reveal_steps,
            max_steps,
            reveal_timestamps,
            scroll_offsets,
            scroll_targets,
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
            on_end_slide: false,
            blackout: false,
            monitor_move: None,
            pending_nav: None,
            pending_reveal_scroll: false,
            grid_seed_scroll: false,
            end_logo_texture: None,
            shared_slide: None,
            incident_log,
            last_frame: now,
        }
    }

    fn slide_count(&self) -> usize {
        self.presentation.slides.len()
    }

    fn display_title(&self) -> String {
        self.presentation.meta.title.clone().unwrap_or_else(|| {
            self.file_path
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
            let Some(slide) = self.presentation.slides.get(self.current_slide + offset) else {
                break;
            };
            for block in &slide.blocks {
                if let parser::Block::Image { path, .. } = block
                    && !path.is_empty()
                    && path != "image-generation"
                {
                    self.image_cache.preload(ctx, path);
                }
            }
        }
    }

    fn navigate_forward(&mut self) {
        if self.transition.is_some() {
            self.pending_nav = Some(PendingNav::Forward);
            return;
        }

        // Already on end slide — nowhere to go
        if self.on_end_slide {
            return;
        }

        let idx = self.current_slide;

        // If we have reveal steps remaining, reveal next item
        if self.reveal_steps[idx] < self.max_steps[idx] {
            self.reveal_steps[idx] += 1;
            self.reveal_timestamps[idx] = Some(Instant::now());
            self.pending_reveal_scroll = true;
            return;
        }

        // On last real slide — transition to end slide
        if idx >= self.slide_count().saturating_sub(1) {
            self.scroll_offsets[idx] = 0.0;
            self.scroll_targets[idx] = 0.0;
            self.on_end_slide = true;
            return;
        }

        // Scroll offsets are reset when the transition completes so the
        // outgoing slide keeps its scroll position while sliding out.
        self.transition = Some(ActiveTransition::new(
            idx,
            idx + 1,
            self.default_transition,
            TransitionDirection::Forward,
        ));
    }

    fn navigate_backward(&mut self) {
        if self.transition.is_some() {
            self.pending_nav = Some(PendingNav::Backward);
            return;
        }

        // Coming back from end slide — return to last real slide
        if self.on_end_slide {
            self.on_end_slide = false;
            return;
        }

        let idx = self.current_slide;

        // If we've revealed items, un-reveal
        if self.reveal_steps[idx] > 0 {
            self.reveal_steps[idx] -= 1;
            return;
        }

        // Otherwise go to previous slide (fully revealed)
        if idx == 0 {
            return;
        }

        let prev = idx - 1;
        // Show previous slide fully revealed
        self.reveal_steps[prev] = self.max_steps[prev];

        self.transition = Some(ActiveTransition::new(
            idx,
            prev,
            self.default_transition,
            TransitionDirection::Backward,
        ));
    }

    /// Jump directly to a slide (Home/End). The target is shown fully
    /// revealed and scrolled to the top, like `navigate_backward`.
    fn jump_to_slide(&mut self, index: usize) {
        if index >= self.slide_count() || self.transition.is_some() {
            return;
        }
        self.on_end_slide = false;
        self.reveal_steps[index] = self.max_steps[index];
        let cur = self.current_slide;
        if index == cur {
            return;
        }
        self.scroll_offsets[index] = 0.0;
        self.scroll_targets[index] = 0.0;
        let direction = if index > cur {
            TransitionDirection::Forward
        } else {
            TransitionDirection::Backward
        };
        self.transition = Some(ActiveTransition::new(
            cur,
            index,
            self.default_transition,
            direction,
        ));
    }

    /// Finish a completed slide transition: land on the target slide, reset
    /// the outgoing slide's scroll, and apply any navigation queued meanwhile.
    fn advance_transition(&mut self) {
        let Some(t) = self.transition.as_ref() else {
            return;
        };
        if !t.is_complete() {
            return;
        }
        let (from, to) = (t.from, t.to);
        self.transition = None;
        self.current_slide = to;
        if let Some(o) = self.scroll_offsets.get_mut(from) {
            *o = 0.0;
        }
        if let Some(t) = self.scroll_targets.get_mut(from) {
            *t = 0.0;
        }
        if let Some(nav) = self.pending_nav.take() {
            match nav {
                PendingNav::Forward => self.navigate_forward(),
                PendingNav::Backward => self.navigate_backward(),
            }
        }
    }

    /// Finish a completed grid zoom animation.
    fn advance_overview_transition(&mut self) {
        let AppMode::OverviewTransition { selected, entering } = self.mode else {
            return;
        };
        let Some(start) = self.overview_transition_start else {
            return;
        };
        if start.elapsed().as_secs_f32() < OVERVIEW_TRANSITION_DURATION {
            return;
        }
        let selected = selected.min(self.slide_count().saturating_sub(1));
        if entering {
            self.mode = AppMode::Grid { selected };
            // The hero slide zoomed out to its unscrolled cell; forget its scroll.
            let cur = self.current_slide;
            self.scroll_offsets[cur] = 0.0;
            self.scroll_targets[cur] = 0.0;
        } else {
            self.current_slide = selected;
            // Leaving the grid behaves like navigating back: fully revealed, top.
            self.reveal_steps[selected] = self.max_steps[selected];
            self.scroll_offsets[selected] = 0.0;
            self.scroll_targets[selected] = 0.0;
            self.mode = AppMode::Presentation;
        }
        self.overview_transition_start = None;
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
                .reveal_timestamps
                .iter()
                .any(|t| keys::reveal_in_flight(*t, reference, REVEAL_IN_FLIGHT_WINDOW))
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
        for t in self.reveal_timestamps.iter_mut().flatten() {
            *t = (*t + jump).min(now);
        }
    }

    fn toggle_theme(&mut self) {
        self.theme = self.theme.next();
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

    fn reload_presentation(&mut self) {
        let content = match std::fs::read_to_string(&self.file_path) {
            Ok(c) => c,
            Err(e) => {
                self.incident_log.record(
                    "file_reload_error",
                    "failed to read presentation file for reload",
                    &format!("{e}\npath: {}", self.file_path.display()),
                );
                self.toast = Some(Toast::new(format!("Reload error: {e}")));
                return;
            }
        };

        // Skip reload if file content hasn't actually changed (macOS FSEvents
        // can fire spuriously, and each reload resets per-slide state).
        let new_hash = hash_content(&content);
        if new_hash == self.last_content_hash {
            return;
        }
        self.last_content_hash = new_hash;

        let base_path = self.file_path.parent().unwrap_or(std::path::Path::new("."));
        let new_presentation = parser::parse(&content, base_path);

        if new_presentation.slides.is_empty() {
            self.toast = Some(Toast::new("Reload: no slides found".to_string()));
            return;
        }

        self.apply_reloaded(new_presentation);
    }

    /// Swap in a re-parsed presentation, preserving as much per-slide state
    /// (position, reveal progress, scroll) as still makes sense.
    fn apply_reloaded(&mut self, new_presentation: Presentation) {
        // Preserve slide position
        let old_current = self.current_slide;
        let old_raw = self
            .presentation
            .slides
            .get(old_current)
            .map(|s| s.raw_source.as_str());
        let old_reveal = self.reveal_steps.get(old_current).copied().unwrap_or(0);
        let old_scroll = self.scroll_targets.get(old_current).copied().unwrap_or(0.0);
        self.current_slide = find_matching_slide(old_raw, old_current, &new_presentation.slides);

        let slide_count = new_presentation.slides.len();

        // Recompute per-slide vectors, keeping the current slide's reveal
        // progress (clamped to the new step count) and scroll position.
        self.max_steps = new_presentation
            .slides
            .iter()
            .map(|s| parser::compute_max_steps(&s.blocks))
            .collect();
        self.reveal_steps = vec![0; slide_count];
        self.reveal_timestamps = vec![None; slide_count];
        self.scroll_offsets = vec![0.0; slide_count];
        self.scroll_targets = vec![0.0; slide_count];
        let cur = self.current_slide;
        self.reveal_steps[cur] = old_reveal.min(self.max_steps[cur]);
        self.scroll_targets[cur] = old_scroll;
        self.scroll_offsets[cur] = old_scroll;

        // Update theme/transition from new frontmatter
        if let Some(name) = &new_presentation.meta.theme {
            self.theme = Theme::from_name(name);
        }
        if let Some(name) = &new_presentation.meta.transition {
            self.default_transition = TransitionKind::from_name(name);
        }

        self.presentation = new_presentation;
        self.image_cache.clear();
        self.precache_cancel.store(true, Ordering::Relaxed);
        render::diagram::clear_route_cache();
        render::visualizations::word_cloud::clear_cache();
        self.precache_cancel = Arc::new(AtomicBool::new(false));
        self.transition = None;
        self.pending_nav = None;
        self.on_end_slide = false;
        self.pen_strokes.clear();
        self.arrows.clear();
        self.active_draw = ActiveDraw::None;

        // Clamp grid selection (both the grid and its zoom animation carry one)
        match self.mode {
            AppMode::Grid { ref mut selected }
            | AppMode::OverviewTransition {
                ref mut selected, ..
            } => {
                *selected = (*selected).min(slide_count.saturating_sub(1));
            }
            AppMode::Presentation => {}
        }

        self.toast = Some(Toast::new("Presentation Change Detected".to_string()));

        self.spawn_diagram_precache();
    }

    /// Collect all diagram content from every slide and spawn a background thread
    /// to pre-compute their routing caches at reference resolution (1920x1080).
    fn spawn_diagram_precache(&mut self) {
        let diagrams: Vec<(usize, String)> = self
            .presentation
            .slides
            .iter()
            .enumerate()
            .flat_map(|(i, s)| {
                s.blocks.iter().filter_map(move |b| {
                    if let parser::Block::Diagram { content } = b {
                        Some((i + 1, content.clone()))
                    } else {
                        None
                    }
                })
            })
            .collect();

        if diagrams.is_empty() {
            return;
        }

        let rx = render::diagram::precache_all_diagrams_with_report(
            diagrams,
            self.precache_cancel.clone(),
        );
        self.precache_report_rx = Some(rx);
        self.precache_report_printed = false;
    }

    fn grid_columns(&self) -> usize {
        let count = self.slide_count();
        if count <= 4 {
            2
        } else if count <= 9 {
            3
        } else {
            4
        }
    }

    fn grid_cell_rect(
        &self,
        index: usize,
        rect: egui::Rect,
        scale: f32,
        scroll_offset: f32,
    ) -> egui::Rect {
        let cols = self.grid_columns();
        let count = self.slide_count();
        let rows = count.div_ceil(cols);

        let padding = 24.0 * scale;
        let gap = 12.0 * scale;

        let grid_top = rect.top() + padding + 40.0 * scale;
        let grid_width = rect.width() - padding * 2.0;
        let grid_height = rect.bottom() - grid_top - padding;

        let cell_width = (grid_width - gap * (cols as f32 - 1.0)) / cols as f32;
        let natural_height = cell_width * 9.0 / 16.0;
        let total_natural = rows as f32 * natural_height + (rows as f32 - 1.0) * gap;

        // If natural layout fits in the viewport, clamp to viewport; otherwise use natural size
        let cell_height = if total_natural <= grid_height {
            let cell_height_max = (grid_height - gap * (rows as f32 - 1.0)) / rows as f32;
            cell_height_max.min(natural_height)
        } else {
            natural_height
        };

        let col = index % cols;
        let row = index / cols;
        let x = rect.left() + padding + col as f32 * (cell_width + gap);
        let y = grid_top + row as f32 * (cell_height + gap) - scroll_offset;

        egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(cell_width, cell_height))
    }

    /// Total content height of the grid (for scroll calculation)
    fn grid_content_height(&self, rect: egui::Rect, scale: f32) -> f32 {
        let cols = self.grid_columns();
        let count = self.slide_count();
        let rows = count.div_ceil(cols);

        let padding = 24.0 * scale;
        let gap = 12.0 * scale;
        let grid_width = rect.width() - padding * 2.0;
        let cell_width = (grid_width - gap * (cols as f32 - 1.0)) / cols as f32;
        let cell_height = cell_width * 9.0 / 16.0;

        rows as f32 * cell_height + (rows as f32 - 1.0) * gap
    }

    /// Available viewport height for grid content
    fn grid_available_height(&self, rect: egui::Rect, scale: f32) -> f32 {
        let padding = 24.0 * scale;
        let grid_top = rect.top() + padding + 40.0 * scale;
        rect.bottom() - grid_top - padding
    }

    /// Grid scroll target that brings `index` into view, starting from `current`.
    fn grid_scroll_to_show(&self, index: usize, rect: egui::Rect, scale: f32, current: f32) -> f32 {
        let content_h = self.grid_content_height(rect, scale);
        let available_h = self.grid_available_height(rect, scale);
        let overflow = (content_h - available_h).max(0.0);
        if overflow <= 0.0 {
            return 0.0;
        }
        let padding = 24.0 * scale;
        let grid_top = rect.top() + padding + 40.0 * scale;
        let grid_bottom = rect.bottom() - padding;
        let cell = self.grid_cell_rect(index, rect, scale, current);
        let target = if cell.top() < grid_top {
            current - (grid_top - cell.top() + padding)
        } else if cell.bottom() > grid_bottom {
            current + (cell.bottom() - grid_bottom + padding)
        } else {
            current
        };
        target.clamp(0.0, overflow)
    }

    fn compute_scale(rect: egui::Rect) -> f32 {
        let ref_w = 1920.0;
        let ref_h = 1080.0;
        (rect.width() / ref_w).min(rect.height() / ref_h)
    }

    /// Convert screen position to slide-local coordinates (accounting for scroll)
    fn screen_to_local(&self, screen_pos: egui::Pos2) -> egui::Pos2 {
        let rect = self.last_slide_rect;
        let scroll = self.scroll_offsets[self.current_slide];
        egui::pos2(
            screen_pos.x - rect.left(),
            screen_pos.y - rect.top() + scroll,
        )
    }

    /// Convert slide-local coordinates back to screen position
    fn local_to_screen(&self, local: egui::Pos2) -> egui::Pos2 {
        let rect = self.last_slide_rect;
        let scroll = self.scroll_offsets[self.current_slide];
        egui::pos2(local.x + rect.left(), local.y + rect.top() - scroll)
    }
}

/// Re-export `lerp_rect` so the drawing module can use it.
use helpers::lerp_rect;

impl PresentationApp {
    /// Drive the monitor-hop state machine one frame. Returns viewport
    /// commands to send after input handling.
    fn tick_monitor_move(&mut self, vp: &ViewportSnapshot) -> Vec<egui::ViewportCommand> {
        let mut cmds = Vec::new();
        let Some(mv) = self.monitor_move.as_mut() else {
            return cmds;
        };
        match mv.phase {
            MonitorMovePhase::Reposition => {
                cmds.push(egui::ViewportCommand::Fullscreen(true));
                mv.phase = MonitorMovePhase::Verify(Instant::now());
            }
            MonitorMovePhase::Verify(since) => {
                if since.elapsed() < MONITOR_MOVE_SETTLE {
                    return cmds;
                }
                let Some(actual) = vp.outer_pos else {
                    self.monitor_move = None;
                    return cmds;
                };
                match evaluate_monitor_move(mv.target.x, actual.x, mv.monitor_width, mv.wrapped) {
                    MonitorMoveOutcome::Landed => {
                        // Remember the real monitor origin for the next launch
                        if let Ok(mut config) = Config::load() {
                            let defaults = config.defaults.get_or_insert_with(Default::default);
                            defaults.monitor_position = Some([actual.x, actual.y]);
                            let _ = config.save();
                        }
                        self.monitor_move = None;
                    }
                    MonitorMoveOutcome::Wrap => {
                        mv.target = egui::pos2(0.0, 0.0);
                        mv.wrapped = true;
                        mv.phase = MonitorMovePhase::Reposition;
                        cmds.push(egui::ViewportCommand::Fullscreen(false));
                        cmds.push(egui::ViewportCommand::OuterPosition(mv.target));
                        self.toast = Some(Toast::new("Wrapping to first monitor...".to_string()));
                    }
                    MonitorMoveOutcome::Failed => {
                        self.toast = Some(Toast::new("No other monitor found".to_string()));
                        self.monitor_move = None;
                    }
                }
            }
        }
        cmds
    }

    /// Apply a keyboard action. Viewport commands are collected in `cmds` and
    /// sent by the caller (sending inside `ctx.input` would deadlock).
    fn handle_action(
        &mut self,
        action: Action,
        vp: &ViewportSnapshot,
        cmds: &mut Vec<egui::ViewportCommand>,
    ) {
        let now = Instant::now();
        match action {
            Action::Quit => {
                if self.quit_tap.tap(now) {
                    cmds.push(egui::ViewportCommand::Close);
                } else {
                    self.toast = Some(Toast::new("Press Q again to quit".to_string()));
                }
            }
            Action::CtrlC => {
                if self.ctrl_c_tap.tap(now) {
                    cmds.push(egui::ViewportCommand::Close);
                } else {
                    self.toast = Some(Toast::new("Press Ctrl+C again to quit".to_string()));
                }
            }
            Action::Escape => {
                // In presentation mode, first ESC clears annotations if any exist
                if matches!(self.mode, AppMode::Presentation) {
                    let idx = self.current_slide;
                    let has_annotations = self.pen_strokes.iter().any(|s| s.slide_index == idx)
                        || self.arrows.iter().any(|a| a.slide_index == idx);
                    if has_annotations {
                        self.pen_strokes.retain(|s| s.slide_index != idx);
                        self.arrows.retain(|a| a.slide_index != idx);
                        self.esc_tap.reset();
                        return;
                    }
                }
                if self.esc_tap.tap(now) {
                    cmds.push(egui::ViewportCommand::Close);
                } else {
                    self.toast = Some(Toast::new("Press Esc again to exit".to_string()));
                }
            }
            Action::ToggleFullscreen => {
                cmds.push(egui::ViewportCommand::Fullscreen(!vp.fullscreen));
            }
            Action::MoveMonitor => {
                if self.monitor_move.is_some() {
                    return;
                }
                if !vp.fullscreen {
                    self.toast = Some(Toast::new(
                        "Press F for fullscreen before moving monitors".to_string(),
                    ));
                    return;
                }
                let Some(monitor_size) = vp.monitor_size else {
                    self.toast = Some(Toast::new("Monitor layout unknown".to_string()));
                    return;
                };
                // Exit fullscreen, move right by one monitor width, re-enter
                // fullscreen next frame, then verify where we landed.
                let current_pos = vp.outer_pos.unwrap_or(egui::pos2(0.0, 0.0));
                let target = keys::next_monitor_position(current_pos, monitor_size.x);
                cmds.push(egui::ViewportCommand::Fullscreen(false));
                cmds.push(egui::ViewportCommand::OuterPosition(target));
                self.monitor_move = Some(MonitorMove {
                    target,
                    monitor_width: monitor_size.x,
                    wrapped: false,
                    phase: MonitorMovePhase::Reposition,
                });
                self.toast = Some(Toast::new("Moving to next monitor...".to_string()));
            }
            Action::CycleTheme => self.toggle_theme(),
            Action::CycleTransition => self.cycle_transition(),
            Action::ToggleBlackout => self.blackout = !self.blackout,
            Action::Next => self.navigate_forward(),
            Action::Previous => self.navigate_backward(),
            Action::ScrollUp => {
                let idx = self.current_slide;
                self.scroll_targets[idx] = (self.scroll_targets[idx] - 120.0).max(0.0);
            }
            Action::ScrollDown => {
                let idx = self.current_slide;
                // Max will be clamped at render time when we know content height
                self.scroll_targets[idx] += 120.0;
            }
            Action::FirstSlide => self.jump_to_slide(0),
            Action::LastSlide => self.jump_to_slide(self.slide_count().saturating_sub(1)),
            Action::EnterGrid => {
                if self.transition.is_none() {
                    self.on_end_slide = false;
                    self.mode = AppMode::OverviewTransition {
                        selected: self.current_slide,
                        entering: true,
                    };
                    self.overview_transition_start = Some(Instant::now());
                    self.show_hud = false;
                    // Grid scroll is seeded at draw time (needs the viewport rect)
                    self.grid_seed_scroll = true;
                    self.hover_slide = None;
                    self.use_hover = false;
                }
            }
            Action::ToggleHud => self.show_hud = !self.show_hud,
            Action::CycleRawOverlay => {
                self.raw_overlay_side = match self.raw_overlay_side {
                    RawOverlaySide::Off => RawOverlaySide::Left,
                    RawOverlaySide::Left => RawOverlaySide::Right,
                    RawOverlaySide::Right => RawOverlaySide::Off,
                };
            }
            Action::GridRight | Action::GridLeft | Action::GridDown | Action::GridUp => {
                let AppMode::Grid { selected } = self.mode else {
                    return;
                };
                let cols = self.grid_columns();
                let last = self.slide_count().saturating_sub(1);
                let next = match action {
                    Action::GridRight => (selected + 1).min(last),
                    Action::GridLeft => selected.saturating_sub(1),
                    Action::GridDown => (selected + cols).min(last),
                    _ => selected.saturating_sub(cols),
                };
                self.mode = AppMode::Grid { selected: next };
                self.use_hover = false;
            }
            Action::GridSelect => {
                let AppMode::Grid { selected } = self.mode else {
                    return;
                };
                self.use_hover = false;
                self.mode = AppMode::OverviewTransition {
                    selected,
                    entering: false,
                };
                self.overview_transition_start = Some(Instant::now());
            }
        }
    }
}

impl eframe::App for PresentationApp {
    fn ui(&mut self, root_ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = &root_ui.ctx().clone();
        self.update_fps();
        self.preload_upcoming_images(ctx);

        // Detect frame gaps (sleep, occlusion, scheduling) and shift animation
        // timestamps forward. Threshold must exceed the repaint heartbeat:
        //   Linux: 500ms heartbeat → 2s threshold
        //   macOS/Windows: 4s heartbeat → 6s threshold
        // macOS also stops redrawing occluded windows, so a gap is only worth
        // an incident entry when an animation was actually interrupted.
        let now = Instant::now();
        let prev_frame = self.last_frame;
        let frame_delta = now.duration_since(prev_frame);
        self.last_frame = now;
        #[cfg(target_os = "linux")]
        let time_jump_threshold_ms = 2000;
        #[cfg(not(target_os = "linux"))]
        let time_jump_threshold_ms = 6000;
        if frame_delta.as_millis() > time_jump_threshold_ms {
            if self.animation_in_flight(prev_frame) {
                self.incident_log.record(
                    "time_jump",
                    &format!("frame delta {}ms", frame_delta.as_millis()),
                    "Power-state or scheduling gap interrupted an animation; shifting timestamps",
                );
            }
            self.shift_timestamps(frame_delta, now);
        }

        // Publish current slide position for the incident log
        if let Some(shared) = &self.shared_slide {
            shared.store(self.current_slide, Ordering::Relaxed);
        }

        // Check for file changes
        if self.watcher_rx.try_recv().is_ok() {
            // Drain any extra queued events
            while self.watcher_rx.try_recv().is_ok() {}
            self.reload_presentation();
        }

        // Poll for diagram precache report
        if let Some(ref rx) = self.precache_report_rx
            && let Ok(report) = rx.try_recv()
        {
            if report.has_warnings() && !self.quiet && !self.precache_report_printed {
                report.print_brief();
                self.precache_report_printed = true;
            }
            self.precache_report_rx = None;
        }

        let mode = self.mode;
        let key_mode = match mode {
            AppMode::Presentation => KeyMode::Presentation,
            AppMode::Grid { .. } => KeyMode::Grid,
            AppMode::OverviewTransition { .. } => KeyMode::Blocked,
        };

        // Snapshot input inside the closure; act on it outside (sending
        // viewport commands inside ctx.input() deadlocks).
        let (pressed, wheel_y, vp) = ctx.input(|i| {
            let pressed: Vec<(egui::Key, egui::Modifiers)> = i
                .events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => Some((*key, *modifiers)),
                    _ => None,
                })
                .collect();
            let vp = ViewportSnapshot {
                fullscreen: i.viewport().fullscreen.unwrap_or(false),
                monitor_size: i.viewport().monitor_size,
                outer_pos: i.viewport().outer_rect.map(|r| r.left_top()),
            };
            (pressed, i.smooth_scroll_delta.y, vp)
        });

        let mut viewport_cmds = self.tick_monitor_move(&vp);
        if self.monitor_move.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }

        for (key, modifiers) in pressed {
            let Some(action) = map_key(key, modifiers, key_mode) else {
                continue;
            };
            // Block everything but global actions while blacked out
            if self.blackout && !action.is_global() {
                continue;
            }
            self.handle_action(action, &vp, &mut viewport_cmds);
        }

        // Mouse wheel scroll (presentation mode only)
        if wheel_y != 0.0 && matches!(mode, AppMode::Presentation) && !self.blackout {
            let idx = self.current_slide;
            self.scroll_targets[idx] -= wheel_y;
        }

        for cmd in viewport_cmds {
            ctx.send_viewport_cmd(cmd);
        }

        // Mouse input handling (presentation mode only)
        if matches!(mode, AppMode::Presentation) && self.transition.is_none() && !self.blackout {
            self.handle_mouse_input(ctx);
        }

        // Expire old annotations
        self.pen_strokes
            .retain(|s| s.start.elapsed().as_secs_f32() < DRAW_FADE_DURATION);
        self.arrows
            .retain(|a| a.start.elapsed().as_secs_f32() < DRAW_FADE_DURATION);
        if !self.pen_strokes.is_empty() || !self.arrows.is_empty() {
            ctx.request_repaint();
        }

        self.advance_transition();
        self.advance_overview_transition();

        // Expire toast
        if self.toast.as_ref().is_some_and(|t| t.is_expired()) {
            self.toast = None;
        }

        let bg = if self.blackout || self.on_end_slide {
            egui::Color32::BLACK
        } else {
            self.theme.background
        };

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(bg).inner_margin(0.0))
            .show(root_ui, |ui| {
                let rect = ui.max_rect();
                ui.painter().rect_filled(rect, 0.0, bg);

                // Blackout mode: solid black, nothing else rendered
                if self.blackout {
                    return;
                }

                let scale = Self::compute_scale(rect);

                // End slide: "The End" with logo attribution
                if self.on_end_slide {
                    self.draw_end_slide(ui, rect, scale);
                    return;
                }

                // Entering the grid: start with the selected cell in view so the
                // zoom-out lands on a visible cell instead of one below the fold.
                if self.grid_seed_scroll {
                    self.grid_seed_scroll = false;
                    if let AppMode::OverviewTransition { selected, .. } = self.mode {
                        let seed = self.grid_scroll_to_show(selected, rect, scale, 0.0);
                        self.grid_scroll_offset = seed;
                        self.grid_scroll_target = seed;
                    }
                }

                match self.mode {
                    AppMode::Presentation => {
                        self.draw_presentation_with_scroll(ui, ctx, rect, scale);
                    }
                    AppMode::Grid { selected } => {
                        self.draw_grid(ui, ctx, rect, selected, scale);
                    }
                    AppMode::OverviewTransition { selected, entering } => {
                        self.draw_overview_transition(ui, ctx, rect, scale, selected, entering);
                    }
                }

                // Toast notification (shown in both modes)
                if let Some(ref toast) = self.toast {
                    let opacity = toast.opacity();
                    if opacity > 0.0 {
                        let toast_color = Theme::with_opacity(self.theme.foreground, opacity * 0.9);
                        let toast_bg =
                            Theme::with_opacity(self.theme.code_background, opacity * 0.9);
                        let galley = ui.painter().layout_no_wrap(
                            toast.message.clone(),
                            egui::FontId::proportional(20.0 * scale),
                            toast_color,
                        );
                        let padding = 16.0 * scale;
                        let toast_rect = egui::Rect::from_min_size(
                            egui::pos2(
                                rect.center().x - galley.rect.width() / 2.0 - padding,
                                rect.bottom() - 80.0 * scale,
                            ),
                            egui::vec2(
                                galley.rect.width() + padding * 2.0,
                                galley.rect.height() + padding * 2.0,
                            ),
                        );
                        ui.painter().rect_filled(toast_rect, 8.0 * scale, toast_bg);
                        let text_pos =
                            egui::pos2(toast_rect.left() + padding, toast_rect.top() + padding);
                        ui.painter().galley(text_pos, galley, toast_color);
                        ctx.request_repaint();
                    }
                }

                // HUD overlay (presentation mode only)
                if self.show_hud && matches!(self.mode, AppMode::Presentation) {
                    draw_hud(ui, &self.theme, rect, scale);
                }

                // Debug overlay (presentation mode only)
                if self.raw_overlay_side != RawOverlaySide::Off
                    && matches!(self.mode, AppMode::Presentation)
                {
                    let slide = &self.presentation.slides[self.current_slide];
                    let raw = &slide.raw_source;
                    let debug_info = slide.blocks.iter().find_map(|b| {
                        if let parser::Block::Diagram { content } = b {
                            Some(render::diagram::diagram_debug_info(content))
                        } else {
                            None
                        }
                    });
                    draw_raw_markdown_overlay(
                        ui,
                        raw,
                        debug_info.as_deref(),
                        self.raw_overlay_side,
                        &self.theme,
                        rect,
                        scale,
                    );
                }
            });

        // Keep the display pipeline alive with periodic repaints. Without this,
        // eframe enters ControlFlow::Wait when idle, and on Linux the EGL/GLX
        // context can become stale after ~30 s, crashing with EINVAL (os error 22).
        // On Linux we repaint more aggressively (500ms) to prevent power-state idle
        // from disrupting GPU context during battery/screen-share scenarios.
        #[cfg(target_os = "linux")]
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
        #[cfg(not(target_os = "linux"))]
        ctx.request_repaint_after(std::time::Duration::from_secs(4));
    }
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

pub fn run(
    file: PathBuf,
    windowed: bool,
    start_slide: Option<usize>,
    start_overview: bool,
    quiet: bool,
) -> anyhow::Result<()> {
    let file = file.canonicalize().unwrap_or(file);

    // Config defaults: start mode, theme/transition fallbacks, monitor position
    let config = Config::load_or_default();
    let defaults = config.defaults.clone().unwrap_or_default();
    let (cli_initial_slide, cli_initial_overview) =
        resolve_start(start_slide, start_overview, defaults.start_mode.as_deref());

    let icon = load_app_icon().map(std::sync::Arc::new);
    let incident_log = Arc::new(IncidentLog::new(&file.display().to_string()));

    let content = std::fs::read_to_string(&file)?;
    let base_path = file.parent().unwrap_or(std::path::Path::new("."));
    let presentation = parser::parse(&content, base_path);

    if presentation.slides.is_empty() {
        anyhow::bail!("No slides found in {}", file.display());
    }

    // Warn about ungenerated AI images
    if !quiet {
        let ungenerated = presentation
            .slides
            .iter()
            .flat_map(|s| s.blocks.iter())
            .filter(
                |b| matches!(b, parser::Block::Image { path, .. } if path == "image-generation"),
            )
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

    let title = presentation.meta.title.clone().unwrap_or_else(|| {
        format!(
            "mdeck \u{2014} {}",
            file.file_name().unwrap_or_default().to_string_lossy()
        )
    });

    let slide_count = presentation.slides.len();
    let initial_slide = cli_initial_slide.min(slide_count.saturating_sub(1));
    let initial_overview = cli_initial_overview;

    // The slide position is shared with the window so a display error can be
    // logged together with where the presentation was.
    let shared_slide = Arc::new(AtomicUsize::new(initial_slide));

    let viewport = if windowed {
        egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_title(&title)
    } else {
        let vp = egui::ViewportBuilder::default()
            .with_fullscreen(true)
            .with_title(&title);
        // If we have a saved monitor position, set it so the window
        // opens fullscreen on the remembered monitor
        if let Some([x, y]) = defaults.monitor_position {
            vp.with_position(egui::pos2(x, y))
        } else {
            vp
        }
    };

    let viewport = if let Some(ref icon) = icon {
        viewport.with_icon(icon.clone())
    } else {
        viewport
    };

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    let shared = shared_slide.clone();
    let file_clone = file.clone();
    let log_clone = incident_log.clone();
    // winit allows exactly one event loop per process, so there is no point
    // retrying `run_native` after a display error: run once, log, and bail.
    let result = eframe::run_native(
        &title,
        options,
        Box::new(move |cc| {
            let content_hash = hash_content(&content);
            let (watcher_rx, watcher) =
                spawn_file_watcher(&file_clone, cc.egui_ctx.clone(), log_clone.clone())?;
            let mut app = PresentationApp::new(
                file_clone,
                presentation,
                watcher_rx,
                Some(watcher),
                content_hash,
                quiet,
                log_clone,
                &defaults,
            );
            app.current_slide = initial_slide;
            app.shared_slide = Some(shared);
            if initial_overview {
                app.mode = AppMode::Grid {
                    selected: initial_slide,
                };
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
        // Old raw doesn't match any new slide — clamp to old index
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
