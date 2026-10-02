//! The deck's theme and logo.

use crate::check::{CheckCategory, CheckWarning};
use crate::parser;

/// Theme warnings: the name does not resolve (on slide 0, since a theme
/// belongs to the deck), the file is invalid, something in it fell back, or
/// text is hard to read on its background (labelled with where the theme
/// comes from, see [`theme_place`]).
pub fn theme_warnings(
    presentation: &parser::Presentation,
    config_default: Option<&str>,
    base: &std::path::Path,
) -> Vec<CheckWarning> {
    use crate::theme::lookup;
    let name = lookup::select(presentation.meta.theme.as_deref(), config_default);
    let themes = lookup::Lookup::for_deck(Some(base));
    let warn = |message: String| CheckWarning {
        slide: 0,
        line: 0,
        category: CheckCategory::Theme,
        message,
        place: None,
    };
    let (theme, mut out) = match themes.load(&name) {
        Err(e) => (
            lookup::load_builtin(lookup::DEFAULT_THEME).expect("default theme"),
            vec![warn(format!(
                "{e}; the deck falls back to {}",
                lookup::DEFAULT_THEME
            ))],
        ),
        Ok(built) => {
            let place = place_of(&name, &built.theme, base);
            let w = built
                .warnings
                .iter()
                .cloned()
                .chain(crate::theme::validate::review(&built.theme))
                .map(|m| CheckWarning {
                    place: Some(place.clone()),
                    ..warn(m)
                })
                .collect();
            (built.theme, w)
        }
    };
    // The deck's own logo keys, and whether the logo file can be drawn.
    let (logos, problems) = crate::render::logo::resolve_slides(&theme, presentation, base);
    out.extend(problems.into_iter().map(warn));
    for logo in logos.distinct() {
        if let Err(e) = crate::render::logo::load_image(&logo.path) {
            out.push(warn(format!("logo: {e}")));
        }
    }
    out
}

/// [`theme_place`] for the deck's theme, a theme file next to the deck named
/// relative to `base` (the deck's folder).
pub fn place_of(name: &str, theme: &crate::theme::Theme, base: &std::path::Path) -> String {
    let source = theme
        .source
        .as_deref()
        .map(|p| p.strip_prefix(base).unwrap_or(p));
    theme_place(name, source, crate::registry::get())
}

/// Where the theme `name` comes from, for labelling its problems: its file,
/// the extension that embeds it, or mdeck itself.
pub fn theme_place(
    name: &str,
    source: Option<&std::path::Path>,
    registry: &crate::registry::Registry,
) -> String {
    let name = name.trim();
    if let Some(path) = source {
        return format!("theme '{name}' ({})", path.display());
    }
    let key = name.to_ascii_lowercase();
    match registry
        .registrations()
        .find(|r| r.kind == "theme" && r.name == key)
        .map(|r| r.origin)
    {
        Some(origin) if origin != "mdeck" => {
            format!("theme '{name}' (from extension {origin})")
        }
        _ => format!("theme '{name}' (built-in)"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_problems_are_reported_on_the_deck() {
        let dir = std::env::temp_dir().join(format!("mdeck-check-theme-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("themes")).unwrap();
        std::fs::write(
            dir.join("themes/murky.yaml"),
            "colors: { background: '#777777', text: '#808080' }\n",
        )
        .unwrap();
        std::fs::write(dir.join("themes/typo.yaml"), "colours: {}\n").unwrap();
        let deck = |theme: &str| parser::parse(&format!("---\ntheme: {theme}\n---\n# A\n"));

        assert!(theme_warnings(&deck("dark"), None, &dir).is_empty());
        let unknown = theme_warnings(&deck("solarized"), None, &dir);
        assert!(
            unknown[0].message.contains("unknown theme 'solarized'"),
            "{unknown:?}"
        );
        let typo = theme_warnings(&deck("typo"), None, &dir);
        assert!(typo[0].message.contains("colours"), "{typo:?}");
        let murky = theme_warnings(&deck("murky"), None, &dir);
        assert!(
            murky.iter().any(|w| w.message.contains("contrast")),
            "{murky:?}"
        );
        assert_eq!(murky[0].category, CheckCategory::Theme);
        // Labelled with the theme file, not as slide 0.
        let label = murky[0].to_string();
        assert!(
            label.starts_with("  theme 'murky' (themes/murky.yaml): [theme] "),
            "{label}"
        );
        // The config default applies when the deck names no theme.
        let plain = parser::parse("# A\n");
        assert!(!theme_warnings(&plain, Some("murky"), &dir).is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn embedded_themes_are_labelled_with_their_extension() {
        let mut registry = crate::registry::Registry::new();
        registry.theme("dark-ish", "name: dark-ish\n").unwrap();
        registry.set_origin("acme-brand");
        registry.theme("acme-night", "name: acme-night\n").unwrap();
        assert_eq!(
            theme_place("acme-night", None, &registry),
            "theme 'acme-night' (from extension acme-brand)"
        );
        assert_eq!(
            theme_place("dark-ish", None, &registry),
            "theme 'dark-ish' (built-in)"
        );
        let path = std::path::Path::new("themes/murky.yaml");
        assert_eq!(
            theme_place("murky", Some(path), &registry),
            "theme 'murky' (themes/murky.yaml)"
        );
        let w = CheckWarning {
            slide: 0,
            line: 0,
            category: CheckCategory::Theme,
            message: "contrast".into(),
            place: Some(theme_place("acme-night", None, &registry)),
        };
        assert_eq!(
            w.to_string(),
            "  theme 'acme-night' (from extension acme-brand): [theme] contrast"
        );
    }
}
