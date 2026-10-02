//! The plain engine: slides on a flat background. It paints nothing; the
//! slide's own design does all the drawing. Every engine name that resolves
//! to nothing falls back to it (EXT-07).

use mdeck_sdk::engine::{Capabilities, Engine, EngineDef, Needs};
use mdeck_sdk::paint::Painter;
use mdeck_sdk::stage::{Frame, Stage};

pub static DEF: EngineDef = EngineDef {
    name: "plain",
    summary: "Slides on the theme's background; nothing moves.",
    capabilities: Capabilities::NONE,
    settings: &[],
    needs: Needs { page: false },
    ending_caption_delay: 0.0,
    create: |_| Box::new(Plain),
    board: None,
};

pub struct Plain;

impl Engine for Plain {
    fn update(&mut self, _frame: &Frame, _stage: &Stage) {}

    fn paint(&mut self, _painter: &mut Painter, _frame: &Frame, _stage: &Stage) {}

    fn animating(&self) -> bool {
        false
    }
}
