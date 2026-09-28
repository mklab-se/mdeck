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

//! Chinese, Japanese and Korean come from system faces found at startup,
//! see [`cjk`].

use std::sync::Arc;

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use eframe::egui::{self, FontData, FontDefinitions, FontFamily};

use crate::theme::{FONT_BODY, FONT_BODY_LIGHT, FONT_BODY_MEDIUM, FONT_DISPLAY, FONT_MONO};

pub mod cjk;
pub use cjk::{CJK_FONT_ENV, Scripts, cjk_coverage};

static SPECTRAL_LIGHT: &[u8] = include_bytes!("../../../fonts/Spectral-Light.ttf");
static HANKEN_LIGHT: &[u8] = include_bytes!("../../../fonts/HankenGrotesk-Light.ttf");
static HANKEN_REGULAR: &[u8] = include_bytes!("../../../fonts/HankenGrotesk-Regular.ttf");
static HANKEN_MEDIUM: &[u8] = include_bytes!("../../../fonts/HankenGrotesk-Medium.ttf");
static JETBRAINS_REGULAR: &[u8] = include_bytes!("../../../fonts/JetBrainsMono-Regular.ttf");
static NOTO_SYMBOLS: &[u8] = include_bytes!("../../../fonts/NotoSansSymbols.ttf");
static DEJAVU_SANS: &[u8] = include_bytes!("../../../fonts/DejaVuSans.ttf");

/// The KaTeX faces RaTeX lays math out with (OFL, `fonts/katex/OFL-KaTeX.txt`),
/// by the font name its display list uses. Each is its own family,
/// `katex-<name>`, falling back to the proportional chain.
pub const KATEX_FACES: [(&str, &[u8]); 19] = [
    (
        "AMS-Regular",
        include_bytes!("../../../fonts/katex/KaTeX_AMS-Regular.ttf"),
    ),
    (
        "Caligraphic-Regular",
        include_bytes!("../../../fonts/katex/KaTeX_Caligraphic-Regular.ttf"),
    ),
    (
        "Fraktur-Regular",
        include_bytes!("../../../fonts/katex/KaTeX_Fraktur-Regular.ttf"),
    ),
    (
        "Fraktur-Bold",
        include_bytes!("../../../fonts/katex/KaTeX_Fraktur-Bold.ttf"),
    ),
    (
        "Main-Bold",
        include_bytes!("../../../fonts/katex/KaTeX_Main-Bold.ttf"),
    ),
    (
        "Main-BoldItalic",
        include_bytes!("../../../fonts/katex/KaTeX_Main-BoldItalic.ttf"),
    ),
    (
        "Main-Italic",
        include_bytes!("../../../fonts/katex/KaTeX_Main-Italic.ttf"),
    ),
    (
        "Main-Regular",
        include_bytes!("../../../fonts/katex/KaTeX_Main-Regular.ttf"),
    ),
    (
        "Math-BoldItalic",
        include_bytes!("../../../fonts/katex/KaTeX_Math-BoldItalic.ttf"),
    ),
    (
        "Math-Italic",
        include_bytes!("../../../fonts/katex/KaTeX_Math-Italic.ttf"),
    ),
    (
        "SansSerif-Bold",
        include_bytes!("../../../fonts/katex/KaTeX_SansSerif-Bold.ttf"),
    ),
    (
        "SansSerif-Italic",
        include_bytes!("../../../fonts/katex/KaTeX_SansSerif-Italic.ttf"),
    ),
    (
        "SansSerif-Regular",
        include_bytes!("../../../fonts/katex/KaTeX_SansSerif-Regular.ttf"),
    ),
    (
        "Script-Regular",
        include_bytes!("../../../fonts/katex/KaTeX_Script-Regular.ttf"),
    ),
    (
        "Size1-Regular",
        include_bytes!("../../../fonts/katex/KaTeX_Size1-Regular.ttf"),
    ),
    (
        "Size2-Regular",
        include_bytes!("../../../fonts/katex/KaTeX_Size2-Regular.ttf"),
    ),
    (
        "Size3-Regular",
        include_bytes!("../../../fonts/katex/KaTeX_Size3-Regular.ttf"),
    ),
    (
        "Size4-Regular",
        include_bytes!("../../../fonts/katex/KaTeX_Size4-Regular.ttf"),
    ),
    (
        "Typewriter-Regular",
        include_bytes!("../../../fonts/katex/KaTeX_Typewriter-Regular.ttf"),
    ),
];

/// The symbol fallback faces, in the order they are tried.
const FONT_SYMBOLS: [&str; 2] = ["NotoSansSymbols", "DejaVuSans"];

/// A font file a theme ships, registered as its own family.
struct ThemeFace {
    family: String,
    bytes: Arc<[u8]>,
    mono: bool,
}

/// Faces from theme files, added by [`register_file_face`] and installed on
/// every context by [`install`]. `GENERATION` counts additions, so a context
/// can tell it needs new fonts.
static THEME_FACES: Mutex<Vec<ThemeFace>> = Mutex::new(Vec::new());
static GENERATION: AtomicU64 = AtomicU64::new(0);

/// Check `bytes` parse as a TTF/OTF font (egui panics on a broken one) and
/// register them as a family. The same file registers once.
pub fn register_file_face(bytes: Vec<u8>, mono: bool) -> anyhow::Result<FontFamily> {
    use skrifa::MetadataProvider;
    let font = skrifa::FontRef::from_index(&bytes, 0)
        .map_err(|e| anyhow::anyhow!("not a readable TTF/OTF font ({e})"))?;
    if font.charmap().map('a').is_none() {
        anyhow::bail!("the font has no Latin letters");
    }
    let mut h = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(&bytes, &mut h);
    let family = format!("theme-face-{:016x}", std::hash::Hasher::finish(&h));
    let mut faces = THEME_FACES.lock().unwrap_or_else(|p| p.into_inner());
    if !faces.iter().any(|f| f.family == family) {
        faces.push(ThemeFace {
            family: family.clone(),
            bytes: bytes.into(),
            mono,
        });
        GENERATION.fetch_add(1, Ordering::SeqCst);
    }
    Ok(FontFamily::Name(family.into()))
}

/// How many theme faces have been registered so far.
pub fn generation() -> u64 {
    GENERATION.load(Ordering::SeqCst)
}

/// Keeps a context's fonts in step with the theme faces. egui applies
/// `set_fonts` at the start of the next pass, so a face registered now can
/// be drawn with from the frame after [`FontSync::sync`] saw it.
#[derive(Debug, Default)]
pub struct FontSync {
    requested: u64,
    active: u64,
}

impl FontSync {
    /// A sync for a context `install` has just run on.
    pub fn installed() -> Self {
        let g = generation();
        FontSync {
            requested: g,
            active: g,
        }
    }

    /// Call at the start of every frame: fonts asked for last frame are
    /// active now, and newly registered faces are asked for.
    pub fn sync(&mut self, ctx: &egui::Context) {
        self.active = self.requested;
        let g = generation();
        if g != self.requested {
            install(ctx);
            self.requested = g;
        }
    }

    /// Whether every face registered so far can be drawn this frame.
    pub fn ready(&self) -> bool {
        self.active == generation()
    }
}

/// Register the bundled families (and every theme face) on `ctx`, keeping
/// egui's defaults as fallbacks so glyphs missing from a face (symbols,
/// emoji) still render.
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
    // fallback, and the symbol faces come next (also for egui's own
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

    // Theme faces: each its own family over the matching fallback chain.
    for face in THEME_FACES.lock().unwrap_or_else(|p| p.into_inner()).iter() {
        defs.font_data.insert(
            face.family.clone(),
            Arc::new(FontData::from_owned(face.bytes.to_vec())),
        );
        let fallback = if face.mono {
            &monospace_fallback
        } else {
            &proportional_fallback
        };
        defs.families.insert(
            FontFamily::Name(face.family.as_str().into()),
            family(&face.family, fallback),
        );
    }

    // Math: one family per KaTeX face, each falling back to the proportional
    // chain so `\text{...}` in any script still draws.
    for (name, bytes) in KATEX_FACES {
        let key = format!("KaTeX_{name}");
        defs.font_data
            .insert(key.clone(), Arc::new(FontData::from_static(bytes)));
        defs.families.insert(
            FontFamily::Name(format!("katex-{name}").into()),
            family(&key, &proportional_fallback),
        );
    }

    // The system CJK faces, when there are any, close every chain.
    let families: Vec<FontFamily> = defs.families.keys().cloned().collect();
    for fam in &families {
        cjk::add_faces(&mut defs, fam);
    }

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
    fn theme_font_files_are_checked_before_egui_sees_them() {
        assert!(register_file_face(b"not a font".to_vec(), false).is_err());
        let fam = register_file_face(HANKEN_MEDIUM.to_vec(), false).unwrap();
        // The same bytes register once.
        assert_eq!(
            register_file_face(HANKEN_MEDIUM.to_vec(), false).unwrap(),
            fam
        );
        let ctx = egui::Context::default();
        install(&ctx);
        let mut output = ctx.run_ui(Default::default(), |ui| {
            let id = egui::FontId::new(40.0, fam.clone());
            let g = ui
                .painter()
                .layout_no_wrap("Theme".into(), id, egui::Color32::WHITE);
            assert!(g.rect.width() > 0.0);
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
