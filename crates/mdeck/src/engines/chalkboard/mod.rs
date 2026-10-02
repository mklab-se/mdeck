//! The chalkboard engine: every slide is a slate in a wooden frame (the
//! theme's `page:`), with the ghosts of earlier drawings wiped off it. The
//! slide's generated line art (`mdeck ai art`) is drawn in chalk, stroke by
//! stroke, the chalk breaking up on the slate and shedding dust as it goes.
//! Without art, the slide's `picture` is drawn in chalk; the
//! countdown and the end words are drawn the same way. Exports show the
//! finished board.

use eframe::egui;

use super::art::{Canvas, Drawing, Hand, Reveal, Tip};
use super::paint::{Sprites, premul};
use super::stage::{FrameCx, Stage};
use super::{Engine, EngineDef};
use crate::render::art::prepare::Strategy;
use crate::render::art::{ArtKind, Medium, style};
use crate::render::illustration::Library;
use crate::render::strokes::Picture;
use chalk::{Chalk, Mote, chalk_lines, dust, emit, slate, step, stick};

mod chalk;

/// A chalkboard asks for line art and draws it in chalk.
pub static MEDIUM: Medium = Medium {
    name: "chalkboard",
    kind: ArtKind::Line,
    tonal: &style::SKETCH,
    tonal_strategy: Strategy::Hatch,
};

/// Seconds into the end slide when the caption fades in.
pub const END_CAPTION_DELAY: f32 = 5.2;

pub static DEF: EngineDef = EngineDef {
    capabilities: super::art::CAPABILITIES,
    create: || Box::new(Chalkboard::new()),
    end_caption_delay: END_CAPTION_DELAY,
    medium: Some(&MEDIUM),
    render_slide: None,
    problems: None,
};
/// The end words hold this long, then fade.
const END_WORDS: f32 = 3.6;
/// Seconds to draw a picture.
const DRAW: f32 = 3.6;

const REVEAL: Reveal = Reveal {
    soft: 0.02,
    ghost: 0.0,
    ghost_speed: 1.0,
    grain: 0.72,
};

pub struct Chalkboard {
    canvas: Canvas,
    motes: Vec<Mote>,
    seed: u32,
    sprites: Sprites,
}

impl Chalkboard {
    pub fn new() -> Self {
        Self {
            canvas: Canvas::new(DRAW, 0.0, END_WORDS, 0.7),
            motes: Vec::new(),
            seed: 0x1234_5679,
            sprites: Sprites::new("mdeck-chalkboard-sprites"),
        }
    }
}

impl Default for Chalkboard {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for Chalkboard {
    fn update(&mut self, cx: &FrameCx, stage: &Stage, lib: &mut Library) {
        self.canvas.update(cx, stage, lib);
        if cx.still {
            self.motes.clear();
            return;
        }
        if let Some(tip) = self.canvas.tip.and_then(Tip::in_front) {
            emit(&mut self.motes, tip, cx.scale, cx.dt, &mut self.seed);
        }
        step(&mut self.motes, cx.dt, cx.scale);
    }

    fn paint(&mut self, ui: &egui::Ui, cx: &FrameCx, _stage: &Stage) {
        let texture = self.sprites.id(ui.ctx());
        let chalk = Chalk::of(cx.theme);
        slate(ui.painter(), texture, cx.rect, cx.scale, &chalk, cx.opacity);
        let hand = Stick {
            chalk,
            motes: &self.motes,
        };
        self.canvas.paint(ui, cx, &hand);
    }
}

/// Chalk: pictures and strokes in chalk white, the dust it sheds, and the
/// stick at the tip.
struct Stick<'a> {
    chalk: Chalk,
    motes: &'a [Mote],
}

impl Hand for Stick<'_> {
    fn backdrop(&self) -> f32 {
        0.42
    }

    fn picture(&self, ui: &egui::Ui, cx: &FrameCx, d: &mut Drawing, now: f32, k: f32, _: bool) {
        d.paint(ui, cx.rect, now, premul(self.chalk.white, k), REVEAL);
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
        chalk_lines(painter, p, now, cx.rect, cx.scale, &self.chalk, k);
    }

    fn finish(&self, painter: &egui::Painter, cx: &FrameCx, tip: Option<Tip>) {
        dust(painter, self.motes, &self.chalk, cx.opacity);
        if let Some(at) = tip.and_then(Tip::in_front) {
            stick(painter, at, cx.scale, &self.chalk, cx.opacity);
        }
    }

    fn busy(&self) -> bool {
        !self.motes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chalkboard_asks_for_line_art_and_breaks_it_up() {
        assert_eq!(MEDIUM.kind, ArtKind::Line);
        const { assert!(REVEAL.grain > 0.0) };
        assert_eq!(
            super::super::EngineKind::Chalkboard
                .medium()
                .map(|m| m.name),
            Some("chalkboard")
        );
    }
}
