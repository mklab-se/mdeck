//! Which engine a deck runs on (`--engine`, `@engine`, the theme's), and
//! what that engine will not show, for `--check` and the startup line.

use super::EngineKind;
use crate::parser::Slide;

/// The engine a deck runs on: `--engine` on the command line, then the deck's
/// `@engine`, then the theme's own. An unknown `--engine` is an error (the
/// caller stops); an unknown or unavailable `@engine` is a warning and the
/// theme's engine is kept.
pub fn choose(
    cli: Option<&str>,
    deck: Option<&str>,
) -> Result<(Option<EngineKind>, Vec<String>), String> {
    let mut warnings = Vec::new();
    if let Some(name) = cli.map(str::trim) {
        let kind = EngineKind::from_name(&name.to_ascii_lowercase()).ok_or_else(|| {
            format!(
                "--engine: '{name}' is not an engine ({})",
                EngineKind::names()
            )
        })?;
        if !kind.available() {
            return Err(format!("--engine: {name} is not in this build of MDeck"));
        }
        return Ok((Some(kind), warnings));
    }
    let Some(name) = deck.map(str::trim).filter(|n| !n.is_empty()) else {
        return Ok((None, warnings));
    };
    match EngineKind::from_name(&name.to_ascii_lowercase()) {
        Some(kind) if kind.available() => Ok((Some(kind), warnings)),
        Some(_) => {
            warnings.push(format!(
                "@engine: {name} is not in this build of MDeck; using the theme's engine"
            ));
            Ok((None, warnings))
        }
        None => {
            warnings.push(format!(
                "@engine: '{name}' is not an engine ({}); using the theme's engine",
                EngineKind::names()
            ));
            Ok((None, warnings))
        }
    }
}

/// `theme` running on `kind` instead of its own engine (colours, fonts and
/// logo stay the theme's). A countdown the new engine cannot draw itself
/// becomes the plain one.
pub fn with_engine(
    mut theme: crate::theme::Theme,
    kind: Option<EngineKind>,
) -> crate::theme::Theme {
    let Some(kind) = kind else {
        return theme;
    };
    theme.engine = kind;
    if theme.countdown == crate::theme::Countdown::Burst && !kind.capabilities().countdown {
        theme.countdown = crate::theme::Countdown::Plain;
    }
    theme
}

/// What `kind` will not show on `slide`, one message per thing, for
/// `--check` and the summary line when presenting or exporting.
/// `has_story`: a story (sidecar or inline) exists for the slide.
pub fn unsupported(kind: EngineKind, slide: &Slide, has_story: bool) -> Vec<String> {
    let caps = kind.capabilities();
    let mut out = Vec::new();
    if let Some(name) = &slide.illustration
        && !caps.illustrations
    {
        out.push(format!(
            "@illustration: {name} is not shown by the {} engine",
            kind.name()
        ));
    }
    if let Some(problems) = kind.def().and_then(|d| d.problems) {
        out.extend(problems(slide));
    }
    if !caps.art
        && let Some(scene) = crate::render::art::slide_scene(slide)
    {
        let short: String = scene.chars().take(40).collect();
        let media: Vec<&str> = EngineKind::ALL
            .iter()
            .filter(|k| k.capabilities().art)
            .map(|k| k.name())
            .collect();
        out.push(format!(
            "@art: '{short}' is not drawn by the {} engine (engines that draw art: {})",
            kind.name(),
            media.join(", ")
        ));
    }
    if has_story && !caps.stories {
        out.push(format!(
            "the slide's story is not played by the {} engine",
            kind.name()
        ));
    }
    out
}

/// One line for stderr when a deck has content its engine will not show, or
/// `None` when everything shows.
pub fn unsupported_summary(
    kind: EngineKind,
    presentation: &crate::parser::Presentation,
    stories: &[bool],
) -> Option<String> {
    let n = presentation
        .slides
        .iter()
        .enumerate()
        .filter(|(i, s)| {
            !unsupported(kind, s, stories.get(*i).copied().unwrap_or(false)).is_empty()
        })
        .count();
    (n > 0).then(|| {
        format!(
            "the {} engine does not show everything on {n} slide{}; run `mdeck <deck> --check` for details",
            kind.name(),
            if n == 1 { "" } else { "s" }
        )
    })
}
// The tests run decks on the particles engine.
#[cfg(all(test, feature = "particles"))]
mod tests {
    use super::*;

    #[test]
    fn cli_beats_deck_beats_theme() {
        assert_eq!(
            choose(Some("plain"), Some("particles")).unwrap().0,
            Some(EngineKind::Plain)
        );
        assert_eq!(
            choose(None, Some(" Particles ")).unwrap().0,
            Some(EngineKind::Particles)
        );
        assert_eq!(choose(None, None).unwrap().0, None);
        // a bad --engine stops; a bad @engine warns and keeps the theme's
        assert!(
            choose(Some("fireworks"), None)
                .unwrap_err()
                .contains("plain, particles")
        );
        let (kind, warnings) = choose(None, Some("fireworks")).unwrap();
        assert_eq!(kind, None);
        assert!(warnings[0].contains("fireworks"), "{warnings:?}");
    }

    #[test]
    fn with_engine_keeps_the_look_and_fixes_the_countdown() {
        let ember = crate::theme::Theme::ember();
        let plain = with_engine(ember.clone(), Some(EngineKind::Plain));
        assert_eq!(plain.engine, EngineKind::Plain);
        assert_eq!(plain.accent, ember.accent);
        assert_eq!(plain.countdown, crate::theme::Countdown::Plain);
        let dark = with_engine(crate::theme::Theme::dark(), Some(EngineKind::Particles));
        assert_eq!(dark.engine, EngineKind::Particles);
        assert_eq!(with_engine(ember.clone(), None).engine, ember.engine);
    }

    #[test]
    fn unsupported_names_illustrations_and_stories() {
        let pres = crate::parser::parse("# A\n@illustration: server\n\n- one\n\n# B\n\n- two\n");
        let a = &pres.slides[0];
        assert!(unsupported(EngineKind::Particles, a, true).is_empty());
        let plain = unsupported(EngineKind::Plain, a, true);
        assert_eq!(plain.len(), 2, "{plain:?}");
        assert!(plain[0].contains("server") && plain[1].contains("story"));
        assert!(unsupported(EngineKind::Plain, &pres.slides[1], false).is_empty());
        let line = unsupported_summary(EngineKind::Plain, &pres, &[false, false]).unwrap();
        assert!(line.contains("1 slide;"), "{line}");
        assert!(unsupported_summary(EngineKind::Particles, &pres, &[]).is_none());
    }
}
