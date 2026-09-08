//! The story sidecar: `deck.scenes.yaml` (or `.yml`) next to a deck, holding
//! one generated script per slide, keyed by a hash of the slide's source so a
//! script goes stale the moment its slide changes.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::Script;
use crate::parser::{Presentation, Slide};

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
    pub scene: Script,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Sidecar {
    pub version: u32,
    #[serde(default)]
    pub slides: Vec<Entry>,
}

/// Candidate sidecar paths for a deck, `.yaml` first.
pub fn candidates(deck: &Path) -> [PathBuf; 2] {
    let stem = deck
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "deck".to_string());
    let dir = deck.parent().unwrap_or(Path::new("."));
    [
        dir.join(format!("{stem}.scenes.yaml")),
        dir.join(format!("{stem}.scenes.yml")),
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
pub fn load(deck: &Path) -> Result<Option<Sidecar>, String> {
    let (path, _) = resolve_path(deck);
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let sidecar: Sidecar =
        serde_yaml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    for entry in &sidecar.slides {
        entry
            .scene
            .validate()
            .map_err(|e| format!("{}: slide {}: {e}", path.display(), entry.slide))?;
    }
    Ok(Some(sidecar))
}

pub fn save(deck: &Path, sidecar: &Sidecar) -> Result<PathBuf, String> {
    let (path, _) = resolve_path(deck);
    let text = serde_yaml::to_string(sidecar).map_err(|e| e.to_string())?;
    let header = "# Story scenes for the Ember theme, written by `mdeck ai story`.\n# Hand edits are fine; a slide's entry goes stale when the slide changes.\n";
    std::fs::write(&path, format!("{header}{text}"))
        .map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

/// Stable hash of everything a story depends on: the slide source (copy,
/// hint and notes) and the deck-level hint.
pub fn slide_hash(slide: &Slide, deck_hint: Option<&str>) -> String {
    let mut h = DefaultHasher::new();
    slide.raw_source.trim().hash(&mut h);
    deck_hint.unwrap_or("").trim().hash(&mut h);
    format!("{:016x}", h.finish())
}

/// Where a slide's story came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    /// A ```@scene fence in the slide itself.
    Inline,
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

/// Resolve the story for every slide: inline scripts win, then sidecar
/// entries by hash, then stale entries by slide number (still played, but
/// flagged). Invalid inline scripts are reported and skipped.
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
            if let Some(text) = &slide.scene_script {
                match Script::parse(text) {
                    Ok(script) => {
                        return Some(Resolved {
                            script,
                            source: Source::Inline,
                        });
                    }
                    Err(e) => problems.push(format!("slide {}: @scene: {e}", i + 1)),
                }
            }
            let sidecar = sidecar?;
            let hash = slide_hash(slide, deck_hint);
            if let Some(entry) = sidecar.slides.iter().find(|e| e.hash == hash) {
                return Some(Resolved {
                    script: entry.scene.clone(),
                    source: Source::Sidecar,
                });
            }
            sidecar
                .slides
                .iter()
                .find(|e| e.slide == i + 1)
                .map(|entry| Resolved {
                    script: entry.scene.clone(),
                    source: Source::Stale,
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
    fn inline_beats_sidecar_and_hash_mismatch_is_stale() {
        let md = "# A\n\n- one\n\n```@scene\ncast:\n  - { id: a, kind: person, cell: left }\nbeats: []\n```\n\n---\n\n# B\n\n- two\n";
        let pres = parser::parse(md, Path::new("."));
        assert_eq!(pres.slides.len(), 2);
        assert!(pres.slides[0].scene_script.is_some());
        let script = Script::parse("cast:\n  - { id: b, kind: box, cell: right }\n").unwrap();
        let sidecar = Sidecar {
            version: VERSION,
            slides: vec![Entry {
                slide: 2,
                title: None,
                hash: "nope".into(),
                generated: None,
                scene: script,
            }],
        };
        let (resolved, problems) = resolve(&pres, Some(&sidecar));
        assert!(problems.is_empty());
        assert_eq!(resolved[0].as_ref().unwrap().source, Source::Inline);
        assert_eq!(resolved[1].as_ref().unwrap().source, Source::Stale);
        let good_hash = slide_hash(&pres.slides[1], None);
        let sidecar2 = Sidecar {
            slides: vec![Entry {
                hash: good_hash,
                ..sidecar.slides[0].clone()
            }],
            ..sidecar
        };
        let (resolved, _) = resolve(&pres, Some(&sidecar2));
        assert_eq!(resolved[1].as_ref().unwrap().source, Source::Sidecar);
    }
}
