//! The manifest: `talk.assets/manifest.yaml` for `talk.md`. One entry per
//! generated asset, saying what kind it is, what it is for (a slide, by
//! number and by a hash of its source, or a placeholder's prompt), what was
//! asked for, in which style, which file holds it and its state.
//!
//! The state is **current** (made from the source as it reads now, in the
//! style in use), **stale** (its slide or style changed since; still shown,
//! and `--check` says so) or **pinned** (kept whatever the source says; never
//! regenerated). `pinned` is the author's choice and is kept as written;
//! `current` and `stale` are worked out again on every read, and refreshed in
//! the file whenever `mdeck ai` writes it.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

use crate::parser::Slide;

pub const VERSION: u32 = 2;

/// The manifest's file name inside the assets folder.
pub const FILE_NAME: &str = "manifest.yaml";

const HEADER: &str = "# Generated assets for this deck, written by `mdeck ai`.\n\
# state: current | stale | pinned. Set `state: pinned` to keep an asset whatever its slide says.\n\
# Files are relative to this folder; replace one by hand and it is used as it is.\n";

/// What an asset is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    /// A slide's picture on an art engine (`mdeck ai pictures`).
    Artwork,
    /// An image placeholder, `![prompt](generate:)` (`mdeck ai images`).
    Image,
    /// A diagram icon placeholder, `(icon: generate:, prompt: "...")` (`mdeck ai icons`).
    Icon,
    /// A point cloud for `@illustration` (`mdeck ai point-cloud`).
    PointCloud,
}

impl Kind {
    /// The sub-folder of the assets folder its files go in.
    pub fn folder(self) -> &'static str {
        match self {
            Kind::Artwork => "artworks",
            Kind::Image => "images",
            Kind::Icon => "icons",
            Kind::PointCloud => "point-clouds",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Kind::Artwork => "artwork",
            Kind::Image => "image",
            Kind::Icon => "icon",
            Kind::PointCloud => "point cloud",
        }
    }
}

/// Where an asset stands against the deck as it reads now.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    #[default]
    Current,
    Stale,
    Pinned,
}

/// One generated asset.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Asset {
    pub kind: Kind,
    /// The file, relative to the assets folder (`artworks/talk-01-line.jpg`).
    pub file: String,
    /// The placeholder it fills: an image's or icon's prompt as written in
    /// the deck, or a point cloud's name. Absent for artworks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    /// 1-based slide number when it was made.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slide: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// [`slide_hash`] of the slide it was made from (artworks, and images
    /// whose placeholder has no prompt).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    /// The style it was made in (a style id, see `assets::style`).
    #[serde(default)]
    pub style: String,
    /// What was asked for: the scene or prompt, without the style.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated: Option<String>,
    #[serde(default)]
    pub state: State,
}

impl Asset {
    /// A fresh entry of `kind` in `file` made in `style`.
    pub fn new(kind: Kind, file: String, style: &str) -> Self {
        Self {
            kind,
            file,
            placeholder: None,
            slide: None,
            title: None,
            hash: None,
            style: style.to_string(),
            prompt: None,
            generated: None,
            state: State::Current,
        }
    }

    pub fn pinned(&self) -> bool {
        self.state == State::Pinned
    }
}

/// `talk.assets/manifest.yaml`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub version: u32,
    #[serde(default)]
    pub assets: Vec<Asset>,
}

/// `<stem>.assets/` next to `deck` (`talk.md` has `talk.assets/`); a deck
/// without a name counts as `deck`.
pub fn folder_for(deck: &Path) -> PathBuf {
    let stem = deck
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "deck".to_string());
    deck.parent()
        .unwrap_or(Path::new("."))
        .join(format!("{stem}.assets"))
}

/// `talk.assets/manifest.yaml` for `talk.md`.
pub fn path_for(deck: &Path) -> PathBuf {
    folder_for(deck).join(FILE_NAME)
}

/// The path of an asset's file relative to the deck's folder
/// (`talk.assets/images/x.png`), the way the deck's own images are written.
pub fn deck_relative(deck: &Path, asset: &Asset) -> String {
    let folder = folder_for(deck);
    let name = folder
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    format!("{name}/{}", asset.file)
}

/// The deck's manifest, if it has one.
pub fn load(deck: &Path) -> Result<Option<Manifest>> {
    let path = path_for(deck);
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path).map_err(|e| anyhow!("{}: {e}", path.display()))?;
    serde_norway::from_str(&text)
        .map(Some)
        .map_err(|e| anyhow!("{}: {e}", path.display()))
}

/// Write the manifest (and its folder), entries ordered by kind and slide.
pub fn save(deck: &Path, manifest: &Manifest) -> Result<PathBuf> {
    let path = path_for(deck);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| anyhow!("{}: {e}", dir.display()))?;
    }
    let mut m = manifest.clone();
    m.version = VERSION;
    m.assets.sort_by(|a, b| {
        (a.kind as u8, a.slide, &a.placeholder, &a.style).cmp(&(
            b.kind as u8,
            b.slide,
            &b.placeholder,
            &b.style,
        ))
    });
    let text = serde_norway::to_string(&m).map_err(|e| anyhow!("{e}"))?;
    std::fs::write(&path, format!("{HEADER}{text}"))
        .map_err(|e| anyhow!("{}: {e}", path.display()))?;
    Ok(path)
}

/// Stable hash of a slide's source and one deck-level text (the artwork
/// world) that what was made for it depends on.
pub fn slide_hash(slide: &Slide, deck_text: Option<&str>) -> String {
    let mut h = DefaultHasher::new();
    slide.raw_source.trim().hash(&mut h);
    deck_text.unwrap_or("").trim().hash(&mut h);
    format!("{:016x}", h.finish())
}

/// An entry found for something the deck asks for: its index in
/// [`Manifest::assets`] and its state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Found {
    pub index: usize,
    pub state: State,
}

impl Manifest {
    pub fn new() -> Self {
        Self {
            version: VERSION,
            assets: Vec::new(),
        }
    }

    /// The absolute path of an asset's file.
    pub fn file(deck: &Path, asset: &Asset) -> PathBuf {
        folder_for(deck).join(&asset.file)
    }

    /// A slide's artwork in `style`, matched as before: a pinned entry for
    /// the slide number first, then an entry made from the slide as it
    /// reads now (whatever its number was), then an older entry for the
    /// number (stale). Artworks in other styles are never shown.
    pub fn artwork(&self, number: usize, hash: &str, style: &str) -> Option<Found> {
        self.keyed(
            |a| a.kind == Kind::Artwork && a.style == style,
            number,
            hash,
        )
    }

    /// The asset filling a placeholder of `kind` with prompt `text` on slide
    /// `number` (1-based; `hash` is its [`slide_hash`]), made in `style`. A
    /// placeholder with a prompt is matched by the prompt: pinned first,
    /// then one in `style` (current), then one in another style (stale). An
    /// empty prompt is matched by the slide, like an artwork.
    pub fn placeholder(
        &self,
        kind: Kind,
        text: &str,
        number: usize,
        hash: &str,
        style: &str,
    ) -> Option<Found> {
        let text = text.trim();
        if text.is_empty() {
            let found = self.keyed(
                |a| a.kind == kind && a.placeholder.as_deref().is_some_and(|p| p.is_empty()),
                number,
                hash,
            )?;
            let state = match found.state {
                State::Current if self.assets[found.index].style != style => State::Stale,
                s => s,
            };
            return Some(Found { state, ..found });
        }
        let candidates = || {
            self.assets.iter().enumerate().filter(|(_, a)| {
                a.kind == kind && a.placeholder.as_deref().map(str::trim) == Some(text)
            })
        };
        candidates()
            .find(|(_, a)| a.pinned())
            .map(|(index, _)| Found {
                index,
                state: State::Pinned,
            })
            .or_else(|| {
                candidates()
                    .find(|(_, a)| a.style == style)
                    .map(|(index, _)| Found {
                        index,
                        state: State::Current,
                    })
            })
            .or_else(|| {
                candidates().next().map(|(index, _)| Found {
                    index,
                    state: State::Stale,
                })
            })
    }

    /// Pinned for the number, then current by hash, then stale by number.
    fn keyed(&self, pred: impl Fn(&Asset) -> bool, number: usize, hash: &str) -> Option<Found> {
        let candidates = || self.assets.iter().enumerate().filter(|(_, a)| pred(a));
        candidates()
            .find(|(_, a)| a.pinned() && a.slide == Some(number))
            .map(|(index, _)| Found {
                index,
                state: State::Pinned,
            })
            .or_else(|| {
                candidates()
                    .find(|(_, a)| a.hash.as_deref() == Some(hash))
                    .map(|(index, _)| Found {
                        index,
                        state: State::Current,
                    })
            })
            .or_else(|| {
                candidates()
                    .find(|(_, a)| a.slide == Some(number))
                    .map(|(index, _)| Found {
                        index,
                        state: State::Stale,
                    })
            })
    }

    /// Record a new asset, replacing what it supersedes: for an artwork the
    /// slide's entry in the same style (by number or hash), for a
    /// placeholder the entries for the same prompt (and, for an empty
    /// prompt, the same slide). Pinned entries are never replaced.
    pub fn upsert(&mut self, asset: Asset) {
        self.assets.retain(|a| {
            if a.kind != asset.kind || a.pinned() {
                return true;
            }
            let same = match asset.kind {
                Kind::Artwork => {
                    a.style == asset.style
                        && (a.slide == asset.slide || (a.hash.is_some() && a.hash == asset.hash))
                }
                _ => {
                    let empty = asset.placeholder.as_deref().is_none_or(str::is_empty);
                    a.placeholder == asset.placeholder && (!empty || a.slide == asset.slide)
                }
            };
            !same
        });
        self.assets.push(asset);
    }

    /// Write back the state a lookup found (pinned entries stay pinned).
    pub fn mark(&mut self, found: Found) {
        if let Some(a) = self.assets.get_mut(found.index)
            && !a.pinned()
        {
            a.state = found.state;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    fn artwork(slide: usize, hash: &str, style: &str, file: &str) -> Asset {
        Asset {
            slide: Some(slide),
            hash: Some(hash.into()),
            ..Asset::new(Kind::Artwork, file.into(), style)
        }
    }

    fn image(prompt: &str, style: &str, file: &str) -> Asset {
        Asset {
            placeholder: Some(prompt.into()),
            slide: Some(1),
            ..Asset::new(Kind::Image, file.into(), style)
        }
    }

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("mdeck-manifest-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn the_folder_and_manifest_sit_next_to_the_deck() {
        let deck = Path::new("/talks/launch.md");
        assert_eq!(folder_for(deck), Path::new("/talks/launch.assets"));
        assert_eq!(
            path_for(deck),
            Path::new("/talks/launch.assets/manifest.yaml")
        );
        let a = Asset::new(Kind::Image, "images/rocket.png".into(), "s");
        assert_eq!(deck_relative(deck, &a), "launch.assets/images/rocket.png");
        assert_eq!(Kind::PointCloud.folder(), "point-clouds");
    }

    #[test]
    fn artworks_are_pinned_then_current_then_stale_and_never_cross_styles() {
        let mut m = Manifest::new();
        m.assets.push(artwork(3, "old", "line-1", "a.jpg"));
        m.assets.push(artwork(5, "now", "line-1", "b.jpg"));
        let mut pin = artwork(3, "hand", "line-1", "mine.png");
        pin.state = State::Pinned;
        m.assets.push(pin);
        let f = m.artwork(3, "now", "line-1").unwrap();
        assert_eq!(
            (m.assets[f.index].file.as_str(), f.state),
            ("mine.png", State::Pinned)
        );
        let f = m.artwork(5, "now", "line-1").unwrap();
        assert_eq!(
            (m.assets[f.index].file.as_str(), f.state),
            ("b.jpg", State::Current)
        );
        let f = m.artwork(4, "gone", "line-1");
        assert!(f.is_none());
        m.assets.remove(2);
        let f = m.artwork(3, "gone", "line-1").unwrap();
        assert_eq!(
            (m.assets[f.index].file.as_str(), f.state),
            ("a.jpg", State::Stale)
        );
        assert!(
            m.artwork(3, "old", "sketch-1").is_none(),
            "styles never mix"
        );
    }

    #[test]
    fn placeholders_match_by_prompt_and_go_stale_with_the_style() {
        let mut m = Manifest::new();
        m.assets
            .push(image("a rocket at dawn", "photo-1", "images/r.png"));
        let f = m
            .placeholder(Kind::Image, " a rocket at dawn ", 2, "h", "photo-1")
            .unwrap();
        assert_eq!(f.state, State::Current);
        let f = m
            .placeholder(Kind::Image, "a rocket at dawn", 2, "h", "flat-2")
            .unwrap();
        assert_eq!(f.state, State::Stale);
        assert!(
            m.placeholder(Kind::Icon, "a rocket at dawn", 2, "h", "photo-1")
                .is_none(),
            "kinds never mix"
        );
        assert!(
            m.placeholder(Kind::Image, "a cat", 2, "h", "photo-1")
                .is_none()
        );
        m.assets[0].state = State::Pinned;
        let f = m
            .placeholder(Kind::Image, "a rocket at dawn", 2, "h", "flat-2")
            .unwrap();
        assert_eq!(f.state, State::Pinned);
    }

    #[test]
    fn an_empty_prompt_is_matched_by_its_slide() {
        let mut m = Manifest::new();
        let mut a = image("", "photo-1", "images/auto.png");
        a.slide = Some(4);
        a.hash = Some("h4".into());
        m.assets.push(a);
        assert_eq!(
            m.placeholder(Kind::Image, "", 4, "h4", "photo-1")
                .unwrap()
                .state,
            State::Current
        );
        assert_eq!(
            m.placeholder(Kind::Image, "", 4, "changed", "photo-1")
                .unwrap()
                .state,
            State::Stale
        );
        assert_eq!(
            m.placeholder(Kind::Image, "", 4, "h4", "other")
                .unwrap()
                .state,
            State::Stale
        );
        assert!(m.placeholder(Kind::Image, "", 2, "h2", "photo-1").is_none());
    }

    #[test]
    fn upsert_replaces_what_it_supersedes_but_never_a_pin() {
        let mut m = Manifest::new();
        m.upsert(artwork(2, "h", "line-1", "a.jpg"));
        m.upsert(artwork(2, "h2", "line-1", "a2.jpg"));
        m.upsert(artwork(2, "h2", "sketch-1", "b.jpg"));
        assert_eq!(m.assets.len(), 2);
        m.upsert(image("cat", "s1", "images/cat.png"));
        m.upsert(image("cat", "s2", "images/cat-2.png"));
        assert_eq!(m.assets.iter().filter(|a| a.kind == Kind::Image).count(), 1);
        m.assets.last_mut().unwrap().state = State::Pinned;
        m.upsert(image("cat", "s3", "images/cat-3.png"));
        assert_eq!(
            m.assets.iter().filter(|a| a.kind == Kind::Image).count(),
            2,
            "the pin stays"
        );
    }

    #[test]
    fn mark_records_stale_but_keeps_pins() {
        let mut m = Manifest::new();
        m.assets.push(artwork(1, "h", "s", "a.jpg"));
        m.mark(Found {
            index: 0,
            state: State::Stale,
        });
        assert_eq!(m.assets[0].state, State::Stale);
        m.assets[0].state = State::Pinned;
        m.mark(Found {
            index: 0,
            state: State::Current,
        });
        assert_eq!(m.assets[0].state, State::Pinned);
    }

    #[test]
    fn save_then_load_round_trips_with_the_header() {
        let dir = tmp("io");
        let deck = dir.join("talk.md");
        assert_eq!(load(&deck).unwrap(), None);
        let mut m = Manifest::new();
        m.upsert(image("cat", "s1", "images/cat.png"));
        m.upsert(artwork(1, "h", "line-1", "artworks/a.jpg"));
        let path = save(&deck, &m).unwrap();
        assert_eq!(path, dir.join("talk.assets/manifest.yaml"));
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("# Generated assets"));
        assert!(text.contains("kind: artwork"), "{text}");
        assert!(text.contains("state: current"), "{text}");
        let back = load(&deck).unwrap().unwrap();
        assert_eq!(back.assets.len(), 2);
        assert_eq!(back.assets[0].kind, Kind::Artwork, "sorted by kind");
        std::fs::write(&path, "assets: [nope\n").unwrap();
        let err = load(&deck).unwrap_err().to_string();
        assert!(err.starts_with(&path.display().to_string()), "{err}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_slide_hash_follows_the_source_and_the_world() {
        let p = parser::parse("# One\n\n- a\n\n# Two\n\n- b\n");
        let a = slide_hash(&p.slides[0], None);
        assert_eq!(a, slide_hash(&p.slides[0], Some("  ")));
        assert_ne!(a, slide_hash(&p.slides[1], None));
        assert_ne!(a, slide_hash(&p.slides[0], Some("a harbour town")));
    }
}
