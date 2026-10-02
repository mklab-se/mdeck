//! The engine, the countdown, sizes, text, charts and code keys.

use std::path::Path;

use super::super::file::{Sizes, ThemeFile};
use super::super::{EngineId, Surface, ThemeError};
use super::range;
use crate::render::syntax;

/// `engine:`, `countdown:` and `transition:`. An engine name the registry
/// does not know (an extension this build leaves out, a misspelling) falls
/// back to plain with a warning (EXT-07).
pub(super) fn engine_and_countdown(
    f: &ThemeFile,
    warnings: &mut Vec<String>,
) -> Result<(EngineId, bool, Option<String>), ThemeError> {
    let engine = match f.engine_name().map(str::trim) {
        None => EngineId::plain(),
        Some(e) => EngineId::find(e).unwrap_or_else(|| {
            warnings.push(format!(
                "engine '{e}' is not an engine in this build of MDeck ({}); using plain",
                crate::engines::names()
            ));
            EngineId::plain()
        }),
    };
    let countdown = match f.countdown.as_deref().map(str::trim) {
        None | Some("off") => false,
        Some("on") => true,
        Some(c) => {
            return Err(ThemeError::invalid(
                "countdown",
                format!("'{c}' is not on or off"),
            ));
        }
    };
    // a built-in or a transition an extension registered
    let transition = match f.transition.as_deref().map(str::trim) {
        None => None,
        Some(t) if TRANSITIONS.contains(&t) => Some(t.to_string()),
        Some(t) if crate::render::transition::TransitionKind::parse(t).is_some() => {
            Some(t.to_string())
        }
        Some(t) => {
            let mut known: Vec<String> = TRANSITIONS.iter().map(|s| s.to_string()).collect();
            known.extend(
                crate::registry::get()
                    .transitions()
                    .map(|t| t.name().to_string()),
            );
            return Err(ThemeError::invalid(
                "transition",
                format!("'{t}' is not {}", known.join(", ")),
            ));
        }
    };
    Ok((engine, countdown, transition))
}

/// The transitions a theme may name.
pub(crate) const TRANSITIONS: [&str; 4] = ["slide", "fade", "spatial", "none"];

/// `engine.surface` (the line engine's ground), `sheet` when unset.
pub(super) fn surface(f: &ThemeFile) -> Result<Surface, ThemeError> {
    match f.surface()? {
        None => Ok(Surface::default()),
        Some(s) => Surface::from_name(s.trim()).ok_or_else(|| {
            ThemeError::invalid("engine.surface", format!("'{s}' is not sheet or slate"))
        }),
    }
}

/// Heading, body and code sizes in px, in the order `h1, h2, h3, body,
/// code`. Every one must be set and in (0, 1000].
pub(super) fn sizes(s: &Sizes) -> Result<[f32; 5], ThemeError> {
    let size = |key: &str, v: Option<f32>| match v {
        Some(s) if s > 0.0 && s <= 1000.0 => Ok(s),
        Some(s) => Err(ThemeError::OutOfRange {
            key: format!("sizes.{key}"),
            value: s,
            lo: 0.0,
            hi: 1000.0,
        }),
        None => Err(ThemeError::missing(format!("sizes.{key}"))),
    };
    Ok([
        size("h1", s.h1)?,
        size("h2", s.h2)?,
        size("h3", s.h3)?,
        size("body", s.body)?,
        size("code", s.code)?,
    ])
}

/// `text.line-height`, a multiple of the font size.
pub(super) fn line_height(f: &ThemeFile) -> Result<Option<f32>, ThemeError> {
    range("text.line-height", f.text.line_height, 0.5, 4.0)
}

/// `charts.fill-opacity`, defaulting to the visualizations' own.
pub(super) fn fill_opacity(f: &ThemeFile) -> Result<f32, ThemeError> {
    Ok(
        range("charts.fill-opacity", f.charts.fill_opacity, 0.0, 1.0)?
            .unwrap_or(crate::render::visualizations::VIZ_OPACITY_FILL),
    )
}

/// `code.syntax`: a bundled syntax theme, or a `.tmTheme` file registered
/// under a key (falling back with a warning when it cannot be used).
pub(super) fn syntax(f: &ThemeFile, warnings: &mut Vec<String>) -> Result<String, ThemeError> {
    let Some(sx) = &f.code.syntax else {
        return Ok(syntax::DEFAULT_THEME.to_string());
    };
    if sx.to_ascii_lowercase().ends_with(".tmtheme") {
        let p = Path::new(sx);
        let loaded = if p.is_absolute() {
            syntax::register_tm_theme(p)
        } else {
            Err(anyhow::anyhow!("'{sx}' must be a file in a theme folder"))
        };
        return Ok(loaded.unwrap_or_else(|e| {
            warnings.push(format!("code.syntax: {e}; using the default"));
            syntax::DEFAULT_THEME.to_string()
        }));
    }
    if !syntax::has_theme(sx) {
        return Err(ThemeError::invalid(
            "code.syntax",
            format!(
                "'{sx}' is not a bundled syntax theme ({}) or a .tmTheme file",
                syntax::bundled_theme_names().join(", ")
            ),
        ));
    }
    Ok(sx.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(yaml: &str) -> ThemeFile {
        ThemeFile::parse(yaml).unwrap()
    }

    #[test]
    fn sizes_must_be_set_and_positive() {
        let all = Sizes {
            h1: Some(90.0),
            h2: Some(60.0),
            h3: Some(50.0),
            body: Some(40.0),
            code: Some(30.0),
        };
        assert_eq!(sizes(&all).unwrap(), [90.0, 60.0, 50.0, 40.0, 30.0]);
        let e = sizes(&Sizes {
            h1: Some(-3.0),
            ..all.clone()
        })
        .unwrap_err();
        assert_eq!(e.to_string(), "sizes.h1: -3 must be between 0 and 1000");
        let e = sizes(&Sizes { code: None, ..all }).unwrap_err();
        assert_eq!(
            e.to_string(),
            "sizes.code is not set (and nothing it extends sets it)"
        );
    }

    #[test]
    fn text_and_chart_ranges() {
        assert_eq!(line_height(&file("{}")).unwrap(), None);
        assert_eq!(
            line_height(&file("text: { line-height: 5 }"))
                .unwrap_err()
                .to_string(),
            "text.line-height: 5 must be between 0.5 and 4"
        );
        assert_eq!(
            fill_opacity(&file("{}")).unwrap(),
            crate::render::visualizations::VIZ_OPACITY_FILL
        );
        assert_eq!(
            fill_opacity(&file("charts: { fill-opacity: 0.3 }")).unwrap(),
            0.3
        );
    }

    #[test]
    fn surfaces_are_sheet_by_default_and_checked() {
        assert_eq!(surface(&file("{}")).unwrap(), Surface::Sheet);
        assert_eq!(
            surface(&file("engine: { name: line, surface: slate }")).unwrap(),
            Surface::Slate
        );
        assert_eq!(
            surface(&file("engine: { name: line, surface: glass }"))
                .unwrap_err()
                .to_string(),
            "engine.surface: 'glass' is not sheet or slate"
        );
    }

    #[test]
    fn countdown_names_are_checked() {
        let mut w = Vec::new();
        let e = engine_and_countdown(&file("countdown: loud"), &mut w).unwrap_err();
        assert_eq!(e.to_string(), "countdown: 'loud' is not on or off");
        let (engine, countdown, transition) = engine_and_countdown(&file("{}"), &mut w).unwrap();
        assert_eq!(
            (engine, countdown, transition),
            (EngineId::plain(), false, None)
        );
        assert!(w.is_empty());
    }

    #[test]
    fn a_relative_tmtheme_falls_back_with_a_warning() {
        let mut w = Vec::new();
        let s = syntax(&file("code: { syntax: x.tmTheme }"), &mut w).unwrap();
        assert_eq!(s, syntax::DEFAULT_THEME);
        assert_eq!(
            w,
            ["code.syntax: 'x.tmTheme' must be a file in a theme folder; using the default"]
        );
    }
}
