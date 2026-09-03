use eframe::egui;
use std::time::Instant;

use super::{ActiveDraw, ArrowAnnotation, DRAG_THRESHOLD, PenStroke, PresentationApp};

/// What a finished mouse interaction should do.
#[derive(Debug, PartialEq)]
pub(super) enum ReleaseOutcome {
    NavigateForward,
    NavigateBackward,
    CommitPen(Vec<egui::Pos2>),
    CommitArrow { from: egui::Pos2, to: egui::Pos2 },
    Nothing,
}

/// Decide what to do with an interaction once no button is held any more.
///
/// A click or stroke only counts when a button release was actually observed
/// this frame. If the pointer left the window while pressed and came back
/// without a release event, the stale interaction is dropped instead of
/// firing a navigation or committing a stroke on the first motion.
pub(super) fn release_outcome(active: ActiveDraw, released_this_frame: bool) -> ReleaseOutcome {
    if !released_this_frame {
        return ReleaseOutcome::Nothing;
    }
    match active {
        ActiveDraw::PenPending { .. } => ReleaseOutcome::NavigateForward,
        ActiveDraw::PenDrawing { points } if points.len() >= 2 => ReleaseOutcome::CommitPen(points),
        ActiveDraw::PenDrawing { .. } => ReleaseOutcome::Nothing,
        ActiveDraw::ArrowPending { .. } => ReleaseOutcome::NavigateBackward,
        ActiveDraw::ArrowDrawing { from, current } => {
            ReleaseOutcome::CommitArrow { from, to: current }
        }
        ActiveDraw::None => ReleaseOutcome::Nothing,
    }
}

impl PresentationApp {
    pub(super) fn handle_mouse_input(&mut self, ctx: &egui::Context) {
        let (
            primary_pressed,
            primary_down,
            primary_released,
            secondary_pressed,
            secondary_down,
            secondary_released,
            pointer_pos,
        ) = ctx.input(|i| {
            let p = &i.pointer;
            (
                p.button_pressed(egui::PointerButton::Primary),
                p.button_down(egui::PointerButton::Primary),
                p.button_released(egui::PointerButton::Primary),
                p.button_pressed(egui::PointerButton::Secondary),
                p.button_down(egui::PointerButton::Secondary),
                p.button_released(egui::PointerButton::Secondary),
                p.hover_pos(),
            )
        });

        let Some(pos) = pointer_pos else {
            // Pointer left the window. Once no button is held, whatever was
            // pending can never complete as a click or stroke — drop it.
            if !primary_down && !secondary_down {
                self.active_draw = ActiveDraw::None;
            }
            return;
        };
        let local = self.screen_to_local(pos);

        // Left button press → start PenPending
        if primary_pressed {
            self.active_draw = ActiveDraw::PenPending {
                origin: local,
                points: vec![local],
            };
            return;
        }

        // Right button press → start ArrowPending
        if secondary_pressed {
            self.active_draw = ActiveDraw::ArrowPending {
                origin: local,
                current: local,
            };
            return;
        }

        // Left button held
        if primary_down {
            match &mut self.active_draw {
                ActiveDraw::PenPending { origin, points } => {
                    points.push(local);
                    if origin.distance(local) > DRAG_THRESHOLD {
                        let pts = std::mem::take(points);
                        self.active_draw = ActiveDraw::PenDrawing { points: pts };
                    }
                }
                ActiveDraw::PenDrawing { points } => {
                    points.push(local);
                }
                _ => {}
            }
            ctx.request_repaint();
            return;
        }

        // Right button held
        if secondary_down {
            match &mut self.active_draw {
                ActiveDraw::ArrowPending { origin, current } => {
                    *current = local;
                    if origin.distance(local) > DRAG_THRESHOLD {
                        let from = *origin;
                        self.active_draw = ActiveDraw::ArrowDrawing {
                            from,
                            current: local,
                        };
                    }
                }
                ActiveDraw::ArrowDrawing { current, .. } => {
                    *current = local;
                }
                _ => {}
            }
            ctx.request_repaint();
            return;
        }

        // No button held — commit or navigate only on an observed release
        if matches!(self.active_draw, ActiveDraw::None) {
            return;
        }
        let active = std::mem::replace(&mut self.active_draw, ActiveDraw::None);
        match release_outcome(active, primary_released || secondary_released) {
            ReleaseOutcome::NavigateForward => self.navigate_forward(),
            ReleaseOutcome::NavigateBackward => self.navigate_backward(),
            ReleaseOutcome::CommitPen(points) => self.pen_strokes.push(PenStroke {
                points,
                start: Instant::now(),
                slide_index: self.current_slide,
            }),
            ReleaseOutcome::CommitArrow { from, to } => self.arrows.push(ArrowAnnotation {
                from,
                to,
                start: Instant::now(),
                slide_index: self.current_slide,
            }),
            ReleaseOutcome::Nothing => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f32, y: f32) -> egui::Pos2 {
        egui::pos2(x, y)
    }

    #[test]
    fn click_navigates_only_on_observed_release() {
        let pending = ActiveDraw::PenPending {
            origin: p(0.0, 0.0),
            points: vec![p(0.0, 0.0)],
        };
        assert_eq!(
            release_outcome(pending, true),
            ReleaseOutcome::NavigateForward
        );
        // Regression: pointer left the window while pressed, release was missed,
        // pointer came back → must NOT navigate on the first motion.
        let stale = ActiveDraw::PenPending {
            origin: p(0.0, 0.0),
            points: vec![p(0.0, 0.0)],
        };
        assert_eq!(release_outcome(stale, false), ReleaseOutcome::Nothing);
    }

    #[test]
    fn right_click_navigates_backward() {
        let pending = ActiveDraw::ArrowPending {
            origin: p(1.0, 1.0),
            current: p(1.0, 1.0),
        };
        assert_eq!(
            release_outcome(pending, true),
            ReleaseOutcome::NavigateBackward
        );
    }

    #[test]
    fn strokes_commit_only_on_release() {
        let pts = vec![p(0.0, 0.0), p(10.0, 10.0), p(20.0, 5.0)];
        let drawing = ActiveDraw::PenDrawing {
            points: pts.clone(),
        };
        assert_eq!(
            release_outcome(drawing, true),
            ReleaseOutcome::CommitPen(pts)
        );
        let stale = ActiveDraw::PenDrawing {
            points: vec![p(0.0, 0.0), p(10.0, 10.0)],
        };
        assert_eq!(release_outcome(stale, false), ReleaseOutcome::Nothing);
        // A one-point "stroke" is not a stroke
        let tiny = ActiveDraw::PenDrawing {
            points: vec![p(0.0, 0.0)],
        };
        assert_eq!(release_outcome(tiny, true), ReleaseOutcome::Nothing);
    }

    #[test]
    fn arrows_commit_with_endpoints() {
        let drawing = ActiveDraw::ArrowDrawing {
            from: p(0.0, 0.0),
            current: p(50.0, 20.0),
        };
        assert_eq!(
            release_outcome(drawing, true),
            ReleaseOutcome::CommitArrow {
                from: p(0.0, 0.0),
                to: p(50.0, 20.0)
            }
        );
    }

    #[test]
    fn idle_state_does_nothing() {
        assert_eq!(
            release_outcome(ActiveDraw::None, true),
            ReleaseOutcome::Nothing
        );
        assert_eq!(
            release_outcome(ActiveDraw::None, false),
            ReleaseOutcome::Nothing
        );
    }
}
