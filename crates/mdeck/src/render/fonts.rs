//! Bundled fonts.
//!
//! egui ships a single sans family. The Ember theme needs an editorial serif
//! for display headings, a precise grotesque for body copy and a monospace for
//! eyebrows and code, so those faces are embedded in the binary and registered
//! under the family names in [`crate::theme`]. All three families are licensed
//! under the SIL Open Font License (see `fonts/OFL-*.txt`).

use std::sync::Arc;

use eframe::egui::{self, FontData, FontDefinitions, FontFamily};

use crate::theme::{FONT_BODY, FONT_BODY_LIGHT, FONT_BODY_MEDIUM, FONT_DISPLAY, FONT_MONO};

static SPECTRAL_LIGHT: &[u8] = include_bytes!("../../fonts/Spectral-Light.ttf");
static HANKEN_LIGHT: &[u8] = include_bytes!("../../fonts/HankenGrotesk-Light.ttf");
static HANKEN_REGULAR: &[u8] = include_bytes!("../../fonts/HankenGrotesk-Regular.ttf");
static HANKEN_MEDIUM: &[u8] = include_bytes!("../../fonts/HankenGrotesk-Medium.ttf");
static JETBRAINS_REGULAR: &[u8] = include_bytes!("../../fonts/JetBrainsMono-Regular.ttf");

/// Register the bundled families on `ctx`, keeping egui's defaults as
/// fallbacks so glyphs missing from a face (symbols, emoji) still render.
pub fn install(ctx: &egui::Context) {
    let mut defs = FontDefinitions::default();

    let faces: [(&str, &'static [u8]); 5] = [
        ("Spectral-Light", SPECTRAL_LIGHT),
        ("HankenGrotesk-Light", HANKEN_LIGHT),
        ("HankenGrotesk-Regular", HANKEN_REGULAR),
        ("HankenGrotesk-Medium", HANKEN_MEDIUM),
        ("JetBrainsMono-Regular", JETBRAINS_REGULAR),
    ];
    for (name, bytes) in faces {
        defs.font_data
            .insert(name.to_string(), Arc::new(FontData::from_static(bytes)));
    }

    // Everything egui already had in its proportional family serves as fallback.
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
