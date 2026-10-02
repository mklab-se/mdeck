//! The heat palettes live with the engines
//! ([`crate::engines::heat_palette`]), which the thermal engine's field
//! shares; the `@thermal` visual draws them in egui colours.

use eframe::egui::Color32;

pub use crate::engines::heat_palette::Palette;

/// `palette` as 256 egui colours, from cold to hot.
pub fn lut(palette: Palette) -> [Color32; 256] {
    palette.lut().map(mdeck_sdk::host::color32)
}

/// The colour at `t` (0 cold .. 1 hot).
#[cfg(test)]
pub fn at(palette: Palette, t: f32) -> Color32 {
    mdeck_sdk::host::color32(palette.at(t))
}
