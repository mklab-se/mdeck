//! The blueprint engine: a draftsman's sheet. Every slide is a Prussian
//! blue drawing sheet with a fine grid, a ruled border and a title block;
//! the slide's generated line art (`mdeck ai art`) is inked onto it the way
//! a draughtsman works: faint construction lines run ahead, the ink follows
//! stroke by stroke under a drafting machine's crosshair, and dimension
//! lines are ruled around the finished drawing. Without art, the slide's
//! `@illustration` is drawn as technical pen lines; the countdown and the
//! end words are drawn the same way. Exports show the finished sheet.

use std::sync::Arc;

use eframe::egui;

use super::Engine;
use super::art::{Drawing, Reveal, fallback_strokes};
use super::led::{premul, sprite_sheet};
use super::stage::{FrameCx, Moment, Stage};
use crate::render::art::prepare::Strategy;
use crate::render::art::{ArtKind, Medium, style};
use crate::render::illustration::Library;
use crate::render::strokes::{Picture, to_screen};
use draw::{Ink, crosshair, dimensions, pen_lines, pen_tip, sheet, title_block};

mod draw;

/// Blueprints ask for line art and ink it themselves.
pub static MEDIUM: Medium = Medium {
    name: "blueprint",
    kind: ArtKind::Line,
    tonal: &style::SKETCH,
    tonal_strategy: Strategy::Hatch,
};

/// Seconds into the end slide when the caption fades in.
pub const END_CAPTION_DELAY: f32 = 5.2;
/// The end words hold this long, then fade.
const END_WORDS: f32 = 3.6;
/// Seconds to ink a picture.
const DRAW: f32 = 3.4;
/// Seconds to rule the dimension lines once the ink is down.
const DIMENSION: f32 = 0.7;

const REVEAL: Reveal = Reveal {
    soft: 0.018,
    ghost: 0.16,
    ghost_speed: 2.4,
};

#[derive(Clone, Copy, PartialEq, Debug)]
enum Look {
    Slide,
    Digit(u8),
    Burst,
    EndWords,
    EndOut,
}

type Key = (usize, Look, usize, usize, bool);

pub struct Blueprint {
    key: Option<Key>,
    now: f32,
    drawing: Option<Drawing>,
    /// The previous sheet's drawing, fading out since the given time.
    fading: Option<(Drawing, f32)>,
    strokes: Option<Picture>,
    fading_strokes: Option<(Picture, f32)>,
    /// The countdown's last digit fades as the first sheet comes in.
    burst: Option<f32>,
    sprites: Option<egui::TextureHandle>,
}

impl Blueprint {
    pub fn new() -> Self {
        Self {
            key: None,
            now: 0.0,
            drawing: None,
            fading: None,
            strokes: None,
            fading_strokes: None,
            burst: None,
            sprites: None,
        }
    }

    fn retire(&mut self) {
        if let Some(d) = self.drawing.take() {
            self.fading = Some((d, self.now));
        }
        if let Some(p) = self.strokes.take() {
            self.fading_strokes = Some((p, self.now));
        }
    }
}

impl Default for Blueprint {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for Blueprint {
    fn update(&mut self, cx: &FrameCx, stage: &Stage, _lib: &mut Library) {
        let look = match &stage.moment {
            Moment::Slide => Look::Slide,
            Moment::Countdown { digit, .. } => Look::Digit(*digit),
            Moment::Burst { .. } => Look::Burst,
            Moment::End { elapsed, .. } if *elapsed < END_WORDS => Look::EndWords,
            Moment::End { .. } => Look::EndOut,
        };
        let art = stage
            .art
            .as_ref()
            .map(|a| Arc::as_ptr(&a.picture) as usize)
            .unwrap_or(0);
        let figure = stage
            .figure
            .as_ref()
            .map(|f| Arc::as_ptr(&f.cloud) as usize)
            .unwrap_or(0);
        let key = (stage.index, look, art, figure, stage.title);
        if self.key != Some(key) {
            match look {
                Look::Burst => self.burst = Some(0.0),
                Look::EndOut => self.retire(),
                _ => {
                    self.burst = None;
                    self.retire();
                    if let (Look::Slide, Some(a)) = (look, &stage.art) {
                        self.drawing = Some(Drawing::new(
                            a.picture.clone(),
                            a.place,
                            a.backdrop,
                            self.now,
                            DRAW,
                        ));
                    } else {
                        self.strokes = fallback_strokes(cx, stage, self.now);
                    }
                }
            }
            self.key = Some(key);
        }
        if let Moment::Burst { progress } = stage.moment {
            self.burst = Some(progress);
        }
        if cx.still {
            self.fading = None;
            self.fading_strokes = None;
            if let Some(d) = &mut self.drawing {
                d.born = self.now - DRAW - DIMENSION - 60.0;
            }
            if let Some(p) = &mut self.strokes {
                p.born = self.now - p.duration - 60.0;
            }
            return;
        }
        self.now += cx.dt;
        if self
            .fading
            .as_ref()
            .is_some_and(|(_, since)| self.now - since > 0.5)
        {
            self.fading = None;
        }
        if self
            .fading_strokes
            .as_ref()
            .is_some_and(|(_, since)| self.now - since > 0.5)
        {
            self.fading_strokes = None;
        }
    }

    fn paint(&mut self, ui: &egui::Ui, cx: &FrameCx, stage: &Stage) {
        let texture = self
            .sprites
            .get_or_insert_with(|| {
                ui.ctx().load_texture(
                    "mdeck-blueprint-sprites",
                    sprite_sheet(),
                    egui::TextureOptions::LINEAR,
                )
            })
            .id();
        let theme = cx.theme;
        let rect = cx.rect;
        let scale = cx.scale;
        let ink = Ink::of(theme);
        let painter = ui.painter();

        sheet(painter, texture, rect, scale, &ink, cx.opacity);
        if matches!(stage.moment, Moment::Slide) {
            title_block(painter, theme, rect, scale, &ink, stage, cx.opacity);
        }

        let now = self.now;
        if let Some((old, since)) = &mut self.fading {
            let k = (1.0 - (now - *since) / 0.45).clamp(0.0, 1.0) * cx.opacity;
            old.paint(ui, rect, now, premul(ink.line, k), REVEAL);
        }
        if let Some((old, since)) = &self.fading_strokes {
            let k = (1.0 - (now - since) / 0.45).clamp(0.0, 1.0) * cx.opacity;
            pen_lines(painter, old, now, rect, scale, &ink, k);
        }

        let burst = self
            .burst
            .map(|b| (1.0 - b * 1.5).clamp(0.0, 1.0))
            .unwrap_or(1.0);
        if let Some(d) = &mut self.drawing {
            let k = if d.backdrop { 0.34 } else { 1.0 } * cx.opacity;
            let box_ = d.screen(rect);
            d.paint(ui, rect, now, premul(ink.line, k), REVEAL);
            if !d.backdrop {
                let t = (now - d.born - DRAW) / DIMENSION;
                dimensions(painter, box_, scale, &ink, t, cx.opacity);
            }
            if !cx.still
                && let Some(tip) = d.tip(now, rect)
            {
                crosshair(
                    painter,
                    texture,
                    box_,
                    tip,
                    scale,
                    &ink,
                    cx.opacity * if d.backdrop { 0.5 } else { 1.0 },
                );
            }
        }
        if let Some(p) = &self.strokes {
            pen_lines(painter, p, now, rect, scale, &ink, burst * cx.opacity);
            if !cx.still
                && let Some((tip, on)) = p.tip(now - p.born)
                && on
            {
                let tip = to_screen(tip, rect);
                pen_tip(painter, texture, tip, scale, &ink, cx.opacity);
            }
        }

        let busy = self.fading.is_some()
            || self.fading_strokes.is_some()
            || self
                .drawing
                .as_ref()
                .is_some_and(|d| now - d.born < DRAW + DIMENSION + 0.2)
            || self
                .strokes
                .as_ref()
                .is_some_and(|p| now - p.born < p.duration + 0.2);
        if busy && !cx.still {
            ui.ctx().request_repaint();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blueprint_asks_for_line_art() {
        assert_eq!(MEDIUM.kind, ArtKind::Line);
        assert_eq!(
            super::super::EngineKind::Blueprint.medium().map(|m| m.name),
            Some("blueprint")
        );
    }
}
