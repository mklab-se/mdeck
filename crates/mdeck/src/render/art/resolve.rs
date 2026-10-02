//! Which artwork each slide shows: the deck's manifest
//! (`talk.assets/manifest.yaml`, see [`crate::assets::manifest`]) read for
//! the style the engine's medium asks for. Each artwork records which slide
//! it belongs to, in which style it was made and from which slide source (a
//! hash), so it goes stale when its slide changes and is never shown in a
//! style it was not made for.

use std::path::{Path, PathBuf};

use crate::assets::manifest::{self, Found, Manifest, State};
use crate::parser::Presentation;

/// A slide's artwork, resolved.
#[derive(Clone, Debug, PartialEq)]
pub struct Resolved {
    pub file: PathBuf,
    pub state: State,
    /// Where it is in the manifest.
    pub found: Found,
}

/// Stable hash of what a slide's artwork depends on: the slide source and
/// the deck's artwork world.
pub fn slide_hash(slide: &crate::parser::Slide, world: Option<&str>) -> String {
    manifest::slide_hash(slide, world)
}

/// The deck's artwork world: its `art-world` setting.
pub fn world(presentation: &Presentation) -> Option<&str> {
    presentation.meta.art_world.as_deref()
}

/// What every slide shows in `style`: pinned entries first, then an entry
/// for the slide as it reads now, then a stale one for its number. Slides
/// that take no art get `None`.
pub fn resolve(
    deck: &Path,
    presentation: &Presentation,
    manifest: Option<&Manifest>,
    style: &str,
) -> Vec<Option<Resolved>> {
    let world = world(presentation);
    presentation
        .slides
        .iter()
        .enumerate()
        .map(|(i, slide)| {
            if !super::wants_art(slide) {
                return None;
            }
            let m = manifest?;
            let found = m.artwork(i + 1, &slide_hash(slide, world), style)?;
            Some(Resolved {
                file: Manifest::file(deck, &m.assets[found.index]),
                state: found.state,
                found,
            })
        })
        .collect()
}

/// Counts for the startup line and `--check`: slides that take art but have
/// none in this style, and slides whose artwork is stale.
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
            Some(r) if r.state == State::Stale => c.stale += 1,
            _ => {}
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::manifest::{Asset, Kind};
    use crate::parser;

    fn deck() -> Presentation {
        parser::parse(
            "# Launch\n\nA talk\n\n# Why\n\n- one\n\n# Code\n\n```rust\nfn main() {}\n```\n\n# How\n\n- two\n",
        )
    }

    fn artwork(pres: &Presentation, i: usize, style: &str, file: &str) -> Asset {
        Asset {
            slide: Some(i + 1),
            hash: Some(slide_hash(&pres.slides[i], world(pres))),
            ..Asset::new(Kind::Artwork, file.into(), style)
        }
    }

    #[test]
    fn current_beats_stale_and_styles_never_mix() {
        let pres = deck();
        let mut m = Manifest::new();
        m.upsert(artwork(&pres, 1, "line-1", "artworks/a.jpg"));
        m.upsert(artwork(&pres, 1, "sketch-1", "artworks/b.jpg"));
        // an old picture for slide 4, made before the slide changed
        m.upsert(Asset {
            slide: Some(4),
            hash: Some("old".into()),
            ..Asset::new(Kind::Artwork, "artworks/c.jpg".into(), "line-1")
        });
        let deck_path = Path::new("/talks/launch.md");
        let r = resolve(deck_path, &pres, Some(&m), "line-1");
        assert_eq!(r.len(), 4);
        assert!(r[0].is_none(), "no picture made for the title yet");
        assert_eq!(
            r[1].as_ref().unwrap().file,
            Path::new("/talks/launch.assets/artworks/a.jpg")
        );
        assert_eq!(r[1].as_ref().unwrap().state, State::Current);
        assert!(r[2].is_none(), "a code slide takes no art");
        assert_eq!(r[3].as_ref().unwrap().state, State::Stale);
        let s = resolve(deck_path, &pres, Some(&m), "sketch-1");
        assert_eq!(
            s[1].as_ref().unwrap().file,
            Path::new("/talks/launch.assets/artworks/b.jpg")
        );
        assert!(s[3].is_none());
        // the files do not exist, so coverage counts them missing
        let c = coverage(&pres, &r);
        assert_eq!(c.wanted, 3);
        assert_eq!(c.missing, 3);
    }

    #[test]
    fn pinned_holds_its_slide_number() {
        let pres = deck();
        let mut m = Manifest::new();
        m.assets.push(Asset {
            slide: Some(2),
            hash: Some("hand".into()),
            state: State::Pinned,
            ..Asset::new(Kind::Artwork, "artworks/mine.png".into(), "line-1")
        });
        let r = resolve(Path::new("talk.md"), &pres, Some(&m), "line-1");
        assert_eq!(r[1].as_ref().unwrap().state, State::Pinned);
    }
}
