//! The art sidecar: `talk.art.yaml` next to `talk.md`, and the images in an
//! `art/` folder beside it. Each entry records which slide a picture belongs
//! to, in which style it was made and from which slide source (a hash), so a
//! picture goes stale when its slide changes and is never shown in a style
//! it was not made for.

use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::parser::{Presentation, Slide};
use crate::render::sidecar::{self as shared, Keyed, Match};

pub const VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    /// 1-based slide number when generated (the hash is the key).
    pub slide: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Hash of the slide source and the deck's `art-world`.
    pub hash: String,
    /// The style it was made in (see [`super::style::Style::id`]).
    pub style: String,
    /// The image, relative to the deck's folder.
    pub file: String,
    /// What was asked for: the scene, as written or as the chat model wrote it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated: Option<String>,
    /// Keep this picture for this slide number whatever the slide says:
    /// never stale, never regenerated.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub pinned: bool,
}

impl Keyed for Entry {
    fn slide(&self) -> usize {
        self.slide
    }
    fn hash(&self) -> &str {
        &self.hash
    }
    fn pinned(&self) -> bool {
        self.pinned
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Sidecar {
    pub version: u32,
    #[serde(default)]
    pub slides: Vec<Entry>,
}

/// `talk.art.yaml` for `talk.md`.
pub fn path_for(deck: &Path) -> PathBuf {
    shared::beside(deck, "art.yaml")
}

/// The folder the images go in: `art/` next to the deck.
pub fn folder_for(deck: &Path) -> PathBuf {
    deck.parent().unwrap_or(Path::new(".")).join("art")
}

/// Load the sidecar for a deck, if there is one.
pub fn load(deck: &Path) -> Result<Option<Sidecar>> {
    shared::read(&path_for(deck))
}

pub fn save(deck: &Path, sidecar: &Sidecar) -> Result<PathBuf> {
    let path = path_for(deck);
    let header = "# Generated art for the art engines (like line), written by `mdeck ai art`.\n# Set `pinned: true` to keep a picture for its slide number whatever the slide says.\n";
    shared::write(&path, header, sidecar)?;
    Ok(path)
}

/// Stable hash of what a slide's picture depends on: the slide source and
/// the deck's `art-world`.
pub fn slide_hash(slide: &Slide, world: Option<&str>) -> String {
    shared::slide_hash(slide, world)
}

/// Where a slide's picture came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// A pinned entry for this slide number.
    Pinned,
    /// An entry made from this slide as it reads now.
    Current,
    /// An entry for this slide number made before the slide changed: still
    /// shown, and `--check` says so.
    Stale,
}

/// A slide's picture, resolved.
#[derive(Clone, Debug, PartialEq)]
pub struct Resolved {
    pub file: PathBuf,
    pub source: Source,
}

/// What every slide shows in `style`: pinned entries first, then an entry
/// for the slide as it reads now, then a stale one for its number. Slides
/// that take no art get `None`.
pub fn resolve(
    deck: &Path,
    presentation: &Presentation,
    sidecar: Option<&Sidecar>,
    style: &str,
) -> Vec<Option<Resolved>> {
    let base = deck.parent().unwrap_or(Path::new("."));
    let world = presentation.meta.art_world.as_deref();
    presentation
        .slides
        .iter()
        .enumerate()
        .map(|(i, slide)| {
            if !super::wants_art(slide) {
                return None;
            }
            let entries = sidecar?.slides.iter().filter(|e| e.style == style);
            let hash = slide_hash(slide, world);
            let (entry, found) = shared::find(entries, i + 1, &hash)?;
            Some(Resolved {
                file: base.join(&entry.file),
                source: match found {
                    Match::Pinned => Source::Pinned,
                    Match::Current => Source::Current,
                    Match::Stale => Source::Stale,
                },
            })
        })
        .collect()
}

/// Counts for the startup line and `--check`: slides that take art but have
/// none in this style, and slides whose picture is stale.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Coverage {
    pub wanted: usize,
    pub missing: usize,
    pub stale: usize,
}

pub fn coverage(presentation: &Presentation, resolved: &[Option<Resolved>]) -> Coverage {
    let mut c = Coverage::default();
    for (slide, r) in presentation.slides.iter().zip(resolved) {
        if !super::wants_art(slide) {
            continue;
        }
        c.wanted += 1;
        match r {
            None => c.missing += 1,
            Some(r) if !r.file.exists() => c.missing += 1,
            Some(r) if r.source == Source::Stale => c.stale += 1,
            _ => {}
        }
    }
    c
}

/// Record a new picture for slide `index`, replacing what that slide had
/// in the same style.
pub fn upsert(
    sc: &mut Sidecar,
    presentation: &Presentation,
    index: usize,
    style: &str,
    file: String,
    scene: Option<String>,
    generated: String,
) {
    let slide = &presentation.slides[index];
    let entry = Entry {
        slide: index + 1,
        title: slide.title(),
        hash: slide_hash(slide, presentation.meta.art_world.as_deref()),
        style: style.to_string(),
        file,
        scene,
        generated: Some(generated),
        pinned: false,
    };
    sc.slides
        .retain(|e| !(e.style == style && (e.slide == index + 1 || e.hash == entry.hash)));
    sc.slides.push(entry);
    sc.slides
        .sort_by(|a, b| a.slide.cmp(&b.slide).then(a.style.cmp(&b.style)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    fn deck() -> Presentation {
        parser::parse(
            "# Launch\n\nA talk\n\n# Why\n\n- one\n\n# Code\n\n```rust\nfn main() {}\n```\n\n# How\n\n- two\n",
        )
    }

    #[test]
    fn paths_sit_next_to_the_deck() {
        let deck = Path::new("/talks/launch.md");
        assert_eq!(path_for(deck), Path::new("/talks/launch.art.yaml"));
        assert_eq!(folder_for(deck), Path::new("/talks/art"));
    }

    #[test]
    fn current_beats_stale_and_styles_never_mix() {
        let pres = deck();
        let mut sc = Sidecar {
            version: VERSION,
            slides: vec![],
        };
        upsert(
            &mut sc,
            &pres,
            1,
            "line-1",
            "art/a.jpg".into(),
            None,
            "t".into(),
        );
        upsert(
            &mut sc,
            &pres,
            1,
            "sketch-1",
            "art/b.jpg".into(),
            None,
            "t".into(),
        );
        // an old picture for slide 4, made before the slide changed
        sc.slides.push(Entry {
            slide: 4,
            title: None,
            hash: "old".into(),
            style: "line-1".into(),
            file: "art/c.jpg".into(),
            scene: None,
            generated: None,
            pinned: false,
        });
        let deck_path = Path::new("/talks/launch.md");
        let r = resolve(deck_path, &pres, Some(&sc), "line-1");
        assert_eq!(r.len(), 4);
        assert!(r[0].is_none(), "no picture made for the title yet");
        assert_eq!(r[1].as_ref().unwrap().file, Path::new("/talks/art/a.jpg"));
        assert_eq!(r[1].as_ref().unwrap().source, Source::Current);
        assert!(r[2].is_none(), "a code slide takes no art");
        assert_eq!(r[3].as_ref().unwrap().source, Source::Stale);
        let s = resolve(deck_path, &pres, Some(&sc), "sketch-1");
        assert_eq!(s[1].as_ref().unwrap().file, Path::new("/talks/art/b.jpg"));
        assert!(s[3].is_none());
        // the files do not exist, so coverage counts them missing
        let c = coverage(&pres, &r);
        assert_eq!(c.wanted, 3);
        assert_eq!(c.missing, 3);
    }

    #[test]
    fn upsert_replaces_the_slide_in_its_style_only() {
        let pres = deck();
        let mut sc = Sidecar::default();
        upsert(
            &mut sc,
            &pres,
            1,
            "line-1",
            "art/a.jpg".into(),
            None,
            "t".into(),
        );
        upsert(
            &mut sc,
            &pres,
            1,
            "line-1",
            "art/a2.jpg".into(),
            None,
            "t".into(),
        );
        upsert(
            &mut sc,
            &pres,
            1,
            "sketch-1",
            "art/b.jpg".into(),
            None,
            "t".into(),
        );
        assert_eq!(sc.slides.len(), 2);
        assert_eq!(sc.slides[0].file, "art/a2.jpg");
        let text = serde_norway::to_string(&sc).unwrap();
        let back: Sidecar = serde_norway::from_str(&text).unwrap();
        assert_eq!(back.slides, sc.slides);
    }

    #[test]
    fn pinned_holds_its_slide_number() {
        let pres = deck();
        let sc = Sidecar {
            version: VERSION,
            slides: vec![Entry {
                slide: 2,
                title: None,
                hash: "hand".into(),
                style: "line-1".into(),
                file: "art/mine.png".into(),
                scene: None,
                generated: None,
                pinned: true,
            }],
        };
        let r = resolve(Path::new("talk.md"), &pres, Some(&sc), "line-1");
        assert_eq!(r[1].as_ref().unwrap().source, Source::Pinned);
    }
}
