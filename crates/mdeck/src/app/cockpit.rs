//! The presenter view in the window: opening it on the other display, the
//! one-display notes overlay, keys typed into the presenter window, the
//! slide-jump number and the timer.

use std::time::{Duration, Instant};

use eframe::egui;

use super::keys::{Action, JumpKey, KeyMode, map_key};
use super::presenter::{self, Placing, Presenter};
use super::toast::Toast;
use super::{AppMode, PresentationApp, ViewportSnapshot};
use crate::theme::Theme;

/// The presenter window's viewport.
fn presenter_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("mdeck-presenter")
}

impl PresentationApp {
    /// A key press in either window: digits and Enter feed the slide jump
    /// first, everything else maps to an action. `from_presenter` keys
    /// cannot move the slide window between monitors.
    pub(super) fn press(
        &mut self,
        key: egui::Key,
        modifiers: egui::Modifiers,
        key_mode: KeyMode,
        vp: &ViewportSnapshot,
        cmds: &mut Vec<egui::ViewportCommand>,
        from_presenter: bool,
    ) {
        if key_mode == KeyMode::Presentation && !self.blackout {
            match self.jump.key(key, modifiers, Instant::now()) {
                JumpKey::Consumed => return,
                JumpKey::Jump(n) => {
                    self.jump_to_number(n);
                    return;
                }
                JumpKey::Pass => {}
            }
        }
        let Some(action) = map_key(key, modifiers, key_mode) else {
            return;
        };
        // Block everything but global actions while blacked out
        if self.blackout && !action.is_global() {
            return;
        }
        if from_presenter && action == Action::MoveMonitor {
            return;
        }
        self.handle_action(action, vp, cmds);
    }

    /// Go to slide `n` (1-based) typed as digits: forward it starts at its
    /// first step, back it shows every step, as stepping there would.
    fn jump_to_number(&mut self, n: usize) {
        let count = self.slide_count();
        if n == 0 || n > count {
            self.toast = Some(Toast::new(format!("No slide {n} (1-{count})")));
            return;
        }
        let from = self.current_slide;
        let target = n - 1;
        if self.transition.is_some() {
            return;
        }
        self.jump_to_slide(target);
        if target > from {
            self.views[target].reveal = 0;
        }
    }

    /// `V`: close the presenter view or the notes overlay if either is
    /// showing; otherwise open the presenter window on the other display
    /// (the overlay takes over when there turns out to be only one).
    pub(super) fn toggle_presenter(&mut self, vp: &ViewportSnapshot) {
        if self.presenter.window || self.presenter.overlay {
            self.presenter.window = false;
            self.presenter.overlay = false;
            self.presenter.placing = None;
            return;
        }
        self.open_presenter(vp);
    }

    fn open_presenter(&mut self, vp: &ViewportSnapshot) {
        let (Some(size), Some(pos)) = (vp.monitor_size, vp.outer_pos) else {
            self.presenter.overlay = true;
            self.toast = Some(Toast::new("Notes overlay (V hides it)".to_string()));
            return;
        };
        let width = size.x.max(1.0);
        // the left edge of the slide window's display, assuming displays
        // side by side and alike
        let main_x = (pos.x / width).floor() * width;
        let target = presenter::presenter_target(egui::pos2(main_x, pos.y), width);
        self.presenter.window = true;
        self.presenter.target = target;
        self.presenter.placing = Some(Placing {
            since: Instant::now(),
            main_x,
            monitor_width: width,
        });
    }

    /// `--presenter`: open on the first frame that knows the display.
    pub(super) fn open_presenter_at_start(&mut self, vp: &ViewportSnapshot) {
        if !self.presenter.open_at_start {
            return;
        }
        self.presenter.start_frames += 1;
        if vp.monitor_size.is_some() || self.presenter.start_frames > 30 {
            self.presenter.open_at_start = false;
            self.open_presenter(vp);
        }
    }

    pub(super) fn reset_timer(&mut self) {
        self.presenter.since = Instant::now();
        self.toast = Some(Toast::new("Timer reset".to_string()));
    }

    /// The slide the presenter is on: the one being moved to during a
    /// transition.
    fn presenter_slide(&self) -> usize {
        self.transition
            .as_ref()
            .map(|t| t.to)
            .unwrap_or(self.current_slide)
    }

    /// Show the presenter window, while it is open.
    pub(super) fn show_presenter(&mut self, ctx: &egui::Context) {
        if !self.presenter.window {
            return;
        }
        ctx.request_repaint_after(Duration::from_millis(500));
        let builder = egui::ViewportBuilder::default()
            .with_title(format!("{} (presenter)", self.display_title()))
            .with_inner_size([1280.0, 800.0])
            .with_position(self.presenter.target);
        ctx.show_viewport_immediate(presenter_id(), builder, |ui, _class| {
            self.presenter_frame(ui);
        });
    }

    /// One frame of the presenter window.
    fn presenter_frame(&mut self, ui: &mut egui::Ui) {
        let pctx = ui.ctx().clone();
        if pctx.input(|i| i.viewport().close_requested()) {
            self.presenter.window = false;
            self.presenter.placing = None;
            return;
        }
        if !self.settle_presenter(&pctx) {
            return;
        }
        self.presenter_keys(&pctx);
        let index = self.presenter_slide();
        let reveal = self.view(index).reveal;
        let end = self.on_end_slide();
        let elapsed = self.presenter.elapsed();
        let mut view = presenter::View {
            deck: &mut self.deck,
            theme: &self.theme,
            engine: Some(&mut self.presenter.engine),
            still: self.reduced_motion,
            index,
            reveal,
            end,
            elapsed,
        };
        egui::CentralPanel::default()
            .frame(egui::Frame::new().inner_margin(0.0))
            .show(ui, |ui| presenter::draw(ui, ui.max_rect(), &mut view));
    }

    /// Once the new window has settled, keep it if it landed on another
    /// display (and fill that display), else close it and show the notes
    /// overlay. Returns whether the window stays.
    fn settle_presenter(&mut self, pctx: &egui::Context) -> bool {
        let Some(p) = self.presenter.placing else {
            return true;
        };
        if p.since.elapsed() < Presenter::SETTLE {
            pctx.request_repaint_after(Duration::from_millis(100));
            return true;
        }
        self.presenter.placing = None;
        let landed = pctx.input(|i| i.viewport().outer_rect.map(|r| r.left()));
        match landed {
            Some(x) if presenter::on_other_display(p.main_x, p.monitor_width, x) => {
                pctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
                true
            }
            _ => {
                self.presenter.window = false;
                self.presenter.overlay = true;
                self.toast = Some(Toast::new(
                    "One display: notes overlay instead (V hides it)".to_string(),
                ));
                false
            }
        }
    }

    /// Keys typed into the presenter window drive the slides as well.
    fn presenter_keys(&mut self, pctx: &egui::Context) {
        let (pressed, vp) = pctx.input(|i| {
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
            (pressed, vp)
        });
        if pressed.is_empty() {
            return;
        }
        if self.countdown_running() {
            self.countdown = None;
            return;
        }
        let key_mode = match self.mode {
            AppMode::Presentation { .. } => KeyMode::Presentation,
            AppMode::Grid { .. } => KeyMode::Grid,
            AppMode::OverviewTransition { .. } => KeyMode::Blocked,
        };
        let mut cmds = Vec::new();
        for (key, modifiers) in pressed {
            self.press(key, modifiers, key_mode, &vp, &mut cmds, true);
        }
        for cmd in cmds {
            if matches!(cmd, egui::ViewportCommand::Close) {
                pctx.send_viewport_cmd_to(egui::ViewportId::ROOT, cmd);
            } else {
                // fullscreen and the like apply to the presenter window
                pctx.send_viewport_cmd(cmd);
            }
        }
        pctx.request_repaint_of(egui::ViewportId::ROOT);
    }

    /// The notes overlay over the slides (one display).
    pub(super) fn draw_notes_overlay(&self, ui: &mut egui::Ui, rect: egui::Rect) {
        if !self.presenter.overlay || !matches!(self.mode, AppMode::Presentation { end: false }) {
            return;
        }
        let notes = presenter::notes_blocks(
            self.deck
                .presentation
                .slides
                .get(self.presenter_slide())
                .and_then(|s| s.notes.as_deref()),
        );
        presenter::draw_overlay(ui, rect, &notes, self.presenter.elapsed());
        ui.ctx().request_repaint_after(Duration::from_millis(500));
    }

    /// The slide number being typed, small in the bottom-left corner.
    pub(super) fn draw_jump(&self, ui: &egui::Ui, rect: egui::Rect, scale: f32) {
        let typed = self.jump.typed();
        if typed.is_empty() {
            return;
        }
        ui.ctx()
            .request_repaint_after(super::keys::SlideJump::TIMEOUT);
        let painter = ui.painter();
        let color = Theme::with_opacity(self.theme.foreground, 0.75);
        let galley = painter.layout_no_wrap(
            format!("Go to {typed}"),
            egui::FontId::new(20.0 * scale, self.theme.mono_family()),
            color,
        );
        let pad = egui::vec2(14.0, 8.0) * scale;
        let pill = egui::Rect::from_min_size(
            egui::pos2(
                rect.left() + 24.0 * scale,
                rect.bottom() - 24.0 * scale - galley.rect.height() - 2.0 * pad.y,
            ),
            galley.rect.size() + 2.0 * pad,
        );
        painter.rect_filled(
            pill,
            pill.height() / 2.0,
            Theme::with_opacity(self.theme.code_background, 0.85),
        );
        painter.galley(pill.min + pad, galley, color);
    }
}
