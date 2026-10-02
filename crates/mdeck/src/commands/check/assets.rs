//! `--check` category `assets`: what the deck asks `mdeck ai` to make and
//! does not have (missing), or has from an older version of its slide or
//! style (stale), and a manifest that cannot be read. Covers artworks on an
//! art engine, `generate:` images and icons, and the manifest's point
//! clouds (a `picture` name that resolves nowhere is the
//! `point-cloud` category's).

use std::path::Path;

use crate::assets::manifest::{self, Kind, Manifest, State};
use crate::assets::{placeholders, style};
use crate::check::{CheckCategory, CheckWarning};
use crate::parser;
use crate::render;

fn warn(slide: usize, line: usize, message: String) -> CheckWarning {
    CheckWarning {
        slide,
        line,
        category: CheckCategory::Assets,
        message,
        place: None,
    }
}

fn deck_name(deck: &Path) -> String {
    deck.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}

/// Every asset warning for the deck at `deck` in `theme`.
pub fn asset_warnings(
    deck: &Path,
    presentation: &parser::Presentation,
    theme: &crate::theme::Theme,
) -> Vec<CheckWarning> {
    let mut out = Vec::new();
    let manifest = match manifest::load(deck) {
        Ok(m) => m,
        Err(e) => {
            out.push(warn(
                0,
                0,
                format!("the asset manifest cannot be read: {e}"),
            ));
            None
        }
    };
    out.extend(artwork_warnings(deck, presentation, theme));
    let (_, problems) = crate::extensions::packs::styles(deck.parent());
    out.extend(
        problems
            .into_iter()
            .map(|p| warn(0, 0, format!("a pack style cannot be used: {p}"))),
    );
    let config = crate::config::Config::load_or_default().with_packs(deck.parent());
    let styles = style::resolve(&config, &presentation.meta, None);
    out.extend(placeholder_warnings(
        deck,
        presentation,
        manifest.as_ref(),
        &styles,
    ));
    if let Some(m) = &manifest {
        for a in m.assets.iter().filter(|a| a.kind == Kind::PointCloud) {
            if !Manifest::file(deck, a).exists() {
                out.push(warn(
                    0,
                    0,
                    format!("point cloud file {} is missing", a.file),
                ));
            }
        }
    }
    out
}

/// Artworks on an art engine: stale per slide, missing in one line.
fn artwork_warnings(
    deck: &Path,
    presentation: &parser::Presentation,
    theme: &crate::theme::Theme,
) -> Vec<CheckWarning> {
    let Some(medium) = theme.engine.medium() else {
        return Vec::new();
    };
    let mut art = render::art::gallery::DeckArt::new(Some(deck), false);
    art.sync(presentation, theme);
    let mut out = Vec::new();
    let mut missing = Vec::new();
    for (i, (slide, r)) in presentation.slides.iter().zip(art.resolved()).enumerate() {
        if !render::art::wants_art(slide) {
            continue;
        }
        match r {
            None => missing.push(i + 1),
            Some(r) if !r.file.exists() => out.push(warn(
                i + 1,
                slide.setting_line("picture-prompt"),
                format!("artwork file {} is missing", r.file.display()),
            )),
            Some(r) if r.state == State::Stale => out.push(warn(
                i + 1,
                slide.setting_line("picture-prompt"),
                format!(
                    "artwork is stale (the slide changed since it was drawn); run `mdeck ai pictures {} --stale`",
                    deck_name(deck)
                ),
            )),
            _ => {}
        }
    }
    if !missing.is_empty() {
        let list: Vec<String> = missing.iter().map(|n| n.to_string()).collect();
        out.push(warn(
            0,
            0,
            format!(
                "{} slide{} no artwork for the {} engine ({}); run `mdeck ai pictures {}` to draw {}",
                missing.len(),
                if missing.len() == 1 { " has" } else { "s have" },
                medium.name,
                list.join(", "),
                deck_name(deck),
                if missing.len() == 1 { "it" } else { "them" },
            ),
        ));
    }
    out
}

/// The line of a slide that holds `needle`, or the slide's own line.
fn line_of(slide: &parser::Slide, needle: &str) -> usize {
    slide
        .raw_source
        .lines()
        .position(|l| l.contains(needle))
        .map_or(slide.line, |offset| slide.line_at(offset))
}

/// `generate:` images and icons with no asset, a missing file, or a stale one.
fn placeholder_warnings(
    deck: &Path,
    presentation: &parser::Presentation,
    manifest: Option<&Manifest>,
    styles: &style::Styles,
) -> Vec<CheckWarning> {
    let mut out = Vec::new();
    for p in placeholders::scan(presentation) {
        let slide = &presentation.slides[p.slide];
        let (style, command, needle) = match p.kind {
            Kind::Icon => (styles.icon.id(), "icons", "generate"),
            _ => (styles.image.id(), "images", "](generate:)"),
        };
        let needle = if p.prompt.is_empty() {
            needle
        } else {
            p.prompt.as_str()
        };
        let line = line_of(slide, needle);
        let what = if p.prompt.is_empty() {
            p.kind.name().to_string()
        } else {
            format!("{} \"{}\"", p.kind.name(), p.prompt)
        };
        let found = manifest.and_then(|m| p.find(m, presentation, &style).map(|f| (m, f)));
        match found {
            None => out.push(warn(
                p.slide + 1,
                line,
                format!(
                    "{what} is not generated yet; run `mdeck ai {command} {}`",
                    deck_name(deck)
                ),
            )),
            Some((m, f)) if !Manifest::file(deck, &m.assets[f.index]).exists() => out.push(warn(
                p.slide + 1,
                line,
                format!("{what}: file {} is missing", m.assets[f.index].file),
            )),
            Some((_, f)) if f.state == State::Stale => out.push(warn(
                p.slide + 1,
                line,
                format!(
                    "{what} is stale (made in another style); run `mdeck ai {command} {} --stale`",
                    deck_name(deck)
                ),
            )),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::manifest::Asset;

    fn tmp(tag: &str) -> std::path::PathBuf {
        let d =
            std::env::temp_dir().join(format!("mdeck-assets-check-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn missing_stale_and_current_placeholders() {
        let dir = tmp("ph");
        let deck = dir.join("talk.md");
        let md = "# One\n\n![a rocket](generate:)\n\n# Two\n\n![a boat](generate:)\n\n# Three\n\n![a kite](generate:)\n\n# Four\n\n![a cat](generate:)\n";
        let pres = parser::parse(md);
        let styles = style::Styles {
            image: style::Style::new("default", "img", Vec::new()),
            icon: style::Style::new("default", "ico", Vec::new()),
        };
        let mut m = Manifest::new();
        let mut add = |prompt: &str, style: &str, file: &str| {
            m.upsert(Asset {
                placeholder: Some(prompt.into()),
                ..Asset::new(Kind::Image, file.into(), style)
            })
        };
        add("a boat", &styles.image.id(), "images/boat.png");
        add("a kite", "old-style", "images/kite.png");
        add("a cat", &styles.image.id(), "images/cat.png");
        std::fs::create_dir_all(dir.join("talk.assets/images")).unwrap();
        std::fs::write(dir.join("talk.assets/images/boat.png"), b"x").unwrap();
        std::fs::write(dir.join("talk.assets/images/kite.png"), b"x").unwrap();
        let w = placeholder_warnings(&deck, &pres, Some(&m), &styles);
        let got: Vec<(usize, bool)> = w
            .iter()
            .map(|w| (w.slide, w.category == CheckCategory::Assets))
            .collect();
        assert_eq!(got, [(1, true), (3, true), (4, true)], "{w:?}");
        assert!(
            w[0].message.contains("not generated yet"),
            "{}",
            w[0].message
        );
        assert!(w[0].message.contains("mdeck ai images talk.md"));
        assert_eq!(w[0].line, 3, "points at the placeholder line");
        assert!(w[1].message.contains("stale"), "{}", w[1].message);
        assert!(w[2].message.contains("missing"), "{}", w[2].message);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn an_unreadable_manifest_is_reported() {
        let dir = tmp("bad");
        let deck = dir.join("talk.md");
        std::fs::create_dir_all(dir.join("talk.assets")).unwrap();
        std::fs::write(dir.join("talk.assets/manifest.yaml"), "assets: [x\n").unwrap();
        let pres = parser::parse("# One\n\n- a\n");
        let w = asset_warnings(&deck, &pres, &crate::theme::Theme::dark());
        assert!(
            w.iter().any(|w| w.message.contains("cannot be read")),
            "{w:?}"
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
