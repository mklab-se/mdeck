//! Point cloud illustrations: named sets of importance-ordered points that
//! the particle field draws as a shape.
//!
//! A cloud is what an AI-generated image was reduced to by [`convert`]: its
//! bright pixels, ordered so that the first few dozen points already sketch
//! the whole subject and the first few hundred fill it in. Consumers take the
//! first *n*, so one file serves a small cast member and a full-frame
//! illustration alike. Clouds live in `.mdpc` files (JSON) and resolve by
//! name through three places, first match wins: the deck's `illustrations/`
//! folder, the user library under the config directory, and the set built
//! into the binary.

pub mod convert;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use serde::{Deserialize, Serialize};

pub const VERSION: u32 = 1;
pub const EXTENSION: &str = "mdpc";
/// Most points a file may carry (about 25 KB).
pub const MAX_POINTS: usize = 1500;
pub const MAX_NAME_LEN: usize = 40;

/// Points in the unit square, importance first.
pub type Points = Arc<Vec<[f32; 2]>>;

/// One illustration.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Cloud {
    pub version: u32,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// The full prompt sent to the image model; `None` for imports.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated: Option<String>,
    /// Height over width of the bounding box.
    pub aspect: f32,
    /// x and y in 0..1, importance order.
    #[serde(with = "arc_points")]
    pub points: Points,
}

mod arc_points {
    use super::Points;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(p: &Points, s: S) -> Result<S::Ok, S::Error> {
        p.as_slice().serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Points, D::Error> {
        Ok(std::sync::Arc::new(Vec::<[f32; 2]>::deserialize(d)?))
    }
}

impl Cloud {
    /// Parse a `.mdpc` document and check it.
    pub fn parse(text: &str) -> Result<Cloud, String> {
        let cloud: Cloud = serde_json::from_str(text).map_err(|e| e.to_string())?;
        cloud.validate()?;
        Ok(cloud)
    }

    /// Serialise to the `.mdpc` document (points rounded to three decimals).
    pub fn to_json(&self) -> String {
        let mut rounded = self.clone();
        rounded.points = Arc::new(
            self.points
                .iter()
                .map(|p| [round3(p[0]), round3(p[1])])
                .collect(),
        );
        rounded.aspect = round3(self.aspect);
        // Written by hand: keys in reading order, one point per line, so
        // diffs stay readable and nothing prints float noise.
        let q = |s: &str| serde_json::to_string(s).unwrap_or_default();
        let opt = |s: &Option<String>| s.as_deref().map(q).unwrap_or_else(|| "null".into());
        let mut out = String::new();
        out.push_str("{\n");
        out.push_str(&format!("  \"version\": {},\n", rounded.version));
        out.push_str(&format!("  \"name\": {},\n", q(&rounded.name)));
        out.push_str(&format!(
            "  \"description\": {},\n",
            q(&rounded.description)
        ));
        out.push_str(&format!("  \"prompt\": {},\n", opt(&rounded.prompt)));
        out.push_str(&format!("  \"generated\": {},\n", opt(&rounded.generated)));
        out.push_str(&format!("  \"aspect\": {},\n", rounded.aspect));
        out.push_str("  \"points\": [\n");
        for (i, p) in rounded.points.iter().enumerate() {
            let sep = if i + 1 == rounded.points.len() {
                ""
            } else {
                ","
            };
            out.push_str(&format!("    [{}, {}]{sep}\n", p[0], p[1]));
        }
        out.push_str("  ]\n}\n");
        out
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.version != VERSION {
            return Err(format!(
                "unsupported version {} (this build reads {VERSION})",
                self.version
            ));
        }
        validate_name(&self.name)?;
        if self.points.is_empty() {
            return Err("no points".into());
        }
        if self.points.len() > MAX_POINTS {
            return Err(format!(
                "{} points, at most {MAX_POINTS} allowed",
                self.points.len()
            ));
        }
        if !(self.aspect.is_finite() && self.aspect > 0.0) {
            return Err(format!("aspect must be positive, got {}", self.aspect));
        }
        for p in self.points.iter() {
            if !(-0.001..=1.001).contains(&p[0]) || !(-0.001..=1.001).contains(&p[1]) {
                return Err(format!(
                    "point ({}, {}) outside the unit square",
                    p[0], p[1]
                ));
            }
        }
        Ok(())
    }
}

fn round3(x: f32) -> f32 {
    (x * 1000.0).round() / 1000.0
}

/// A cloud name: lowercase letters, digits and hyphens.
pub fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("name is empty".into());
    }
    if name.len() > MAX_NAME_LEN {
        return Err(format!(
            "name `{name}` is longer than {MAX_NAME_LEN} characters"
        ));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(format!(
            "name `{name}` may only contain lowercase letters, digits and hyphens"
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Lookup
// ---------------------------------------------------------------------------

/// Where a resolved cloud came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    /// `<deck dir>/illustrations/<name>.mdpc`.
    Deck(PathBuf),
    /// `~/.config/mdeck/illustrations/<name>.mdpc`.
    User(PathBuf),
    /// Built into the binary.
    Builtin,
}

impl Source {
    pub fn label(&self) -> &'static str {
        match self {
            Source::Deck(_) => "deck",
            Source::User(_) => "user",
            Source::Builtin => "built-in",
        }
    }
}

/// The deck-local library folder for a deck (or the working directory).
pub fn deck_dir(base: &Path) -> PathBuf {
    base.join("illustrations")
}

/// The user library folder.
pub fn user_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("mdeck").join("illustrations"))
}

include!(concat!(env!("OUT_DIR"), "/builtin_illustrations.rs"));

type ParsedBuiltins = Mutex<Vec<(&'static str, Arc<Cloud>)>>;

fn builtin(name: &str) -> Option<Arc<Cloud>> {
    static PARSED: OnceLock<ParsedBuiltins> = OnceLock::new();
    let (_, text) = BUILTIN.iter().find(|(n, _)| *n == name)?;
    let cache = PARSED.get_or_init(|| Mutex::new(Vec::new()));
    let mut guard = cache.lock().unwrap_or_else(|p| p.into_inner());
    if let Some((_, c)) = guard.iter().find(|(n, _)| *n == name) {
        return Some(Arc::clone(c));
    }
    let cloud = Arc::new(
        Cloud::parse(text)
            .unwrap_or_else(|e| panic!("built-in illustration `{name}` is invalid: {e}")),
    );
    if let Some(slot) = BUILTIN.iter().find(|(n, _)| *n == name) {
        guard.push((slot.0, Arc::clone(&cloud)));
    }
    Some(cloud)
}

/// Names of the built-in clouds, in library order.
pub fn builtin_names() -> Vec<&'static str> {
    BUILTIN.iter().map(|(n, _)| *n).collect()
}

/// Load a cloud from a file.
pub fn load_file(path: &Path) -> Result<Cloud, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Cloud::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Resolve `name` through explicit library folders (deck first, then user),
/// then the built-in set. Errors from unreadable files are reported so the
/// caller can warn; a missing file just falls through.
pub fn resolve_in(
    name: &str,
    deck: Option<&Path>,
    user: Option<&Path>,
) -> Result<Option<(Source, Arc<Cloud>)>, String> {
    validate_name(name)?;
    let file = format!("{name}.{EXTENSION}");
    if let Some(dir) = deck {
        let path = dir.join(&file);
        if path.is_file() {
            return Ok(Some((
                Source::Deck(path.clone()),
                Arc::new(load_file(&path)?),
            )));
        }
    }
    if let Some(dir) = user {
        let path = dir.join(&file);
        if path.is_file() {
            return Ok(Some((
                Source::User(path.clone()),
                Arc::new(load_file(&path)?),
            )));
        }
    }
    Ok(builtin(name).map(|c| (Source::Builtin, c)))
}

/// Resolve `name` for a deck at `deck_base` (its directory), using the real
/// user library.
pub fn resolve(
    name: &str,
    deck_base: Option<&Path>,
) -> Result<Option<(Source, Arc<Cloud>)>, String> {
    let deck = deck_base.map(deck_dir);
    let user = user_dir();
    resolve_in(name, deck.as_deref(), user.as_deref())
}

/// Every name visible from a deck, with the source that wins and the sources
/// it shadows, sorted by name.
pub fn catalogue_in(
    deck: Option<&Path>,
    user: Option<&Path>,
) -> Vec<(String, Source, Vec<Source>)> {
    let mut found: Vec<(String, Vec<Source>)> = Vec::new();
    let mut add = |name: String, src: Source| match found.iter_mut().find(|(n, _)| *n == name) {
        Some((_, v)) => v.push(src),
        None => found.push((name, vec![src])),
    };
    for (dir, mk) in [
        (deck, Source::Deck as fn(PathBuf) -> Source),
        (user, Source::User as fn(PathBuf) -> Source),
    ] {
        let Some(dir) = dir else { continue };
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        let mut names: Vec<(String, PathBuf)> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == EXTENSION))
            .filter_map(|p| {
                let stem = p.file_stem()?.to_str()?.to_string();
                validate_name(&stem).ok()?;
                Some((stem, p))
            })
            .collect();
        names.sort();
        for (n, p) in names {
            add(n, mk(p));
        }
    }
    for n in builtin_names() {
        add(n.to_string(), Source::Builtin);
    }
    let mut out: Vec<(String, Source, Vec<Source>)> = found
        .into_iter()
        .map(|(n, mut srcs)| {
            let first = srcs.remove(0);
            (n, first, srcs)
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// [`catalogue_in`] for a deck directory and the real user library.
pub fn catalogue(deck_base: Option<&Path>) -> Vec<(String, Source, Vec<Source>)> {
    let deck = deck_base.map(deck_dir);
    let user = user_dir();
    catalogue_in(deck.as_deref(), user.as_deref())
}

// ---------------------------------------------------------------------------
// Session cache
// ---------------------------------------------------------------------------

/// Clouds resolved for one deck, cached by name (misses included) until
/// [`Library::reset`].
#[derive(Debug, Default)]
pub struct Library {
    deck: Option<PathBuf>,
    user: Option<PathBuf>,
    cache: std::collections::HashMap<String, Option<Arc<Cloud>>>,
    /// Problems met while resolving (unreadable files), once each.
    problems: Vec<String>,
}

impl Library {
    /// A library for a deck whose directory is `deck_base`, with the real
    /// user library.
    pub fn for_deck(deck_base: Option<&Path>) -> Self {
        Self {
            deck: deck_base.map(deck_dir),
            user: user_dir(),
            cache: Default::default(),
            problems: Vec::new(),
        }
    }

    /// A library over explicit folders (tests).
    #[cfg(test)]
    pub fn with_dirs(deck: Option<PathBuf>, user: Option<PathBuf>) -> Self {
        Self {
            deck,
            user,
            cache: Default::default(),
            problems: Vec::new(),
        }
    }

    /// Pre-seed a cloud under `name` (tests).
    #[cfg(test)]
    pub fn insert(&mut self, cloud: Cloud) {
        self.cache.insert(cloud.name.clone(), Some(Arc::new(cloud)));
    }

    /// The cloud named `name`, if it resolves.
    pub fn get(&mut self, name: &str) -> Option<Arc<Cloud>> {
        if let Some(hit) = self.cache.get(name) {
            return hit.clone();
        }
        let found = match resolve_in(name, self.deck.as_deref(), self.user.as_deref()) {
            Ok(found) => found.map(|(_, c)| c),
            Err(e) => {
                if !self.problems.contains(&e) {
                    self.problems.push(e);
                }
                None
            }
        };
        self.cache.insert(name.to_string(), found.clone());
        found
    }

    /// Whether `name` resolves (cached like [`Library::get`]).
    pub fn has(&mut self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// Every name visible to this library, sorted.
    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = catalogue_in(self.deck.as_deref(), self.user.as_deref())
            .into_iter()
            .map(|(n, _, _)| n)
            .collect();
        for n in self
            .cache
            .iter()
            .filter(|(_, v)| v.is_some())
            .map(|(k, _)| k)
        {
            if !names.contains(n) {
                names.push(n.clone());
            }
        }
        names.sort();
        names
    }

    /// Problems met so far (unreadable files), drained.
    pub fn take_problems(&mut self) -> Vec<String> {
        std::mem::take(&mut self.problems)
    }

    /// Forget everything cached (the deck reloaded).
    pub fn reset(&mut self) {
        self.cache.clear();
        self.problems.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(name: &str) -> Cloud {
        Cloud {
            version: VERSION,
            name: name.into(),
            description: "a thing".into(),
            prompt: None,
            generated: None,
            aspect: 1.25,
            points: Arc::new(vec![[0.0, 0.0], [0.5, 0.123456], [1.0, 1.0]]),
        }
    }

    #[test]
    fn round_trips_through_json_with_three_decimals() {
        let c = sample("server");
        let text = c.to_json();
        assert!(text.contains("[0.5, 0.123]"), "{text}");
        let back = Cloud::parse(&text).unwrap();
        assert_eq!(back.name, "server");
        assert_eq!(back.points.len(), 3);
        assert_eq!(back.points[1], [0.5, 0.123]);
        assert_eq!(back.aspect, 1.25);
        assert!(back.prompt.is_none());
    }

    #[test]
    fn validation_rejects_bad_documents() {
        let mut c = sample("Server");
        assert!(c.validate().unwrap_err().contains("lowercase"));
        c.name = "ok".into();
        c.points = Arc::new(vec![]);
        assert!(c.validate().unwrap_err().contains("no points"));
        c.points = Arc::new(vec![[1.5, 0.0]]);
        assert!(c.validate().unwrap_err().contains("outside"));
        c.points = Arc::new(vec![[0.5, 0.5]]);
        c.aspect = 0.0;
        assert!(c.validate().unwrap_err().contains("aspect"));
        c.aspect = 1.0;
        c.version = 9;
        assert!(c.validate().unwrap_err().contains("version"));
        assert!(Cloud::parse("not json").is_err());
    }

    #[test]
    fn names_are_lowercase_ascii_with_hyphens() {
        assert!(validate_name("server-rack-2").is_ok());
        assert!(validate_name("").is_err());
        assert!(validate_name("Server").is_err());
        assert!(validate_name("a b").is_err());
        assert!(validate_name(&"x".repeat(41)).is_err());
    }

    #[test]
    fn deck_shadows_user_and_missing_files_fall_through() {
        let tmp = std::env::temp_dir().join(format!("mdpc-lookup-{}", std::process::id()));
        let deck = tmp.join("deck");
        let user = tmp.join("user");
        std::fs::create_dir_all(&deck).unwrap();
        std::fs::create_dir_all(&user).unwrap();
        let mut d = sample("kettle");
        d.description = "deck".into();
        let mut u = sample("kettle");
        u.description = "user".into();
        std::fs::write(deck.join("kettle.mdpc"), d.to_json()).unwrap();
        std::fs::write(user.join("kettle.mdpc"), u.to_json()).unwrap();
        std::fs::write(user.join("teapot.mdpc"), sample("teapot").to_json()).unwrap();
        std::fs::write(user.join("BAD.mdpc"), "{}").unwrap();

        let (src, c) = resolve_in("kettle", Some(&deck), Some(&user))
            .unwrap()
            .unwrap();
        assert!(matches!(src, Source::Deck(_)));
        assert_eq!(c.description, "deck");
        let (src, _) = resolve_in("teapot", Some(&deck), Some(&user))
            .unwrap()
            .unwrap();
        assert!(matches!(src, Source::User(_)));
        assert!(
            resolve_in("nothing", Some(&deck), Some(&user))
                .unwrap()
                .is_none()
        );
        assert!(resolve_in("Bad Name", None, None).is_err());

        let cat = catalogue_in(Some(&deck), Some(&user));
        let names: Vec<&str> = cat.iter().map(|(n, _, _)| n.as_str()).collect();
        assert!(names.contains(&"kettle") && names.contains(&"teapot"));
        assert!(!names.contains(&"BAD"));
        let server = cat.iter().find(|(n, _, _)| n == "kettle").unwrap();
        assert!(matches!(server.1, Source::Deck(_)));
        assert_eq!(server.2.len(), 1);

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn every_builtin_parses_and_is_registered_under_its_own_name() {
        for name in builtin_names() {
            let cloud = builtin(name).expect("registered");
            assert_eq!(cloud.name, name, "built-in `{name}` carries another name");
            assert!(
                cloud.points.len() >= 100,
                "built-in `{name}` has only {} points",
                cloud.points.len()
            );
            assert!(cloud.prompt.is_some(), "built-in `{name}` lost its prompt");
        }
        assert!(builtin_names().len() >= 20, "the built-in set shrank");
    }

    #[test]
    fn library_caches_hits_and_misses_and_resets() {
        let tmp = std::env::temp_dir().join(format!("mdpc-lib-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let mut lib = Library::with_dirs(Some(tmp.clone()), None);
        assert!(lib.get("kettle").is_none());
        std::fs::write(tmp.join("kettle.mdpc"), sample("kettle").to_json()).unwrap();
        // the miss is cached until reset
        assert!(lib.get("kettle").is_none());
        lib.reset();
        assert!(lib.get("kettle").is_some());
        assert!(lib.names().contains(&"kettle".to_string()));
        lib.insert(sample("teapot"));
        assert!(lib.has("teapot"));
        assert!(lib.names().contains(&"teapot".to_string()));
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn unreadable_files_are_reported_not_skipped() {
        let tmp = std::env::temp_dir().join(format!("mdpc-broken-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("server.mdpc"), "{ broken").unwrap();
        let err = resolve_in("server", Some(&tmp), None).unwrap_err();
        assert!(err.contains("server.mdpc"), "{err}");
        std::fs::remove_dir_all(&tmp).ok();
    }
}
