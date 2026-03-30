mod drawing;
mod helpers;
mod input;

use drawing::{draw_hud, draw_raw_markdown_overlay};
use helpers::{
    find_matching_slide, hash_content, load_app_icon, print_incident_summary, spawn_file_watcher,
};

use eframe::egui;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Instant;

use notify_debouncer_mini::{Debouncer, notify};

use crate::check::CheckReport;
use crate::config::Config;
use crate::incident_log::IncidentLog;
use crate::parser::{self, Presentation};
use crate::render;
use crate::render::image_cache::ImageCache;
use crate::render::transition::{ActiveTransition, TransitionDirection, TransitionKind};
use crate::theme::Theme;

const OVERVIEW_TRANSITION_DURATION: f32 = 0.4;
const DRAW_FADE_DURATION: f32 = 8.0;
const DRAG_THRESHOLD: f32 = 5.0;

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
    _watcher: Debouncer<notify::RecommendedWatcher>,
    mode: AppMode,
    theme: Theme,
    default_transition: TransitionKind,
    transition: Option<ActiveTransition>,
    image_cache: ImageCache,
    show_hud: bool,
    raw_overlay_side: RawOverlaySide,
    toast: Option<Toast>,
    last_ctrl_c: Option<Instant>,
    last_esc: Option<Instant>,
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
    /// Whether the screen is blacked out (toggled with `.` key).
    blackout: bool,
    /// Pending re-enter fullscreen after monitor move (delayed one frame).
    pending_fullscreen: bool,
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
        windowed: bool,
        watcher_rx: mpsc::Receiver<()>,
        watcher: Debouncer<notify::RecommendedWatcher>,
        content_hash: u64,
        quiet: bool,
        incident_log: Arc<IncidentLog>,
    ) -> Self {
        let _ = windowed; // used at window creation time

        let theme_name = presentation.meta.theme.as_deref().unwrap_or("light");
        let theme = Theme::from_name(theme_name);

        let transition_name = presentation.meta.transition.as_deref().unwrap_or("slide");
        let default_transition = TransitionKind::from_name(transition_name);

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
            last_ctrl_c: None,
            last_esc: None,
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
            pending_fullscreen: false,
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

    fn navigate_forward(&mut self) {
        if self.transition.is_some() {
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
            return;
        }

        // On last real slide — transition to end slide
        if idx >= self.slide_count().saturating_sub(1) {
            self.scroll_offsets[idx] = 0.0;
            self.scroll_targets[idx] = 0.0;
            self.on_end_slide = true;
            return;
        }

        self.scroll_offsets[idx] = 0.0;
        self.scroll_targets[idx] = 0.0;
        self.transition = Some(ActiveTransition::new(
            idx,
            idx + 1,
            self.default_transition,
            TransitionDirection::Forward,
        ));
    }

    fn navigate_backward(&mut self) {
        if self.transition.is_some() {
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

        self.scroll_offsets[idx] = 0.0;
        self.scroll_targets[idx] = 0.0;
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

    fn jump_to_slide(&mut self, index: usize) {
        if index < self.slide_count() && self.transition.is_none() {
            let cur = self.current_slide;
            self.scroll_offsets[cur] = 0.0;
            self.scroll_targets[cur] = 0.0;
            self.current_slide = index;
            self.on_end_slide = false;
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

        // Preserve slide position
        let old_raw = self
            .presentation
            .slides
            .get(self.current_slide)
            .map(|s| s.raw_source.as_str());
        self.current_slide =
            find_matching_slide(old_raw, self.current_slide, &new_presentation.slides);

        let slide_count = new_presentation.slides.len();

        // Recompute per-slide vectors
        self.max_steps = new_presentation
            .slides
            .iter()
            .map(|s| parser::compute_max_steps(&s.blocks))
            .collect();
        self.reveal_steps = vec![0; slide_count];
        self.reveal_timestamps = vec![None; slide_count];
        self.scroll_offsets = vec![0.0; slide_count];
        self.scroll_targets = vec![0.0; slide_count];

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
        self.on_end_slide = false;
        self.pen_strokes.clear();
        self.arrows.clear();
        self.active_draw = ActiveDraw::None;

        // Clamp grid selection if in overview mode
        if let AppMode::Grid { ref mut selected } = self.mode {
            *selected = (*selected).min(slide_count.saturating_sub(1));
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

impl eframe::App for PresentationApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle pending fullscreen after monitor move (delayed one frame
        // to allow the window position to take effect first)
        if self.pending_fullscreen {
            self.pending_fullscreen = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
        }

        self.update_fps();

        // Detect power-state time jumps and shift animation timestamps forward.
        // Threshold must exceed the platform's repaint heartbeat interval:
        //   Linux: 500ms heartbeat → 2s threshold
        //   macOS/Windows: 4s heartbeat → 6s threshold
        let now = Instant::now();
        let frame_delta = now.duration_since(self.last_frame);
        self.last_frame = now;
        #[cfg(target_os = "linux")]
        let time_jump_threshold_ms = 2000;
        #[cfg(not(target_os = "linux"))]
        let time_jump_threshold_ms = 6000;
        if frame_delta.as_millis() > time_jump_threshold_ms {
            let jump = frame_delta;
            self.incident_log.record(
                "time_jump",
                &format!("frame delta {}ms", jump.as_millis()),
                "Power-state or scheduling gap detected; shifting animation timestamps",
            );

            // Shift all in-flight animation timestamps forward by the jump amount
            // so they resume smoothly instead of snapping to completion.
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

        // Publish current slide position for recovery after display errors
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
        if let Some(ref rx) = self.precache_report_rx {
            if let Ok(report) = rx.try_recv() {
                if report.has_warnings() && !self.quiet && !self.precache_report_printed {
                    report.print_brief();
                    self.precache_report_printed = true;
                }
                self.precache_report_rx = None;
            }
        }

        let mode = self.mode;

        // Collect viewport commands to send AFTER the input closure
        // (sending inside ctx.input() causes RwLock deadlock)
        let mut viewport_cmds: Vec<egui::ViewportCommand> = Vec::new();

        // Handle keyboard input
        ctx.input(|i| {
            // Quit: Q from any mode
            if i.key_pressed(egui::Key::Q) {
                viewport_cmds.push(egui::ViewportCommand::Close);
                return;
            }

            // Ctrl+C double-tap to quit
            if i.modifiers.ctrl && i.key_pressed(egui::Key::C) {
                if let Some(last) = self.last_ctrl_c {
                    if last.elapsed().as_secs_f32() < 1.0 {
                        viewport_cmds.push(egui::ViewportCommand::Close);
                        return;
                    }
                }
                self.last_ctrl_c = Some(Instant::now());
                self.toast = Some(Toast::new("Press Ctrl+C again to quit".to_string()));
                return;
            }

            // ESC: clear drawings first (presentation mode), then double-tap to quit
            if i.key_pressed(egui::Key::Escape) {
                // In presentation mode, first ESC clears annotations if any exist
                if matches!(mode, AppMode::Presentation) {
                    let idx = self.current_slide;
                    let has_annotations = self.pen_strokes.iter().any(|s| s.slide_index == idx)
                        || self.arrows.iter().any(|a| a.slide_index == idx);
                    if has_annotations {
                        self.pen_strokes.retain(|s| s.slide_index != idx);
                        self.arrows.retain(|a| a.slide_index != idx);
                        self.last_esc = None;
                        return;
                    }
                }
                // Double-tap to quit (from any mode)
                if let Some(last) = self.last_esc {
                    if last.elapsed().as_secs_f32() < 1.0 {
                        viewport_cmds.push(egui::ViewportCommand::Close);
                        return;
                    }
                }
                self.last_esc = Some(Instant::now());
                self.toast = Some(Toast::new("Press Esc again to exit".to_string()));
                return;
            }

            // Fullscreen toggle: F (from any mode)
            if i.key_pressed(egui::Key::F) {
                viewport_cmds.push(egui::ViewportCommand::Fullscreen(
                    !i.viewport().fullscreen.unwrap_or(false),
                ));
                return;
            }

            // Move fullscreen to next monitor: M (from any mode when fullscreen)
            if i.key_pressed(egui::Key::M) {
                if i.viewport().fullscreen.unwrap_or(false) {
                    if let Some(monitor_size) = i.viewport().monitor_size {
                        // Exit fullscreen, move right by monitor width, re-enter fullscreen
                        // This lands the window on the next monitor
                        let current_pos = i
                            .viewport()
                            .outer_rect
                            .map(|r| r.left_top())
                            .unwrap_or(egui::pos2(0.0, 0.0));
                        let next_pos =
                            egui::pos2(current_pos.x + monitor_size.x + 100.0, current_pos.y);
                        viewport_cmds.push(egui::ViewportCommand::Fullscreen(false));
                        viewport_cmds.push(egui::ViewportCommand::OuterPosition(next_pos));
                        // Store the move request — fullscreen will be re-enabled next frame
                        self.pending_fullscreen = true;
                        // Remember this monitor position in config
                        if let Ok(mut config) = crate::config::Config::load() {
                            let defaults = config.defaults.get_or_insert_with(Default::default);
                            defaults.monitor_position = Some([next_pos.x, next_pos.y]);
                            let _ = config.save();
                        }
                        self.toast = Some(Toast::new("Moving to next monitor...".to_string()));
                    }
                }
                return;
            }

            // Cycle theme: Shift+T (from any mode)
            if i.modifiers.shift && i.key_pressed(egui::Key::T) {
                self.toggle_theme();
                return;
            }

            // Cycle transition: T (from any mode)
            if !i.modifiers.shift && i.key_pressed(egui::Key::T) {
                self.cycle_transition();
                return;
            }

            // Blackout toggle: . (period)
            if i.key_pressed(egui::Key::Period) {
                self.blackout = !self.blackout;
                return;
            }

            // Block all other input while blacked out
            if self.blackout {
                return;
            }

            match mode {
                AppMode::Presentation => {
                    // Forward: Right, N, Space
                    if i.key_pressed(egui::Key::ArrowRight)
                        || i.key_pressed(egui::Key::N)
                        || i.key_pressed(egui::Key::Space)
                    {
                        self.navigate_forward();
                    }
                    // Backward: Left, P
                    if i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::P) {
                        self.navigate_backward();
                    }
                    // Toggle HUD: H
                    if i.key_pressed(egui::Key::H) {
                        self.show_hud = !self.show_hud;
                    }
                    // Cycle debug overlay: R (Off → Left → Right → Off)
                    if i.key_pressed(egui::Key::R) {
                        self.raw_overlay_side = match self.raw_overlay_side {
                            RawOverlaySide::Off => RawOverlaySide::Left,
                            RawOverlaySide::Left => RawOverlaySide::Right,
                            RawOverlaySide::Right => RawOverlaySide::Off,
                        };
                    }
                    // Scroll: Up/Down (animate toward target)
                    if i.key_pressed(egui::Key::ArrowUp) {
                        let idx = self.current_slide;
                        self.scroll_targets[idx] = (self.scroll_targets[idx] - 120.0).max(0.0);
                    }
                    if i.key_pressed(egui::Key::ArrowDown) {
                        let idx = self.current_slide;
                        // Max will be clamped at render time when we know content height
                        self.scroll_targets[idx] += 120.0;
                    }
                    // Mouse wheel scroll
                    let scroll = i.smooth_scroll_delta;
                    if scroll.y != 0.0 {
                        let idx = self.current_slide;
                        self.scroll_targets[idx] -= scroll.y;
                    }
                    // Home/End
                    if i.key_pressed(egui::Key::Home) {
                        self.jump_to_slide(0);
                    }
                    if i.key_pressed(egui::Key::End) {
                        self.jump_to_slide(self.slide_count().saturating_sub(1));
                    }
                    // G: animate into grid overview
                    if i.key_pressed(egui::Key::G) && self.transition.is_none() {
                        self.on_end_slide = false;
                        self.mode = AppMode::OverviewTransition {
                            selected: self.current_slide,
                            entering: true,
                        };
                        self.overview_transition_start = Some(Instant::now());
                        self.show_hud = false;
                        self.grid_scroll_offset = 0.0;
                        self.grid_scroll_target = 0.0;
                        self.hover_slide = None;
                        self.use_hover = false;
                    }
                }
                AppMode::Grid { selected } => {
                    let cols = self.grid_columns();
                    let count = self.slide_count();

                    // Arrow navigation in grid
                    if i.key_pressed(egui::Key::ArrowRight) {
                        let next = (selected + 1).min(count.saturating_sub(1));
                        self.mode = AppMode::Grid { selected: next };
                        self.use_hover = false;
                    }
                    if i.key_pressed(egui::Key::ArrowLeft) {
                        let prev = selected.saturating_sub(1);
                        self.mode = AppMode::Grid { selected: prev };
                        self.use_hover = false;
                    }
                    if i.key_pressed(egui::Key::ArrowDown) {
                        let next = (selected + cols).min(count.saturating_sub(1));
                        self.mode = AppMode::Grid { selected: next };
                        self.use_hover = false;
                    }
                    if i.key_pressed(egui::Key::ArrowUp) {
                        let prev = selected.saturating_sub(cols);
                        self.mode = AppMode::Grid { selected: prev };
                        self.use_hover = false;
                    }

                    // Enter / Space / E: animate back to selected slide
                    if i.key_pressed(egui::Key::Enter)
                        || i.key_pressed(egui::Key::Space)
                        || i.key_pressed(egui::Key::E)
                    {
                        self.use_hover = false;
                        self.mode = AppMode::OverviewTransition {
                            selected,
                            entering: false,
                        };
                        self.overview_transition_start = Some(Instant::now());
                    }
                }
                AppMode::OverviewTransition { .. } => {
                    // Block input during overview animation
                }
            }
        });

        // Send collected viewport commands outside the input closure
        for cmd in viewport_cmds {
            ctx.send_viewport_cmd(cmd);
        }

        // Mouse input handling (presentation mode only, outside ctx.input closure)
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

        // Advance transition
        if let Some(ref t) = self.transition {
            if t.is_complete() {
                let to = t.to;
                self.transition = None;
                self.current_slide = to;
            }
        }

        // Complete overview transition
        if let AppMode::OverviewTransition { selected, entering } = self.mode {
            if let Some(start) = self.overview_transition_start {
                if start.elapsed().as_secs_f32() >= OVERVIEW_TRANSITION_DURATION {
                    if entering {
                        self.mode = AppMode::Grid { selected };
                    } else {
                        self.current_slide = selected;
                        self.mode = AppMode::Presentation;
                    }
                    self.overview_transition_start = None;
                }
            }
        }

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
            .show(ctx, |ui| {
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

pub fn run(
    file: PathBuf,
    windowed: bool,
    start_slide: Option<usize>,
    start_overview: bool,
    quiet: bool,
) -> anyhow::Result<()> {
    let file = file.canonicalize().unwrap_or(file);

    // Determine start mode: CLI flags override config
    let config = Config::load_or_default();
    let config_start = config
        .defaults
        .as_ref()
        .and_then(|d| d.start_mode.as_deref())
        .map(String::from);

    let (cli_initial_slide, cli_initial_overview) = if start_overview {
        (start_slide.map(|s| s.saturating_sub(1)).unwrap_or(0), true)
    } else if let Some(s) = start_slide {
        (s.saturating_sub(1), false)
    } else {
        match config_start.as_deref() {
            Some("overview") => (0, true),
            Some("first") | None => (0, false),
            Some(n) => {
                if let Ok(num) = n.parse::<usize>() {
                    (num.saturating_sub(1), false)
                } else {
                    (0, false)
                }
            }
        }
    };

    let icon = load_app_icon().map(std::sync::Arc::new);

    // Shared slide position survives display errors so we can resume
    let shared_slide = Arc::new(AtomicUsize::new(cli_initial_slide));

    let incident_log = Arc::new(IncidentLog::new(&file.display().to_string()));

    const MAX_RETRIES: usize = 5;
    for attempt in 0..=MAX_RETRIES {
        let content = std::fs::read_to_string(&file)?;
        let base_path = file.parent().unwrap_or(std::path::Path::new("."));
        let presentation = parser::parse(&content, base_path);

        if presentation.slides.is_empty() {
            anyhow::bail!("No slides found in {}", file.display());
        }

        // Warn about ungenerated AI images (first attempt only, not on hot-reload)
        if attempt == 0 && !quiet {
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

        let title = presentation.meta.title.clone().unwrap_or_else(|| {
            format!(
                "mdeck \u{2014} {}",
                file.file_name().unwrap_or_default().to_string_lossy()
            )
        });

        let slide_count = presentation.slides.len();

        // On first attempt use CLI args; on retry resume from last known slide
        let (initial_slide, initial_overview) = if attempt == 0 {
            (
                cli_initial_slide.min(slide_count.saturating_sub(1)),
                cli_initial_overview,
            )
        } else {
            (
                shared_slide
                    .load(Ordering::Relaxed)
                    .min(slide_count.saturating_sub(1)),
                false,
            )
        };

        // Check for remembered monitor position from config
        let saved_monitor_pos = crate::config::Config::load()
            .ok()
            .and_then(|c| c.defaults.and_then(|d| d.monitor_position));

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
            if let Some([x, y]) = saved_monitor_pos {
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
                    windowed,
                    watcher_rx,
                    watcher,
                    content_hash,
                    quiet,
                    log_clone,
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
                return Ok(());
            }
            Err(e) if attempt < MAX_RETRIES => {
                let slide = shared_slide.load(Ordering::Relaxed);
                let summary = format!(
                    "eframe display error, restarting (attempt {}/{})",
                    attempt + 1,
                    MAX_RETRIES,
                );
                incident_log.record("display_error", &summary, &format!("{e}\nslide: {slide}"));
                eprintln!(
                    "Display error: {e}. Restarting presentation (attempt {}/{MAX_RETRIES})...",
                    attempt + 1,
                );
                continue;
            }
            Err(e) => {
                let slide = shared_slide.load(Ordering::Relaxed);
                incident_log.record(
                    "display_error_fatal",
                    "all display error retries exhausted",
                    &format!("{e}\nslide: {slide}"),
                );
                print_incident_summary(&incident_log);
                return Err(anyhow::anyhow!("{e}"));
            }
        }
    }

    print_incident_summary(&incident_log);
    Ok(())
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
        let _slides = vec![slide("a"), slide("b"), slide("c")];
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
