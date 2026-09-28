//! What a theme adds around the slide: `logo:`, `page:` and `art:`.

use std::path::{Path, PathBuf};

use eframe::egui::Color32;

use super::super::file::{self, ThemeFile};
use super::super::{Page, ThemeArt, ThemeError};
use super::{colors, range};
use crate::render::logo::{self as lg, Corner, Logo};

/// `logo:`. Placement keys are checked even without a file; a file that
/// cannot be loaded means no logo, with a warning.
pub(super) fn logo(f: &file::Logo, warnings: &mut Vec<String>) -> Result<Option<Logo>, ThemeError> {
    let corner = match &f.position {
        None => lg::DEFAULT_CORNER,
        Some(p) => Corner::from_name(p).ok_or_else(|| {
            ThemeError::invalid(
                "logo.position",
                format!("'{p}' is not top-left, top-right, bottom-left or bottom-right"),
            )
        })?,
    };
    let height = match f.height {
        None => lg::DEFAULT_HEIGHT,
        Some(h) if lg::valid_height(h) => h,
        Some(h) => {
            return Err(ThemeError::invalid(
                "logo.height",
                format!("{h} must be 8 to 400 (px at 1920x1080)"),
            ));
        }
    };
    let opacity = range("logo.opacity", f.opacity, 0.0, 1.0)?.unwrap_or(lg::DEFAULT_OPACITY);
    let Some(file) = &f.file else { return Ok(None) };
    if !Path::new(file).is_absolute() {
        return Err(ThemeError::invalid(
            "logo.file",
            format!("'{file}' must be a .png or .svg file in the theme folder"),
        ));
    }
    match lg::load_image(Path::new(file)) {
        Ok(_) => Ok(Some(Logo {
            path: PathBuf::from(file),
            corner,
            height,
            opacity,
        })),
        Err(e) => {
            warnings.push(format!("logo.file: {e}; no logo"));
            Ok(None)
        }
    }
}

/// `page:`; setting `surface` turns the page on.
pub(super) fn page(f: &file::Page) -> Result<Option<Page>, ThemeError> {
    let Some(s) = &f.surface else { return Ok(None) };
    let surface = colors::parse("page.surface", s)?;
    let key = |k: &str| format!("page.{k}");
    Ok(Some(Page {
        surface: Color32::from_rgb(surface.r(), surface.g(), surface.b()),
        margin: range(&key("margin"), f.margin, 0.0, 300.0)?.unwrap_or(56.0),
        shadow: range(&key("shadow"), f.shadow, 0.0, 1.0)?.unwrap_or(0.5),
        grain: range(&key("grain"), f.grain, 0.0, 1.0)?.unwrap_or(0.5),
        radius: range(&key("radius"), f.radius, 0.0, 60.0)?.unwrap_or(6.0),
    }))
}

/// `art:`; a blank style counts as unset.
pub(super) fn art(f: &ThemeFile) -> Result<ThemeArt, ThemeError> {
    let kind = match f.art.kind.as_deref() {
        None => None,
        Some(k) => Some(crate::render::art::ArtKind::from_name(k).ok_or_else(|| {
            ThemeError::invalid("art.kind", format!("'{k}' is not line or tonal"))
        })?),
    };
    Ok(ThemeArt {
        kind,
        style: f.art.style.clone().filter(|s| !s.trim().is_empty()),
        references: f
            .art
            .references
            .iter()
            .flatten()
            .map(PathBuf::from)
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logo_placement_is_checked_without_a_file() {
        let mut w = Vec::new();
        assert_eq!(logo(&file::Logo::default(), &mut w).unwrap(), None);
        let bad = |l: file::Logo| logo(&l, &mut Vec::new()).unwrap_err().to_string();
        assert!(
            bad(file::Logo {
                position: Some("middle".into()),
                ..Default::default()
            })
            .starts_with("logo.position: 'middle'")
        );
        assert_eq!(
            bad(file::Logo {
                height: Some(2.0),
                ..Default::default()
            }),
            "logo.height: 2 must be 8 to 400 (px at 1920x1080)"
        );
        assert_eq!(
            bad(file::Logo {
                opacity: Some(3.0),
                ..Default::default()
            }),
            "logo.opacity: 3 must be between 0 and 1"
        );
        assert!(
            bad(file::Logo {
                file: Some("logo.png".into()),
                ..Default::default()
            })
            .contains("in the theme folder")
        );
    }

    #[test]
    fn page_needs_a_surface_and_has_defaults() {
        assert_eq!(page(&file::Page::default()).unwrap(), None);
        let p = page(&file::Page {
            surface: Some("#808080".into()),
            ..Default::default()
        })
        .unwrap()
        .unwrap();
        assert_eq!(p.surface, Color32::from_rgb(0x80, 0x80, 0x80));
        assert_eq!(
            (p.margin, p.shadow, p.grain, p.radius),
            (56.0, 0.5, 0.5, 6.0)
        );
        let e = page(&file::Page {
            surface: Some("#fff".into()),
            margin: Some(400.0),
            ..Default::default()
        })
        .unwrap_err();
        assert_eq!(e.to_string(), "page.margin: 400 must be between 0 and 300");
    }

    #[test]
    fn art_kind_and_blank_style() {
        let f = ThemeFile::parse("art: { style: '  ', references: [a.png] }").unwrap();
        let a = art(&f).unwrap();
        assert_eq!(a.style, None);
        assert_eq!(a.references, [PathBuf::from("a.png")]);
        let f = ThemeFile::parse("art: { kind: oil }").unwrap();
        assert_eq!(
            art(&f).unwrap_err().to_string(),
            "art.kind: 'oil' is not line or tonal"
        );
    }
}
