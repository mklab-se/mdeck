//! The plain engine: slides on a flat background. It paints nothing; the
//! slide's own design does all the drawing. Every engine name that resolves
//! to nothing falls back to it (EXT-07).

use mdeck_sdk::engine::{Engine, EngineDef};
use mdeck_sdk::paint::Painter;
use mdeck_sdk::stage::{Frame, Stage};

pub static DEF: EngineDef = EngineDef::new(
    "plain",
    "Slides on the theme's background; nothing moves.",
    |_| Box::new(Plain),
)
.with_ending_caption_delay(0.0);

pub struct Plain;

impl Engine for Plain {
    fn update(&mut self, _frame: &Frame, _stage: &Stage) {}

    fn paint(&mut self, _painter: &mut Painter, _frame: &Frame, _stage: &Stage) {}

    fn animating(&self) -> bool {
        false
    }
}
