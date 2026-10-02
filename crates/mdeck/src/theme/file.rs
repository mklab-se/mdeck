//! The theme file format (`theme.yaml`): every key optional, unknown keys
//! rejected, and `extends` merged key by key before a [`super::Theme`] is
//! built from the result.

use serde::Deserialize;

use super::ThemeError;

/// One theme file as written. Colours are `#rgb`, `#rrggbb` or `#rrggbbaa`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ThemeFile {
    /// Display name; defaults to the file name.
    pub name: Option<String>,
    /// Theme to inherit unset keys from (default: `dark`).
    pub extends: Option<String>,
    /// `plain` or `particles`.
    pub engine: Option<String>,
    /// `on` or `off`: whether decks open with the 3-2-1 countdown.
    pub countdown: Option<String>,
    /// The line engine's surface: `sheet` or `slate` (interim, see
    /// [`super::Surface`]).
    pub surface: Option<String>,
    /// `slide`, `fade`, `spatial` or `none`.
    pub transition: Option<String>,
    /// The design set: `standard` or `editorial`.
    pub designs: Option<String>,
    /// Arrangement overrides per design (`all` for every design), merged
    /// key by key over the design set and through `extends`.
    pub arrangements: Option<serde_norway::Value>,
    #[serde(default)]
    pub spacing: Spacing,
    /// Corner radius of cards (code, tables, callouts), px at 1920x1080.
    pub radius: Option<f32>,
    #[serde(default)]
    pub colors: Colors,
    #[serde(default)]
    pub annotations: Annotations,
    #[serde(default)]
    pub particles: Particles,
    #[serde(default)]
    pub fonts: Fonts,
    #[serde(default)]
    pub sizes: Sizes,
    #[serde(default)]
    pub text: Text,
    #[serde(default)]
    pub charts: Charts,
    #[serde(default)]
    pub code: Code,
    #[serde(default)]
    pub logo: Logo,
    #[serde(default)]
    pub page: Page,
    #[serde(default)]
    pub art: Art,
    #[serde(default)]
    pub heat: Heat,
}

/// The spacing scale, px at 1920x1080 (THM-07).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Spacing {
    pub xs: Option<f32>,
    pub sm: Option<f32>,
    pub md: Option<f32>,
    pub lg: Option<f32>,
    pub xl: Option<f32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Colors {
    pub background: Option<String>,
    pub text: Option<String>,
    pub heading: Option<String>,
    pub muted: Option<String>,
    pub strong: Option<String>,
    pub rule: Option<String>,
    pub accent: Option<String>,
    pub accent_soft: Option<String>,
    pub secondary: Option<String>,
    pub code_background: Option<String>,
    pub code_text: Option<String>,
    pub positive: Option<String>,
    pub negative: Option<String>,
    pub series: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Annotations {
    pub pen: Option<String>,
    pub pen_outline: Option<String>,
    pub arrow: Option<String>,
    pub arrow_outline: Option<String>,
}

/// The heat field of the thermal engine.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Heat {
    /// The palette the field glows in: iron, white-hot, black-hot, rainbow,
    /// arctic or lava.
    pub palette: Option<String>,
    /// Embers drift through the dark on ordinary slides (off: a still,
    /// dark background).
    pub drift: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Particles {
    pub light: Option<String>,
    pub cool: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Fonts {
    pub display: Option<String>,
    pub body: Option<String>,
    pub lead: Option<String>,
    pub strong: Option<String>,
    pub mono: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Sizes {
    pub h1: Option<f32>,
    pub h2: Option<f32>,
    pub h3: Option<f32>,
    pub body: Option<f32>,
    pub code: Option<f32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Text {
    pub line_height: Option<f32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Charts {
    pub fill_opacity: Option<f32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Code {
    pub syntax: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Logo {
    /// A PNG or SVG in the theme folder.
    pub file: Option<String>,
    /// `top-left`, `top-right`, `bottom-left` or `bottom-right`.
    pub position: Option<String>,
    /// Height in px on a 1920x1080 slide.
    pub height: Option<f32>,
    /// 0 to 1.
    pub opacity: Option<f32>,
}

/// The slide as a sheet on a surface: paper on a desk, a board on a wall.
/// The sheet is `colors.background`; setting `surface` turns the page on.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Page {
    /// The colour around the sheet.
    pub surface: Option<String>,
    /// Space between the sheet and the slide's edge, px on a 1920x1080 slide.
    pub margin: Option<f32>,
    /// How strongly the sheet's shadow shows, 0 to 1.
    pub shadow: Option<f32>,
    /// How strongly the sheet's texture shows, 0 to 1.
    pub grain: Option<f32>,
    /// Corner radius of the sheet, px on a 1920x1080 slide.
    pub radius: Option<f32>,
}

/// Generated artwork: what kind of picture the deck's art engine asks for,
/// in what style, with which reference images.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Art {
    /// `line` (ink lines on white, drawn in the engine's medium) or `tonal`
    /// (a picture in the medium itself).
    pub kind: Option<String>,
    /// The style prompt, instead of the engine's built-in one.
    pub style: Option<String>,
    /// Style swatches in the theme folder, sent as references.
    pub references: Option<Vec<String>>,
}

/// `child` wins wherever it sets a key.
fn pick<T: Clone>(child: &Option<T>, parent: &Option<T>) -> Option<T> {
    child.clone().or_else(|| parent.clone())
}

impl ThemeFile {
    pub fn parse(yaml: &str) -> Result<Self, ThemeError> {
        serde_norway::from_str(yaml).map_err(|e| ThemeError::Parse(e.to_string()))
    }

    /// This file with every unset key taken from `parent`. `name` and
    /// `extends` are the child's own.
    pub fn over(&self, parent: &ThemeFile) -> ThemeFile {
        let (c, p) = (&self.colors, &parent.colors);
        let (a, pa) = (&self.annotations, &parent.annotations);
        let (f, pf) = (&self.fonts, &parent.fonts);
        let (s, ps) = (&self.sizes, &parent.sizes);
        ThemeFile {
            name: self.name.clone(),
            extends: self.extends.clone(),
            engine: pick(&self.engine, &parent.engine),
            countdown: pick(&self.countdown, &parent.countdown),
            surface: pick(&self.surface, &parent.surface),
            transition: pick(&self.transition, &parent.transition),
            designs: pick(&self.designs, &parent.designs),
            arrangements: match (&self.arrangements, &parent.arrangements) {
                (Some(c), Some(p)) => {
                    let mut merged = p.clone();
                    super::arrangement::merge(&mut merged, c);
                    Some(merged)
                }
                (c, p) => c.clone().or_else(|| p.clone()),
            },
            spacing: Spacing {
                xs: pick(&self.spacing.xs, &parent.spacing.xs),
                sm: pick(&self.spacing.sm, &parent.spacing.sm),
                md: pick(&self.spacing.md, &parent.spacing.md),
                lg: pick(&self.spacing.lg, &parent.spacing.lg),
                xl: pick(&self.spacing.xl, &parent.spacing.xl),
            },
            radius: pick(&self.radius, &parent.radius),
            colors: Colors {
                background: pick(&c.background, &p.background),
                text: pick(&c.text, &p.text),
                heading: pick(&c.heading, &p.heading),
                muted: pick(&c.muted, &p.muted),
                strong: pick(&c.strong, &p.strong),
                rule: pick(&c.rule, &p.rule),
                accent: pick(&c.accent, &p.accent),
                accent_soft: pick(&c.accent_soft, &p.accent_soft),
                secondary: pick(&c.secondary, &p.secondary),
                code_background: pick(&c.code_background, &p.code_background),
                code_text: pick(&c.code_text, &p.code_text),
                positive: pick(&c.positive, &p.positive),
                negative: pick(&c.negative, &p.negative),
                series: pick(&c.series, &p.series),
            },
            annotations: Annotations {
                pen: pick(&a.pen, &pa.pen),
                pen_outline: pick(&a.pen_outline, &pa.pen_outline),
                arrow: pick(&a.arrow, &pa.arrow),
                arrow_outline: pick(&a.arrow_outline, &pa.arrow_outline),
            },
            particles: Particles {
                light: pick(&self.particles.light, &parent.particles.light),
                cool: pick(&self.particles.cool, &parent.particles.cool),
            },
            fonts: Fonts {
                display: pick(&f.display, &pf.display),
                body: pick(&f.body, &pf.body),
                lead: pick(&f.lead, &pf.lead),
                strong: pick(&f.strong, &pf.strong),
                mono: pick(&f.mono, &pf.mono),
            },
            sizes: Sizes {
                h1: pick(&s.h1, &ps.h1),
                h2: pick(&s.h2, &ps.h2),
                h3: pick(&s.h3, &ps.h3),
                body: pick(&s.body, &ps.body),
                code: pick(&s.code, &ps.code),
            },
            text: Text {
                line_height: pick(&self.text.line_height, &parent.text.line_height),
            },
            charts: Charts {
                fill_opacity: pick(&self.charts.fill_opacity, &parent.charts.fill_opacity),
            },
            code: Code {
                syntax: pick(&self.code.syntax, &parent.code.syntax),
            },
            logo: Logo {
                file: pick(&self.logo.file, &parent.logo.file),
                position: pick(&self.logo.position, &parent.logo.position),
                height: pick(&self.logo.height, &parent.logo.height),
                opacity: pick(&self.logo.opacity, &parent.logo.opacity),
            },
            page: Page {
                surface: pick(&self.page.surface, &parent.page.surface),
                margin: pick(&self.page.margin, &parent.page.margin),
                shadow: pick(&self.page.shadow, &parent.page.shadow),
                grain: pick(&self.page.grain, &parent.page.grain),
                radius: pick(&self.page.radius, &parent.page.radius),
            },
            heat: Heat {
                palette: pick(&self.heat.palette, &parent.heat.palette),
                drift: pick(&self.heat.drift, &parent.heat.drift),
            },
            art: Art {
                kind: pick(&self.art.kind, &parent.art.kind),
                style: pick(&self.art.style, &parent.art.style),
                references: pick(&self.art.references, &parent.art.references),
            },
        }
    }
}

/// Parse `#rgb`, `#rrggbb` or `#rrggbbaa` (the `#` is optional).
pub fn parse_color(s: &str) -> Option<[u8; 4]> {
    let h = s.trim().trim_start_matches('#');
    let hex = |i: usize, n: usize| u8::from_str_radix(&h[i..i + n], 16).ok();
    if !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    match h.len() {
        3 => {
            let d = |i| hex(i, 1).map(|v| v * 17);
            Some([d(0)?, d(1)?, d(2)?, 255])
        }
        6 => Some([hex(0, 2)?, hex(2, 2)?, hex(4, 2)?, 255]),
        8 => Some([hex(0, 2)?, hex(2, 2)?, hex(4, 2)?, hex(6, 2)?]),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_parse_in_every_form() {
        assert_eq!(parse_color("#ff4d1c"), Some([0xff, 0x4d, 0x1c, 255]));
        assert_eq!(parse_color("FF4D1C"), Some([0xff, 0x4d, 0x1c, 255]));
        assert_eq!(parse_color("#fff"), Some([255, 255, 255, 255]));
        assert_eq!(parse_color("#00000080"), Some([0, 0, 0, 0x80]));
        assert_eq!(parse_color("#12345"), None);
        assert_eq!(parse_color("#gggggg"), None);
        assert_eq!(parse_color("rgb(1,2,3)"), None);
    }

    #[test]
    fn unknown_keys_are_rejected() {
        let err = ThemeFile::parse("colors:\n  backgorund: '#000'\n")
            .unwrap_err()
            .to_string();
        assert!(err.contains("backgorund"), "{err}");
        let err = ThemeFile::parse("colour: {}\n").unwrap_err().to_string();
        assert!(err.contains("colour"), "{err}");
    }

    #[test]
    fn child_keys_win_and_the_rest_is_inherited() {
        let parent = ThemeFile::parse(
            "engine: particles\ncolors: { background: '#000', accent: '#f00' }\nsizes: { h1: 90 }\n",
        )
        .unwrap();
        let child = ThemeFile::parse("name: x\ncolors: { accent: '#0f0' }\n").unwrap();
        let m = child.over(&parent);
        assert_eq!(m.name.as_deref(), Some("x"));
        assert_eq!(m.engine.as_deref(), Some("particles"));
        assert_eq!(m.colors.background.as_deref(), Some("#000"));
        assert_eq!(m.colors.accent.as_deref(), Some("#0f0"));
        assert_eq!(m.sizes.h1, Some(90.0));
    }

    #[test]
    fn arrangements_merge_through_extends() {
        let parent = ThemeFile::parse(
            "designs: editorial\narrangements: { quote: { ornaments: { quote-bar: none, bullet: x } } }\n",
        )
        .unwrap();
        let child =
            ThemeFile::parse("arrangements: { quote: { ornaments: { bullet: y } } }\n").unwrap();
        let m = child.over(&parent);
        assert_eq!(m.designs.as_deref(), Some("editorial"));
        let o = m.arrangements.unwrap();
        let orn = &o["quote"]["ornaments"];
        assert_eq!(orn["bullet"].as_str(), Some("y"));
        assert_eq!(orn["quote-bar"].as_str(), Some("none"));
    }
}
