//! The line engine: the slide's generated line art (`mdeck ai art`) drawn in
//! stroke by stroke on a surface the theme chooses with `surface:`.
//!
//! - **`sheet`** (the `blueprint` theme): a draughtsman's sheet. Every slide
//!   is a Prussian blue drawing sheet with a fine grid, a ruled border and a
//!   title block that numbers the sheets; faint construction lines run
//!   ahead, the ink follows under a drafting machine's crosshair, and
//!   dimension lines are ruled around the finished drawing.
//! - **`slate`** (the `chalkboard` theme): a slate in a wooden frame (the
//!   theme's `page:`) with the ghosts of earlier drawings wiped off it; the
//!   chalk breaks up on the slate and sheds dust as it goes.
//!
//! Without art, the slide's `@illustration` is drawn in the same hand; the
//! countdown and the end words are drawn the same way. Exports show the
//! finished drawing.

use eframe::egui::{self, Pos2, Rect};

use super::art::{Canvas, Tip};
use super::paint::Sprites;
use super::stage::{FrameCx, Moment, Stage};
use super::{Capabilities, Engine, EngineDef};
use crate::render::art::prepare::Strategy;
use crate::render::art::{ArtKind, Medium, style};
use crate::render::illustration::Library;
use crate::render::strokes::{Picture, to_screen};
use crate::theme::Surface;

mod sheet;
mod slate;

/// The line engine asks for line art and draws it itself.
pub static MEDIUM: Medium = Medium {
    name: "line",
    kind: ArtKind::Line,
    tonal: &style::SKETCH,
    tonal_strategy: Strategy::Hatch,
};

/// Seconds into the end slide when the caption fades in.
pub const END_CAPTION_DELAY: f32 = 5.2;

pub static DEF: EngineDef = EngineDef {
    capabilities: Capabilities {
        // the sheet's title block numbers it (Theme::numbers_slides)
        numbers_slides: true,
        ..super::art::CAPABILITIES
    },
    create: || Box::new(Line::new()),
    end_caption_delay: END_CAPTION_DELAY,
    medium: Some(&MEDIUM),
    render_slide: None,
    problems: None,
};
/// The end words hold this long, then fade.
const END_WORDS: f32 = 3.6;

/// How a surface paces its drawing: seconds to draw a picture, to finish
/// around it after (the sheet's dimension lines), and for the old picture to
/// fade.
struct Pace {
    draw: f32,
    after: f32,
    fade: f32,
}

fn pace(surface: Surface) -> Pace {
    match surface {
        Surface::Sheet => sheet::PACE,
        Surface::Slate => slate::PACE,
    }
}

pub struct Line {
    /// The surface the canvas was paced for; a theme switch to the other
    /// surface starts a fresh canvas.
    surface: Surface,
    canvas: Canvas,
    /// Chalk dust (the slate only).
    motes: Vec<slate::Mote>,
    seed: u32,
    sprites: Sprites,
}

impl Line {
    pub fn new() -> Self {
        Self {
            surface: Surface::default(),
            canvas: canvas(Surface::default()),
            motes: Vec::new(),
            seed: 0x1234_5679,
            sprites: Sprites::new("mdeck-line-sprites"),
        }
    }
}

fn canvas(surface: Surface) -> Canvas {
    let p = pace(surface);
    Canvas::new(p.draw, p.after, END_WORDS, p.fade)
}

impl Default for Line {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for Line {
    fn update(&mut self, cx: &FrameCx, stage: &Stage, lib: &mut Library) {
        if cx.theme.surface != self.surface {
            self.surface = cx.theme.surface;
            self.canvas = canvas(self.surface);
            self.motes.clear();
        }
        self.canvas.update(cx, stage, lib);
        if self.surface != Surface::Slate {
            return;
        }
        if cx.still {
            self.motes.clear();
            return;
        }
        if let Some(tip) = self.canvas.tip.and_then(Tip::in_front) {
            slate::emit(&mut self.motes, tip, cx.scale, cx.dt, &mut self.seed);
        }
        slate::step(&mut self.motes, cx.dt, cx.scale);
    }

    fn paint(&mut self, ui: &egui::Ui, cx: &FrameCx, stage: &Stage) {
        let texture = self.sprites.id(ui.ctx());
        let painter = ui.painter();
        match self.surface {
            Surface::Sheet => {
                let ink = sheet::Ink::of(cx.theme);
                sheet::sheet(painter, texture, cx.rect, cx.scale, &ink, cx.opacity);
                if matches!(stage.moment, Moment::Slide) {
                    sheet::title_block(
                        painter, cx.theme, cx.rect, cx.scale, &ink, stage, cx.opacity,
                    );
                }
                self.canvas.paint(ui, cx, &sheet::Pen { ink, texture });
            }
            Surface::Slate => {
                let chalk = slate::Chalk::of(cx.theme);
                slate::slate(painter, texture, cx.rect, cx.scale, &chalk, cx.opacity);
                let hand = slate::Stick {
                    chalk,
                    motes: &self.motes,
                };
                self.canvas.paint(ui, cx, &hand);
            }
        }
    }
}

/// Segment `i` of `pic` (from point `i - 1` to point `i`) on screen, as far
/// as the hand has come `t` seconds into the picture: `None` while the pen
/// is lifted or has not reached it, cut short while it is being drawn.
fn drawn_segment(pic: &Picture, i: usize, t: f32, rect: Rect) -> Option<(Pos2, Pos2)> {
    if !pic.pen[i] || pic.at[i - 1] > t {
        return None;
    }
    let a = to_screen(pic.points[i - 1], rect);
    let mut b = to_screen(pic.points[i], rect);
    if pic.at[i] > t {
        let f = (t - pic.at[i - 1]) / (pic.at[i] - pic.at[i - 1]).max(1e-4);
        b = a + (b - a) * f.clamp(0.0, 1.0);
    }
    Some((a, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_line_engine_asks_for_line_art() {
        assert_eq!(MEDIUM.kind, ArtKind::Line);
        assert_eq!(
            super::super::EngineKind::Line.medium().map(|m| m.name),
            Some("line")
        );
    }

    #[test]
    fn each_surface_keeps_its_pace() {
        let (sheet, slate) = (pace(Surface::Sheet), pace(Surface::Slate));
        assert_eq!((sheet.draw, sheet.after, sheet.fade), (3.4, 0.7, 0.5));
        assert_eq!((slate.draw, slate.after, slate.fade), (3.6, 0.0, 0.7));
    }

    #[test]
    fn the_blueprint_and_chalkboard_themes_choose_their_surface() {
        for (name, surface) in [
            ("blueprint", Surface::Sheet),
            ("chalkboard", Surface::Slate),
        ] {
            let theme = crate::theme::lookup::load_builtin(name).expect(name);
            assert_eq!(theme.engine, super::super::EngineKind::Line, "{name}");
            assert_eq!(theme.surface, surface, "{name}");
        }
    }

    #[test]
    fn a_segment_is_cut_where_the_hand_is() {
        let pic = Picture {
            points: vec![
                Pos2::new(0.0, 0.0),
                Pos2::new(1.0, 0.0),
                Pos2::new(1.0, 1.0),
            ],
            pen: vec![true, true, false],
            at: vec![0.0, 1.0, 2.0],
            duration: 2.0,
            born: 0.0,
            weight: 1.0,
        };
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(100.0, 100.0));
        let (a, b) = drawn_segment(&pic, 1, 0.5, rect).expect("drawing");
        assert_eq!(a, to_screen(pic.points[0], rect));
        assert!((b.x - (a.x + to_screen(pic.points[1], rect).x) / 2.0).abs() < 1e-3);
        assert!(drawn_segment(&pic, 2, 5.0, rect).is_none(), "pen lifted");
    }
}
