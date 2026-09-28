use eframe::egui;
use std::time::Instant;

use super::{DRAG_THRESHOLD, PresentationApp};

/// A freehand pen stroke (left-drag)
pub(super) struct PenStroke {
    pub(super) points: Vec<egui::Pos2>,
    pub(super) start: Instant,
    pub(super) slide_index: usize,
}

/// An arrow annotation (right-drag)
pub(super) struct ArrowAnnotation {
    pub(super) from: egui::Pos2,
    pub(super) to: egui::Pos2,
    pub(super) start: Instant,
    pub(super) slide_index: usize,
}

/// Pen strokes and arrows over the slides, and the one being drawn.
#[derive(Default)]
pub(super) struct Ink {
    pub(super) strokes: Vec<PenStroke>,
    pub(super) arrows: Vec<ArrowAnnotation>,
    pub(super) active: ActiveDraw,
}

/// Tracks an in-progress mouse interaction
#[derive(Default)]
pub(super) enum ActiveDraw {
    #[default]
    None,
    /// Left button held: collecting points, might still be a click
    PenPending {
        origin: egui::Pos2,
        points: Vec<egui::Pos2>,
    },
    /// Left button held: drag threshold exceeded, definitely drawing
    PenDrawing { points: Vec<egui::Pos2> },
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

/// One mouse button's state this frame.
#[derive(Debug, Clone, Copy)]
struct Button {
    pressed: bool,
    down: bool,
    released: bool,
}

impl Button {
    fn read(pointer: &egui::PointerState, button: egui::PointerButton) -> Self {
        Self {
            pressed: pointer.button_pressed(button),
            down: pointer.button_down(button),
            released: pointer.button_released(button),
        }
    }
}

impl ActiveDraw {
    /// The pointer moved to `local` with the left (`pen`) or right button
    /// held: grow the stroke or move the arrow's head, and turn a pending
    /// click into a drag once it passes the threshold.
    fn drag(&mut self, local: egui::Pos2, pen: bool) {
        match self {
            ActiveDraw::PenPending { origin, points } if pen => {
                points.push(local);
                if origin.distance(local) > DRAG_THRESHOLD {
                    let points = std::mem::take(points);
                    *self = ActiveDraw::PenDrawing { points };
                }
            }
            ActiveDraw::PenDrawing { points } if pen => points.push(local),
            ActiveDraw::ArrowPending { origin, current } if !pen => {
                *current = local;
                if origin.distance(local) > DRAG_THRESHOLD {
                    let from = *origin;
                    *self = ActiveDraw::ArrowDrawing {
                        from,
                        current: local,
                    };
                }
            }
            ActiveDraw::ArrowDrawing { current, .. } if !pen => *current = local,
            _ => {}
        }
    }
}

impl PresentationApp {
    pub(super) fn handle_mouse_input(&mut self, ctx: &egui::Context) {
        let (primary, secondary, pointer_pos) = ctx.input(|i| {
            (
                Button::read(&i.pointer, egui::PointerButton::Primary),
                Button::read(&i.pointer, egui::PointerButton::Secondary),
                i.pointer.hover_pos(),
            )
        });

        let Some(pos) = pointer_pos else {
            // Pointer left the window. Once no button is held, whatever was
            // pending can never complete as a click or stroke: drop it.
            if !primary.down && !secondary.down {
                self.ink.active = ActiveDraw::None;
            }
            return;
        };
        let local = self.screen_to_local(pos);

        if primary.pressed {
            // Left button press: a click or the start of a pen stroke
            self.ink.active = ActiveDraw::PenPending {
                origin: local,
                points: vec![local],
            };
        } else if secondary.pressed {
            // Right button press: a click or the start of an arrow
            self.ink.active = ActiveDraw::ArrowPending {
                origin: local,
                current: local,
            };
        } else if primary.down || secondary.down {
            self.ink.active.drag(local, primary.down);
            ctx.request_repaint();
        } else if !matches!(self.ink.active, ActiveDraw::None) {
            // No button held: commit or navigate only on an observed release
            let active = std::mem::replace(&mut self.ink.active, ActiveDraw::None);
            self.release(active, primary.released || secondary.released);
        }
    }

    fn release(&mut self, active: ActiveDraw, released_this_frame: bool) {
        match release_outcome(active, released_this_frame) {
            ReleaseOutcome::NavigateForward => self.navigate_forward(),
            ReleaseOutcome::NavigateBackward => self.navigate_backward(),
            ReleaseOutcome::CommitPen(points) => self.ink.strokes.push(PenStroke {
                points,
                start: Instant::now(),
                slide_index: self.current_slide,
            }),
            ReleaseOutcome::CommitArrow { from, to } => self.ink.arrows.push(ArrowAnnotation {
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
    fn dragging_past_the_threshold_turns_a_click_into_a_stroke() {
        let mut d = ActiveDraw::PenPending {
            origin: p(0.0, 0.0),
            points: vec![p(0.0, 0.0)],
        };
        d.drag(p(1.0, 1.0), true);
        assert!(matches!(d, ActiveDraw::PenPending { .. }));
        d.drag(p(20.0, 0.0), true);
        let ActiveDraw::PenDrawing { points } = &d else {
            panic!("still pending");
        };
        assert_eq!(points.len(), 3);
        // the right button does not move a pen stroke
        d.drag(p(40.0, 0.0), false);
        assert!(matches!(&d, ActiveDraw::PenDrawing { points } if points.len() == 3));
    }

    #[test]
    fn dragging_an_arrow_moves_its_head() {
        let mut d = ActiveDraw::ArrowPending {
            origin: p(0.0, 0.0),
            current: p(0.0, 0.0),
        };
        d.drag(p(30.0, 0.0), false);
        d.drag(p(50.0, 10.0), false);
        assert!(matches!(
            d,
            ActiveDraw::ArrowDrawing { from, current } if from == p(0.0, 0.0) && current == p(50.0, 10.0)
        ));
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
