//! The blueprint engine: a draftsman's sheet. Every slide is a Prussian
//! blue drawing sheet with a fine grid, a ruled border and a title block;
//! the slide's generated line art (`mdeck ai pictures`) is inked onto it the way
//! a draughtsman works: faint construction lines run ahead, the ink follows
//! stroke by stroke under a drafting machine's crosshair, and dimension
//! lines are ruled around the finished drawing. Without art, the slide's
//! `@illustration` is drawn as technical pen lines; the countdown and the
//! end words are drawn the same way. Exports show the finished sheet.

use eframe::egui;

use super::art::{Canvas, Drawing, Hand, Reveal, Tip};
use super::paint::{Sprites, premul};
use super::stage::{FrameCx, Moment, Stage};
use super::{Capabilities, Engine, EngineDef};
use crate::render::art::prepare::Strategy;
use crate::render::art::{ArtKind, Medium, style};
use crate::render::illustration::Library;
use crate::render::strokes::Picture;
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

pub static DEF: EngineDef = EngineDef {
    capabilities: Capabilities {
        numbers_slides: true,
        ..super::art::CAPABILITIES
    },
    create: || Box::new(Blueprint::new()),
    end_caption_delay: END_CAPTION_DELAY,
    medium: Some(&MEDIUM),
    render_slide: None,
    problems: None,
};
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
    sprites: Sprites,
}

impl Blueprint {
    pub fn new() -> Self {
        Self {
            canvas: Canvas::new(DRAW, DIMENSION, END_WORDS, 0.5),
            sprites: Sprites::new("mdeck-blueprint-sprites"),
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
        let texture = self.sprites.id(ui.ctx());
        let theme = cx.theme;
        let ink = Ink::of(theme);
        let painter = ui.painter();
        sheet(painter, texture, cx.rect, cx.scale, &ink, cx.opacity);
        if matches!(stage.moment, Moment::Slide) {
            title_block(painter, theme, cx.rect, cx.scale, &ink, stage, cx.opacity);
        }
        self.canvas.paint(ui, cx, &Pen { ink, texture });
    }
}

/// A technical pen: ink lines, dimension lines ruled around a finished
/// picture, and the drafting machine's crosshair at the tip.
struct Pen {
    ink: Ink,
    texture: egui::TextureId,
}

impl Hand for Pen {
    fn backdrop(&self) -> f32 {
        0.34
    }

    fn picture(
        &self,
        ui: &egui::Ui,
        cx: &FrameCx,
        d: &mut Drawing,
        now: f32,
        k: f32,
        current: bool,
    ) {
        let box_ = d.screen(cx.rect);
        d.paint(ui, cx.rect, now, premul(self.ink.line, k), REVEAL);
        if current && !d.backdrop {
            let t = (now - d.born - DRAW) / DIMENSION;
            dimensions(ui.painter(), box_, cx.scale, &self.ink, t, cx.opacity);
        }
    }

    fn strokes(
        &self,
        painter: &egui::Painter,
        cx: &FrameCx,
        p: &Picture,
        now: f32,
        k: f32,
        _: bool,
    ) {
        pen_lines(painter, p, now, cx.rect, cx.scale, &self.ink, k);
    }

    fn finish(&self, painter: &egui::Painter, cx: &FrameCx, tip: Option<Tip>) {
        let (scale, ink) = (cx.scale, &self.ink);
        match tip {
            Some(Tip::Picture {
                at,
                frame,
                backdrop,
                ..
            }) => {
                let k = cx.opacity * if backdrop { 0.5 } else { 1.0 };
                crosshair(painter, self.texture, frame, at, scale, ink, k);
            }
            Some(Tip::Pen { at, .. }) => pen_tip(painter, self.texture, at, scale, ink, cx.opacity),
            None => {}
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
