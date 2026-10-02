//! The engine a deck runs on: content it will not show, and the generated
//! art an art engine needs.

use crate::check::{CheckCategory, CheckWarning};
use crate::parser;
use crate::render;

/// The theme the deck runs in, on the engine it runs on (`--engine`,
/// `@engine`, then the theme's), and any problem with the deck's `@engine`.
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

/// Generated art on an art engine: the sidecar cannot be read, pictures
/// are stale (per slide) or missing (one line for the deck).
pub fn art_warnings(
    deck: &std::path::Path,
    presentation: &parser::Presentation,
    theme: &crate::theme::Theme,
) -> Vec<CheckWarning> {
    use render::art::sidecar::Source;
    let Some(medium) = theme.engine.medium() else {
        return Vec::new();
    };
    let mut art = render::art::gallery::DeckArt::new(Some(deck), false);
    art.sync(presentation, theme);
    let mut out: Vec<CheckWarning> = art
        .problems()
        .iter()
        .map(|p| CheckWarning {
            slide: 0,
            line: 0,
            category: CheckCategory::Art,
            message: format!("art sidecar could not be read: {p}"),
        })
        .collect();
    let mut missing = Vec::new();
    for (i, (slide, r)) in presentation.slides.iter().zip(art.resolved()).enumerate() {
        if !render::art::wants_art(slide) {
            continue;
        }
        match r {
            None => missing.push(i + 1),
            Some(r) if !r.file.exists() => out.push(CheckWarning {
                slide: i + 1,
                line: slide.directive_line("art"),
                category: CheckCategory::Art,
                message: format!("art file {} is missing", r.file.display()),
            }),
            Some(r) if r.source == Source::Stale => out.push(CheckWarning {
                slide: i + 1,
                line: slide.directive_line("art"),
                category: CheckCategory::Art,
                message: "art is stale (the slide changed since it was drawn); run `mdeck ai art --stale`".into(),
            }),
            _ => {}
        }
    }
    if !missing.is_empty() {
        let list: Vec<String> = missing.iter().map(|n| n.to_string()).collect();
        out.push(CheckWarning {
            slide: 0,
            line: 0,
            category: CheckCategory::Art,
            message: format!(
                "{} slide{} no picture for the {} engine ({}); run `mdeck ai art {}` to draw {}",
                missing.len(),
                if missing.len() == 1 { " has" } else { "s have" },
                medium.name,
                list.join(", "),
                deck.file_name().unwrap_or_default().to_string_lossy(),
                if missing.len() == 1 { "it" } else { "them" },
            ),
        });
    }
    out
}

/// A message about a directive (`@illustration: ...`) points at that
/// directive's line; any other at the slide's.
fn message_line(slide: &parser::Slide, message: &str) -> usize {
    message
        .strip_prefix('@')
        .and_then(|rest| rest.split_once(':'))
        .map_or(slide.line, |(name, _)| slide.directive_line(name))
}

/// Content the deck's engine will not show, one warning per slide and thing.
pub fn engine_warnings(
    presentation: &parser::Presentation,
    kind: crate::engines::EngineKind,
) -> Vec<CheckWarning> {
    let mut out = Vec::new();
    for (i, slide) in presentation.slides.iter().enumerate() {
        for message in crate::engines::unsupported(kind, slide) {
            out.push(CheckWarning {
                slide: i + 1,
                line: message_line(slide, &message),
                category: CheckCategory::Engine,
                message,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_message_about_a_directive_names_its_line() {
        let p = parser::parse("# A\n\n- one\n\n---\n\n# B\n\n- two\n@illustration: cup\n");
        let slide = &p.slides[1];
        assert_eq!(message_line(slide, "@illustration: cup is not shown"), 10);
        assert_eq!(message_line(slide, "@art: x is not drawn"), 7);
        assert_eq!(message_line(slide, "code blocks are not shown"), 7);
    }
}
