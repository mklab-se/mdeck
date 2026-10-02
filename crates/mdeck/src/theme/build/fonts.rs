//! `fonts:`: a bundled face by name, or a `.ttf`/`.otf` file in a theme
//! folder registered with egui.

use std::path::Path;

use eframe::egui::FontFamily;

use super::super::file::Fonts;
use super::super::{
    BUNDLED_FACES, FONT_BODY, FONT_BODY_LIGHT, FONT_BODY_MEDIUM, FONT_DISPLAY, FONT_MONO,
    ThemeError, ThemeFonts,
};

/// The family a bundled face name stands for.
fn bundled_family(name: &str) -> Option<FontFamily> {
    Some(match name {
        "sans" => FontFamily::Proportional,
        "mono" => FontFamily::Monospace,
        "spectral-light" => FontFamily::Name(FONT_DISPLAY.into()),
        "hanken-light" => FontFamily::Name(FONT_BODY_LIGHT.into()),
        "hanken-regular" => FontFamily::Name(FONT_BODY.into()),
        "hanken-medium" => FontFamily::Name(FONT_BODY_MEDIUM.into()),
        "jetbrains-mono" => FontFamily::Name(FONT_MONO.into()),
        _ => return None,
    })
}

/// `path` as the user sees it: relative to where they are, when it can be.
fn shown(path: &Path) -> String {
    std::env::current_dir()
        .ok()
        .and_then(|cwd| cwd.canonicalize().ok())
        .and_then(|cwd| path.strip_prefix(cwd).ok().map(Path::to_path_buf))
        .unwrap_or_else(|| path.to_path_buf())
        .display()
        .to_string()
}

/// A font file's name as shown and its bytes: an absolute path (a theme on
/// disk, made absolute against its folder), else the font an extension
/// registered under that name (`Registry::font`, for the themes it embeds).
fn font_file(
    key: &str,
    v: &str,
    registered: Option<&'static [u8]>,
) -> Result<(String, Vec<u8>), ThemeError> {
    let path = Path::new(v);
    if path.is_absolute() {
        let shown = shown(path);
        let bytes = std::fs::read(path).map_err(|e| ThemeError::file(&shown, e))?;
        return Ok((shown, bytes));
    }
    match registered {
        Some(bytes) => Ok((v.to_string(), bytes.to_vec())),
        None => Err(ThemeError::invalid(
            key,
            format!("'{v}' must be a file in a theme folder, or a font an extension registers"),
        )),
    }
}

/// The face `fonts.<role>` names, or `None` when unset. A file that cannot
/// be read is an error; one egui cannot use falls back with a warning.
fn face(
    role: &str,
    v: &Option<String>,
    mono: bool,
    warnings: &mut Vec<String>,
) -> Result<Option<FontFamily>, ThemeError> {
    let Some(v) = v else { return Ok(None) };
    if let Some(fam) = bundled_family(v) {
        return Ok(Some(fam));
    }
    let key = format!("fonts.{role}");
    let lower = v.to_ascii_lowercase();
    if !(lower.ends_with(".ttf") || lower.ends_with(".otf")) {
        let names: Vec<&str> = BUNDLED_FACES.iter().map(|(n, _)| *n).collect();
        return Err(ThemeError::invalid(
            key,
            format!(
                "'{v}' is neither a bundled face ({}) nor a .ttf/.otf file",
                names.join(", ")
            ),
        ));
    }
    let (v, bytes) = font_file(&key, v, crate::registry::get().font_bytes(v))?;
    match crate::render::fonts::register_file_face(bytes, mono) {
        Ok(fam) => Ok(Some(fam)),
        Err(e) => {
            warnings.push(format!("{key}: {v}: {e}; using the default face"));
            Ok(None)
        }
    }
}

/// Every role's face; unset roles fall back to `body` (or egui's defaults).
pub(super) fn resolve(f: &Fonts, warnings: &mut Vec<String>) -> Result<ThemeFonts, ThemeError> {
    let body = face("body", &f.body, false, warnings)?.unwrap_or(FontFamily::Proportional);
    let display = face("display", &f.display, false, warnings)?.unwrap_or(body.clone());
    let lead = face("lead", &f.lead, false, warnings)?.unwrap_or(body.clone());
    let strong = face("strong", &f.strong, false, warnings)?.unwrap_or(body.clone());
    let mono = face("mono", &f.mono, true, warnings)?.unwrap_or(FontFamily::Monospace);
    Ok(ThemeFonts {
        display,
        body,
        lead,
        strong,
        mono,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_names_resolve_and_others_are_errors() {
        let mut w = Vec::new();
        let fam = face("body", &Some("hanken-light".into()), false, &mut w).unwrap();
        assert_eq!(fam, Some(FontFamily::Name(FONT_BODY_LIGHT.into())));
        assert_eq!(face("body", &None, false, &mut w).unwrap(), None);
        let e = face("body", &Some("comic-sans".into()), false, &mut w).unwrap_err();
        assert!(
            e.to_string()
                .starts_with("fonts.body: 'comic-sans' is neither")
        );
        let e = face("mono", &Some("x.ttf".into()), true, &mut w).unwrap_err();
        assert_eq!(
            e.to_string(),
            "fonts.mono: 'x.ttf' must be a file in a theme folder, or a font an extension registers"
        );
        assert!(w.is_empty());
    }

    #[test]
    fn an_embedded_theme_finds_a_font_its_extension_registers() {
        static FONT: &[u8] = b"the font file";
        let (shown, bytes) = font_file("fonts.body", "Acme.ttf", Some(FONT)).unwrap();
        assert_eq!((shown.as_str(), bytes.as_slice()), ("Acme.ttf", FONT));
        assert!(font_file("fonts.body", "Acme.ttf", None).is_err());
        // A file on disk wins over the name lookup.
        let dir = std::env::temp_dir().join(format!("mdeck-font-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("Acme.ttf");
        std::fs::write(&file, b"on disk").unwrap();
        let (_, bytes) = font_file("fonts.body", file.to_str().unwrap(), Some(FONT)).unwrap();
        assert_eq!(bytes, b"on disk");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn every_bundled_face_has_a_family() {
        for (name, _) in BUNDLED_FACES {
            assert!(bundled_family(name).is_some(), "{name}");
        }
    }

    #[test]
    fn unset_roles_follow_body() {
        let f = Fonts {
            body: Some("hanken-regular".into()),
            ..Fonts::default()
        };
        let t = resolve(&f, &mut Vec::new()).unwrap();
        assert_eq!(t.display, t.body);
        assert_eq!(t.strong, t.body);
        assert_eq!(t.mono, FontFamily::Monospace);
    }
}
