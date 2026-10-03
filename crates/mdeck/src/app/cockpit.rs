//! The presenter view in the window: opening it on another display or
//! beside the slides on the same one, keys typed into the presenter window,
//! the slide-jump number and the timer.

use std::time::{Duration, Instant};

use eframe::egui;

use super::keys::{Action, JumpKey, KeyMode, map_key};
use super::presenter::{self, Placing, Presenter, Shared, Stage, place};
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

    /// `V`: close the presenter window if it is open; otherwise open it on
    /// another display, or beside the slides when there is only one.
    pub(super) fn toggle_presenter(&mut self, vp: &ViewportSnapshot) {
        if self.presenter.open() {
            self.presenter.close();
            return;
        }
        self.open_presenter(vp);
    }

    fn open_presenter(&mut self, vp: &ViewportSnapshot) {
        let displays = place::displays();
        let own = vp.outer_pos.and_then(|p| place::display_of(&displays, p));
        if let Some(own) = own
            && let Some(other) = place::presenter_display(&displays, own)
        {
            self.presenter.window = true;
            self.presenter.target = (
                displays[other].rect.left_top() + egui::vec2(40.0, 40.0),
                egui::vec2(1280.0, 800.0),
            );
            self.presenter.placing = Some(Placing {
                since: Instant::now(),
                own: displays[own].rect,
                slides_fullscreen: vp.fullscreen,
            });
            return;
        }
        // One display: the slides make room and the presenter goes beside.
        let display = own.map(|i| displays[i].rect).or_else(|| {
            vp.monitor_size
                .map(|size| egui::Rect::from_min_size(egui::Pos2::ZERO, size))
        });
        let Some(display) = display else {
            self.toast = Some(Toast::new("Display layout unknown".to_string()));
            return;
        };
        self.share_display(display, vp.fullscreen);
    }

    /// Put both windows on `display`, the slides leaving fullscreen first.
    fn share_display(&mut self, display: egui::Rect, fullscreen: bool) {
        self.presenter.shared = Some(Shared {
            display,
            was_fullscreen: fullscreen,
            stage: Stage::Leave,
        });
    }

    /// `--presenter`: open on the first frame that knows the display, once
    /// the deck shows.
    pub(super) fn open_presenter_at_start(&mut self, vp: &ViewportSnapshot) {
        if !self.presenter.open_at_start || !self.opening.shown() {
            return;
        }
        self.presenter.start_frames += 1;
        if vp.monitor_size.is_some() || self.presenter.start_frames > 30 {
            self.presenter.open_at_start = false;
            self.open_presenter(vp);
        }
    }

    /// The slides window's part in the presenter's arrangement, each frame:
    /// back to fullscreen after a shared display, or leave fullscreen and
    /// make room for the presenter window beside it.
    pub(super) fn tick_presenter(
        &mut self,
        vp: &ViewportSnapshot,
        cmds: &mut Vec<egui::ViewportCommand>,
    ) {
        if std::mem::take(&mut self.presenter.restore_fullscreen) {
            cmds.push(egui::ViewportCommand::Fullscreen(true));
        }
        let Some(shared) = self.presenter.shared.as_mut() else {
            return;
        };
        let arrange = match shared.stage {
            Stage::Leave if vp.fullscreen => {
                cmds.push(egui::ViewportCommand::Fullscreen(false));
                shared.stage = Stage::Leaving(Instant::now());
                false
            }
            Stage::Leave => true,
            Stage::Leaving(since) => !vp.fullscreen && since.elapsed() >= Presenter::LEAVE,
            Stage::Arranged => false,
        };
        if !arrange {
            return;
        }
        shared.stage = Stage::Arranged;
        let layout = place::side_by_side(shared.display);
        cmds.push(egui::ViewportCommand::OuterPosition(layout.slides.0));
        cmds.push(egui::ViewportCommand::InnerSize(layout.slides.1));
        self.presenter.target = layout.presenter;
        self.presenter.window = true;
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
        let (pos, size) = self.presenter.target;
        let builder = egui::ViewportBuilder::default()
            .with_title(format!("{} (presenter)", self.display_title()))
            .with_inner_size(size)
            .with_position(pos);
        ctx.show_viewport_immediate(presenter_id(), builder, |ui, _class| {
            self.presenter_frame(ui);
        });
    }

    /// One frame of the presenter window.
    fn presenter_frame(&mut self, ui: &mut egui::Ui) {
        let pctx = ui.ctx().clone();
        if pctx.input(|i| i.viewport().close_requested()) {
            self.presenter.close();
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

    /// Once a presenter window sent to another display has settled, keep
    /// it there (filling that display) if the system let it go; otherwise
    /// close it and put both windows on the slides' display. Returns
    /// whether the window stays.
    fn settle_presenter(&mut self, pctx: &egui::Context) -> bool {
        let Some(p) = self.presenter.placing else {
            return true;
        };
        if p.since.elapsed() < Presenter::SETTLE {
            pctx.request_repaint_after(Duration::from_millis(100));
            return true;
        }
        self.presenter.placing = None;
        let landed = pctx.input(|i| i.viewport().outer_rect.map(|r| r.left_top()));
        if landed.is_some_and(|at| !p.own.contains(at + egui::vec2(8.0, 8.0))) {
            pctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
            return true;
        }
        self.presenter.window = false;
        self.share_display(p.own, p.slides_fullscreen);
        pctx.request_repaint_of(egui::ViewportId::ROOT);
        false
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
