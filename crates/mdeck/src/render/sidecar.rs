//! What the deck sidecars share: a `<deck>.<kind>.yaml` file next to the
//! deck, read and written as YAML under a comment header, whose entries
//! belong to a slide by a hash of its source, or by its number when pinned.
//! The art sidecar (`render::art::sidecar`) keeps its own entry format and
//! header.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::parser::Slide;

/// `<stem>.<suffix>` next to `deck` (`talk.md` with `art.yaml` is
/// `talk.art.yaml`); a deck without a name counts as `deck`.
pub fn beside(deck: &Path, suffix: &str) -> PathBuf {
    let stem = deck
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "deck".to_string());
    deck.parent()
        .unwrap_or(Path::new("."))
        .join(format!("{stem}.{suffix}"))
}

/// Read and parse the YAML file at `path`; `None` when there is no file.
pub fn read<T: DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(path).map_err(|e| anyhow!("{}: {e}", path.display()))?;
    serde_norway::from_str(&text)
        .map(Some)
        .map_err(|e| anyhow!("{}: {e}", path.display()))
}

/// Write `value` to `path` as YAML under `header` (comment lines, each
/// ending in a newline).
pub fn write<T: Serialize>(path: &Path, header: &str, value: &T) -> Result<()> {
    let text = serde_norway::to_string(value).map_err(|e| anyhow!("{e}"))?;
    std::fs::write(path, format!("{header}{text}")).map_err(|e| anyhow!("{}: {e}", path.display()))
}

/// Stable hash of a slide's source and one deck-level text (a hint or a
/// world) that whatever was generated for it depends on.
pub fn slide_hash(slide: &Slide, deck_text: Option<&str>) -> String {
    let mut h = DefaultHasher::new();
    slide.raw_source.trim().hash(&mut h);
    deck_text.unwrap_or("").trim().hash(&mut h);
    format!("{:016x}", h.finish())
}

/// What a sidecar entry says about which slide it belongs to.
pub trait Keyed {
    /// 1-based slide number when the entry was made.
    fn slide(&self) -> usize;
    /// [`slide_hash`] of the slide when the entry was made.
    fn hash(&self) -> &str;
    /// Held by slide number whatever the slide says.
    fn pinned(&self) -> bool;
}

/// How an entry matched its slide.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Match {
    /// A pinned entry for this slide number.
    Pinned,
    /// An entry made from the slide as it reads now.
    Current,
    /// An entry for this slide number made before the slide changed.
    Stale,
}

/// The entry for slide `number` (1-based) whose source hashes to `hash`: a
/// pinned entry for the number first, then the first entry with the hash,
/// then the first entry for the number.
pub fn find<'a, E, I>(entries: I, number: usize, hash: &str) -> Option<(&'a E, Match)>
where
    E: Keyed + 'a,
    I: IntoIterator<Item = &'a E>,
    I::IntoIter: Clone,
{
    let entries = entries.into_iter();
    entries
        .clone()
        .find(|e| e.pinned() && e.slide() == number)
        .map(|e| (e, Match::Pinned))
        .or_else(|| {
            entries
                .clone()
                .find(|e| e.hash() == hash)
                .map(|e| (e, Match::Current))
        })
        .or_else(|| {
            entries
                .clone()
                .find(|e| e.slide() == number)
                .map(|e| (e, Match::Stale))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Item {
        slide: usize,
        hash: String,
        #[serde(default)]
        pinned: bool,
    }

    impl Keyed for Item {
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

    fn item(slide: usize, hash: &str, pinned: bool) -> Item {
        Item {
            slide,
            hash: hash.into(),
            pinned,
        }
    }

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("mdeck-sidecar-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn beside_names_the_file_after_the_deck() {
        assert_eq!(
            beside(Path::new("/talks/launch.md"), "art.yaml"),
            Path::new("/talks/launch.art.yaml")
        );
        assert_eq!(
            beside(Path::new("talk.md"), "scenes.yml"),
            Path::new("talk.scenes.yml")
        );
    }

    #[test]
    fn write_then_read_round_trips_under_the_header() {
        let dir = tmp("io");
        let path = dir.join("talk.test.yaml");
        assert_eq!(read::<Vec<Item>>(&path).unwrap(), None);
        let items = vec![item(1, "a", false), item(2, "b", true)];
        write(&path, "# made by a test\n", &items).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("# made by a test\n"));
        assert_eq!(read::<Vec<Item>>(&path).unwrap(), Some(items));
        std::fs::write(&path, "- [not, an, item\n").unwrap();
        let err = read::<Vec<Item>>(&path).unwrap_err().to_string();
        assert!(err.starts_with(&format!("{}: ", path.display())), "{err}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn write_names_the_file_it_could_not_write() {
        let path = tmp("nodir").join("missing").join("talk.test.yaml");
        let err = write(&path, "", &vec![item(1, "a", false)])
            .unwrap_err()
            .to_string();
        assert!(err.starts_with(&format!("{}: ", path.display())), "{err}");
    }

    #[test]
    fn pinned_then_current_then_stale() {
        let items = [
            item(3, "old", false),
            item(5, "now", false),
            item(3, "hand", true),
        ];
        let (e, m) = find(&items, 3, "now").unwrap();
        assert_eq!((e.hash.as_str(), m), ("hand", Match::Pinned));
        let (e, m) = find(&items, 5, "now").unwrap();
        assert_eq!((e.hash.as_str(), m), ("now", Match::Current));
        // a hash match wins over the number, even from another slide number
        let (e, m) = find(&items[..2], 3, "now").unwrap();
        assert_eq!((e.slide, m), (5, Match::Current));
        let (e, m) = find(&items[..2], 3, "gone").unwrap();
        assert_eq!((e.hash.as_str(), m), ("old", Match::Stale));
        assert!(find(&items, 4, "gone").is_none());
        // a pin holds only its own number
        assert!(find(&items[2..], 4, "gone").is_none());
    }
}
