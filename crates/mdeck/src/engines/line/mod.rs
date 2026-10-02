//! The line engine: the slide's generated line art (`mdeck ai pictures`) drawn
//! stroke by stroke on a surface the theme chooses with the engine setting
//! `surface`.
//!
//! - **`sheet`** (the default, the `blueprint` theme): a draughtsman's
//!   sheet. Every slide is a Prussian blue drawing sheet with a fine grid, a
//!   ruled border and a title block that numbers the sheets; faint
//!   construction lines run ahead, the ink follows under a drafting
//!   machine's crosshair, and dimension lines are ruled around the finished
//!   drawing.
//! - **`slate`** (the `chalkboard` theme): a slate in a wooden frame (the
//!   theme's `page:`) with the ghosts of earlier drawings wiped off it; the
//!   chalk breaks up on the slate and sheds dust as it goes.
//!
//! Without art, the slide's point cloud picture is drawn in the same hand;
//! the countdown and the end words are drawn the same way. Exports show the
//! finished drawing.

use mdeck_sdk::engine::{Engine, EngineDef, Medium, MediumKind, Needs, SettingKind, SettingSpec};
use mdeck_sdk::paint::Painter;
use mdeck_sdk::stage::{Frame, Moment, Stage, Strategy};
use mdeck_sdk::tokens::EngineSettings;

use super::art::{Canvas, Sprites, Tip};

mod sheet;
mod slate;

/// The line engine asks for line art and draws it itself; tonal pictures
/// (a theme's `art:` choosing them) are hatched in like a sketch.
pub const MEDIUM: Medium = Medium::new("line", MediumKind::Line, Strategy::Hatch);

/// The surfaces the line engine draws on.
const SURFACES: &[&str] = &["sheet", "slate"];

pub static DEF: EngineDef = EngineDef::new(
    "line",
    "Generated line art inked onto a blueprint sheet or drawn in chalk on a slate.",
    |settings| Box::new(Line::new(Surface::of(settings))),
)
.with_capabilities(super::art::CAPABILITIES.with_medium(MEDIUM))
.with_settings(&[SettingSpec::new(
    "surface",
    SettingKind::OneOf(SURFACES),
    "What the lines are drawn on: a blueprint `sheet` (the default) or a chalk `slate`.",
)])
.with_needs(Needs::NONE.with_page())
.with_ending_caption_delay(5.2);

/// The end words hold this long, then fade.
const END_WORDS: f32 = 3.6;

/// What the line engine draws on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Surface {
    /// A blueprint: ink on a blue drawing sheet.
    #[default]
    Sheet,
    /// A chalkboard: chalk on a slate.
    Slate,
}

impl Surface {
    /// The surface the settings choose (the sheet unless they say `slate`).
    pub fn of(settings: &EngineSettings) -> Self {
        match settings.one_of("surface", SURFACES) {
            Some("slate") => Surface::Slate,
            _ => Surface::Sheet,
        }
    }
}

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

/// A point cloud without an artwork: its lines traced clean, with section
/// hatching on the shadow side, the way a drafter marks a cut surface.
const FILL: crate::engines::art::Fill =
    crate::engines::art::Fill::Lines(Some(crate::engines::art::trace::Shading {
        spacing: 0.045,
        everywhere: false,
        cross: false,
    }));

pub struct Line {
    surface: Surface,
    canvas: Canvas,
    /// Chalk dust (the slate only).
    motes: Vec<slate::Mote>,
    seed: u32,
    sprites: Sprites,
}

impl Line {
    pub fn new(surface: Surface) -> Self {
        let p = pace(surface);
        Self {
            surface,
            canvas: Canvas::new(p.draw, p.after, END_WORDS, p.fade).with_fill(FILL),
            motes: Vec::new(),
            seed: 0x1234_5679,
            sprites: Sprites::new("mdeck-line-sprites"),
        }
    }
}

impl Engine for Line {
    fn update(&mut self, frame: &Frame, stage: &Stage) {
        self.canvas.update(frame, stage);
        if self.surface != Surface::Slate {
            return;
        }
        if frame.settled() {
            self.motes.clear();
            return;
        }
        if let Some(tip) = self.canvas.tip.and_then(Tip::in_front) {
            slate::emit(&mut self.motes, tip, frame.scale, frame.dt, &mut self.seed);
        }
        slate::step(&mut self.motes, frame.dt, frame.scale);
    }

    fn paint(&mut self, painter: &mut Painter, frame: &Frame, stage: &Stage) {
        let texture = self.sprites.get(painter);
        match self.surface {
            Surface::Sheet => {
                let ink = sheet::Ink::of(frame.tokens);
                sheet::sheet(
                    painter,
                    &texture,
                    frame.rect,
                    frame.scale,
                    &ink,
                    frame.opacity,
                );
                if matches!(stage.moment, Moment::Slide) {
                    sheet::title_block(
                        painter,
                        frame.rect,
                        frame.scale,
                        &ink,
                        stage,
                        frame.opacity,
                    );
                }
                self.canvas
                    .paint(painter, frame, &sheet::Pen { ink, texture });
            }
            Surface::Slate => {
                let chalk = slate::Chalk::of(frame.tokens);
                slate::slate(
                    painter,
                    &texture,
                    frame.rect,
                    frame.scale,
                    &chalk,
                    frame.opacity,
                );
                let hand = slate::Stick {
                    chalk,
                    motes: &self.motes,
                };
                self.canvas.paint(painter, frame, &hand);
            }
        }
    }

    /// The sheet's title block prints the sheet number.
    fn numbers_slides(&self) -> bool {
        self.surface == Surface::Sheet
    }

    fn animating(&self) -> bool {
        self.canvas.moving
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mdeck_sdk::tokens::Value;

    fn settings(surface: &str) -> EngineSettings {
        EngineSettings::from_pairs([("surface", Value::String(surface.into()))])
    }

    #[test]
    fn the_line_engine_asks_for_line_art() {
        assert_eq!(MEDIUM.kind, MediumKind::Line);
        assert_eq!(DEF.capabilities.medium.map(|m| m.name), Some("line"));
        assert!(DEF.capabilities.picture);
    }

    #[test]
    fn each_surface_keeps_its_pace() {
        let (sheet, slate) = (pace(Surface::Sheet), pace(Surface::Slate));
        assert_eq!((sheet.draw, sheet.after, sheet.fade), (3.4, 0.7, 0.5));
        assert_eq!((slate.draw, slate.after, slate.fade), (3.6, 0.0, 0.7));
    }

    #[test]
    fn the_surface_is_a_setting_and_the_sheet_numbers_its_slides() {
        assert_eq!(Surface::of(&EngineSettings::new()), Surface::Sheet);
        assert_eq!(Surface::of(&settings("slate")), Surface::Slate);
        assert_eq!(Surface::of(&settings("Sheet")), Surface::Sheet);
        assert!((DEF.create)(&EngineSettings::new()).numbers_slides());
        assert!(!(DEF.create)(&settings("slate")).numbers_slides());
        assert!(DEF.check_settings(&settings("slate")).is_empty());
        assert_eq!(DEF.check_settings(&settings("paper")).len(), 1);
    }
}
