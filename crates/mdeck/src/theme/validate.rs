//! Checks on a resolved theme that are advice, not errors: text that is
//! hard to read on its background (THM-09), and keys that do nothing with
//! the theme's engine and design set (THM-12, THM-13).

use eframe::egui::Color32;

use super::arrangement::{Emphasis, Font, Ink, RoleStyle, Size};
use super::color::mix;
use super::{EngineId, Theme, contrast};
use crate::parser::Design;

/// Minimum contrast ratios (WCAG 2 AA: 4.5 for body text, 3 for large text).
const BODY_TEXT: f32 = 4.5;
const LARGE_TEXT: f32 = 3.0;

/// Text at least this tall (px on a 1920x1080 slide, which a full screen
/// 1080p display shows 1:1) is large text in WCAG's sense (24 CSS px).
const LARGE_PX: f32 = 24.0;

/// Readability problems in `theme`, one line each: every text colour the
/// theme renders on the ground it is drawn on.
pub fn review(theme: &Theme) -> Vec<String> {
    let mut out = Vec::new();
    for (what, fg, bg, min) in pairs(theme) {
        let r = contrast(fg, bg);
        if r < min - 0.005 {
            out.push(format!(
                "{what} has a contrast of {r:.1}:1 (at least {min}:1 reads comfortably)"
            ));
        }
    }
    out
}

/// Every text/ground pair the theme renders, with the contrast it needs.
fn pairs(t: &Theme) -> Vec<(String, Color32, Color32, f32)> {
    let bg = t.background;
    let mut v = vec![
        (
            "colors.text on colors.background".to_string(),
            t.foreground,
            bg,
            BODY_TEXT,
        ),
        (
            "colors.heading on colors.background".into(),
            t.heading_color,
            bg,
            LARGE_TEXT,
        ),
        // `**bold**` runs are body text
        (
            "colors.strong on colors.background".into(),
            t.strong,
            bg,
            BODY_TEXT,
        ),
        // links and inline accents in body text, eyebrow numerals
        (
            "colors.accent on colors.background".into(),
            t.accent,
            bg,
            BODY_TEXT,
        ),
        // captions, footers, eyebrows, the slide counter: small text
        (
            "colors.muted on colors.background".into(),
            t.muted,
            bg,
            BODY_TEXT,
        ),
        (
            "colors.code-text on colors.code-background".into(),
            t.code_foreground,
            t.code_background,
            BODY_TEXT,
        ),
    ];
    // `*emphasis*` and links in the soft accent, where the design set
    // draws them so (the editorial body)
    if Design::ALL
        .iter()
        .any(|d| t.arrangement(*d).ornaments.emphasis == Emphasis::Accent)
    {
        v.push((
            "colors.accent-soft on colors.background".into(),
            t.accent_soft,
            bg,
            BODY_TEXT,
        ));
    }
    // every role of every design, at the opacity and size it is drawn
    let mut seen = std::collections::HashSet::new();
    for design in Design::ALL {
        let a = t.arrangement(design);
        let r = &a.roles;
        for (name, style) in [
            ("eyebrow", &r.eyebrow),
            ("title", &r.title),
            ("subtitle", &r.subtitle),
            ("kicker", &r.kicker),
            ("byline", &r.byline),
            ("heading", &r.heading),
            ("statement", &r.statement),
            ("lead", &r.lead),
            ("body", &r.body),
            ("list", &r.list),
            ("nested", &r.nested),
            ("quote", &r.quote),
            ("attribution", &r.attribution),
            ("caption", &r.caption),
        ] {
            let fg = mix(bg, ink(t, style.color), style.opacity);
            let px = size_px(t, style);
            let min = if px >= LARGE_PX {
                LARGE_TEXT
            } else {
                BODY_TEXT
            };
            if seen.insert((fg, min.to_bits())) {
                v.push((
                    format!("the {name} role ({} design)", design.name()),
                    fg,
                    bg,
                    min,
                ));
            }
        }
    }
    v
}

/// The colour a role's colour token names.
fn ink(t: &Theme, i: Ink) -> Color32 {
    match i {
        Ink::Text => t.foreground,
        Ink::Heading => t.heading_color,
        Ink::Muted => t.muted,
        Ink::Strong => t.strong,
        Ink::Accent => t.accent,
        Ink::AccentSoft => t.accent_soft,
        Ink::Secondary => t.secondary,
        Ink::Bright => t.bright_text(),
        Ink::Rule => t.rule,
    }
}

/// A role's smallest size in px at 1920x1080 (`level` counts as body,
/// the smallest heading level drawn).
fn size_px(t: &Theme, s: &RoleStyle) -> f32 {
    let base = match &s.size {
        Size::Px(p) => *p,
        Size::Token(tok) => match tok.as_str() {
            "h1" => t.h1_size,
            "h2" => t.h2_size,
            "h3" => t.h3_size,
            "code" => t.code_size,
            _ => t.body_size,
        },
    };
    base * s.scale
}

/// Settings `engine` reads from the theme's `engine:` block: the keys its
/// definition declares (`EngineDef::settings`), plus the core's own keys
/// that apply to it (the particle tints on every engine that paints a layer
/// under the copy, the art keys on an engine with a medium).
pub fn engine_keys(engine: EngineId) -> Vec<&'static str> {
    let mut keys: Vec<&'static str> = engine.def().settings.iter().map(|s| s.key).collect();
    if engine.paints() && !engine.is_board() {
        keys.extend(["light", "cool"]);
    }
    if engine.medium().is_some() {
        keys.extend(["kind", "style", "references"]);
    }
    keys
}

/// Engines that draw on the theme's `page:` (paper, a slate) and look wrong
/// without one (ENG-12).
fn needs_page(engine: EngineId) -> bool {
    engine.def().needs.page
}

/// Keys of `theme` that have no effect with its engine and design set
/// (THM-13), and what its engine needs but the theme lacks (THM-12).
pub fn inert(theme: &Theme) -> Vec<String> {
    let mut out = Vec::new();
    let engine = theme.engine;
    let known = engine_keys(engine);
    for (key, _) in &theme.engine_block {
        if !known.contains(&key.as_str()) {
            let reads = if known.is_empty() {
                "no settings".to_string()
            } else {
                known.join(", ")
            };
            out.push(format!(
                "engine.{key} does nothing on the {} engine (it reads {reads})",
                engine.name()
            ));
        }
    }
    if needs_page(engine) && theme.page.is_none() {
        out.push(format!(
            "the {} engine draws on a page; without `page:` it draws on the bare background",
            engine.name()
        ));
    }
    // a board draws every slide itself, in its own faces
    if !engine.is_board() && theme.fonts.lead != theme.fonts.body && !uses_font(theme, Font::Lead) {
        out.push(format!(
            "fonts.lead is not used by the {} design set (no arrangement role uses the lead font)",
            theme.arrangements.set
        ));
    }
    out
}

/// Whether any role of any design is drawn in `font`.
fn uses_font(t: &Theme, font: Font) -> bool {
    Design::ALL.iter().any(|d| {
        let r = &t.arrangement(*d).roles;
        [
            &r.eyebrow,
            &r.title,
            &r.subtitle,
            &r.kicker,
            &r.byline,
            &r.heading,
            &r.statement,
            &r.lead,
            &r.body,
            &r.list,
            &r.nested,
            &r.quote,
            &r.attribution,
            &r.caption,
        ]
        .iter()
        .any(|s| s.font == font)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::file::ThemeFile;
    use crate::theme::lookup::{BUILTIN, builtin_file, load_builtin};

    fn over_dark(yaml: &str) -> Theme {
        let f = ThemeFile::parse(yaml)
            .unwrap()
            .over(&builtin_file("dark").unwrap());
        Theme::build("x", &f).unwrap().theme
    }

    #[test]
    fn builtins_read_comfortably() {
        let failing: Vec<String> = BUILTIN
            .iter()
            .flat_map(|(name, _)| {
                let t = load_builtin(name).unwrap();
                review(&t).into_iter().map(move |r| format!("{name}: {r}"))
            })
            .collect();
        assert!(failing.is_empty(), "{failing:#?}");
    }

    #[test]
    fn every_rendered_text_pair_is_checked() {
        let t = load_builtin("dark").unwrap();
        let p = pairs(&t);
        for what in ["colors.muted", "colors.strong", "code-text"] {
            assert!(p.iter().any(|(w, ..)| w.contains(what)), "{what}");
        }
        assert!(p.iter().any(|(w, ..)| w.contains(" role (")));
        // the soft accent is text only where emphasis is drawn in it
        assert!(!p.iter().any(|(w, ..)| w.contains("accent-soft")));
        let e = over_dark("designs: editorial");
        assert!(pairs(&e).iter().any(|(w, ..)| w.contains("accent-soft")));
    }

    #[test]
    fn grey_on_grey_is_flagged() {
        let r = review(&over_dark(
            "colors: { background: '#777777', text: '#888888' }",
        ));
        assert!(r.iter().any(|l| l.starts_with("colors.text")), "{r:?}");
        // small muted text needs body contrast
        let r = review(&over_dark("colors: { muted: '#555555' }"));
        assert!(r.iter().any(|l| l.starts_with("colors.muted")), "{r:?}");
    }

    #[test]
    fn builtins_have_no_inert_keys() {
        for (name, _) in BUILTIN.iter().copied() {
            let t = load_builtin(name).unwrap();
            assert!(inert(&t).is_empty(), "{name}: {:?}", inert(&t));
        }
    }

    #[test]
    fn inert_keys_are_named() {
        let t = over_dark("engine: { name: plain, palette: iron }");
        assert_eq!(
            inert(&t),
            ["engine.palette does nothing on the plain engine (it reads no settings)"]
        );
        let t = over_dark("fonts: { lead: hanken-light }");
        assert!(inert(&t)[0].starts_with("fonts.lead is not used by the standard"));
        let t = over_dark("designs: editorial\nfonts: { lead: hanken-light }");
        assert!(inert(&t).is_empty(), "{:?}", inert(&t));
    }

    #[cfg(feature = "sketch")]
    #[test]
    fn a_paper_engine_without_a_page_is_reported() {
        let t = over_dark("engine: sketch");
        assert!(inert(&t).iter().any(|l| l.contains("draws on a page")));
    }
}
