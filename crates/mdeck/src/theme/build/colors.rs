//! Every colour a theme carries: `colors:`, `annotations:` and
//! `particles:`, with the fallbacks unset keys derive from the rest.

use eframe::egui::Color32;

use super::super::color::{darken, flatten, luminance, mix};
use super::super::file::{ThemeFile, parse_color};
use super::super::{EDGE_PALETTE_LEN, ThemeError};

/// The resolved colours, before translucent ones are flattened.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Palette {
    pub background: Color32,
    pub foreground: Color32,
    pub heading: Color32,
    pub muted: Color32,
    pub strong: Color32,
    pub rule: Color32,
    pub accent: Color32,
    pub accent_soft: Color32,
    pub secondary: Color32,
    pub code_background: Color32,
    pub code_foreground: Color32,
    pub positive: Color32,
    pub negative: Color32,
    pub series: [Color32; EDGE_PALETTE_LEN],
    pub pen: Color32,
    pub pen_outline: Color32,
    pub arrow: Color32,
    pub arrow_outline: Color32,
    pub particle_light: Color32,
    pub particle_cool: Color32,
}

/// Parse one colour value; `key` is its full key path, for the error.
pub(super) fn parse(key: &str, s: &str) -> Result<Color32, ThemeError> {
    channels(key, s).map(|[r, g, b, a]| Color32::from_rgba_unmultiplied(r, g, b, a))
}

/// A colour painted opaque: its channels as written, any alpha ignored.
pub(super) fn parse_opaque(key: &str, s: &str) -> Result<Color32, ThemeError> {
    channels(key, s).map(|[r, g, b, _]| Color32::from_rgb(r, g, b))
}

fn channels(key: &str, s: &str) -> Result<[u8; 4], ThemeError> {
    parse_color(s).ok_or_else(|| {
        ThemeError::invalid(key, format!("'{s}' is not a colour (use #rrggbb)"))
    })
}

/// An optional colour under `colors:`: `None` when unset, an error when
/// not a colour.
fn optional(key: &str, v: &Option<String>) -> Result<Option<Color32>, ThemeError> {
    optional_at(&format!("colors.{key}"), v)
}

/// An optional colour at the full key path `key`.
fn optional_at(key: &str, v: &Option<String>) -> Result<Option<Color32>, ThemeError> {
    v.as_deref().map(|s| parse(key, s)).transpose()
}

/// A colour the theme (or what it extends) must set.
fn required(key: &str, v: &Option<String>) -> Result<Color32, ThemeError> {
    optional(key, v)?.ok_or_else(|| ThemeError::missing(format!("colors.{key}")))
}

/// Chart and diagram series, cycled to fill every slot. More colours than
/// slots is a warning.
fn series(
    src: &Option<Vec<String>>,
    warnings: &mut Vec<String>,
) -> Result<[Color32; EDGE_PALETTE_LEN], ThemeError> {
    let src = src.as_deref().unwrap_or_default();
    if src.is_empty() {
        return Err(ThemeError::EmptySeries);
    }
    let parsed = src
        .iter()
        .enumerate()
        .map(|(i, s)| parse(&format!("colors.series[{}]", i + 1), s))
        .collect::<Result<Vec<_>, _>>()?;
    if parsed.len() > EDGE_PALETTE_LEN {
        warnings.push(format!(
            "colors.series has {} colours; only the first {EDGE_PALETTE_LEN} are used",
            parsed.len()
        ));
    }
    Ok(std::array::from_fn(|i| parsed[i % parsed.len()]))
}

/// Bold runs: the heading colour when it is visibly brighter than body
/// text (dark themes); otherwise the accent, so bold still stands out.
fn default_strong(heading: Color32, foreground: Color32, accent: Color32) -> Color32 {
    if luminance(heading) - luminance(foreground) > 0.08 {
        heading
    } else {
        accent
    }
}

impl Palette {
    /// Resolve every colour in `f`, in the order errors are reported.
    pub fn resolve(f: &ThemeFile, warnings: &mut Vec<String>) -> Result<Self, ThemeError> {
        let c = &f.colors;
        let background = required("background", &c.background)?;
        let foreground = required("text", &c.text)?;
        let heading = required("heading", &c.heading)?;
        let accent = required("accent", &c.accent)?;
        let code_background = required("code-background", &c.code_background)?;
        let code_foreground = required("code-text", &c.code_text)?;
        let positive = required("positive", &c.positive)?;
        let negative = required("negative", &c.negative)?;
        let muted = optional("muted", &c.muted)?.unwrap_or(mix(background, foreground, 0.6));
        let rule = optional("rule", &c.rule)?.unwrap_or(mix(background, foreground, 0.18));
        let strong =
            optional("strong", &c.strong)?.unwrap_or(default_strong(heading, foreground, accent));
        let accent_soft =
            optional("accent-soft", &c.accent_soft)?.unwrap_or(mix(accent, Color32::WHITE, 0.35));
        let secondary = optional("secondary", &c.secondary)?.unwrap_or(accent_soft);
        let series = series(&c.series, warnings)?;

        let a = &f.annotations;
        let pen = optional_at("annotations.pen", &a.pen)?.unwrap_or(accent);
        let pen_outline = optional_at("annotations.pen-outline", &a.pen_outline)?.unwrap_or(darken(pen, 0.6));
        let arrow = optional_at("annotations.arrow", &a.arrow)?.unwrap_or(secondary);
        let arrow_outline =
            optional_at("annotations.arrow-outline", &a.arrow_outline)?.unwrap_or(darken(arrow, 0.6));
        let tints = f.particles()?;
        let particle_light = optional_at("engine.light", &tints.light)?.unwrap_or(heading);
        let particle_cool = optional_at("engine.cool", &tints.cool)?
            .unwrap_or(Color32::from_rgb(0xAF, 0xC3, 0xF0));

        Ok(Palette {
            background,
            foreground,
            heading,
            muted,
            strong,
            rule,
            accent,
            accent_soft,
            secondary,
            code_background,
            code_foreground,
            positive,
            negative,
            series,
            pen,
            pen_outline,
            arrow,
            arrow_outline,
            particle_light,
            particle_cool,
        })
    }

    /// Renderers set their own alpha on theme colours (fades, glows), so a
    /// translucent colour from a design system (`#ffffff0f`, a 6% hairline)
    /// is composited over the background once, here. The background itself
    /// becomes opaque.
    pub fn flattened(self) -> Self {
        let bg = self.background;
        let background = Color32::from_rgb(bg.r(), bg.g(), bg.b());
        let flat = |c: Color32| flatten(c, background);
        Palette {
            background,
            foreground: flat(self.foreground),
            heading: flat(self.heading),
            muted: flat(self.muted),
            strong: flat(self.strong),
            rule: flat(self.rule),
            accent: flat(self.accent),
            accent_soft: flat(self.accent_soft),
            secondary: flat(self.secondary),
            code_background: flat(self.code_background),
            code_foreground: flat(self.code_foreground),
            positive: flat(self.positive),
            negative: flat(self.negative),
            series: self.series.map(flat),
            pen: flat(self.pen),
            pen_outline: flat(self.pen_outline),
            arrow: flat(self.arrow),
            arrow_outline: flat(self.arrow_outline),
            particle_light: flat(self.particle_light),
            particle_cool: flat(self.particle_cool),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colour_errors_name_the_key() {
        let e = parse("page.surface", "paper").unwrap_err().to_string();
        assert_eq!(
            e,
            "page.surface: 'paper' is not a colour (use #rrggbb)"
        );
        let e = required("accent", &None).unwrap_err().to_string();
        assert_eq!(
            e,
            "colors.accent is not set (and nothing it extends sets it)"
        );
        assert_eq!(optional("muted", &None).unwrap(), None);
    }

    #[test]
    fn series_errors_and_warnings() {
        let mut w = Vec::new();
        assert_eq!(series(&None, &mut w), Err(ThemeError::EmptySeries));
        let bad = Some(vec!["#fff".to_string(), "nope".to_string()]);
        assert!(
            series(&bad, &mut w)
                .unwrap_err()
                .to_string()
                .starts_with("colors.series[2]:")
        );
        let many = Some(vec!["#000".to_string(); 9]);
        series(&many, &mut w).unwrap();
        assert_eq!(w.len(), 1);
        assert!(w[0].contains("9 colours"));
    }

    #[test]
    fn strong_prefers_a_brighter_heading() {
        let (white, grey, red) = (
            Color32::WHITE,
            Color32::from_rgb(0xAA, 0xAA, 0xAA),
            Color32::RED,
        );
        assert_eq!(default_strong(white, grey, red), white);
        assert_eq!(default_strong(grey, grey, red), red);
    }
}
