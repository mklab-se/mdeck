//! Bundled fonts.
//!
//! egui ships a single sans family. The Ember theme needs an editorial serif
//! for display headings, a precise grotesque for body copy and a monospace for
//! eyebrows and code, so those faces are embedded in the binary and registered
//! under the family names in [`crate::theme`]. All three families are licensed
//! under the SIL Open Font License (see `fonts/OFL-*.txt`).
//!
//! Two symbol faces close every family's fallback chain, in every theme, so
//! glyphs the text faces and egui's defaults lack draw instead of showing as
//! boxes: Noto Sans Symbols (OFL, `fonts/OFL-NotoSansSymbols.txt`) for the
//! enclosed alphanumerics (①②③, ⓐ) and letterlike symbols, and DejaVu Sans
//! (Bitstream Vera licence, `fonts/LICENSE-DejaVu.txt`) for arrows, geometric
//! shapes, dingbats and a broad sweep of everything else.

use std::sync::Arc;

use eframe::egui::{self, FontData, FontDefinitions, FontFamily};

use crate::theme::{FONT_BODY, FONT_BODY_LIGHT, FONT_BODY_MEDIUM, FONT_DISPLAY, FONT_MONO};

static SPECTRAL_LIGHT: &[u8] = include_bytes!("../../fonts/Spectral-Light.ttf");
static HANKEN_LIGHT: &[u8] = include_bytes!("../../fonts/HankenGrotesk-Light.ttf");
static HANKEN_REGULAR: &[u8] = include_bytes!("../../fonts/HankenGrotesk-Regular.ttf");
static HANKEN_MEDIUM: &[u8] = include_bytes!("../../fonts/HankenGrotesk-Medium.ttf");
static JETBRAINS_REGULAR: &[u8] = include_bytes!("../../fonts/JetBrainsMono-Regular.ttf");
static NOTO_SYMBOLS: &[u8] = include_bytes!("../../fonts/NotoSansSymbols.ttf");
static DEJAVU_SANS: &[u8] = include_bytes!("../../fonts/DejaVuSans.ttf");

/// The symbol fallback faces, in the order they are tried.
const FONT_SYMBOLS: [&str; 2] = ["NotoSansSymbols", "DejaVuSans"];

/// Register the bundled families on `ctx`, keeping egui's defaults as
/// fallbacks so glyphs missing from a face (symbols, emoji) still render.
pub fn install(ctx: &egui::Context) {
    let mut defs = FontDefinitions::default();

    let faces: [(&str, &'static [u8]); 7] = [
        ("Spectral-Light", SPECTRAL_LIGHT),
        ("HankenGrotesk-Light", HANKEN_LIGHT),
        ("HankenGrotesk-Regular", HANKEN_REGULAR),
        ("HankenGrotesk-Medium", HANKEN_MEDIUM),
        ("JetBrainsMono-Regular", JETBRAINS_REGULAR),
        (FONT_SYMBOLS[0], NOTO_SYMBOLS),
        (FONT_SYMBOLS[1], DEJAVU_SANS),
    ];
    for (name, bytes) in faces {
        defs.font_data
            .insert(name.to_string(), Arc::new(FontData::from_static(bytes)));
    }

    // Everything egui already had in its proportional family serves as
    // fallback, and the symbol faces close the chain (also for egui's own
    // families, which the light, dark and nord themes draw with).
    for fam in [FontFamily::Proportional, FontFamily::Monospace] {
        defs.families
            .entry(fam)
            .or_default()
            .extend(FONT_SYMBOLS.iter().map(|s| s.to_string()));
    }
    let proportional_fallback: Vec<String> = defs
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let monospace_fallback: Vec<String> = defs
        .families
        .get(&FontFamily::Monospace)
        .cloned()
        .unwrap_or_default();

    let family = |primary: &str, fallback: &[String]| -> Vec<String> {
        let mut v = vec![primary.to_string()];
        v.extend(fallback.iter().cloned());
        v
    };

    defs.families.insert(
        FontFamily::Name(FONT_DISPLAY.into()),
        family("Spectral-Light", &proportional_fallback),
    );
    defs.families.insert(
        FontFamily::Name(FONT_BODY.into()),
        family("HankenGrotesk-Regular", &proportional_fallback),
    );
    defs.families.insert(
        FontFamily::Name(FONT_BODY_LIGHT.into()),
        family("HankenGrotesk-Light", &proportional_fallback),
    );
    defs.families.insert(
        FontFamily::Name(FONT_BODY_MEDIUM.into()),
        family("HankenGrotesk-Medium", &proportional_fallback),
    );
    defs.families.insert(
        FontFamily::Name(FONT_MONO.into()),
        family("JetBrainsMono-Regular", &monospace_fallback),
    );

    ctx.set_fonts(defs);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression for GitHub issue 7: circled numbers (and other common
    /// symbols) must resolve to real glyphs in every family and theme.
    #[test]
    fn symbols_have_glyphs_in_every_family() {
        let ctx = egui::Context::default();
        install(&ctx);
        let families = [
            FontFamily::Proportional,
            FontFamily::Monospace,
            FontFamily::Name(FONT_DISPLAY.into()),
            FontFamily::Name(FONT_BODY.into()),
            FontFamily::Name(FONT_MONO.into()),
        ];
        let mut output = ctx.run_ui(Default::default(), |ui| {
            ui.fonts_mut(|f| {
                for fam in &families {
                    let id = egui::FontId::new(24.0, fam.clone());
                    for s in ["①②③⑩⑪⑳", "ⓐⓩ⒜"] {
                        assert!(f.has_glyphs(&id, s), "{fam:?} lacks glyphs for {s}");
                    }
                }
                // Hack, the first monospace face, already carries arrows and
                // shapes, and egui's `has_glyph` gives a false negative for
                // the face it also takes the replacement glyph from; the
                // proportional families prove the rest of the chain.
                for fam in [
                    FontFamily::Proportional,
                    FontFamily::Name(FONT_DISPLAY.into()),
                    FontFamily::Name(FONT_BODY.into()),
                ] {
                    let id = egui::FontId::new(24.0, fam.clone());
                    for s in ["✓✗", "→←↑↓", "■□●○◆", "★☆"] {
                        assert!(f.has_glyphs(&id, s), "{fam:?} lacks glyphs for {s}");
                    }
                }
            });
        });
        output.textures_delta.clear();
    }

    #[test]
    fn bundled_faces_parse_and_families_resolve() {
        let ctx = egui::Context::default();
        install(&ctx);
        // Laying out text in each family must not panic (unknown families do).
        let mut output = ctx.run_ui(Default::default(), |ui| {
            for name in [
                FONT_DISPLAY,
                FONT_BODY,
                FONT_BODY_LIGHT,
                FONT_BODY_MEDIUM,
                FONT_MONO,
            ] {
                let id = egui::FontId::new(40.0, FontFamily::Name(name.into()));
                let galley = ui
                    .painter()
                    .layout_no_wrap("Ember".into(), id, egui::Color32::WHITE);
                assert!(galley.rect.width() > 0.0, "{name} produced no glyphs");
            }
        });
        output.textures_delta.clear();
    }
}
