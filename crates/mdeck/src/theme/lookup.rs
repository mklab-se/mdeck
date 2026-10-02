//! Finding themes by name: the deck's `themes/` folder, then
//! `~/.config/mdeck/themes/`, then the built-ins (the same order as
//! illustrations). A theme is `<name>.yaml` or `<name>/theme.yaml`.

use std::path::{Path, PathBuf};

use super::file::ThemeFile;
use super::{Built, Theme, ThemeError};

/// The built-in themes, in `Shift+T` order. A theme that runs on an engine
/// behind a cargo feature (`ember`, `autumn` and `winter` on particles,
/// `marquee` on led, `departures` on splitflap, `stack` on blocks,
/// `blueprint` and `chalkboard` on line, `sketchbook` on sketch,
/// `watercolour` on watercolour, `darkroom` on darkroom, `thermal` on
/// thermal) is left out of a build without that
/// feature.
pub const BUILTIN: &[(&str, &str)] = &[
    ("dark", include_str!("../../themes/dark.yaml")),
    ("light", include_str!("../../themes/light.yaml")),
    ("nord", include_str!("../../themes/nord.yaml")),
    #[cfg(feature = "particles")]
    ("ember", include_str!("../../themes/ember.yaml")),
    ("spring", include_str!("../../themes/spring.yaml")),
    ("summer", include_str!("../../themes/summer.yaml")),
    #[cfg(feature = "particles")]
    ("autumn", include_str!("../../themes/autumn.yaml")),
    #[cfg(feature = "particles")]
    ("winter", include_str!("../../themes/winter.yaml")),
    #[cfg(feature = "led")]
    ("marquee", include_str!("../../themes/marquee.yaml")),
    #[cfg(feature = "splitflap")]
    ("departures", include_str!("../../themes/departures.yaml")),
    #[cfg(feature = "blocks")]
    ("stack", include_str!("../../themes/stack.yaml")),
    #[cfg(feature = "line")]
    ("blueprint", include_str!("../../themes/blueprint.yaml")),
    #[cfg(feature = "sketch")]
    ("sketchbook", include_str!("../../themes/sketchbook.yaml")),
    #[cfg(feature = "line")]
    ("chalkboard", include_str!("../../themes/chalkboard.yaml")),
    #[cfg(feature = "watercolour")]
    ("watercolour", include_str!("../../themes/watercolour.yaml")),
    #[cfg(feature = "darkroom")]
    ("darkroom", include_str!("../../themes/darkroom.yaml")),
    #[cfg(feature = "thermal")]
    ("thermal", include_str!("../../themes/thermal.yaml")),
];

/// The theme used when nothing names one.
pub const DEFAULT_THEME: &str = "dark";

/// Longest `extends` chain followed before giving up (catches cycles).
const MAX_DEPTH: usize = 8;

/// Where a theme was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// `<deck dir>/themes/`.
    Deck(PathBuf),
    /// `~/.config/mdeck/themes/`.
    User(PathBuf),
    /// Built into the binary.
    Builtin,
}

impl Origin {
    pub fn label(&self) -> &'static str {
        match self {
            Origin::Deck(_) => "deck",
            Origin::User(_) => "user",
            Origin::Builtin => "built-in",
        }
    }

    pub fn path(&self) -> Option<&Path> {
        match self {
            Origin::Deck(p) | Origin::User(p) => Some(p),
            Origin::Builtin => None,
        }
    }
}

/// A theme that can be loaded by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub name: String,
    pub origin: Origin,
}

/// The folders themes are looked up in.
#[derive(Debug, Clone, Default)]
pub struct Lookup {
    /// The deck's `themes/` folder.
    pub deck: Option<PathBuf>,
    /// The user's themes folder.
    pub user: Option<PathBuf>,
}

/// The user theme folder.
pub fn user_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("mdeck").join("themes"))
}

impl Lookup {
    /// Lookup for a deck in `deck_dir` (its folder), with the user folder.
    pub fn for_deck(deck_dir: Option<&Path>) -> Self {
        Lookup {
            deck: deck_dir.map(|d| d.join("themes")),
            user: user_dir(),
        }
    }

    fn dirs(&self) -> Vec<(PathBuf, bool)> {
        let mut v = Vec::new();
        if let Some(d) = &self.deck {
            v.push((d.clone(), true));
        }
        if let Some(u) = &self.user {
            v.push((u.clone(), false));
        }
        v
    }

    /// Every place `name` is defined, highest priority first.
    pub fn find_all(&self, name: &str) -> Vec<Found> {
        let mut out = Vec::new();
        for (dir, deck) in self.dirs() {
            for path in [
                dir.join(format!("{name}.yaml")),
                dir.join(format!("{name}.yml")),
                dir.join(name).join("theme.yaml"),
            ] {
                if path.is_file() {
                    let origin = if deck {
                        Origin::Deck(path)
                    } else {
                        Origin::User(path)
                    };
                    out.push(Found {
                        name: name.to_string(),
                        origin,
                    });
                    break;
                }
            }
        }
        if BUILTIN.iter().any(|(n, _)| *n == name) {
            out.push(Found {
                name: name.to_string(),
                origin: Origin::Builtin,
            });
        }
        out
    }

    /// Every theme visible here, one per name: built-ins first (in their
    /// `Shift+T` order, possibly shadowed), then user and deck themes by name.
    pub fn available(&self) -> Vec<Found> {
        let mut names: Vec<String> = BUILTIN.iter().map(|(n, _)| n.to_string()).collect();
        let mut extra = std::collections::BTreeSet::new();
        for (dir, _) in self.dirs() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for e in entries.flatten() {
                let path = e.path();
                let name = if path.is_dir() {
                    path.join("theme.yaml")
                        .is_file()
                        .then(|| e.file_name().to_string_lossy().to_string())
                } else {
                    match path.extension().and_then(|x| x.to_str()) {
                        Some("yaml" | "yml") => {
                            path.file_stem().map(|s| s.to_string_lossy().to_string())
                        }
                        _ => None,
                    }
                };
                if let Some(n) = name.filter(|n| valid_name(n)) {
                    extra.insert(n);
                }
            }
        }
        for n in extra {
            if !names.contains(&n) {
                names.push(n);
            }
        }
        names
            .into_iter()
            .filter_map(|n| self.find_all(&n).into_iter().next())
            .collect()
    }

    /// Load a theme by name, or by path when `name` ends in `.yaml`/`.yml`
    /// (relative to the deck's folder).
    pub fn load(&self, name: &str) -> Result<Built, ThemeError> {
        let name = name.trim();
        if is_path(name) {
            let base = self
                .deck
                .as_ref()
                .and_then(|d| d.parent())
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."));
            let path = base.join(name);
            if !path.is_file() {
                return Err(ThemeError::NotFound(path));
            }
            let stem = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| name.to_string());
            return self.load_found(&Found {
                name: stem,
                origin: Origin::Deck(path),
            });
        }
        let key = name.to_ascii_lowercase();
        match self.find_all(&key).into_iter().next() {
            Some(found) => self.load_found(&found),
            None => Err(self.unknown(name)),
        }
    }

    /// The error for a name nothing defines, listing what exists.
    pub fn unknown(&self, name: &str) -> ThemeError {
        ThemeError::Unknown {
            name: name.to_string(),
            available: self.available().into_iter().map(|f| f.name).collect(),
        }
    }

    pub fn load_found(&self, found: &Found) -> Result<Built, ThemeError> {
        let mut warnings = Vec::new();
        let file = self.resolve(found, 0, &mut warnings)?;
        let mut built = Theme::build(&found.name, &file)?;
        built.theme.source = found.origin.path().map(Path::to_path_buf);
        warnings.append(&mut built.warnings);
        built.warnings = warnings;
        Ok(built)
    }

    /// The merged file for `found`, its `extends` chain applied and every
    /// file path made absolute against the folder of the file that names it.
    fn resolve(
        &self,
        found: &Found,
        depth: usize,
        warnings: &mut Vec<String>,
    ) -> Result<ThemeFile, ThemeError> {
        if depth > MAX_DEPTH {
            return Err(ThemeError::Cycle {
                name: found.name.clone(),
            });
        }
        let file = match &found.origin {
            Origin::Builtin => builtin_file(&found.name)?,
            Origin::Deck(p) | Origin::User(p) => {
                let text =
                    std::fs::read_to_string(p).map_err(|e| ThemeError::file(p.display(), e))?;
                let mut file =
                    ThemeFile::parse(&text).map_err(|e| ThemeError::file(p.display(), e))?;
                let base = p.parent().unwrap_or(Path::new("."));
                absolutize(&mut file, base, warnings);
                file
            }
        };
        // Built-ins without `extends` are complete; everything else extends
        // `dark` unless it says otherwise.
        let parent_name = match (&file.extends, &found.origin) {
            (Some(p), _) => Some(p.to_ascii_lowercase()),
            (None, Origin::Builtin) => None,
            (None, _) => Some("dark".to_string()),
        };
        let Some(parent_name) = parent_name else {
            return Ok(file);
        };
        // A theme may extend the one it shadows (`dark.yaml` extending `dark`):
        // skip itself when looking the parent up.
        let parent = self
            .find_all(&parent_name)
            .into_iter()
            .find(|f| f != found)
            .ok_or_else(|| ThemeError::NoParent {
                name: found.name.clone(),
                parent: parent_name.clone(),
            })?;
        let parent_file = self.resolve(&parent, depth + 1, warnings)?;
        Ok(file.over(&parent_file))
    }
}

/// Whether a font or syntax value names a file rather than a bundled face.
fn names_file(v: &str) -> bool {
    let l = v.to_ascii_lowercase();
    [".ttf", ".otf", ".tmtheme", ".png", ".svg"]
        .iter()
        .any(|x| l.ends_with(x))
}

/// Make every file path in `file` absolute against `base`, the folder of
/// the file that names it. A path that leaves the folder or does not exist
/// is dropped with a warning, so the value falls back to what the theme
/// extends.
fn absolutize(file: &mut ThemeFile, base: &Path, warnings: &mut Vec<String>) {
    let f = &mut file.fonts;
    let slots: [(&str, &mut Option<String>); 7] = [
        ("fonts.display", &mut f.display),
        ("fonts.body", &mut f.body),
        ("fonts.lead", &mut f.lead),
        ("fonts.strong", &mut f.strong),
        ("fonts.mono", &mut f.mono),
        ("code.syntax", &mut file.code.syntax),
        ("logo.file", &mut file.logo.file),
    ];
    for (key, slot) in slots {
        let Some(v) = slot.as_ref().filter(|v| names_file(v)) else {
            continue;
        };
        match super::confined_path(base, v) {
            Ok(p) => *slot = Some(p.to_string_lossy().to_string()),
            Err(e) => {
                warnings.push(format!("{key}: {e}; using the default"));
                *slot = None;
            }
        }
    }
    let refs = file.engine.as_ref().and_then(|b| b.list("references").ok().flatten());
    if let (Some(mut refs), Some(block)) = (refs, file.engine.as_mut()) {
        refs.retain_mut(|v| match super::confined_path(base, v) {
            Ok(p) => {
                *v = p.to_string_lossy().to_string();
                true
            }
            Err(e) => {
                warnings.push(format!("engine.references: {e}; left out"));
                false
            }
        });
        let list = refs.into_iter().map(serde_norway::Value::String).collect();
        block.set("references", serde_norway::Value::Sequence(list));
    }
}

/// A built-in theme file, unmerged.
pub fn builtin_file(name: &str) -> Result<ThemeFile, ThemeError> {
    let (_, text) = BUILTIN
        .iter()
        .find(|(n, _)| *n == name)
        .ok_or_else(|| ThemeError::NoBuiltin(name.to_string()))?;
    ThemeFile::parse(text)
}

/// A built-in theme, resolved without looking at any folder.
pub fn load_builtin(name: &str) -> Result<Theme, ThemeError> {
    Lookup::default()
        .load_found(&Found {
            name: name.to_string(),
            origin: Origin::Builtin,
        })
        .map(|b| b.theme)
}

fn is_path(name: &str) -> bool {
    let l = name.to_ascii_lowercase();
    l.ends_with(".yaml") || l.ends_with(".yml")
}

/// Theme names: lowercase letters, digits, `-` and `_`.
pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

/// Resolve the theme a deck asks for, falling back to the default theme
/// with a message when it cannot be loaded. Returns the theme and every
/// problem found (unknown name, invalid file, fallbacks inside the file).
pub fn resolve_or_default(lookup: &Lookup, name: &str) -> (Theme, Vec<String>) {
    match lookup.load(name) {
        Ok(b) => (b.theme, b.warnings),
        Err(e) => {
            let fallback = load_builtin(DEFAULT_THEME).expect("default theme");
            (fallback, vec![format!("{e}; using {DEFAULT_THEME}")])
        }
    }
}

/// The theme name a deck asks for: `theme`, then the config default, then
/// the built-in default. Blank values count as unset.
pub fn select(frontmatter: Option<&str>, config_default: Option<&str>) -> String {
    frontmatter
        .filter(|s| !s.trim().is_empty())
        .or(config_default.filter(|s| !s.trim().is_empty()))
        .unwrap_or(DEFAULT_THEME)
        .trim()
        .to_string()
}

/// Load every theme visible from a deck once so their font files are
/// registered before a window installs its fonts (switching with `Shift+T`
/// then never waits a frame). Problems are reported when a theme is used.
pub fn preload(lookup: &Lookup) {
    for found in lookup.available() {
        let _ = lookup.load_found(&found);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Tmp(PathBuf);
    impl Tmp {
        fn new(tag: &str) -> Self {
            let d = std::env::temp_dir().join(format!(
                "mdeck-themes-{tag}-{}-{:?}",
                std::process::id(),
                std::thread::current().id()
            ));
            let _ = std::fs::remove_dir_all(&d);
            std::fs::create_dir_all(&d).unwrap();
            Tmp(d)
        }
        fn write(&self, rel: &str, text: &str) -> PathBuf {
            let p = self.0.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, text).unwrap();
            p
        }
    }
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn lookup(t: &Tmp) -> Lookup {
        Lookup {
            deck: Some(t.0.join("deck").join("themes")),
            user: Some(t.0.join("user")),
        }
    }

    #[test]
    fn frontmatter_beats_config_beats_default() {
        assert_eq!(select(Some("nord"), Some("dark")), "nord");
        assert_eq!(select(None, Some("dark")), "dark");
        assert_eq!(select(None, None), "dark");
        assert_eq!(select(Some("  "), Some("dark")), "dark");
    }

    #[test]
    fn every_builtin_loads() {
        for (name, _) in BUILTIN.iter().copied() {
            load_builtin(name).unwrap_or_else(|e| panic!("{name}: {e}"));
        }
    }

    #[test]
    fn deck_beats_user_beats_builtin() {
        let t = Tmp::new("order");
        t.write("user/brand.yaml", "colors: { accent: '#00ff00' }\n");
        t.write(
            "user/dark.yaml",
            "extends: dark\ncolors: { accent: '#0000ff' }\n",
        );
        t.write(
            "deck/themes/brand/theme.yaml",
            "colors: { accent: '#ff0000' }\n",
        );
        let l = lookup(&t);
        let brand = l.load("brand").unwrap().theme;
        assert_eq!(brand.accent, eframe::egui::Color32::from_rgb(255, 0, 0));
        // A user theme may shadow a built-in and still extend it.
        let dark = l.load("dark").unwrap().theme;
        assert_eq!(dark.accent, eframe::egui::Color32::from_rgb(0, 0, 255));
        assert_eq!(dark.background, Theme::dark().background);
        let names: Vec<String> = l.available().into_iter().map(|f| f.name).collect();
        // the built-ins this build has, in order, then the user's own
        let expected: Vec<&str> = BUILTIN.iter().map(|(n, _)| *n).chain(["brand"]).collect();
        assert_eq!(names, expected);
        assert!(matches!(l.available()[0].origin, Origin::User(_)));
    }

    #[test]
    fn unset_keys_come_from_dark_by_default() {
        let t = Tmp::new("default-parent");
        t.write("user/tiny.yaml", "colors: { accent: '#123456' }\n");
        let th = lookup(&t).load("tiny").unwrap().theme;
        assert_eq!(th.background, Theme::dark().background);
        assert_eq!(th.name, "tiny");
    }

    #[cfg(feature = "particles")]
    #[test]
    fn extends_chains_and_cycles() {
        let t = Tmp::new("chain");
        t.write(
            "user/a.yaml",
            "extends: ember\ncolors: { accent: '#00ff00' }\n",
        );
        t.write("user/b.yaml", "extends: a\nsizes: { h1: 120 }\n");
        t.write("user/c.yaml", "extends: d\n");
        t.write("user/d.yaml", "extends: c\n");
        let l = lookup(&t);
        let b = l.load("b").unwrap().theme;
        assert_eq!(b.h1_size, 120.0);
        assert_eq!(b.accent, eframe::egui::Color32::from_rgb(0, 255, 0));
        assert_eq!(b.background, Theme::ember().background);
        assert!(l.load("c").unwrap_err().to_string().contains("cycle"));
        t.write("user/e.yaml", "extends: nothing-here\n");
        assert!(
            l.load("e")
                .unwrap_err()
                .to_string()
                .contains("nothing-here")
        );
    }

    #[cfg(feature = "particles")]
    #[test]
    fn unknown_names_list_what_exists() {
        let t = Tmp::new("unknown");
        let err = lookup(&t).load("solarized").unwrap_err().to_string();
        assert!(
            err.contains("solarized") && err.contains("dark, light, nord, ember, spring"),
            "{err}"
        );
        let (th, problems) = resolve_or_default(&lookup(&t), "solarized");
        assert_eq!(th.background, Theme::light().background);
        assert!(problems[0].contains("using light"));
    }

    #[test]
    fn a_path_loads_relative_to_the_deck() {
        let t = Tmp::new("path");
        t.write("deck/brand/look.yaml", "colors: { accent: '#010203' }\n");
        let th = lookup(&t).load("brand/look.yaml").unwrap().theme;
        assert_eq!(th.accent, eframe::egui::Color32::from_rgb(1, 2, 3));
        assert!(lookup(&t).load("missing.yaml").is_err());
    }

    #[test]
    fn a_broken_file_names_itself() {
        let t = Tmp::new("broken");
        t.write("user/typo.yaml", "colours: {}\n");
        let err = lookup(&t).load("typo").unwrap_err().to_string();
        assert!(
            err.contains("typo.yaml") && err.contains("colours"),
            "{err}"
        );
    }
}
