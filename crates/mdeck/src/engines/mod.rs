//! Engines: what brings a slide to life around its content (ENG-01).
//!
//! The core decides *what* a slide wants to show (its picture and where it
//! goes, the countdown digit, the end words, the geometry the renderers
//! drew) and the [`host`] hands that over through the SDK
//! ([`mdeck_sdk::stage::Stage`], [`mdeck_sdk::stage::Frame`],
//! [`mdeck_sdk::paint::Painter`]). An engine decides *how* it looks: a
//! particle field, an LED wall, a departure board, falling blocks.
//!
//! The built-in engines are written exactly like an extension's (EXT-06,
//! ENG-13): each module implements [`mdeck_sdk::engine::Engine`], defines
//! its [`EngineDef`] as `DEF` and is registered by [`register`]. They use
//! only `mdeck_sdk` and engine helpers under `engines/` (a test checks). The
//! guide is `crates/mdeck/doc/engines.md` and the SDK's `docs/sdk/`.

#[cfg(feature = "blocks")]
pub mod blocks;
pub mod heat_palette;
pub mod host;
#[cfg(feature = "led")]
pub mod led;
#[cfg(feature = "particles")]
pub mod particles;
pub mod plain;
pub mod rng;
#[cfg(feature = "thermal")]
pub mod thermal;

pub use host::{
    CountPhase, Host, Shot, choose, settings_problems, unsupported, unsupported_summary,
    with_engine,
};
pub use mdeck_sdk::engine::{Capabilities, EngineDef, Medium};

use mdeck_sdk::design::DesignSet;
use mdeck_sdk::registry::{Registry, RegistryError};

/// Register the built-in engines (each one but plain is a cargo feature).
pub fn register(r: &mut Registry) -> Result<(), RegistryError> {
    r.engine(&plain::DEF)?;
    #[cfg(feature = "led")]
    r.engine(&led::DEF)?;
    #[cfg(feature = "blocks")]
    r.engine(&blocks::DEF)?;
    #[cfg(feature = "particles")]
    r.engine(&particles::DEF)?;
    #[cfg(feature = "thermal")]
    r.engine(&thermal::DEF)?;
    Ok(())
}

/// A registered engine: what a theme (or a deck's `engine`) runs. Looked
/// up by name in the registry, so extension engines are chosen exactly like
/// built-in ones (EXT-07).
#[derive(Clone, Copy)]
pub struct EngineId(&'static EngineDef);

impl PartialEq for EngineId {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}

impl Eq for EngineId {}

impl std::fmt::Debug for EngineId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EngineId({})", self.0.name)
    }
}

impl EngineId {
    /// The plain engine: slides on their own. What every unknown name
    /// falls back to.
    pub fn plain() -> Self {
        EngineId(&plain::DEF)
    }

    /// The engine registered as `name`.
    pub fn find(name: &str) -> Option<Self> {
        crate::registry::get().engine_def(name).map(EngineId)
    }

    /// The engine's name.
    pub fn name(self) -> &'static str {
        self.0.name
    }

    /// The engine's definition.
    pub fn def(self) -> &'static EngineDef {
        self.0
    }

    /// What the core must do differently for it.
    pub fn capabilities(self) -> Capabilities {
        self.0.capabilities
    }

    /// Paints a layer of its own under the slide (every engine but the
    /// core's own fallback, plain).
    pub fn paints(self) -> bool {
        self != Self::plain()
    }

    /// The medium an art engine draws generated pictures in.
    pub fn medium(self) -> Option<Medium> {
        self.0.capabilities.medium
    }

    /// A board engine's design set, which draws every slide (ENG-03).
    pub fn board(self) -> Option<&'static dyn DesignSet> {
        if self.0.capabilities.board {
            self.0.board
        } else {
            None
        }
    }

    /// Draws every slide itself (a board), text included.
    pub fn is_board(self) -> bool {
        self.board().is_some()
    }

    /// Frames an export waits on a slide before capturing it, so the engine
    /// has seen the geometry the slide's renderers publish.
    pub fn settle_frames(self) -> u32 {
        if self.paints() { 2 } else { 0 }
    }

    /// Seconds into the end slide when the "powered by" caption fades in.
    pub fn end_caption_delay(self) -> f32 {
        self.0.ending_caption_delay
    }
}

/// Every engine name in this build, for messages: "blocks, darkroom, ...".
pub fn names() -> String {
    crate::registry::get()
        .engines()
        .map(|d| d.name)
        .collect::<Vec<_>>()
        .join(", ")
}

/// A stable pseudo-random number in 0..1 for an index: engines seed their
/// per-element variation from it, never from time, so stills are
/// reproducible.
pub fn hash01(i: u32) -> f32 {
    let mut x = i.wrapping_mul(0x9E37_79B1) ^ 0x85EB_CA6B;
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297A_2D39);
    x ^= x >> 15;
    (x & 0x00FF_FFFF) as f32 / 16_777_215.0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The engine boundary (ENG-13, EXT-06): an engine is written the way
    /// an extension is. Every file under `src/engines/` except the host
    /// (the core's side) may use `mdeck_sdk` and engine helpers under
    /// `crate::engines`, nothing else of mdeck, and no egui. A new engine
    /// is covered without being listed.
    #[test]
    fn engines_stay_inside_their_boundary() {
        fn files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(dir).expect("engines dir").flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if path.file_name().is_some_and(|n| n == "host") {
                        continue;
                    }
                    files(&path, out);
                } else if path.extension().is_some_and(|e| e == "rs")
                    && path
                        .file_name()
                        .is_some_and(|n| n != "mod.rs" || path.parent() != Some(dir))
                {
                    out.push(path);
                }
            }
        }
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/engines");
        let mut all = Vec::new();
        files(&dir, &mut all);
        assert!(!all.is_empty(), "no engine files found");
        let mut bad = Vec::new();
        for path in all {
            let src = std::fs::read_to_string(&path).expect("read");
            for (n, line) in src.lines().enumerate() {
                let code = line.split("//").next().unwrap_or("");
                let mut rest = code;
                while let Some(at) = rest.find("crate::") {
                    let tail = &rest[at..];
                    let quoted = rest[..at].ends_with('"');
                    let ok = tail.starts_with("crate::engines::")
                        && !tail.starts_with("crate::engines::host");
                    if !quoted && !ok {
                        bad.push(format!("{}:{}: {}", path.display(), n + 1, line.trim()));
                    }
                    rest = &tail[7..];
                }
                if code.contains("egui") || code.contains("super::super::host") {
                    bad.push(format!("{}:{}: {}", path.display(), n + 1, line.trim()));
                }
            }
        }
        assert!(
            bad.is_empty(),
            "engines reach outside their boundary:\n{}",
            bad.join("\n")
        );
    }

    #[test]
    fn names_resolve_through_the_registry() {
        for def in crate::registry::get().engines() {
            assert_eq!(EngineId::find(def.name).map(EngineId::name), Some(def.name));
        }
        assert_eq!(EngineId::find("fireworks"), None);
        assert!(names().contains("plain"));
    }

    /// The hooks agree with the capabilities: a board brings its design
    /// set, and only plain paints nothing.
    #[test]
    fn the_registry_agrees_with_the_capabilities() {
        for def in crate::registry::get().engines() {
            let id = EngineId::find(def.name).unwrap();
            assert_eq!(def.capabilities.board, id.board().is_some(), "{}", def.name);
            assert_eq!(id.paints(), def.name != "plain", "{}", def.name);
        }
        let plain = EngineId::plain();
        assert_eq!(plain.capabilities(), Capabilities::NONE);
        assert_eq!(plain.settle_frames(), 0);
    }
}
