//! The sketch engine: a sketchbook. Every slide is a sheet of drawing paper
//! on a desk (the theme's `page:`), and the slide's generated picture
//! (`mdeck ai art`), graphite and ink in the MKLab house style by default,
//! is drawn in with a pencil: the outlines first, along the lines, then the
//! shading laid in stroke by stroke in bands that sweep across the picture.
//! Without art, the slide's `@illustration` is drawn in pencil; the
//! countdown and the end words are drawn the same way. Exports show the
//! finished drawing.

use eframe::egui;

use super::Engine;
use super::art::{Canvas, Reveal};
use super::led::premul;
use super::stage::{FrameCx, Stage};
use crate::render::art::prepare::Strategy;
use crate::render::art::{ArtKind, Medium, style};
use crate::render::illustration::Library;
use crate::render::strokes::to_screen;
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
        let rect = cx.rect;
        let scale = cx.scale;
        let g = Graphite::of(cx.theme);
        let painter = ui.painter();
        let c = &mut self.canvas;
        let now = c.now;
        // tonal pictures keep their own greys; line art is drawn in graphite
        let tint = |line: bool, k: f32| {
            if line {
                premul(g.lead, k)
            } else {
                premul(egui::Color32::WHITE, k)
            }
        };

        let left = c.fading.as_ref().map(|(_, since)| c.fade_left(*since));
        if let (Some((old, _)), Some(left)) = (&mut c.fading, left) {
            let line = old.picture.strategy == Strategy::Draw;
            let reveal = if line { LINE_REVEAL } else { REVEAL };
            old.paint(ui, rect, now, tint(line, left * cx.opacity), reveal);
        }
        if let Some((old, since)) = &c.fading_strokes {
            let k = c.fade_left(*since) * cx.opacity;
            graphite_lines(painter, old, now, rect, scale, &g, k);
        }

        let burst = c.burst_left();
        if let Some(d) = &mut c.drawing {
            let line = d.picture.strategy == Strategy::Draw;
            let reveal = if line { LINE_REVEAL } else { REVEAL };
            let k = if d.backdrop { 0.3 } else { 1.0 } * cx.opacity;
            d.paint(ui, rect, now, tint(line, k), reveal);
            if !cx.still
                && !d.backdrop
                && let Some(tip) = d.tip(now, rect)
            {
                pencil(painter, tip, scale, &g, d.progress(now) * 3.0, cx.opacity);
            }
        }
        if let Some(p) = &c.strokes {
            graphite_lines(painter, p, now, rect, scale, &g, burst * cx.opacity);
            if !cx.still
                && let Some((tip, on)) = p.tip(now - p.born)
                && on
            {
                let wobble = (now - p.born) / p.duration.max(0.1) * 4.0;
                pencil(painter, to_screen(tip, rect), scale, &g, wobble, cx.opacity);
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
    fn a_sketchbook_asks_for_graphite_drawings() {
        assert_eq!(MEDIUM.kind, ArtKind::Tonal);
        assert_eq!(MEDIUM.tonal_strategy, Strategy::Hatch);
        assert_eq!(
            super::super::EngineKind::Sketch.medium().map(|m| m.name),
            Some("sketch")
        );
    }
}
