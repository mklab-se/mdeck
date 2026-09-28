//! The blueprint engine: a draftsman's sheet. Every slide is a Prussian
//! blue drawing sheet with a fine grid, a ruled border and a title block;
//! the slide's generated line art (`mdeck ai art`) is inked onto it the way
//! a draughtsman works: faint construction lines run ahead, the ink follows
//! stroke by stroke under a drafting machine's crosshair, and dimension
//! lines are ruled around the finished drawing. Without art, the slide's
//! `@illustration` is drawn as technical pen lines; the countdown and the
//! end words are drawn the same way. Exports show the finished sheet.

use eframe::egui;

use super::Engine;
use super::art::{Canvas, Reveal};
use super::led::{premul, sprite_sheet};
use super::stage::{FrameCx, Moment, Stage};
use crate::render::art::prepare::Strategy;
use crate::render::art::{ArtKind, Medium, style};
use crate::render::illustration::Library;
use crate::render::strokes::to_screen;
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
    grain: 0.0,
};

pub struct Blueprint {
    canvas: Canvas,
    sprites: Option<egui::TextureHandle>,
}

impl Blueprint {
    pub fn new() -> Self {
        Self {
            canvas: Canvas::new(DRAW, DIMENSION, END_WORDS, 0.5),
            sprites: None,
        }
    }
}

impl Default for Blueprint {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for Blueprint {
    fn update(&mut self, cx: &FrameCx, stage: &Stage, lib: &mut Library) {
        self.canvas.update(cx, stage, lib);
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

        let c = &mut self.canvas;
        let now = c.now;
        let left = c.fading.as_ref().map(|(_, since)| c.fade_left(*since));
        if let (Some((old, _)), Some(left)) = (&mut c.fading, left) {
            old.paint(ui, rect, now, premul(ink.line, left * cx.opacity), REVEAL);
        }
        if let Some((old, since)) = &c.fading_strokes {
            let k = c.fade_left(*since) * cx.opacity;
            pen_lines(painter, old, now, rect, scale, &ink, k);
        }

        let burst = c.burst_left();
        if let Some(d) = &mut c.drawing {
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
        if let Some(p) = &c.strokes {
            pen_lines(painter, p, now, rect, scale, &ink, burst * cx.opacity);
            if !cx.still
                && let Some((tip, on)) = p.tip(now - p.born)
                && on
            {
                let tip = to_screen(tip, rect);
                pen_tip(painter, texture, tip, scale, &ink, cx.opacity);
            }
        }

        if c.busy() && !cx.still {
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
