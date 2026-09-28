//! Themes are data. Each one is a `theme.yaml` (see [`file`]); the four
//! built-in themes are embedded files in the same format. What a theme
//! *does* beyond colours and type (a particle field, editorial layouts,
//! story beats) comes from its engine ([`EngineKind`], see `crate::engines`),
//! which MDeck provides.

use std::path::{Path, PathBuf};

use eframe::egui::{self, Color32};

pub mod file;
pub mod lookup;
pub mod validate;

use file::{ThemeFile, parse_color};

pub use crate::engines::EngineKind;

/// The opening countdown before the first slide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Countdown {
    None,
    /// Numerals on the bare background.
    Plain,
    /// The engine's own countdown (particle numerals that burst into the
    /// first slide on the particles engine).
    Burst,
}

impl Countdown {
    pub fn name(self) -> &'static str {
        match self {
            Countdown::None => "none",
            Countdown::Plain => "plain",
            Countdown::Burst => "burst",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "none" => Some(Countdown::None),
            "plain" => Some(Countdown::Plain),
            "burst" => Some(Countdown::Burst),
            _ => None,
        }
    }
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
    pub engine: EngineKind,
    pub countdown: Countdown,
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
    /// Particle tints for the particles engine besides the accents.
    pub particle_light: Color32,
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
    /// A logo in a corner of every slide (a deck's `@logo` overrides it).
    pub logo: Option<crate::render::logo::Logo>,
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

fn bundled_family(name: &str) -> Option<egui::FontFamily> {
    Some(match name {
        "sans" => egui::FontFamily::Proportional,
        "mono" => egui::FontFamily::Monospace,
        "spectral-light" => egui::FontFamily::Name(FONT_DISPLAY.into()),
        "hanken-light" => egui::FontFamily::Name(FONT_BODY_LIGHT.into()),
        "hanken-regular" => egui::FontFamily::Name(FONT_BODY.into()),
        "hanken-medium" => egui::FontFamily::Name(FONT_BODY_MEDIUM.into()),
        "jetbrains-mono" => egui::FontFamily::Name(FONT_MONO.into()),
        _ => return None,
    })
}

/// Number of colors in every edge palette.
pub const EDGE_PALETTE_LEN: usize = 8;

/// A theme built from a file, plus what was wrong with it but survivable
/// (a missing font falls back, for example).
#[derive(Debug, Clone)]
pub struct Built {
    pub theme: Theme,
    pub warnings: Vec<String>,
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

fn darken(c: Color32, f: f32) -> Color32 {
    mix(Color32::BLACK, c, f)
}

/// Relative luminance (0..1) of a colour, ignoring alpha (Rec. 709 weights
/// on the stored values; good enough to rank colours).
pub fn luminance(c: Color32) -> f32 {
    (0.2126 * c.r() as f32 + 0.7152 * c.g() as f32 + 0.0722 * c.b() as f32) / 255.0
}

/// WCAG 2 contrast ratio between two colours (1..21).
pub fn contrast(a: Color32, b: Color32) -> f32 {
    let lin = |v: u8| {
        let c = v as f32 / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let rl = |c: Color32| 0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b());
    let (x, y) = (rl(a), rl(b));
    let (hi, lo) = if x > y { (x, y) } else { (y, x) };
    (hi + 0.05) / (lo + 0.05)
}

/// Resolve a theme-relative path, refusing anything that leaves `base`.
pub fn confined_path(base: &Path, rel: &str) -> Result<PathBuf, String> {
    let p = Path::new(rel);
    if p.is_absolute() {
        return Err(format!("'{rel}' must be a path inside the theme folder"));
    }
    let joined = base.join(p);
    let real = joined
        .canonicalize()
        .map_err(|_| format!("'{rel}' was not found in {}", base.display()))?;
    let root = base.canonicalize().map_err(|e| e.to_string())?;
    if !real.starts_with(&root) {
        return Err(format!("'{rel}' points outside the theme folder"));
    }
    Ok(real)
}

impl Theme {
    /// Build a theme from a fully merged file whose font and syntax paths
    /// are already absolute and confined (see [`lookup`]). Invalid values
    /// are errors; a font or syntax file that cannot be used falls back
    /// with a warning.
    pub fn build(name: &str, f: &ThemeFile) -> Result<Built, String> {
        let mut warnings = Vec::new();
        let color = |key: &str, v: &Option<String>| -> Result<Option<Color32>, String> {
            match v {
                None => Ok(None),
                Some(s) => parse_color(s)
                    .map(|[r, g, b, a]| Some(Color32::from_rgba_unmultiplied(r, g, b, a)))
                    .ok_or_else(|| format!("colors.{key}: '{s}' is not a colour (use #rrggbb)")),
            }
        };
        let need = |key: &str, v: Option<Color32>| {
            v.ok_or_else(|| format!("colors.{key} is not set (and nothing it extends sets it)"))
        };
        let c = &f.colors;
        let background = need("background", color("background", &c.background)?)?;
        let foreground = need("text", color("text", &c.text)?)?;
        let heading_color = need("heading", color("heading", &c.heading)?)?;
        let accent = need("accent", color("accent", &c.accent)?)?;
        let code_background = need(
            "code-background",
            color("code-background", &c.code_background)?,
        )?;
        let code_foreground = need("code-text", color("code-text", &c.code_text)?)?;
        let positive = need("positive", color("positive", &c.positive)?)?;
        let negative = need("negative", color("negative", &c.negative)?)?;
        let muted = color("muted", &c.muted)?.unwrap_or(mix(background, foreground, 0.6));
        let rule = color("rule", &c.rule)?.unwrap_or(mix(background, foreground, 0.18));
        let strong = color("strong", &c.strong)?.unwrap_or(
            // The heading colour when it is visibly brighter than body text
            // (dark themes); otherwise the accent, so bold still stands out.
            if luminance(heading_color) - luminance(foreground) > 0.08 {
                heading_color
            } else {
                accent
            },
        );
        let accent_soft =
            color("accent-soft", &c.accent_soft)?.unwrap_or(mix(accent, Color32::WHITE, 0.35));
        let secondary = color("secondary", &c.secondary)?.unwrap_or(accent_soft);

        let series_src = c.series.clone().unwrap_or_default();
        if series_src.is_empty() {
            return Err("colors.series needs at least one colour".into());
        }
        let mut parsed = Vec::with_capacity(series_src.len());
        for (i, s) in series_src.iter().enumerate() {
            parsed.push(need(
                "series",
                color(&format!("series[{}]", i + 1), &Some(s.clone()))?,
            )?);
        }
        if parsed.len() > EDGE_PALETTE_LEN {
            warnings.push(format!(
                "colors.series has {} colours; only the first {EDGE_PALETTE_LEN} are used",
                parsed.len()
            ));
        }
        let series: [Color32; EDGE_PALETTE_LEN] = std::array::from_fn(|i| parsed[i % parsed.len()]);

        let a = &f.annotations;
        let pen = color("pen", &a.pen)?.unwrap_or(accent);
        let pen_outline = color("pen-outline", &a.pen_outline)?.unwrap_or(darken(pen, 0.6));
        let arrow = color("arrow", &a.arrow)?.unwrap_or(secondary);
        let arrow_outline = color("arrow-outline", &a.arrow_outline)?.unwrap_or(darken(arrow, 0.6));
        let particle_light = color("light", &f.particles.light)?.unwrap_or(heading_color);
        let particle_cool =
            color("cool", &f.particles.cool)?.unwrap_or(Color32::from_rgb(0xAF, 0xC3, 0xF0));

        let engine = match &f.engine {
            None => EngineKind::Plain,
            Some(e) => EngineKind::from_name(e).ok_or_else(|| {
                format!("engine: '{e}' is not an engine ({})", EngineKind::names())
            })?,
        };
        let engine = if engine.available() {
            engine
        } else {
            warnings.push(format!(
                "engine '{}' is not in this build of MDeck; using plain",
                engine.name()
            ));
            EngineKind::Plain
        };
        let mut countdown = match &f.countdown {
            None => Countdown::None,
            Some(c) => Countdown::from_name(c)
                .ok_or_else(|| format!("countdown: '{c}' is not none, plain or burst"))?,
        };
        if countdown == Countdown::Burst && !engine.capabilities().countdown {
            warnings.push(format!(
                "countdown: burst needs an engine with its own countdown, not {}; using plain",
                engine.name()
            ));
            countdown = Countdown::Plain;
        }

        let mut font = |role: &str,
                        v: &Option<String>,
                        mono: bool|
         -> Result<Option<egui::FontFamily>, String> {
            let Some(v) = v else { return Ok(None) };
            if let Some(fam) = bundled_family(v) {
                return Ok(Some(fam));
            }
            let lower = v.to_ascii_lowercase();
            if !(lower.ends_with(".ttf") || lower.ends_with(".otf")) {
                let names: Vec<&str> = BUNDLED_FACES.iter().map(|(n, _)| *n).collect();
                return Err(format!(
                    "fonts.{role}: '{v}' is neither a bundled face ({}) nor a .ttf/.otf file",
                    names.join(", ")
                ));
            }
            let path = Path::new(v);
            if !path.is_absolute() {
                return Err(format!(
                    "fonts.{role}: '{v}' must be a file in a theme folder"
                ));
            }
            // Name the file as the user sees it: relative to where they are.
            let shown = std::env::current_dir()
                .ok()
                .and_then(|cwd| cwd.canonicalize().ok())
                .and_then(|cwd| path.strip_prefix(cwd).ok().map(Path::to_path_buf))
                .unwrap_or_else(|| path.to_path_buf());
            let v = shown.display().to_string();
            let loaded = {
                let bytes = std::fs::read(path).map_err(|e| format!("{v}: {e}"))?;
                crate::render::fonts::register_file_face(bytes, mono)
                    .map_err(|e| format!("{v}: {e}"))
            };
            match loaded {
                Ok(fam) => Ok(Some(fam)),
                Err(e) => {
                    warnings.push(format!("fonts.{role}: {e}; using the default face"));
                    Ok(None)
                }
            }
        };
        let fonts_src = &f.fonts;
        let body = font("body", &fonts_src.body, false)?.unwrap_or(egui::FontFamily::Proportional);
        let display = font("display", &fonts_src.display, false)?.unwrap_or(body.clone());
        let lead = font("lead", &fonts_src.lead, false)?.unwrap_or(body.clone());
        let strong_face = font("strong", &fonts_src.strong, false)?.unwrap_or(body.clone());
        let mono = font("mono", &fonts_src.mono, true)?.unwrap_or(egui::FontFamily::Monospace);

        let size = |key: &str, v: Option<f32>| -> Result<f32, String> {
            match v {
                Some(s) if s > 0.0 && s <= 1000.0 => Ok(s),
                Some(s) => Err(format!("sizes.{key}: {s} must be between 0 and 1000")),
                None => Err(format!(
                    "sizes.{key} is not set (and nothing it extends sets it)"
                )),
            }
        };
        let s = &f.sizes;
        let line_height = match f.text.line_height {
            Some(lh) if !(0.5..=4.0).contains(&lh) => {
                return Err(format!("text.line-height: {lh} must be between 0.5 and 4"));
            }
            lh => lh,
        };
        let fill_opacity = match f.charts.fill_opacity {
            Some(o) if !(0.0..=1.0).contains(&o) => {
                return Err(format!("charts.fill-opacity: {o} must be between 0 and 1"));
            }
            Some(o) => o,
            None => crate::render::visualizations::VIZ_OPACITY_FILL,
        };

        let syntax = match &f.code.syntax {
            None => crate::render::syntax::DEFAULT_THEME.to_string(),
            Some(sx) if sx.to_ascii_lowercase().ends_with(".tmtheme") => {
                let p = Path::new(sx);
                let loaded = if p.is_absolute() {
                    crate::render::syntax::register_tm_theme(p)
                } else {
                    Err(format!("'{sx}' must be a file in a theme folder"))
                };
                match loaded {
                    Ok(key) => key,
                    Err(e) => {
                        warnings.push(format!("code.syntax: {e}; using the default"));
                        crate::render::syntax::DEFAULT_THEME.to_string()
                    }
                }
            }
            Some(sx) => {
                if !crate::render::syntax::has_theme(sx) {
                    return Err(format!(
                        "code.syntax: '{sx}' is not a bundled syntax theme ({}) or a .tmTheme file",
                        crate::render::syntax::bundled_theme_names().join(", ")
                    ));
                }
                sx.clone()
            }
        };

        let logo = {
            use crate::render::logo::{self as lg, Corner, Logo};
            let l = &f.logo;
            let corner = match &l.position {
                None => lg::DEFAULT_CORNER,
                Some(p) => Corner::from_name(p).ok_or_else(|| {
                    format!("logo.position: '{p}' is not top-left, top-right, bottom-left or bottom-right")
                })?,
            };
            let height = match l.height {
                None => lg::DEFAULT_HEIGHT,
                Some(h) if lg::valid_height(h) => h,
                Some(h) => {
                    return Err(format!(
                        "logo.height: {h} must be 8 to 400 (px at 1920x1080)"
                    ));
                }
            };
            let opacity = match l.opacity {
                None => lg::DEFAULT_OPACITY,
                Some(o) if (0.0..=1.0).contains(&o) => o,
                Some(o) => return Err(format!("logo.opacity: {o} must be between 0 and 1")),
            };
            match &l.file {
                None => None,
                Some(file) if !Path::new(file).is_absolute() => {
                    return Err(format!(
                        "logo.file: '{file}' must be a .png or .svg file in the theme folder"
                    ));
                }
                Some(file) => match lg::load_image(Path::new(file)) {
                    Ok(_) => Some(Logo {
                        path: PathBuf::from(file),
                        corner,
                        height,
                        opacity,
                    }),
                    Err(e) => {
                        warnings.push(format!("logo.file: {e}; no logo"));
                        None
                    }
                },
            }
        };

        // Renderers set their own alpha on theme colours (fades, glows), so a
        // translucent colour from a design system (`#ffffff0f`, a 6% hairline)
        // is composited over the background once, here.
        let background = Color32::from_rgb(background.r(), background.g(), background.b());
        let flat = |c: Color32| {
            if c.a() == 255 {
                return c;
            }
            let [r, g, b, a] = c.to_srgba_unmultiplied();
            mix(background, Color32::from_rgb(r, g, b), a as f32 / 255.0)
        };
        let theme = Theme {
            name: f.name.clone().unwrap_or_else(|| name.to_string()),
            engine,
            countdown,
            background,
            foreground: flat(foreground),
            heading_color: flat(heading_color),
            muted: flat(muted),
            strong: flat(strong),
            rule: flat(rule),
            accent: flat(accent),
            accent_soft: flat(accent_soft),
            secondary: flat(secondary),
            code_background: flat(code_background),
            code_foreground: flat(code_foreground),
            positive: flat(positive),
            negative: flat(negative),
            series: series.map(flat),
            pen: flat(pen),
            pen_outline: flat(pen_outline),
            arrow: flat(arrow),
            arrow_outline: flat(arrow_outline),
            particle_light: flat(particle_light),
            particle_cool: flat(particle_cool),
            fonts: ThemeFonts {
                display,
                body,
                lead,
                strong: strong_face,
                mono,
            },
            h1_size: size("h1", s.h1)?,
            h2_size: size("h2", s.h2)?,
            h3_size: size("h3", s.h3)?,
            body_size: size("body", s.body)?,
            code_size: size("code", s.code)?,
            line_height,
            fill_opacity,
            syntax,
            logo,
            source: None,
        };
        Ok(Built { theme, warnings })
    }

    /// A built-in theme by name. Panics only if an embedded file is broken,
    /// which the tests rule out.
    fn builtin(name: &str) -> Self {
        lookup::load_builtin(name)
            .unwrap_or_else(|e| panic!("built-in theme {name} is invalid: {e}"))
    }

    #[cfg(test)]
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

    #[cfg(test)]
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
        assert_eq!(d.engine, EngineKind::Plain);
        assert_eq!(d.countdown, Countdown::None);
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
        assert_eq!(n.countdown, Countdown::Plain);
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
            assert_eq!(e.engine, EngineKind::Particles);
            assert_eq!(e.countdown, Countdown::Burst);
        }
    }

    #[test]
    fn translucent_colours_are_flattened_onto_the_background() {
        let f = ThemeFile::parse(
            "colors: { background: '#000000', rule: '#ffffff80', series: ['#ff000080'] }",
        )
        .unwrap()
        .over(&lookup::builtin_file("dark").unwrap());
        let t = Theme::build("x", &f).unwrap().theme;
        assert_eq!(t.rule, Color32::from_rgb(128, 128, 128));
        assert_eq!(t.series[0], Color32::from_rgb(128, 0, 0));
        assert_eq!(t.background.a(), 255);
    }

    #[test]
    fn contrast_matches_wcag() {
        assert!((contrast(Color32::BLACK, Color32::WHITE) - 21.0).abs() < 0.01);
        assert!((contrast(Color32::WHITE, Color32::WHITE) - 1.0).abs() < 0.01);
    }

    #[test]
    fn series_cycles_to_fill_the_palette() {
        let f = ThemeFile::parse("colors: { series: ['#ff0000', '#00ff00'] }")
            .unwrap()
            .over(&lookup::builtin_file("dark").unwrap());
        let t = Theme::build("x", &f).unwrap().theme;
        assert_eq!(t.series[0], Color32::from_rgb(255, 0, 0));
        assert_eq!(t.series[1], Color32::from_rgb(0, 255, 0));
        assert_eq!(t.series[2], Color32::from_rgb(255, 0, 0));
    }

    #[test]
    fn invalid_values_are_errors() {
        let dark = lookup::builtin_file("dark").unwrap();
        let bad = |yaml: &str| {
            let f = ThemeFile::parse(yaml).unwrap().over(&dark);
            Theme::build("x", &f).unwrap_err()
        };
        assert!(bad("colors: { accent: 'orange' }").contains("colors.accent"));
        assert!(bad("engine: fireworks").contains("engine"));
        assert!(bad("countdown: loud").contains("countdown"));
        assert!(bad("sizes: { h1: -3 }").contains("sizes.h1"));
        assert!(bad("charts: { fill-opacity: 2 }").contains("fill-opacity"));
        assert!(bad("fonts: { body: comic-sans }").contains("bundled face"));
        assert!(bad("code: { syntax: rainbow }").contains("code.syntax"));
        assert!(bad("colors: { series: [] }").contains("series"));
    }

    #[test]
    fn burst_countdown_needs_the_particles_engine() {
        let dark = lookup::builtin_file("dark").unwrap();
        let f = ThemeFile::parse("countdown: burst").unwrap().over(&dark);
        let b = Theme::build("x", &f).unwrap();
        assert_eq!(b.theme.countdown, Countdown::Plain);
        assert!(b.warnings.iter().any(|w| w.contains("burst")));
    }

    #[test]
    fn paths_cannot_leave_the_theme_folder() {
        let dir = std::env::temp_dir().join(format!("mdeck-confine-{}", std::process::id()));
        let inner = dir.join("inner");
        std::fs::create_dir_all(&inner).unwrap();
        std::fs::write(dir.join("secret.ttf"), b"x").unwrap();
        std::fs::write(inner.join("ok.ttf"), b"x").unwrap();
        assert!(confined_path(&inner, "ok.ttf").is_ok());
        assert!(
            confined_path(&inner, "../secret.ttf")
                .unwrap_err()
                .contains("outside")
        );
        assert!(
            confined_path(&inner, "/etc/hosts")
                .unwrap_err()
                .contains("inside")
        );
        assert!(
            confined_path(&inner, "missing.ttf")
                .unwrap_err()
                .contains("not found")
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
