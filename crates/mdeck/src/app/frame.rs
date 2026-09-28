//! One frame of the presenting window: keep up with fonts, the file and
//! background work, take input, expire what has run its course, then paint.

use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use eframe::egui;

use super::input::ActiveDraw;
use super::keys::{self, KeyMode, map_key};
use super::overlays::{draw_hud, draw_raw_markdown_overlay};
use super::{
    AppMode, CountdownPhase, DRAW_FADE_DURATION, PresentationApp, REVEAL_IN_FLIGHT_WINDOW,
    RawOverlaySide, ViewportSnapshot,
};
use crate::deck::EngineFrame;
use crate::parser;
use crate::render;
use crate::theme::Theme;

impl eframe::App for PresentationApp {
    fn ui(&mut self, root_ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = &root_ui.ctx().clone();
        self.housekeeping(ctx);
        self.handle_input(ctx);
        self.expire(ctx);

        let bg = if self.blackout || self.on_end_slide() {
            egui::Color32::BLACK
        } else {
            self.theme.background
        };

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(bg).inner_margin(0.0))
            .show(root_ui, |ui| self.paint(ui, ctx, bg));

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

impl PresentationApp {
    /// Whether any time-based animation is currently running. Used to decide
    /// whether a long frame gap (sleep, occlusion) is worth an incident entry.
    fn animation_in_flight(&self, reference: Instant) -> bool {
        self.transition.is_some()
            || self.overview_transition_start.is_some()
            || self.toast.is_some()
            || !self.ink.strokes.is_empty()
            || !self.ink.arrows.is_empty()
            || !matches!(self.ink.active, ActiveDraw::None)
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
        for stroke in &mut self.ink.strokes {
            stroke.start = (stroke.start + jump).min(now);
        }
        for arrow in &mut self.ink.arrows {
            arrow.start = (arrow.start + jump).min(now);
        }
        if let Some(ref mut t) = self.toast {
            t.start = (t.start + jump).min(now);
        }
        for t in self.views.iter_mut().filter_map(|v| v.revealed_at.as_mut()) {
            *t = (*t + jump).min(now);
        }
    }

    /// Fonts, the frame clock, background results and file changes.
    fn housekeeping(&mut self, ctx: &egui::Context) {
        // Theme font files registered since last frame become drawable one
        // frame after they are installed; a waiting theme switches then.
        self.font_sync.sync(ctx);
        if self.font_sync.ready()
            && let Some(theme) = self.pending_theme.take()
        {
            self.apply_theme(theme);
        } else if self.pending_theme.is_some() {
            ctx.request_repaint();
        }
        self.fps.tick();
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

        self.poll_story();
        self.poll_art();

        // Check for file changes
        if self.watcher_rx.try_recv().is_ok() {
            // Drain any extra queued events
            while self.watcher_rx.try_recv().is_ok() {}
            self.reload_presentation();
        }

        // Poll for diagram precache report
        if let Some(ref rx) = self.jobs.precache_report
            && let Ok(report) = rx.try_recv()
        {
            if report.has_warnings() && !self.quiet && !self.jobs.report_printed {
                report.print_brief();
                self.jobs.report_printed = true;
            }
            self.jobs.precache_report = None;
        }
    }

    /// Keys, the wheel and the mouse.
    fn handle_input(&mut self, ctx: &egui::Context) {
        let mode = self.mode;
        let key_mode = match mode {
            AppMode::Presentation { .. } => KeyMode::Presentation,
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

        // The opening countdown ends on its own, or on any key or click.
        if let Some(cd) = &mut self.countdown {
            cd.start.get_or_insert_with(Instant::now);
            let clicked = ctx.input(|i| i.pointer.any_pressed());
            if cd.phase(Instant::now()) == CountdownPhase::Done || !pressed.is_empty() || clicked {
                self.countdown = None;
            }
            ctx.request_repaint();
        }
        // keys pressed during the countdown only cancel it
        let pressed = if self.countdown_running() {
            Vec::new()
        } else {
            pressed
        };

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
        if wheel_y != 0.0 && matches!(mode, AppMode::Presentation { .. }) && !self.blackout {
            let idx = self.current_slide;
            self.views[idx].scroll_target -= wheel_y;
        }

        for cmd in viewport_cmds {
            ctx.send_viewport_cmd(cmd);
        }

        // Mouse input handling (presentation mode only)
        if matches!(mode, AppMode::Presentation { .. })
            && self.transition.is_none()
            && !self.blackout
        {
            self.handle_mouse_input(ctx);
        }
    }

    /// Drop annotations and toasts that have faded, and finish animations.
    fn expire(&mut self, ctx: &egui::Context) {
        // Expire old annotations
        self.ink
            .strokes
            .retain(|s| s.start.elapsed().as_secs_f32() < DRAW_FADE_DURATION);
        self.ink
            .arrows
            .retain(|a| a.start.elapsed().as_secs_f32() < DRAW_FADE_DURATION);
        if !self.ink.strokes.is_empty() || !self.ink.arrows.is_empty() {
            ctx.request_repaint();
        }

        self.advance_transition();
        self.advance_overview_transition();

        // Expire toast
        if self.toast.as_ref().is_some_and(|t| t.is_expired()) {
            self.toast = None;
        }
    }

    fn paint(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg: egui::Color32) {
        let rect = ui.max_rect();
        ui.painter().rect_filled(rect, 0.0, bg);

        // Blackout mode: solid black, nothing else rendered
        if self.blackout {
            return;
        }

        // A theme's page puts the slide on a sheet.
        let rect = if self.on_end_slide() {
            rect
        } else {
            render::page::draw(ui.painter(), rect, &self.theme, Self::compute_scale(rect))
        };
        let scale = Self::compute_scale(rect);

        // The engine's layer goes under everything.
        if self.theme.engine.paints() && matches!(self.mode, AppMode::Presentation { .. }) {
            self.paint_engine_layer(ui, ctx, rect, scale);
        }

        // A plain countdown: numerals on the bare background, no slide yet.
        if self.countdown_running() && !self.theme.engine.capabilities().countdown {
            self.draw_countdown_numeral(ui, rect, scale);
            return;
        }

        // End slide: "The End" with logo attribution
        if self.on_end_slide() {
            self.draw_end_slide(ui, rect, scale);
            return;
        }

        // Entering the grid: start with the selected cell in view so the
        // zoom-out lands on a visible cell instead of one below the fold.
        if self.grid.seed_scroll {
            self.grid.seed_scroll = false;
            if let AppMode::OverviewTransition { selected, .. } = self.mode {
                let seed = self.grid_layout(rect, scale).scroll_to_show(selected, 0.0);
                self.grid.scroll = seed;
                self.grid.scroll_target = seed;
            }
        }

        match self.mode {
            AppMode::Presentation { .. } => {
                self.draw_presentation_with_scroll(ui, ctx, rect, scale);
            }
            AppMode::Grid { selected } => {
                self.draw_grid(ui, ctx, rect, selected, scale);
            }
            AppMode::OverviewTransition { selected, entering } => {
                self.draw_overview_transition(ui, ctx, rect, scale, (selected, entering));
            }
        }

        self.draw_toast(ui, ctx, rect, scale);

        // HUD overlay (presentation mode only)
        if self.show_hud && matches!(self.mode, AppMode::Presentation { .. }) {
            draw_hud(ui, &self.theme, rect, scale);
        }

        // Debug overlay (presentation mode only)
        if self.raw_overlay_side != RawOverlaySide::Off
            && matches!(self.mode, AppMode::Presentation { .. })
        {
            self.draw_debug_overlay(ui, rect, scale);
        }
    }

    fn paint_engine_layer(
        &mut self,
        ui: &egui::Ui,
        ctx: &egui::Context,
        rect: egui::Rect,
        scale: f32,
    ) {
        let target = self
            .transition
            .as_ref()
            .map(|t| t.to)
            .unwrap_or(self.current_slide);
        let countdown = self
            .countdown
            .as_ref()
            .and_then(|cd| match cd.phase(Instant::now()) {
                CountdownPhase::Digit(d, p) => Some((crate::engines::CountPhase::Digit(d), p)),
                CountdownPhase::Burst(p) => Some((crate::engines::CountPhase::Burst, p)),
                CountdownPhase::Done => None,
            });
        self.deck.art.repaint_on(ctx);
        self.deck.engine_layer(
            ui,
            &self.theme,
            EngineFrame {
                rect,
                scale,
                index: target,
                reveal: self.view(target).reveal,
                end: self.on_end_slide(),
                countdown,
                still: false,
            },
            None,
        );
    }

    /// The toast notification (shown in every mode).
    fn draw_toast(&self, ui: &egui::Ui, ctx: &egui::Context, rect: egui::Rect, scale: f32) {
        if let Some(ref toast) = self.toast {
            let opacity = toast.opacity();
            if opacity > 0.0 {
                let toast_color = Theme::with_opacity(self.theme.foreground, opacity * 0.9);
                let toast_bg = Theme::with_opacity(self.theme.code_background, opacity * 0.9);
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
                let text_pos = egui::pos2(toast_rect.left() + padding, toast_rect.top() + padding);
                ui.painter().galley(text_pos, galley, toast_color);
                ctx.request_repaint();
            }
        }
    }

    /// The raw markdown of the current slide, with diagram debug info.
    fn draw_debug_overlay(&self, ui: &egui::Ui, rect: egui::Rect, scale: f32) {
        let slide = &self.deck.presentation.slides[self.current_slide];
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
}
