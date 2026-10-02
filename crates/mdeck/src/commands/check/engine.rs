//! The engine a deck runs on: content it will not show.

use crate::check::{CheckCategory, CheckWarning};
use crate::parser;

/// The theme the deck runs in, on the engine it runs on (`--engine`,
/// `engine`, then the theme's), and any problem with the deck's `engine`.
/// An unknown `--engine` is an error.
pub fn deck_theme(
    presentation: &parser::Presentation,
    config_default: Option<&str>,
    base: &std::path::Path,
    cli: Option<&str>,
) -> anyhow::Result<(crate::theme::Theme, Vec<String>)> {
    use crate::theme::lookup;
    let (kind, problems) = crate::engines::choose(cli, presentation.meta.engine.as_deref())
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let name = lookup::select(presentation.meta.theme.as_deref(), config_default);
    let themes = lookup::Lookup::for_deck(Some(base));
    let (theme, _) = lookup::resolve_or_default(&themes, &name);
    Ok((crate::engines::with_engine(theme, kind), problems))
}

/// A message about a setting (`picture: ...`) points at that setting's
/// line; any other at the slide's.
fn message_line(slide: &parser::Slide, message: &str) -> usize {
    message
        .split_once(':')
        .filter(|(name, _)| crate::language::setting(name).is_some())
        .map_or(slide.line, |(name, _)| slide.setting_line(name))
}

/// What the code design set the theme names (EXT-05) does not show, one
/// warning per slide and thing, as the set's `unsupported` reports it.
pub fn design_set_warnings(
    presentation: &parser::Presentation,
    theme: &crate::theme::Theme,
) -> Vec<CheckWarning> {
    // a board engine draws every slide with its own set (see `engine_warnings`)
    if theme.engine.is_board() {
        return Vec::new();
    }
    let Some(set) = theme.code_design_set() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (i, slide) in presentation.slides.iter().enumerate() {
        let content = crate::engines::host::convert::slide(slide);
        for p in set.unsupported(&content) {
            out.push(CheckWarning {
                slide: i + 1,
                line: p.line.unwrap_or(slide.line),
                category: CheckCategory::Theme,
                message: format!("design set {}: {}", set.name(), p.message),
                place: None,
            });
        }
    }
    out
}

/// Content the deck's engine will not show, one warning per slide and thing.
pub fn engine_warnings(
    presentation: &parser::Presentation,
    kind: crate::engines::EngineId,
) -> Vec<CheckWarning> {
    let mut out = Vec::new();
    for (i, slide) in presentation.slides.iter().enumerate() {
        for message in crate::engines::unsupported(kind, slide) {
            out.push(CheckWarning {
                slide: i + 1,
                line: message_line(slide, &message),
                category: CheckCategory::Engine,
                message,
                place: None,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// EXT-05: what a code design set does not show is reported, on the
    /// slide it is on, under the theme.
    #[test]
    fn a_code_design_set_reports_what_it_does_not_show() {
        let p = parser::parse("# A\n\nx\n\n# B\n\n```rust\nfn x() {}\n```\n");
        let themes = crate::theme::lookup::Lookup::for_deck(None);
        let theme = themes
            .load(crate::registry::test_extensions::THEME)
            .unwrap()
            .theme;
        assert!(theme.code_design_set().is_some());
        let w = design_set_warnings(&p, &theme);
        assert_eq!(w.len(), 1, "{w:?}");
        assert_eq!((w[0].slide, w[0].line), (2, p.slides[1].line));
        assert_eq!(w[0].category, CheckCategory::Theme);
        assert_eq!(w[0].message, "design set test-cards: code is not shown");
        assert!(design_set_warnings(&p, &crate::theme::Theme::dark()).is_empty());
    }

    #[test]
    fn a_message_about_a_directive_names_its_line() {
        let p = parser::parse("# A\n\n- one\n\n---\n\n# B\n\n- two\n<!-- picture: cup -->\n");
        let slide = &p.slides[1];
        assert_eq!(message_line(slide, "picture: cup is not shown"), 10);
        assert_eq!(message_line(slide, "picture-prompt: x is not drawn"), 7);
        assert_eq!(message_line(slide, "code blocks are not shown"), 7);
    }
}
