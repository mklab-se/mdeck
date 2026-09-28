//! The story sidecar: `deck.scenes.yaml` (or `.yml`) next to a deck, holding
//! one generated script per slide, keyed by a hash of the slide's source so a
//! script goes stale the moment its slide changes.

use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

use super::Script;
use crate::parser::{Presentation, Slide};
use crate::render::sidecar::{self as shared, Keyed, Match};

pub const VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    /// 1-based slide number when generated (informational; the hash is the key).
    pub slide: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Hash of the slide source plus the deck-level hint.
    pub hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated: Option<String>,
    /// A hand-written entry: matched by slide number, never stale, never
    /// regenerated. Set it when you edit a scene by hand.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub pinned: bool,
    pub scene: Script,
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

/// Candidate sidecar paths for a deck, `.yaml` first.
pub fn candidates(deck: &Path) -> [PathBuf; 2] {
    [
        shared::beside(deck, "scenes.yaml"),
        shared::beside(deck, "scenes.yml"),
    ]
}

/// The sidecar file to read or write for a deck: whichever exists (`.yaml`
/// wins if both do), else a new `.yaml`. Also reports whether both exist.
pub fn resolve_path(deck: &Path) -> (PathBuf, bool) {
    let [yaml, yml] = candidates(deck);
    let both = yaml.exists() && yml.exists();
    if yaml.exists() {
        (yaml, both)
    } else if yml.exists() {
        (yml, both)
    } else {
        (yaml, false)
    }
}

/// Load the sidecar for a deck, if present. Errors are returned so callers
/// can warn without failing the presentation.
pub fn load(deck: &Path) -> Result<Option<Sidecar>> {
    let (path, _) = resolve_path(deck);
    let Some(sidecar) = shared::read::<Sidecar>(&path)? else {
        return Ok(None);
    };
    for entry in &sidecar.slides {
        entry
            .scene
            .validate()
            .map_err(|e| anyhow!("{}: slide {}: {e}", path.display(), entry.slide))?;
    }
    Ok(Some(sidecar))
}

pub fn save(deck: &Path, sidecar: &Sidecar) -> Result<PathBuf> {
    let (path, _) = resolve_path(deck);
    let header = "# Story scenes for the particles engine (the Ember theme and others), written by `mdeck ai story`.\n# Hand edits are fine; a slide's entry goes stale when the slide changes.\n";
    shared::write(&path, header, sidecar)?;
    Ok(path)
}

/// Stable hash of everything a story depends on: the slide source (copy,
/// hint and notes) and the deck-level hint.
pub fn slide_hash(slide: &Slide, deck_hint: Option<&str>) -> String {
    shared::slide_hash(slide, deck_hint)
}

/// Where a slide's story came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    /// A pinned (hand-written) sidecar entry for this slide number.
    Pinned,
    /// A matching sidecar entry.
    Sidecar,
    /// A sidecar entry for this slide number whose hash no longer matches.
    Stale,
}

/// A slide's resolved story.
#[derive(Clone, Debug)]
pub struct Resolved {
    pub script: Script,
    pub source: Source,
}

/// Resolve the story for every slide: pinned entries win, then sidecar
/// entries by hash, then stale entries by slide number (still played, but
/// flagged). Slides without a stage (see [`super::allowed`]) never get a
/// story, and a leftover inline `@scene` fence is reported.
pub fn resolve(
    presentation: &Presentation,
    sidecar: Option<&Sidecar>,
) -> (Vec<Option<Resolved>>, Vec<String>) {
    let mut problems = Vec::new();
    let deck_hint = presentation.meta.story.as_deref();
    let resolved = presentation
        .slides
        .iter()
        .enumerate()
        .map(|(i, slide)| {
            if slide.scene_script.is_some() {
                problems.push(format!(
                    "slide {}: inline @scene fences are no longer used; scripts live in the \
                     sidecar (add the entry there with `pinned: true` to hand-write it)",
                    i + 1
                ));
            }
            if !super::allowed(slide, i) {
                return None;
            }
            let sidecar = sidecar?;
            let hash = slide_hash(slide, deck_hint);
            let (entry, found) = shared::find(&sidecar.slides, i + 1, &hash)?;
            Some(Resolved {
                script: entry.scene.clone(),
                source: match found {
                    Match::Pinned => Source::Pinned,
                    Match::Current => Source::Sidecar,
                    Match::Stale => Source::Stale,
                },
            })
        })
        .collect();
    (resolved, problems)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    #[test]
    fn candidates_and_resolution_prefer_yaml() {
        let dir = std::env::temp_dir().join(format!("mdeck-sidecar-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let deck = dir.join("talk.md");
        let [yaml, yml] = candidates(&deck);
        assert!(yaml.ends_with("talk.scenes.yaml"));
        assert!(yml.ends_with("talk.scenes.yml"));
        assert_eq!(resolve_path(&deck).0, yaml);
        std::fs::write(&yml, "version: 1\nslides: []\n").unwrap();
        assert_eq!(resolve_path(&deck).0, yml);
        std::fs::write(&yaml, "version: 1\nslides: []\n").unwrap();
        let (p, both) = resolve_path(&deck);
        assert_eq!(p, yaml);
        assert!(both);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn slides_without_a_stage_never_get_a_story() {
        // A code slide and a chart slide: a sidecar entry for them is ignored.
        let md = "# Code

```rust
fn main() {}
```

---

# Chart

```@barchart
- A: 1
```

---

# Words

- a bullet
";
        let pres = parser::parse(md);
        assert_eq!(pres.slides.len(), 3);
        let script = Script::parse("cast:\n  - { id: b, kind: box, cell: right }\n").unwrap();
        let entry = |n: usize| Entry {
            slide: n,
            title: None,
            hash: slide_hash(&pres.slides[n - 1], None),
            generated: None,
            pinned: false,
            scene: script.clone(),
        };
        let sidecar = Sidecar {
            version: VERSION,
            slides: vec![entry(1), entry(2), entry(3)],
        };
        let (resolved, problems) = resolve(&pres, Some(&sidecar));
        assert!(problems.is_empty());
        assert!(resolved[0].is_none(), "code slide must not play a story");
        assert!(resolved[1].is_none(), "chart slide must not play a story");
        assert!(resolved[2].is_some(), "bullet slide keeps its story");
    }

    #[test]
    fn pinned_beats_sidecar_and_hash_mismatch_is_stale() {
        let md = "# A\n\n- one\n\n```@scene\ncast: []\n```\n\n---\n\n# B\n\n- two\n";
        let pres = parser::parse(md);
        assert_eq!(pres.slides.len(), 2);
        let script = Script::parse("cast:\n  - { id: b, kind: box, cell: right }\n").unwrap();
        let sidecar = Sidecar {
            version: VERSION,
            slides: vec![
                Entry {
                    slide: 1,
                    title: None,
                    hash: "hand".into(),
                    generated: None,
                    pinned: true,
                    scene: script.clone(),
                },
                Entry {
                    slide: 2,
                    title: None,
                    hash: "nope".into(),
                    generated: None,
                    pinned: false,
                    scene: script,
                },
            ],
        };
        let (resolved, problems) = resolve(&pres, Some(&sidecar));
        // the leftover inline fence is reported, not used
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("no longer used"));
        assert_eq!(resolved[0].as_ref().unwrap().source, Source::Pinned);
        assert_eq!(resolved[1].as_ref().unwrap().source, Source::Stale);
        let good_hash = slide_hash(&pres.slides[1], None);
        let sidecar2 = Sidecar {
            slides: vec![Entry {
                hash: good_hash,
                ..sidecar.slides[1].clone()
            }],
            ..sidecar
        };
        let (resolved, _) = resolve(&pres, Some(&sidecar2));
        assert_eq!(resolved[1].as_ref().unwrap().source, Source::Sidecar);
    }
}
