//! Placeholders in the deck: what it asks `mdeck ai` to make, left in the
//! source as written.
//!
//! - An image: `![a rocket at dawn](generate:)`. The alt text is the prompt;
//!   an empty one (`![](generate:)`) has the chat model write a prompt from
//!   the slide.
//! - A diagram icon: `Gateway (icon: generate:, prompt: "an API gateway")`.
//!   Without `prompt:` the node's label is the prompt.
//!
//! When the deck opens, [`apply`] points each placeholder at the asset the
//! manifest has for it, in memory only. One with no asset yet stays a
//! placeholder: an image draws as a framed card with its prompt, an icon as
//! the generic node icon, and `--check` reports it.

use std::path::Path;

use super::manifest::{self, Found, Kind, Manifest};
use super::style::Styles;
use crate::parser::{Block, Layout, Presentation};
use crate::prompt::Orientation;

/// The image path that asks for a generated image.
pub const IMAGE: &str = "generate:";

/// Whether an image path is the generate placeholder.
pub fn is_image(path: &str) -> bool {
    path.trim() == IMAGE
}

/// Whether a diagram node's `icon:` value asks for a generated icon.
pub fn is_icon(icon: &str) -> bool {
    matches!(icon.trim(), "generate:" | "generate")
}

/// One thing the deck asks to have generated.
#[derive(Clone, Debug, PartialEq)]
pub struct Placeholder {
    pub kind: Kind,
    /// 0-based slide index.
    pub slide: usize,
    /// The prompt as written (an image's alt text, an icon's `prompt:` or
    /// label); empty for an image that leaves the prompt to the chat model.
    pub prompt: String,
    /// The shape the picture should take on its slide (images).
    pub orientation: Orientation,
}

impl Placeholder {
    /// The manifest entry filling it, if any, and its state.
    pub fn find(&self, manifest: &Manifest, pres: &Presentation, style: &str) -> Option<Found> {
        let slide = &pres.slides[self.slide];
        manifest.placeholder(
            self.kind,
            &self.prompt,
            self.slide + 1,
            &manifest::slide_hash(slide, None),
            style,
        )
    }
}

/// The shape an image takes on a slide of `layout`.
fn orientation_for(layout: Layout) -> Orientation {
    match layout {
        Layout::Bullet | Layout::Code | Layout::Quote | Layout::Content | Layout::TwoColumn => {
            Orientation::Vertical
        }
        _ => Orientation::Horizontal,
    }
}

/// The `generate:` icon on a diagram line, as `(start, end)` of the icon
/// value, and the prompt (`prompt: "..."`, or the node's label).
fn icon_on_line(line: &str) -> Option<((usize, usize), String)> {
    let trimmed = line.trim_end();
    if !trimmed.ends_with(')') {
        return None;
    }
    let open = trimmed.rfind(" (")? + 1;
    let meta = &trimmed[open + 1..trimmed.len() - 1];
    let key = meta.find("icon:")?;
    let value_start = open + 1 + key + "icon:".len();
    let rest = &trimmed[value_start..trimmed.len() - 1];
    let lead = rest.len() - rest.trim_start().len();
    let value = rest.trim_start();
    let len = value
        .find(|c: char| c == ',' || c.is_whitespace())
        .unwrap_or(value.len());
    if !is_icon(&value[..len]) {
        return None;
    }
    let span = (value_start + lead, value_start + lead + len);
    let prompt = quoted_prompt(meta).unwrap_or_else(|| {
        trimmed[..open]
            .trim()
            .trim_start_matches(['-', '*'])
            .trim()
            .to_string()
    });
    Some((span, prompt))
}

/// `prompt: "..."` or `prompt: '...'` inside node metadata.
fn quoted_prompt(meta: &str) -> Option<String> {
    let after = meta[meta.find("prompt:")? + "prompt:".len()..].trim_start();
    let quote = after.chars().next().filter(|c| *c == '"' || *c == '\'')?;
    let body = &after[1..];
    Some(body[..body.find(quote)?].trim().to_string())
}

/// Every placeholder in the deck, slide by slide: images, then icons.
/// Several placeholders with the same prompt are one asset, listed once.
pub fn scan(pres: &Presentation) -> Vec<Placeholder> {
    let mut out: Vec<Placeholder> = Vec::new();
    let mut push = |p: Placeholder| {
        let dup = !p.prompt.is_empty()
            && out
                .iter()
                .any(|q| q.kind == p.kind && q.prompt.trim() == p.prompt.trim());
        let same_slide = p.prompt.is_empty()
            && out
                .iter()
                .any(|q| q.kind == p.kind && q.prompt.is_empty() && q.slide == p.slide);
        if !dup && !same_slide {
            out.push(p);
        }
    };
    for (i, slide) in pres.slides.iter().enumerate() {
        for block in &slide.blocks {
            match block {
                Block::Image { alt, path, .. } if is_image(path) => push(Placeholder {
                    kind: Kind::Image,
                    slide: i,
                    prompt: alt.trim().to_string(),
                    orientation: orientation_for(slide.layout),
                }),
                Block::Diagram { content } => {
                    for line in content.lines() {
                        if let Some((_, prompt)) = icon_on_line(line) {
                            push(Placeholder {
                                kind: Kind::Icon,
                                slide: i,
                                prompt,
                                orientation: Orientation::Horizontal,
                            });
                        }
                    }
                }
                _ => {}
            }
        }
    }
    out
}

/// Point every placeholder that has an asset at its file (relative to the
/// deck's folder), in memory. Pinned, current and stale assets all show;
/// placeholders without one are left as they are.
pub fn apply(pres: &mut Presentation, deck: &Path, manifest: Option<&Manifest>, styles: &Styles) {
    let Some(manifest) = manifest else {
        return;
    };
    let resolved: Vec<(Placeholder, String)> = scan(pres)
        .into_iter()
        .filter_map(|p| {
            let style = match p.kind {
                Kind::Icon => styles.icon.id(),
                _ => styles.image.id(),
            };
            let found = p.find(manifest, pres, &style)?;
            let file = manifest::deck_relative(deck, &manifest.assets[found.index]);
            Some((p, file))
        })
        .collect();
    let lookup = |kind: Kind, slide: usize, prompt: &str| {
        resolved
            .iter()
            .find(|(p, _)| {
                p.kind == kind
                    && p.prompt.trim() == prompt.trim()
                    && (!prompt.trim().is_empty() || p.slide == slide)
            })
            .map(|(_, f)| f.clone())
    };
    for (i, slide) in pres.slides.iter_mut().enumerate() {
        for block in &mut slide.blocks {
            match block {
                Block::Image { alt, path, .. } if is_image(path) => {
                    if let Some(file) = lookup(Kind::Image, i, alt) {
                        *path = file;
                    }
                }
                Block::Diagram { content } => {
                    let mut changed = false;
                    let lines: Vec<String> = content
                        .lines()
                        .map(|line| match icon_on_line(line) {
                            Some(((a, b), prompt)) => match lookup(Kind::Icon, i, &prompt) {
                                Some(file) => {
                                    changed = true;
                                    format!("{}{file}{}", &line[..a], &line[b..])
                                }
                                None => line.to_string(),
                            },
                            None => line.to_string(),
                        })
                        .collect();
                    if changed {
                        *content = lines.join("\n");
                    }
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::manifest::Asset;
    use crate::assets::style::Style;
    use crate::parser;

    const DECK: &str = "# Launch\n\n![a rocket at dawn](generate:)\n\n# Again\n\n- point\n\n![a rocket at dawn @fill](generate:)\n\n# Auto\n\n![](generate:)\n\n# Arch\n\n```@architecture\n- Gateway (icon: generate:, prompt: \"an API gateway\", pos: 1,1)\n- Db (icon: generate, pos: 2,1)\n- Cache (icon: database, pos: 3,1)\n```\n\n# Real\n\n![photo](images/real.png)\n";

    fn styles() -> Styles {
        Styles {
            image: Style::new("default", "img", Vec::new()),
            icon: Style::new("default", "ico", Vec::new()),
        }
    }

    #[test]
    fn scan_finds_images_and_icons_once_each() {
        let pres = parser::parse(DECK);
        let found = scan(&pres);
        let got: Vec<(Kind, usize, &str)> = found
            .iter()
            .map(|p| (p.kind, p.slide, p.prompt.as_str()))
            .collect();
        assert_eq!(
            got,
            [
                (Kind::Image, 0, "a rocket at dawn"),
                (Kind::Image, 2, ""),
                (Kind::Icon, 3, "an API gateway"),
                (Kind::Icon, 3, "Db"),
            ]
        );
        assert_eq!(found[0].orientation, Orientation::Horizontal);
    }

    #[test]
    fn icon_lines_without_generate_are_left_alone() {
        assert!(icon_on_line("- Cache (icon: database, pos: 3,1)").is_none());
        assert!(icon_on_line("- Gateway").is_none());
        let (span, prompt) = icon_on_line("- Api (icon: generate:, prompt: 'the api')").unwrap();
        assert_eq!(prompt, "the api");
        assert_eq!(
            &"- Api (icon: generate:, prompt: 'the api')"[span.0..span.1],
            "generate:"
        );
    }

    #[test]
    fn apply_points_placeholders_at_their_files_and_leaves_the_rest() {
        let mut pres = parser::parse(DECK);
        let s = styles();
        let mut m = Manifest::new();
        m.upsert(Asset {
            placeholder: Some("a rocket at dawn".into()),
            ..Asset::new(Kind::Image, "images/rocket.png".into(), &s.image.id())
        });
        m.upsert(Asset {
            placeholder: Some("an API gateway".into()),
            ..Asset::new(Kind::Icon, "icons/gateway.png".into(), "an-old-style")
        });
        apply(&mut pres, Path::new("/t/talk.md"), Some(&m), &s);
        let images: Vec<&str> = pres
            .slides
            .iter()
            .flat_map(|s| &s.blocks)
            .filter_map(|b| match b {
                Block::Image { path, .. } => Some(path.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            images,
            [
                "talk.assets/images/rocket.png",
                "talk.assets/images/rocket.png",
                "generate:",
                "images/real.png"
            ]
        );
        let Block::Diagram { content } = pres.slides[3]
            .blocks
            .iter()
            .find(|b| matches!(b, Block::Diagram { .. }))
            .unwrap()
        else {
            unreachable!()
        };
        assert!(
            content.contains("(icon: talk.assets/icons/gateway.png, prompt:"),
            "a stale icon still shows: {content}"
        );
        assert!(
            content.contains("Db (icon: generate, pos: 2,1)"),
            "{content}"
        );
        assert!(content.contains("Cache (icon: database"), "{content}");
    }

    #[test]
    fn apply_without_a_manifest_changes_nothing() {
        let mut pres = parser::parse(DECK);
        let before = format!("{:?}", pres.slides[0].blocks);
        apply(&mut pres, Path::new("talk.md"), None, &styles());
        assert_eq!(before, format!("{:?}", pres.slides[0].blocks));
    }
}
