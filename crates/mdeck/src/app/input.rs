use eframe::egui;
use std::time::Instant;

use super::{ActiveDraw, ArrowAnnotation, DRAG_THRESHOLD, PenStroke, PresentationApp};

impl PresentationApp {
    pub(super) fn handle_mouse_input(&mut self, ctx: &egui::Context) {
        let (primary_pressed, primary_down, secondary_pressed, secondary_down, pointer_pos) = ctx
            .input(|i| {
                let pp = i.pointer.button_pressed(egui::PointerButton::Primary);
                let pd = i.pointer.button_down(egui::PointerButton::Primary);
                let sp = i.pointer.button_pressed(egui::PointerButton::Secondary);
                let sd = i.pointer.button_down(egui::PointerButton::Secondary);
                let pos = i.pointer.hover_pos();
                (pp, pd, sp, sd, pos)
            });

        let Some(pos) = pointer_pos else { return };
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

        // Button released — commit or navigate
        match std::mem::replace(&mut self.active_draw, ActiveDraw::None) {
            ActiveDraw::PenPending { .. } => {
                self.navigate_forward();
            }
            ActiveDraw::PenDrawing { points } => {
                if points.len() >= 2 {
                    self.pen_strokes.push(PenStroke {
                        points,
                        start: Instant::now(),
                        slide_index: self.current_slide,
                    });
                }
            }
            ActiveDraw::ArrowPending { .. } => {
                self.navigate_backward();
            }
            ActiveDraw::ArrowDrawing { from, current } => {
                self.arrows.push(ArrowAnnotation {
                    from,
                    to: current,
                    start: Instant::now(),
                    slide_index: self.current_slide,
                });
            }
            ActiveDraw::None => {}
        }
    }
}
