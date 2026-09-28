//! The deck's theme and logo.

use crate::check::{CheckCategory, CheckWarning};
use crate::parser;

/// Theme warnings (on slide 0, since a theme belongs to the deck): the name
/// does not resolve, the file is invalid, something in it fell back, or
/// text is hard to read on its background.
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
        category: CheckCategory::Theme,
        message,
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
            let w = built
                .warnings
                .iter()
                .cloned()
                .chain(crate::theme::validate::review(&built.theme))
                .map(|m| warn(format!("{name}: {m}")))
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
        let deck = |theme: &str| parser::parse(&format!("---\n@theme: {theme}\n---\n# A\n"));

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
        // The config default applies when the deck names no theme.
        let plain = parser::parse("# A\n");
        assert!(!theme_warnings(&plain, Some("murky"), &dir).is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }
}
