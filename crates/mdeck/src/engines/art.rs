//! What the art engines share: a generated picture being drawn in (the
//! reveal runs on the CPU into a texture, a frame at a time, and stops
//! uploading once the picture is finished), and the pen strokes a medium
//! draws when a slide has no picture (its `@illustration`, the countdown,
//! the end words).

use std::sync::Arc;

use eframe::egui::{self, Color32, Pos2, Rect};

use super::stage::{FrameCx, Mask, Moment, Place, Stage};
use crate::render::art::prepare::Prepared;
use crate::render::strokes::{Picture, plan, toured};

/// A generated picture on the slide, drawn in over `duration` seconds.
pub struct Drawing {
    pub picture: Arc<Prepared>,
    pub place: Place,
    pub backdrop: bool,
    pub born: f32,
    pub duration: f32,
    texture: Option<egui::TextureHandle>,
    /// The moment the texture shows (negative: nothing uploaded yet).
    shown: f32,
}

/// How a medium reveals its pictures.
#[derive(Clone, Copy, Debug)]
pub struct Reveal {
    /// How long a pixel takes to arrive, as a share of the reveal.
    pub soft: f32,
    /// A faint first pass (construction lines, an underdrawing): its
    /// opacity, and how much faster than the ink it runs.
    pub ghost: f32,
    pub ghost_speed: f32,
}

impl Drawing {
    pub fn new(
        picture: Arc<Prepared>,
        place: Place,
        backdrop: bool,
        born: f32,
        duration: f32,
    ) -> Self {
        Self {
            picture,
            place,
            backdrop,
            born,
            duration,
            texture: None,
            shown: -1.0,
        }
    }

    /// 0..1 through the reveal (1 and beyond: finished).
    pub fn progress(&self, now: f32) -> f32 {
        ((now - self.born) / self.duration.max(1e-3)).max(0.0)
    }

    /// Where the picture goes on screen.
    pub fn screen(&self, rect: Rect) -> Rect {
        to_rect(self.place, rect)
    }

    /// Where the drawing hand is at `now`, on screen.
    pub fn tip(&self, now: f32, rect: Rect) -> Option<Pos2> {
        let (u, v) = self.picture.tip(self.progress(now))?;
        let r = self.screen(rect);
        Some(Pos2::new(
            r.left() + u * r.width(),
            r.top() + v * r.height(),
        ))
    }

    /// Paint the picture as far as it has come, tinted by `tint` (line art
    /// is white, so the tint is its ink colour).
    pub fn paint(&mut self, ui: &egui::Ui, rect: Rect, now: f32, tint: Color32, reveal: Reveal) {
        let t = self.progress(now).min(1.0 + reveal.soft);
        let stale = self.texture.is_none() || (t - self.shown).abs() > 0.002;
        if stale {
            let pixels = self
                .picture
                .reveal(t, reveal.soft, reveal.ghost, reveal.ghost_speed);
            let image = egui::ColorImage::new([self.picture.width, self.picture.height], pixels);
            match &mut self.texture {
                Some(tex) => tex.set(image, egui::TextureOptions::LINEAR),
                None => {
                    let name = format!("mdeck-art-{:p}", Arc::as_ptr(&self.picture));
                    self.texture = Some(ui.ctx().load_texture(
                        name,
                        image,
                        egui::TextureOptions::LINEAR,
                    ));
                }
            }
            self.shown = t;
        }
        if let Some(tex) = &self.texture {
            let mut mesh = egui::Mesh::with_texture(tex.id());
            mesh.add_rect_with_uv(
                self.screen(rect),
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                tint,
            );
            ui.painter().add(egui::Shape::mesh(mesh));
        }
    }
}

/// A place in slide fractions, on screen.
pub fn to_rect(place: Place, rect: Rect) -> Rect {
    Rect::from_min_size(
        Pos2::new(
            rect.left() + place.u * rect.width(),
            rect.top() + place.v * rect.height(),
        ),
        egui::vec2(place.w * rect.width(), place.h * rect.height()),
    )
}

/// The pen strokes for a moment without a picture: the slide's
/// `@illustration`, a countdown digit or the end words. `None` when there is
/// nothing to draw.
pub fn fallback_strokes(cx: &FrameCx, stage: &Stage, now: f32) -> Option<Picture> {
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
    let (strokes, duration, weight) = match &stage.moment {
        Moment::Countdown { mask, .. } => (
            vec![toured(&mask.0, place_mask(mask, 0.56), aspect)],
            0.85,
            1.0,
        ),
        Moment::End { words, .. } => {
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
        Moment::Slide => {
            let fig = stage.figure.as_ref()?;
            let weight = if fig.backdrop { 0.4 } else { 1.0 };
            (
                vec![toured(&fig.cloud.points, fig.place, aspect)],
                2.4,
                weight,
            )
        }
        Moment::Burst { .. } => return None,
    };
    Some(plan(strokes, duration, weight, aspect, now))
}
