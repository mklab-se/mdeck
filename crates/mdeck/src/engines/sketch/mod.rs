//! The sketch engine: a sketchbook. Every slide is a sheet of drawing paper
//! on a desk (the theme's `page:`), and the slide's generated picture
//! (`mdeck ai pictures`), graphite and ink in the MKLab house style by default,
//! is drawn in with a pencil: the outlines first, along the lines, then the
//! shading laid in stroke by stroke in bands that sweep across the picture.
//! Without art, the slide's point cloud picture is drawn in pencil; the
//! countdown and the end words are drawn the same way. Exports show the
//! finished drawing.

use mdeck_sdk::engine::{Capabilities, Engine, EngineDef, Medium, MediumKind, Needs};
use mdeck_sdk::paint::{Color, Painter, premul};
use mdeck_sdk::stage::{Frame, Stage, Strategy};

use super::art::strokes::Strokes;
use super::art::{Canvas, Drawing, Hand, Reveal, Tip};
use pencil::{Graphite, graphite_lines, pencil};

mod pencil;

/// A sketchbook asks for finished graphite drawings and draws them in with
/// outlines first, then hatching.
pub const MEDIUM: Medium = Medium {
    name: "sketch",
    kind: MediumKind::Tonal,
    strategy: Strategy::Hatch,
};

pub static DEF: EngineDef = EngineDef {
    name: "sketch",
    summary: "Generated graphite drawings drawn in with a pencil on a sketchbook page.",
    capabilities: Capabilities {
        medium: Some(MEDIUM),
        ..super::art::CAPABILITIES
    },
    settings: &[],
    needs: Needs { page: true },
    ending_caption_delay: 5.2,
    create: |_| Box::new(Sketch::new()),
    board: None,
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

/// A point cloud without an artwork: its lines in pencil, the shape shaded
/// with hatching and cross-hatched on its shadow side.
const FILL: crate::engines::art::Fill =
    crate::engines::art::Fill::Lines(Some(crate::engines::art::trace::Shading {
        spacing: 0.024,
        everywhere: true,
        cross: true,
    }));

pub struct Sketch {
    canvas: Canvas,
}

impl Sketch {
    pub fn new() -> Self {
        Self {
            canvas: Canvas::new(DRAW, 0.0, END_WORDS, 0.6).with_fill(FILL),
        }
    }
}

impl Default for Sketch {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for Sketch {
    fn update(&mut self, frame: &Frame, stage: &Stage) {
        self.canvas.update(frame, stage);
    }

    fn paint(&mut self, painter: &mut Painter, frame: &Frame, _stage: &Stage) {
        let hand = Pencil {
            g: Graphite::of(frame.tokens),
        };
        self.canvas.paint(painter, frame, &hand);
    }

    fn animating(&self) -> bool {
        self.canvas.moving
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

    fn picture(
        &self,
        painter: &Painter,
        frame: &Frame,
        d: &mut Drawing,
        now: f32,
        k: f32,
        _: bool,
    ) {
        let line = d.picture.strategy == Strategy::Draw;
        let (tint, reveal) = if line {
            (premul(self.g.lead, k), LINE_REVEAL)
        } else {
            (premul(Color::WHITE, k), REVEAL)
        };
        d.paint(painter, frame.rect, now, tint, reveal);
    }

    fn strokes(&self, painter: &Painter, frame: &Frame, p: &Strokes, now: f32, k: f32, _: bool) {
        graphite_lines(painter, p, now, frame.rect, frame.scale, &self.g, k);
    }

    fn finish(&self, painter: &Painter, frame: &Frame, tip: Option<Tip>) {
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
        pencil(painter, at, frame.scale, &self.g, wobble, frame.opacity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sketchbook_asks_for_graphite_drawings() {
        assert_eq!(MEDIUM.kind, MediumKind::Tonal);
        assert_eq!(MEDIUM.strategy, Strategy::Hatch);
        assert_eq!(DEF.capabilities.medium.map(|m| m.name), Some("sketch"));
        assert!(DEF.capabilities.picture);
    }
}
