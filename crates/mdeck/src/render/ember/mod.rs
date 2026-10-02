//! What remains of the v1 editorial renderer: its slide chrome (the counter
//! and the progress hairline), and the questions engines still ask about a
//! slide. The editorial look itself is the `editorial` design set
//! (`crates/mdeck/designs/editorial.yaml`, drawn by `render::designs`).

mod chrome;

use eframe::egui::Color32;

use crate::parser::{Design, Slide};
use crate::theme::Theme;

pub use crate::render::designs::COLD_OPEN_HOLD;
pub use chrome::draw_chrome;

fn fade(c: Color32, a: f32) -> Color32 {
    crate::render::designs::style::fade(c, a)
}

/// Whether the slide is a title page. The first slide's lone H1 is a title
/// by the recognition table, so the index is no longer needed; engines
/// still pass it.
pub fn is_title(slide: &Slide, _index: usize) -> bool {
    slide.design == Design::Title
}

/// Whether the slide's design leaves a stage for a picture in the editorial
/// set. Engines ask this until they call `render::design_has_stage` with
/// the deck's theme.
pub fn handles(slide: &Slide) -> bool {
    use std::sync::LazyLock;
    static EDITORIAL: LazyLock<std::sync::Arc<crate::theme::arrangement::Arrangements>> =
        LazyLock::new(|| {
            crate::theme::arrangement::Arrangements::resolve("editorial", None)
                .expect("the built-in editorial set is valid")
        });
    let a = EDITORIAL.get(slide.design);
    if a.wide.is_some() && slide.blocks.iter().any(crate::render::is_wide_block) {
        return false;
    }
    a.stage != crate::theme::arrangement::Stage::None
}

/// Whether `theme` draws the editorial chrome.
pub fn draws_chrome(theme: &Theme) -> bool {
    crate::theme::uses_editorial(theme)
}
