//! What each keyboard action does, and the monitor hop it can start.

use std::time::{Duration, Instant};

use eframe::egui;

use crate::config::Config;

use super::grid::GridLayout;
use super::keys::{self, Action, DoubleTap, MonitorMoveOutcome, evaluate_monitor_move};
use super::toast::Toast;
use super::{AppMode, DOUBLE_TAP_WINDOW, PresentationApp, RawOverlaySide};

/// State machine for hopping a fullscreen window to the next monitor.
pub(super) struct MonitorMove {
    /// Requested window position.
    pub(super) target: egui::Pos2,
    pub(super) monitor_width: f32,
    /// Whether we already wrapped around to the origin.
    pub(super) wrapped: bool,
    pub(super) phase: MonitorMovePhase,
}

pub(super) enum MonitorMovePhase {
    /// Fullscreen was dropped and the window repositioned; re-enter fullscreen next frame.
    Reposition,
    /// Fullscreen re-entered at this instant; verify where the window landed after settling.
    Verify(Instant),
}

/// Viewport facts captured inside `ctx.input` for use outside the closure.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct ViewportSnapshot {
    pub(super) fullscreen: bool,
    pub(super) monitor_size: Option<egui::Vec2>,
    pub(super) outer_pos: Option<egui::Pos2>,
}

/// How long to wait for the window to settle after a monitor move.
const MONITOR_MOVE_SETTLE: Duration = Duration::from_millis(1000);

/// The double-tap quit keys: a second press within the window quits.
pub(super) struct QuitTaps {
    pub(super) ctrl_c: DoubleTap,
    pub(super) esc: DoubleTap,
    pub(super) quit: DoubleTap,
}

impl Default for QuitTaps {
    fn default() -> Self {
        Self {
            ctrl_c: DoubleTap::new(DOUBLE_TAP_WINDOW),
            esc: DoubleTap::new(DOUBLE_TAP_WINDOW),
            quit: DoubleTap::new(DOUBLE_TAP_WINDOW),
        }
    }
}

impl PresentationApp {
    /// Drive the monitor-hop state machine one frame. Returns viewport
    /// commands to send after input handling.
    pub(super) fn tick_monitor_move(
        &mut self,
        vp: &ViewportSnapshot,
    ) -> Vec<egui::ViewportCommand> {
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
    pub(super) fn handle_action(
        &mut self,
        action: Action,
        vp: &ViewportSnapshot,
        cmds: &mut Vec<egui::ViewportCommand>,
    ) {
        match action {
            Action::Quit | Action::CtrlC | Action::Escape => self.quit_tap_action(action, cmds),
            Action::ToggleFullscreen => {
                cmds.push(egui::ViewportCommand::Fullscreen(!vp.fullscreen));
            }
            Action::MoveMonitor => self.start_monitor_move(vp, cmds),
            Action::CycleTheme => self.toggle_theme(),
            Action::CycleTransition => self.cycle_transition(),
            Action::ToggleBlackout => self.blackout = !self.blackout,
            Action::Next => self.navigate_forward(),
            Action::Previous => self.navigate_backward(),
            Action::ScrollUp => {
                let view = &mut self.views[self.current_slide];
                view.scroll_target = (view.scroll_target - 120.0).max(0.0);
            }
            Action::ScrollDown => {
                // Max will be clamped at render time when we know content height
                self.views[self.current_slide].scroll_target += 120.0;
            }
            Action::FirstSlide => self.jump_to_slide(0),
            Action::LastSlide => self.jump_to_slide(self.slide_count().saturating_sub(1)),
            Action::EnterGrid => self.enter_grid(),
            Action::ToggleHud => self.show_hud = !self.show_hud,
            Action::Generate => self.generate(),
            Action::CyclePalette => self.cycle_palette(),
            Action::ResetPalette => self.reset_palette(),
            Action::CycleRawOverlay => {
                self.raw_overlay_side = match self.raw_overlay_side {
                    RawOverlaySide::Off => RawOverlaySide::Left,
                    RawOverlaySide::Left => RawOverlaySide::Right,
                    RawOverlaySide::Right => RawOverlaySide::Off,
                };
            }
            Action::GridRight | Action::GridLeft | Action::GridDown | Action::GridUp => {
                self.move_grid_selection(action);
            }
            Action::GridSelect => self.leave_grid(),
        }
    }

    /// Q, Ctrl+C and Esc quit on a second press within the double-tap
    /// window. Esc first clears the current slide's annotations, if any.
    fn quit_tap_action(&mut self, action: Action, cmds: &mut Vec<egui::ViewportCommand>) {
        let now = Instant::now();
        let (tap, again) = match action {
            Action::Quit => (&mut self.taps.quit, "Press Q again to quit"),
            Action::CtrlC => (&mut self.taps.ctrl_c, "Press Ctrl+C again to quit"),
            _ => {
                if matches!(self.mode, AppMode::Presentation { .. }) && self.clear_annotations() {
                    self.taps.esc.reset();
                    return;
                }
                (&mut self.taps.esc, "Press Esc again to exit")
            }
        };
        if tap.tap(now) {
            cmds.push(egui::ViewportCommand::Close);
        } else {
            self.toast = Some(Toast::new(again.to_string()));
        }
    }

    /// Remove the current slide's pen strokes and arrows. Returns whether
    /// there were any.
    fn clear_annotations(&mut self) -> bool {
        let idx = self.current_slide;
        let had = self.ink.strokes.iter().any(|s| s.slide_index == idx)
            || self.ink.arrows.iter().any(|a| a.slide_index == idx);
        self.ink.strokes.retain(|s| s.slide_index != idx);
        self.ink.arrows.retain(|a| a.slide_index != idx);
        had
    }

    /// `M`: leave fullscreen, move right by one monitor width, re-enter
    /// fullscreen next frame, then verify where the window landed.
    fn start_monitor_move(&mut self, vp: &ViewportSnapshot, cmds: &mut Vec<egui::ViewportCommand>) {
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

    /// `G`: zoom out to the grid with the current slide selected.
    fn enter_grid(&mut self) {
        if self.transition.is_some() {
            return;
        }
        self.mode = AppMode::OverviewTransition {
            selected: self.current_slide,
            entering: true,
        };
        self.overview_transition_start = Some(Instant::now());
        self.show_hud = false;
        // Grid scroll is seeded at draw time (needs the viewport rect)
        self.grid.seed_scroll = true;
        self.grid.hover = None;
        self.grid.use_hover = false;
    }

    /// Arrow keys in the grid.
    fn move_grid_selection(&mut self, action: Action) {
        let AppMode::Grid { selected } = self.mode else {
            return;
        };
        let cols = GridLayout::columns(self.slide_count());
        let last = self.slide_count().saturating_sub(1);
        let next = match action {
            Action::GridRight => (selected + 1).min(last),
            Action::GridLeft => selected.saturating_sub(1),
            Action::GridDown => (selected + cols).min(last),
            _ => selected.saturating_sub(cols),
        };
        self.mode = AppMode::Grid { selected: next };
        self.grid.use_hover = false;
    }

    /// Enter in the grid: zoom back in to the selected slide.
    fn leave_grid(&mut self) {
        let AppMode::Grid { selected } = self.mode else {
            return;
        };
        self.grid.use_hover = false;
        self.mode = AppMode::OverviewTransition {
            selected,
            entering: false,
        };
        self.overview_transition_start = Some(Instant::now());
    }
}
