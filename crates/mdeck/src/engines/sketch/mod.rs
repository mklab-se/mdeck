//! The sketch engine: a sketchbook. Every slide is a sheet of drawing paper
//! on a desk (the theme's `page:`), and the slide's generated picture
//! (`mdeck ai art`), graphite and ink in the MKLab house style by default,
//! is drawn in with a pencil: the outlines first, along the lines, then the
//! shading laid in stroke by stroke in bands that sweep across the picture.
//! Without art, the slide's `@illustration` is drawn in pencil; the
//! countdown and the end words are drawn the same way. Exports show the
//! finished drawing.

use eframe::egui;

use super::art::{Canvas, Drawing, Hand, Reveal, Tip};
use super::paint::premul;
use super::stage::{FrameCx, Stage};
use super::{Engine, EngineDef};
use crate::render::art::prepare::Strategy;
use crate::render::art::{ArtKind, Medium, style};
use crate::render::illustration::Library;
use crate::render::strokes::Picture;
use pencil::{Graphite, graphite_lines, pencil};

mod pencil;

/// A sketchbook asks for finished graphite drawings and draws them in with
/// outlines first, then hatching.
pub static MEDIUM: Medium = Medium {
    name: "sketch",
    kind: ArtKind::Tonal,
    tonal: &style::SKETCH,
    tonal_strategy: Strategy::Hatch,
};

/// Seconds into the end slide when the caption fades in.
pub const END_CAPTION_DELAY: f32 = 5.2;

pub static DEF: EngineDef = EngineDef {
    capabilities: super::art::CAPABILITIES,
    create: || Box::new(Sketch::new()),
    end_caption_delay: END_CAPTION_DELAY,
    medium: Some(&MEDIUM),
    render_slide: None,
    problems: None,
};
/// The end words hold this long, then fade.
const END_WORDS: f32 = 3.6;
/// Seconds to draw a picture.
const DRAW: f32 = 4.2;

const REVEAL: Reveal = Reveal {
    soft: 0.02,
    ghost: 0.0,
    ghost_speed: 1.0,
    grain: 0.0,
};

/// A line-art picture on the sketchbook is drawn with a faint underdrawing.
const LINE_REVEAL: Reveal = Reveal {
    soft: 0.02,
    ghost: 0.12,
    ghost_speed: 2.0,
    grain: 0.0,
};

pub struct Sketch {
    canvas: Canvas,
}

impl Sketch {
    pub fn new() -> Self {
        Self {
            canvas: Canvas::new(DRAW, 0.0, END_WORDS, 0.6),
        }
    }
}

impl Default for Sketch {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for Sketch {
    fn update(&mut self, cx: &FrameCx, stage: &Stage, lib: &mut Library) {
        self.canvas.update(cx, stage, lib);
    }

    fn paint(&mut self, ui: &egui::Ui, cx: &FrameCx, _stage: &Stage) {
        let hand = Pencil {
            g: Graphite::of(cx.theme),
        };
        self.canvas.paint(ui, cx, &hand);
    }
}

/// Graphite: tonal pictures keep their own greys, line art is drawn in
/// lead, and a pencil follows the tip.
struct Pencil {
    g: Graphite,
}

impl Hand for Pencil {
    fn backdrop(&self) -> f32 {
        0.3
    }

    fn picture(&self, ui: &egui::Ui, cx: &FrameCx, d: &mut Drawing, now: f32, k: f32, _: bool) {
        let line = d.picture.strategy == Strategy::Draw;
        let (tint, reveal) = if line {
            (premul(self.g.lead, k), LINE_REVEAL)
        } else {
            (premul(egui::Color32::WHITE, k), REVEAL)
        };
        d.paint(ui, cx.rect, now, tint, reveal);
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
        graphite_lines(painter, p, now, cx.rect, cx.scale, &self.g, k);
    }

    fn finish(&self, painter: &egui::Painter, cx: &FrameCx, tip: Option<Tip>) {
        let (at, wobble) = match tip {
            Some(Tip::Picture {
                at,
                progress,
                backdrop: false,
                ..
            }) => (at, progress * 3.0),
            Some(Tip::Pen { at, progress }) => (at, progress * 4.0),
            _ => return,
        };
        pencil(painter, at, cx.scale, &self.g, wobble, cx.opacity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sketchbook_asks_for_graphite_drawings() {
        assert_eq!(MEDIUM.kind, ArtKind::Tonal);
        assert_eq!(MEDIUM.tonal_strategy, Strategy::Hatch);
        assert_eq!(
            super::super::EngineKind::Sketch.medium().map(|m| m.name),
            Some("sketch")
        );
    }
}
