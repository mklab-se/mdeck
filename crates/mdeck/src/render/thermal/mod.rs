//! Thermal images: a `@thermal` block asks for a thermal reading of one
//! image. MDeck never guesses; ordinary images are always shown as they are.
//!
//! A frame runs: parse the block (`spec`), look up its source, read once
//! when the deck opened (`library`, `source`), work out where its reveal is
//! (`state`), compose the picture in the palette (`compose`, `palette`) and
//! draw it with its legend and spots (`draw`).

#[cfg(test)]
mod acceptance;
mod compose;
mod draw;
mod legend;
mod lens;
mod library;
mod palette;
mod source;
mod spec;
mod spots;
mod state;
mod units;

use eframe::egui;

pub use draw::draw;
pub use library::{Library, blocks};
pub use palette::Palette;
pub use spec::Spec;
pub use spots::spot_anchor;

/// The reveal steps of a block when its source is a display image (what
/// the parser can tell without reading the file).
pub fn default_steps(content: &str) -> usize {
    Spec::parse(content).step_count(&spec::Support::DISPLAY)
}

/// The reveal steps of a block with its source read: steps the source
/// cannot show are left out.
pub fn block_steps(content: &str, lib: &Library) -> usize {
    let spec = Spec::parse(content);
    spec.step_count(&lib.support(&spec))
}

fn live_id() -> egui::Id {
    egui::Id::new("thermal-live-palette")
}

fn deck_id() -> egui::Id {
    egui::Id::new("thermal-deck-palette")
}

/// The palette the presenter picked live (the palette key), or `None` for
/// each block's own.
pub fn set_live_palette(ctx: &egui::Context, palette: Option<Palette>) {
    ctx.data_mut(|d| d.insert_temp(live_id(), palette));
}

/// The deck's `@palette`, for blocks that name none.
pub fn set_deck_palette(ctx: &egui::Context, palette: Option<Palette>) {
    ctx.data_mut(|d| d.insert_temp(deck_id(), palette));
}

/// The palette a block draws with: the live choice, else its own, else
/// the deck's, else iron.
pub fn palette_for(ctx: &egui::Context, own: Option<Palette>) -> Palette {
    let get = |id| ctx.data(|d| d.get_temp::<Option<Palette>>(id)).flatten();
    get(live_id())
        .or(own)
        .or_else(|| get(deck_id()))
        .unwrap_or(Palette::DEFAULT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_live_palette_wins_then_the_blocks_then_the_decks() {
        let ctx = egui::Context::default();
        assert_eq!(palette_for(&ctx, None), Palette::Iron);
        set_deck_palette(&ctx, Some(Palette::Lava));
        assert_eq!(palette_for(&ctx, None), Palette::Lava);
        assert_eq!(palette_for(&ctx, Some(Palette::Arctic)), Palette::Arctic);
        set_live_palette(&ctx, Some(Palette::WhiteHot));
        assert_eq!(palette_for(&ctx, Some(Palette::Arctic)), Palette::WhiteHot);
        set_live_palette(&ctx, None);
        assert_eq!(palette_for(&ctx, Some(Palette::Arctic)), Palette::Arctic);
    }
}
