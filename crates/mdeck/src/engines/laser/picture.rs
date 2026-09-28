//! What the beam etches: the countdown digit, the end words, or a slide's
//! illustration and whatever its renderers drew, planned into one path.

use eframe::egui::{self, Pos2, Rect};

use super::{Laser, Look};
use crate::engines::stage::{FrameCx, Mask, Moment, Place, Stage};
use crate::render::hints::Hint;
use crate::render::strokes::{Picture, plan, toured};

impl Laser {
    pub(super) fn build(&self, cx: &FrameCx, stage: &Stage, look: Look) -> Option<Picture> {
        let rect = cx.rect;
        let aspect = rect.width() / rect.height();
        let place_mask = |mask: &Mask, h: f32| -> Place {
            let w = h * mask.1 / aspect;
            Place {
                u: 0.5 - w / 2.0,
                v: 0.47 - h / 2.0,
                w,
                h,
            }
        };
        let (strokes, duration, weight): (Vec<Vec<Pos2>>, f32, f32) = match (&stage.moment, look) {
            (Moment::Countdown { mask, .. }, _) => {
                let place = place_mask(mask, 0.56);
                (vec![toured(&mask.0, place, aspect)], 0.85, 1.0)
            }
            (Moment::End { words, .. }, Look::EndWords) => {
                let w = 0.60;
                let h = w / words.1 * aspect;
                let place = Place {
                    u: 0.5 - w / 2.0,
                    v: 0.47 - h / 2.0,
                    w,
                    h,
                };
                (vec![toured(&words.0, place, aspect)], 1.9, 1.0)
            }
            (Moment::Slide, _) => {
                let mut strokes = Vec::new();
                let mut weight = 1.0;
                if let Some(fig) = &stage.figure {
                    strokes.push(toured(&fig.cloud.points, fig.place, aspect));
                    if fig.backdrop {
                        weight = 0.45;
                    }
                }
                strokes.extend(hint_strokes(stage.hints, rect));
                if strokes.is_empty() {
                    return None;
                }
                let duration = if stage.figure.is_some() { 2.3 } else { 1.6 };
                (strokes, duration, weight)
            }
            _ => return None,
        };
        Some(plan(strokes, duration, weight, aspect, self.now))
    }
}

/// What the renderers drew, as strokes for the beam: lines and edges,
/// circles, and the tops of bars.
fn hint_strokes(hints: &[Hint], rect: Rect) -> Vec<Vec<Pos2>> {
    let frac = |p: Pos2| {
        Pos2::new(
            (p.x - rect.left()) / rect.width(),
            (p.y - rect.top()) / rect.height(),
        )
    };
    let mut out = Vec::new();
    for h in hints {
        match h {
            Hint::Path(points) if points.len() > 1 => {
                // densify so the beam's pace follows the line's length
                let mut s = Vec::new();
                for w in points.windows(2) {
                    let steps = ((w[1] - w[0]).length() / 6.0).ceil().max(1.0) as usize;
                    for k in 0..steps {
                        s.push(frac(w[0] + (w[1] - w[0]) * (k as f32 / steps as f32)));
                    }
                }
                s.push(frac(*points.last().expect("two points")));
                out.push(s);
            }
            Hint::Circle { center, radius } => {
                let r = radius + 10.0;
                out.push(
                    (0..=96)
                        .map(|k| {
                            let a = k as f32 / 96.0 * std::f32::consts::TAU;
                            frac(*center + egui::vec2(a.cos() * r, a.sin() * r))
                        })
                        .collect(),
                );
            }
            Hint::Bar(b) => {
                let y = b.top() - 6.0;
                let steps = (b.width() / 6.0).ceil().max(1.0) as usize;
                out.push(
                    (0..=steps)
                        .map(|k| frac(Pos2::new(b.left() + b.width() * k as f32 / steps as f32, y)))
                        .collect(),
                );
            }
            _ => {}
        }
    }
    out
}
