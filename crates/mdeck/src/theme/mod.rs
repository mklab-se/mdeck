//! Themes are data. Each one is a `theme.yaml` (see [`file`]); the four
//! built-in themes are embedded files in the same format. What a theme
//! *does* beyond colours and type (a particle field, editorial layouts,
//! pictures) comes from its engine ([`EngineId`], see `crate::engines`),
//! looked up by name in the registry.

use std::path::PathBuf;

use eframe::egui::{self, Color32};

pub mod arrangement;
mod build;
mod color;
mod error;
pub mod file;
pub mod lookup;
mod paths;
pub mod schema;
pub mod spacing;
pub mod validate;

use color::mix;
pub use color::{contrast, luminance};
pub use error::ThemeError;
pub use paths::confined_path;

pub use crate::engines::EngineId;

/// The slide as a sheet on a surface: paper on a desk, a board on a wall.
/// The sheet is the theme's background; sizes are px on a 1920x1080 slide.
/// The thermal engine's heat field: the palette it glows in and whether
/// embers drift on ordinary slides.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Heat {
    pub palette: crate::render::thermal::Palette,
    pub drift: bool,
}

impl Default for Heat {
    fn default() -> Self {
        Heat {
            palette: crate::render::thermal::Palette::Iron,
            drift: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    pub surface: Color32,
    pub margin: f32,
    /// 0 to 1.
    pub shadow: f32,
    /// 0 to 1.
    pub grain: f32,
    pub radius: f32,
}

/// A theme's say in generated artwork (`art:` in the theme file).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ThemeArt {
    pub kind: Option<crate::render::art::ArtKind>,
    pub style: Option<String>,
    pub references: Vec<PathBuf>,
}

/// Font family per role. Roles, not weights: egui draws one face per family.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeFonts {
    /// Headings and big moments.
    pub display: egui::FontFamily,
    /// Body text and list items.
    pub body: egui::FontFamily,
    /// Lead paragraphs in the particles engine's copy column (defaults to `body`).
    pub lead: egui::FontFamily,
    /// `**bold**` runs (defaults to `body`; bold is also brighter).
    pub strong: egui::FontFamily,
    /// Code, labels and eyebrows.
    pub mono: egui::FontFamily,
}

/// A resolved theme: every colour, size and face a renderer needs.
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    /// The engine; change it with [`Theme::set_engine`].
    pub engine: EngineId,
    /// Seconds the copy of title and section slides holds back while the
    /// engine forms the heading ([`mdeck_sdk::engine::Engine::copy_hold`]).
    pub copy_hold: f32,
    /// The engine prints the slide number itself
    /// ([`mdeck_sdk::engine::Engine::numbers_slides`]).
    engine_numbers_slides: bool,
    /// Whether decks in this theme open with the 3-2-1 countdown (`countdown:
    /// on|off`; a deck's own `countdown` wins). The engine decides its look.
    pub countdown: bool,
    /// The theme's slide transition (`slide`, `fade`, `spatial`, `none`), if
    /// it names one. A deck's own `transition` wins; the user config and the
    /// built-in `fade` come after.
    pub transition: Option<String>,
    /// The engine block's settings as written (every key but `name`), for
    /// [`engine_settings`].
    pub engine_block: Vec<(String, serde_norway::Value)>,
    /// How every design looks: the design set (`designs:`) with the theme's
    /// `arrangements:` applied.
    pub arrangements: std::sync::Arc<arrangement::Arrangements>,
    /// The spacing scale arrangements name.
    pub spacing: spacing::Spacing,
    /// Corner radius of cards, px at 1920x1080.
    pub radius: f32,
    pub background: Color32,
    pub foreground: Color32,
    pub heading_color: Color32,
    /// Captions, footers, eyebrows and slide numbers.
    pub muted: Color32,
    /// Colour of `**bold**` runs.
    pub strong: Color32,
    /// Hairlines and inactive markers.
    pub rule: Color32,
    pub accent: Color32,
    /// A lighter accent: emphasis, links and glows.
    pub accent_soft: Color32,
    /// A warm secondary for small highlights.
    pub secondary: Color32,
    pub code_background: Color32,
    pub code_foreground: Color32,
    pub positive: Color32,
    pub negative: Color32,
    /// Chart and diagram series, cycled to fill eight slots.
    pub series: [Color32; EDGE_PALETTE_LEN],
    pub pen: Color32,
    pub pen_outline: Color32,
    pub arrow: Color32,
    pub arrow_outline: Color32,
    /// Particle tints for the particles engine besides the accents. Every
    /// build reads them from theme files; only that engine uses them.
    #[cfg_attr(
        not(all_engines),
        allow(dead_code, reason = "read by the particles engine")
    )]
    pub particle_light: Color32,
    #[cfg_attr(
        not(all_engines),
        allow(dead_code, reason = "read by the particles engine")
    )]
    pub particle_cool: Color32,
    pub fonts: ThemeFonts,
    pub h1_size: f32,
    pub h2_size: f32,
    pub h3_size: f32,
    pub body_size: f32,
    pub code_size: f32,
    /// Line height as a multiple of the font size, when the theme sets one.
    pub line_height: Option<f32>,
    /// Opacity of filled chart shapes.
    pub fill_opacity: f32,
    /// Syntax theme: a syntect default name or a registered `.tmTheme` key.
    pub syntax: String,
    /// A logo in a corner of every slide (a deck's `logo` overrides it).
    pub logo: Option<crate::render::logo::Logo>,
    /// The slide as a sheet on a surface (`page:`); `None` fills the window.
    pub page: Option<Page>,
    /// What generated artwork looks like (`engine.kind`, `engine.style`,
    /// `engine.references`), over the engine's own style.
    pub art: ThemeArt,
    /// The file this theme was read from (`None` for built-ins).
    pub source: Option<PathBuf>,
}

/// Font families installed by [`crate::render::fonts::install`]. Themes
/// name them by the bundled face names in [`BUNDLED_FACES`].
pub const FONT_DISPLAY: &str = "mdeck-display";
pub const FONT_BODY: &str = "mdeck-body";
pub const FONT_BODY_LIGHT: &str = "mdeck-body-light";
pub const FONT_BODY_MEDIUM: &str = "mdeck-body-medium";
pub const FONT_MONO: &str = "mdeck-mono";

/// Faces a theme can name without shipping a file.
pub const BUNDLED_FACES: [(&str, &str); 7] = [
    ("sans", "egui's default sans (Ubuntu Light)"),
    ("mono", "egui's default monospace (Hack)"),
    ("spectral-light", "Spectral Light, an editorial serif"),
    ("hanken-light", "Hanken Grotesk Light"),
    ("hanken-regular", "Hanken Grotesk Regular"),
    ("hanken-medium", "Hanken Grotesk Medium"),
    ("jetbrains-mono", "JetBrains Mono Regular"),
];

/// Number of colors in every edge palette.
pub const EDGE_PALETTE_LEN: usize = 8;

/// A theme built from a file, plus what was wrong with it but survivable
/// (a missing font falls back, for example).
#[derive(Debug, Clone)]
pub struct Built {
    pub theme: Theme,
    pub warnings: Vec<String>,
}

/// The settings of the theme's `engine:` block (every key but `name`), as
/// the engine reads them (THM-11). Seam for the engines: an engine reads
/// its settings from here, never from theme sections named after it.
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "the engines read it once they run on the SDK (phase 2b)"
    )
)]
/// Keys of the `engine:` block the core reads itself (the particle tints
/// become tokens, the art keys steer generated art); the engine never sees
/// them.
pub const CORE_ENGINE_KEYS: [&str; 5] = ["light", "cool", "kind", "style", "references"];

pub fn engine_settings(theme: &Theme) -> mdeck_sdk::tokens::EngineSettings {
    mdeck_sdk::tokens::EngineSettings::from_pairs(
        theme
            .engine_block
            .iter()
            .filter(|(k, _)| !CORE_ENGINE_KEYS.contains(&k.as_str()))
            .map(|(k, v)| (k.clone(), sdk_value(v))),
    )
}

/// A YAML value as the SDK's library-free [`mdeck_sdk::tokens::Value`].
fn sdk_value(v: &serde_norway::Value) -> mdeck_sdk::tokens::Value {
    use mdeck_sdk::tokens::Value as V;
    use serde_norway::Value as Y;
    match v {
        Y::Null => V::Null,
        Y::Bool(b) => V::Bool(*b),
        Y::Number(n) => V::Number(n.as_f64().unwrap_or(0.0)),
        Y::String(s) => V::String(s.clone()),
        Y::Sequence(items) => V::List(items.iter().map(sdk_value).collect()),
        Y::Mapping(map) => V::Map(
            map.iter()
                .map(|(k, v)| (k.as_str().unwrap_or_default().to_string(), sdk_value(v)))
                .collect(),
        ),
        Y::Tagged(t) => sdk_value(&t.value),
    }
}

/// Whether `theme` arranges slides with the editorial design set (seam for
/// the engines: the set comes from the theme, never from the engine).
pub fn uses_editorial(theme: &Theme) -> bool {
    theme.arrangements.is_editorial()
}

impl Theme {
    /// The arrangement of `design` in this theme.
    pub fn arrangement(&self, design: crate::parser::Design) -> &arrangement::Arrangement {
        self.arrangements.get(design)
    }

    /// The engine prints the slide number itself (the line engine's sheet,
    /// in its title block), so the editorial counter is left out. A slate
    /// is a board, not a numbered sheet.
    pub fn numbers_slides(&self) -> bool {
        self.engine_numbers_slides
    }

    /// Run on `engine`, and learn from a runtime made with this theme's
    /// engine settings what the core must do around it: hold the copy of
    /// title slides back, leave out the counter.
    pub fn set_engine(&mut self, engine: EngineId) {
        self.engine = engine;
        let settings = engine_settings(self);
        let runtime = (engine.def().create)(&settings);
        self.copy_hold = runtime.copy_hold();
        self.engine_numbers_slides = runtime.numbers_slides();
    }

    /// A built-in theme by name. Panics only if an embedded file is broken,
    /// which the tests rule out.
    fn builtin(name: &str) -> Self {
        lookup::load_builtin(name)
            .unwrap_or_else(|e| panic!("built-in theme {name} is invalid: {e}"))
    }

    pub fn dark() -> Self {
        Self::builtin("dark")
    }

    pub fn light() -> Self {
        Self::builtin("light")
    }

    #[cfg(test)]
    pub fn nord() -> Self {
        Self::builtin("nord")
    }

    #[cfg(all(test, feature = "particles"))]
    pub fn ember() -> Self {
        Self::builtin("ember")
    }

    /// Opacity of filled chart shapes.
    pub fn fill_opacity(&self) -> f32 {
        self.fill_opacity
    }

    /// Font family for display headings.
    pub fn display_family(&self) -> egui::FontFamily {
        self.fonts.display.clone()
    }

    /// Font family for body text.
    pub fn body_family(&self) -> egui::FontFamily {
        self.fonts.body.clone()
    }

    /// Font family for lead paragraphs (the particles copy column).
    pub fn lead_family(&self) -> egui::FontFamily {
        self.fonts.lead.clone()
    }

    /// Font family for bold runs.
    pub fn strong_family(&self) -> egui::FontFamily {
        self.fonts.strong.clone()
    }

    /// Font family for code and labels.
    pub fn mono_family(&self) -> egui::FontFamily {
        self.fonts.mono.clone()
    }

    pub fn with_opacity(color: Color32, opacity: f32) -> Color32 {
        Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), (opacity * 255.0) as u8)
    }

    pub fn heading_size(&self, level: u8) -> f32 {
        match level {
            1 => self.h1_size,
            2 => self.h2_size,
            3 => self.h3_size,
            _ => self.body_size,
        }
    }

    /// The syntax theme code blocks are highlighted with.
    pub fn syntect_theme_name(&self) -> &str {
        &self.syntax
    }

    /// Positive trend colour (KPI deltas).
    pub fn positive_color(&self) -> Color32 {
        self.positive
    }

    /// Negative trend colour (KPI deltas).
    pub fn negative_color(&self) -> Color32 {
        self.negative
    }

    /// Distinct colours for diagram edges and visualizations.
    ///
    /// Returns a copy of a small array (a few bytes on the stack): this is
    /// called from hot render paths every frame, so it must not allocate.
    pub fn edge_palette(&self) -> [Color32; EDGE_PALETTE_LEN] {
        self.series
    }

    /// Body text one step brighter, for list items in the particles copy column.
    /// A light page (engines that add light switch to ink on one).
    #[cfg_attr(
        not(all_engines),
        allow(dead_code, reason = "asked by the engines that draw light")
    )]
    pub fn is_light(&self) -> bool {
        luminance(self.background) > 0.5
    }

    pub fn bright_text(&self) -> Color32 {
        mix(self.foreground, self.heading_color, 0.6)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_settings_come_from_the_engine_block() {
        // `plain`, which every build has: a missing engine's settings are dropped.
        let f = file::ThemeFile::parse(
            "engine: { name: plain, palette: lava, drift: true, extra: [1, x] }",
        )
        .unwrap()
        .over(&lookup::builtin_file("dark").unwrap());
        let t = Theme::build("x", &f).unwrap().theme;
        let s = engine_settings(&t);
        assert_eq!(s.str("palette"), Some("lava"));
        assert_eq!(s.bool("drift"), Some(true));
        assert!(s.get("name").is_none());
        assert_eq!(
            s.get("extra"),
            Some(&mdeck_sdk::tokens::Value::List(vec![
                mdeck_sdk::tokens::Value::Number(1.0),
                mdeck_sdk::tokens::Value::String("x".into())
            ]))
        );
        assert!(engine_settings(&Theme::dark()).is_empty());
    }

    #[test]
    fn edge_palette_is_distinct_per_theme_and_stable() {
        let dark = Theme::dark().edge_palette();
        let light = Theme::light().edge_palette();
        let nord = Theme::nord().edge_palette();
        assert_eq!(dark.len(), EDGE_PALETTE_LEN);
        assert_ne!(dark[0], light[0]);
        assert_ne!(dark[0], nord[0]);
        assert_eq!(Theme::dark().edge_palette(), dark);
        fn takes_slice(p: &[Color32]) -> usize {
            p.len()
        }
        assert_eq!(takes_slice(&nord), EDGE_PALETTE_LEN);
    }

    /// The built-ins moved from Rust constructors to embedded files; these
    /// are the values the constructors had, so nothing on screen changes.
    #[cfg(feature = "particles")]
    #[test]
    fn builtins_keep_their_previous_values() {
        let d = Theme::dark();
        assert_eq!(d.background, Color32::from_rgb(0x1E, 0x1E, 0x1E));
        assert_eq!(d.heading_color, Color32::WHITE);
        assert_eq!(d.accent, Color32::from_rgb(0x52, 0x94, 0xE2));
        assert_eq!((d.h1_size, d.body_size, d.code_size), (96.0, 44.0, 30.0));
        assert_eq!(d.positive, Color32::from_rgb(0x5C, 0xDB, 0x95));
        assert_eq!(d.syntax, "base16-ocean.dark");
        assert_eq!(d.fill_opacity, 0.85);
        assert_eq!(d.engine, EngineId::plain());
        assert!(!d.countdown);
        assert_eq!(d.transition, None);
        assert_eq!(d.fonts.display, egui::FontFamily::Proportional);
        assert_eq!(d.strong, d.heading_color);
        assert_eq!(d.line_height, None);

        let l = Theme::light();
        assert_eq!(l.background, Color32::WHITE);
        assert_eq!(l.syntax, "InspiredGitHub");
        assert_eq!(l.strong, l.accent);
        assert_eq!(l.pen, Color32::from_rgb(30, 80, 200));
        assert_eq!(l.negative, Color32::from_rgb(0xB9, 0x2D, 0x2D));

        let n = Theme::nord();
        assert_eq!(n.background, Color32::from_rgb(0x2E, 0x34, 0x40));
        assert!(n.countdown);
        assert_eq!(n.strong, n.accent);
        // Nord used to get the light theme's pen colours on its dark ground.
        assert_eq!(n.pen, d.pen);

        let e = Theme::ember();
        assert_eq!(e.background, Color32::from_rgb(5, 5, 5));
        assert_eq!(e.accent, Color32::from_rgb(0xFF, 0x4D, 0x1C));
        assert_eq!(e.accent_soft, Color32::from_rgb(0xFF, 0x8A, 0x66));
        assert_eq!(e.muted, Color32::from_rgb(0x8F, 0x8F, 0x98));
        assert_eq!(e.rule, Color32::from_rgb(0x24, 0x24, 0x29));
        assert_eq!((e.h1_size, e.body_size), (104.0, 38.0));
        assert_eq!(e.line_height, Some(1.45));
        assert_eq!(e.fill_opacity, 0.80);
        assert_eq!(e.fonts.display, egui::FontFamily::Name(FONT_DISPLAY.into()));
        assert_eq!(
            e.fonts.strong,
            egui::FontFamily::Name(FONT_BODY_MEDIUM.into())
        );
        assert_eq!(e.fonts.lead, egui::FontFamily::Name(FONT_BODY_LIGHT.into()));
        assert_eq!(e.fonts.mono, egui::FontFamily::Name(FONT_MONO.into()));
        assert_eq!(e.strong, e.heading_color);
        if cfg!(feature = "particles") {
            assert_eq!(Some(e.engine), EngineId::find("particles"));
            assert!(e.countdown);
        }
    }
}
