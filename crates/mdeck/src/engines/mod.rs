//! Engines: what a theme does beyond colours and type.
//!
//! The core decides *what* a slide wants to show (its illustration and where
//! it goes, the countdown digit, the end words, the geometry the renderers
//! drew) and hands that over as a [`Stage`]. An engine decides *how* it looks:
//! a particle field, an LED wall, a laser, falling blocks. Engines never parse
//! markdown or resolve illustrations themselves.
//!
//! Adding one: a module here behind a cargo feature, with a type implementing
//! [`Engine`] and its [`EngineDef`] (`DEF`); a variant in [`EngineKind`] with
//! its name, and its entry in [`EngineKind::def`]. The guide is
//! `crates/mdeck/doc/engines.md`.

#[cfg(feature = "art")]
pub mod art;
#[cfg(feature = "blocks")]
pub mod blocks;
#[cfg(feature = "blueprint")]
pub mod blueprint;
#[cfg(feature = "chalkboard")]
pub mod chalkboard;
mod choice;
#[cfg(feature = "darkroom")]
pub mod darkroom;
#[cfg(test)]
mod example;
mod host;
#[cfg(feature = "laser")]
pub mod laser;
#[cfg(feature = "led")]
pub mod led;
mod masks;
pub mod paint;
#[cfg(feature = "particles")]
pub mod particles;
pub mod plain;
#[cfg(feature = "sketch")]
pub mod sketch;
#[cfg(feature = "splitflap")]
pub mod splitflap;
pub mod stage;
#[cfg(feature = "watercolour")]
pub mod watercolour;

pub use choice::{choose, unsupported, unsupported_summary, with_engine};
pub use host::{Host, Shot};
pub use stage::{CountPhase, FrameCx, Mask, Place, Stage};

use crate::parser::Slide;
use crate::render::art::Medium;
use crate::render::illustration::Library;

/// Which engine a theme (or a deck's `@engine`) runs. Every engine has a
/// variant in every build, so a theme can name one the build leaves out; it
/// is then reported as not available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineKind {
    /// Slides on a flat background.
    Plain,
    /// A living particle field under every slide, editorial copy layouts,
    /// story beats and point cloud illustrations.
    Particles,
    /// A wall of RGB LEDs that light up illustrations, digits and words.
    Led,
    /// A departure board: every slide's text on a grid of split flaps.
    SplitFlap,
    /// A laser etches illustrations, digits and words onto the slide.
    Laser,
    /// Illustrations, digits and words built from falling blocks.
    Blocks,
    /// A draftsman's blueprint: generated line art inked onto a blue sheet.
    Blueprint,
    /// A sketchbook: generated graphite drawings drawn in with a pencil.
    Sketch,
    /// A chalkboard: generated line art drawn in chalk on a slate.
    Chalkboard,
    /// Watercolour: generated paintings that bloom onto cold-press paper.
    Watercolour,
    /// A darkroom: generated photographs that develop under a safelight.
    Darkroom,
}

/// What an engine can show. The core uses it for fallbacks (an illustration
/// an engine cannot show is not drawn) and `--check` for warnings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capabilities {
    /// Paints a layer of its own every frame (plain paints nothing).
    pub paints: bool,
    /// Copy slides use the editorial layouts: a copy column on the left and
    /// a stage on the right, display headings, the counter chrome.
    pub editorial: bool,
    /// The engine draws every slide itself, text included, and owns the
    /// transitions between slides (the split-flap board). Its
    /// [`EngineDef::render_slide`] draws the slide.
    pub board: bool,
    /// Shows `@illustration` point clouds.
    pub illustrations: bool,
    /// Plays story beats (`@story`, `mdeck ai story`).
    pub stories: bool,
    /// Draws the opening countdown itself (`countdown: burst` in a theme).
    pub countdown: bool,
    /// Plays an act of its own on the end slide.
    pub end_act: bool,
    /// Draws generated art (`@art`, `mdeck ai art`) in its medium.
    pub art: bool,
    /// Prints the slide number itself, so the editorial counter is left out.
    pub numbers_slides: bool,
}

impl Capabilities {
    /// Nothing: slides on their own (plain).
    pub const NONE: Capabilities = Capabilities {
        paints: false,
        editorial: false,
        board: false,
        illustrations: false,
        stories: false,
        countdown: false,
        end_act: false,
        art: false,
        numbers_slides: false,
    };

    /// What most engines show: a layer under editorial slides, the slide's
    /// illustration, the countdown and an end act.
    pub const PICTURES: Capabilities = Capabilities {
        paints: true,
        editorial: true,
        illustrations: true,
        countdown: true,
        end_act: true,
        ..Capabilities::NONE
    };
}

/// A board engine's static renderer: the slide the way the board shows it
/// when no engine runs live (thumbnails, the overview), with the arguments
/// of the core's `render_slide`.
pub type RenderSlide =
    fn(&crate::render::BlockCx, &Slide, eframe::egui::Rect, &crate::render::SlideContext);

/// One engine as the core sees it: everything but its name. Each engine
/// module has one as `DEF`, and [`EngineKind::def`] lists them.
pub struct EngineDef {
    pub capabilities: Capabilities,
    /// A new, empty runtime.
    pub create: fn() -> Box<dyn Engine>,
    /// Seconds into the end slide when the "powered by" caption fades in.
    pub end_caption_delay: f32,
    /// The medium an art engine draws in.
    pub medium: Option<&'static Medium>,
    /// A board engine's static renderer (see [`Capabilities::board`]).
    pub render_slide: Option<RenderSlide>,
    /// What the engine does not show on a slide, one message per thing, for
    /// `--check` (beyond what its capabilities say).
    pub problems: Option<fn(&Slide) -> Vec<String>>,
}

impl EngineKind {
    /// Every engine, in the order `mdeck theme list` and the docs show them.
    pub const ALL: &'static [EngineKind] = &[
        EngineKind::Plain,
        EngineKind::Particles,
        EngineKind::Led,
        EngineKind::SplitFlap,
        EngineKind::Laser,
        EngineKind::Blocks,
        EngineKind::Blueprint,
        EngineKind::Sketch,
        EngineKind::Chalkboard,
        EngineKind::Watercolour,
        EngineKind::Darkroom,
    ];

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|k| k.name() == name)
    }

    /// The engine's name, known in every build.
    pub fn name(self) -> &'static str {
        match self {
            EngineKind::Plain => "plain",
            EngineKind::Particles => "particles",
            EngineKind::Led => "led",
            EngineKind::SplitFlap => "splitflap",
            EngineKind::Laser => "laser",
            EngineKind::Blocks => "blocks",
            EngineKind::Blueprint => "blueprint",
            EngineKind::Sketch => "sketch",
            EngineKind::Chalkboard => "chalkboard",
            EngineKind::Watercolour => "watercolour",
            EngineKind::Darkroom => "darkroom",
        }
    }

    /// The engine's definition, `None` when this build leaves it out (each
    /// engine but plain is a cargo feature).
    pub fn def(self) -> Option<&'static EngineDef> {
        match self {
            EngineKind::Plain => Some(&plain::DEF),
            #[cfg(feature = "particles")]
            EngineKind::Particles => Some(&particles::DEF),
            #[cfg(feature = "led")]
            EngineKind::Led => Some(&led::DEF),
            #[cfg(feature = "splitflap")]
            EngineKind::SplitFlap => Some(&splitflap::DEF),
            #[cfg(feature = "laser")]
            EngineKind::Laser => Some(&laser::DEF),
            #[cfg(feature = "blocks")]
            EngineKind::Blocks => Some(&blocks::DEF),
            #[cfg(feature = "blueprint")]
            EngineKind::Blueprint => Some(&blueprint::DEF),
            #[cfg(feature = "sketch")]
            EngineKind::Sketch => Some(&sketch::DEF),
            #[cfg(feature = "chalkboard")]
            EngineKind::Chalkboard => Some(&chalkboard::DEF),
            #[cfg(feature = "watercolour")]
            EngineKind::Watercolour => Some(&watercolour::DEF),
            #[cfg(feature = "darkroom")]
            EngineKind::Darkroom => Some(&darkroom::DEF),
            #[allow(unreachable_patterns)]
            _ => None,
        }
    }

    /// Every engine name, for error messages: "plain, particles, ...".
    pub fn names() -> String {
        Self::ALL
            .iter()
            .map(|k| k.name())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Whether this build includes the engine (each one is a cargo feature).
    pub fn available(self) -> bool {
        self.def().is_some()
    }

    /// What the engine shows; an engine this build leaves out shows nothing.
    pub fn capabilities(self) -> Capabilities {
        self.def().map_or(Capabilities::NONE, |d| d.capabilities)
    }

    /// A new, empty runtime for this engine (plain's when it is left out).
    pub fn create(self) -> Box<dyn Engine> {
        (self.def().unwrap_or(&plain::DEF).create)()
    }

    /// The medium an art engine draws in (`None`: the engine draws no art).
    pub fn medium(self) -> Option<&'static Medium> {
        self.def().and_then(|d| d.medium)
    }

    /// A board engine's renderer for whole slides (`None`: not a board).
    pub fn board(self) -> Option<RenderSlide> {
        self.def().and_then(|d| d.render_slide)
    }

    /// Prints the slide number itself, so the editorial counter is left out.
    pub fn numbers_slides(self) -> bool {
        self.capabilities().numbers_slides
    }

    /// Paints a layer of its own under the slide.
    pub fn paints(self) -> bool {
        self.capabilities().paints
    }

    /// Lays `slide` out with the editorial layouts instead of the generic ones.
    pub fn lays_out(self, slide: &Slide) -> bool {
        self.capabilities().editorial && crate::render::ember::handles(slide)
    }

    /// Draws every slide itself (a board), text and transitions included.
    pub fn is_board(self) -> bool {
        self.capabilities().board
    }

    /// Story beats add reveal steps to a slide.
    pub fn plays_stories(self) -> bool {
        self.capabilities().stories
    }

    /// Frames an export waits on a slide before capturing it, so the engine
    /// has seen the geometry the slide's renderers publish.
    pub fn settle_frames(self) -> u32 {
        if self.paints() { 2 } else { 0 }
    }

    /// Seconds into the end slide when the "powered by" caption fades in.
    pub fn end_caption_delay(self) -> f32 {
        self.def().map_or(0.0, |d| d.end_caption_delay)
    }
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

/// An engine's runtime: the state behind one presentation window or export.
///
/// Each frame the host calls [`Engine::update`] (advance the clock by
/// `cx.dt`, or settle at once when `cx.still`), then [`Engine::paint`].
/// Painting happens under the slide; everything an engine draws must look
/// finished when `cx.still` is set, because that is what export captures.
pub trait Engine {
    /// Bring the state up to date with `stage` and advance its clock.
    fn update(&mut self, cx: &FrameCx, stage: &Stage, lib: &mut Library);

    /// Paint the engine's layer into `cx.rect`.
    fn paint(&mut self, ui: &eframe::egui::Ui, cx: &FrameCx, stage: &Stage);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The engine boundary: an engine uses the stage, the theme, the parsed
    /// slide and the render helpers engines share, never the app, the
    /// commands, the config or the CLI. Every file under `src/engines/` is
    /// checked, so a new engine is covered without being listed.
    #[test]
    fn engines_stay_inside_their_boundary() {
        const ALLOWED: &[&str] = &[
            "crate::engines",
            "crate::parser",
            "crate::theme",
            "crate::render::hints",
            "crate::render::illustration",
            "crate::render::image_cache",
            "crate::render::story",
            "crate::render::art",
            "crate::render::strokes",
            "crate::render::particles",
            "crate::render::fonts",
            "crate::render::ember",
            "crate::render::SlideContext",
            "crate::render::BlockCx",
            "crate::render::test_support",
        ];
        fn files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(dir).expect("engines dir").flatten() {
                let path = entry.path();
                if path.is_dir() {
                    files(&path, out);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    out.push(path);
                }
            }
        }
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/engines");
        let mut all = Vec::new();
        files(&dir, &mut all);
        assert!(all.len() >= 10, "found {} engine files", all.len());
        let mut bad = Vec::new();
        for path in all {
            let src = std::fs::read_to_string(&path).expect("read");
            for (n, line) in src.lines().enumerate() {
                let mut rest = line;
                while let Some(at) = rest.find("crate::") {
                    let tail = &rest[at..];
                    let quoted = rest[..at].ends_with('"');
                    if !quoted && !ALLOWED.iter().any(|a| tail.starts_with(a)) {
                        bad.push(format!("{}:{}: {}", path.display(), n + 1, line.trim()));
                    }
                    rest = &tail[7..];
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
    fn names_round_trip() {
        for &k in EngineKind::ALL {
            assert_eq!(EngineKind::from_name(k.name()), Some(k));
        }
        assert_eq!(EngineKind::from_name("fireworks"), None);
        assert_eq!(
            EngineKind::names(),
            "plain, particles, led, splitflap, laser, blocks, blueprint, sketch, chalkboard, watercolour, darkroom"
        );
    }

    /// The hooks agree with the capabilities: a board brings its renderer,
    /// an art engine its medium, and an engine the build leaves out shows
    /// nothing and runs as plain.
    #[test]
    fn the_registry_agrees_with_the_capabilities() {
        assert!(EngineKind::Plain.available());
        for &k in EngineKind::ALL {
            let caps = k.capabilities();
            assert_eq!(caps.board, k.board().is_some(), "{}", k.name());
            assert_eq!(caps.art, k.medium().is_some(), "{}", k.name());
            if !k.available() {
                assert_eq!(caps, Capabilities::NONE, "{}", k.name());
                assert_eq!(k.end_caption_delay(), 0.0);
            }
        }
    }

    #[cfg(feature = "blueprint")]
    #[test]
    fn blueprint_numbers_its_own_slides() {
        assert!(EngineKind::Blueprint.numbers_slides());
        assert!(!EngineKind::Plain.numbers_slides());
    }

    #[cfg(feature = "particles")]
    #[test]
    fn plain_paints_nothing_and_particles_does_everything() {
        let plain = EngineKind::Plain.capabilities();
        assert!(!plain.paints && !plain.editorial && !plain.illustrations);
        assert_eq!(EngineKind::Plain.settle_frames(), 0);
        let particles = EngineKind::Particles.capabilities();
        assert!(particles.paints && particles.stories && particles.countdown);
        assert_eq!(EngineKind::Particles.settle_frames(), 2);
    }
}
