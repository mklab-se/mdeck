//! Engines: what a theme does beyond colours and type.
//!
//! The core decides *what* a slide wants to show (its illustration and where
//! it goes, the countdown digit, the end words, the geometry the renderers
//! drew) and hands that over as a [`Stage`]. An engine decides *how* it looks:
//! a particle field, an LED wall, a laser, falling blocks. Engines never parse
//! markdown or resolve illustrations themselves.
//!
//! Adding one: a module here with a type implementing [`Engine`], a variant in
//! [`EngineKind`] with its [`Capabilities`], and a cargo feature. The guide is
//! `crates/mdeck/doc/engines.md`.

pub mod art;
pub mod blocks;
pub mod blueprint;
pub mod chalkboard;
pub mod darkroom;
#[cfg(test)]
mod example;
mod host;
pub mod laser;
pub mod led;
mod masks;
pub mod particles;
pub mod plain;
pub mod sketch;
pub mod splitflap;
pub mod stage;
pub mod watercolour;

pub use host::{Host, Shot};
pub use stage::{CountPhase, FrameCx, Mask, Place, Stage};

use crate::parser::Slide;
use crate::render::illustration::Library;

/// Which engine a theme (or a deck's `@engine`) runs.
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
    /// transitions between slides (the split-flap board).
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
        match self {
            EngineKind::Plain => true,
            EngineKind::Particles => cfg!(feature = "particles"),
            EngineKind::Led => cfg!(feature = "led"),
            EngineKind::SplitFlap => cfg!(feature = "splitflap"),
            EngineKind::Laser => cfg!(feature = "laser"),
            EngineKind::Blocks => cfg!(feature = "blocks"),
            EngineKind::Blueprint => cfg!(feature = "blueprint"),
            EngineKind::Sketch => cfg!(feature = "sketch"),
            EngineKind::Chalkboard => cfg!(feature = "chalkboard"),
            EngineKind::Watercolour => cfg!(feature = "watercolour"),
            EngineKind::Darkroom => cfg!(feature = "darkroom"),
        }
    }

    pub fn capabilities(self) -> Capabilities {
        match self {
            EngineKind::Plain => Capabilities {
                paints: false,
                editorial: false,
                board: false,
                illustrations: false,
                stories: false,
                countdown: false,
                end_act: false,
                art: false,
            },
            EngineKind::Particles => Capabilities {
                paints: true,
                editorial: true,
                board: false,
                illustrations: true,
                stories: true,
                countdown: true,
                end_act: true,
                art: false,
            },
            EngineKind::Led => Capabilities {
                paints: true,
                editorial: true,
                board: false,
                illustrations: true,
                stories: false,
                countdown: true,
                end_act: true,
                art: false,
            },
            EngineKind::SplitFlap => Capabilities {
                paints: true,
                editorial: false,
                board: true,
                illustrations: false,
                stories: false,
                countdown: true,
                end_act: true,
                art: false,
            },
            EngineKind::Blueprint
            | EngineKind::Sketch
            | EngineKind::Chalkboard
            | EngineKind::Watercolour
            | EngineKind::Darkroom => Capabilities {
                paints: true,
                editorial: true,
                board: false,
                illustrations: true,
                stories: false,
                countdown: true,
                end_act: true,
                art: true,
            },
            EngineKind::Laser | EngineKind::Blocks => Capabilities {
                paints: true,
                editorial: true,
                board: false,
                illustrations: true,
                stories: false,
                countdown: true,
                end_act: true,
                art: false,
            },
        }
    }

    /// A new, empty runtime for this engine.
    pub fn create(self) -> Box<dyn Engine> {
        match self {
            EngineKind::Plain => Box::new(plain::Plain),
            EngineKind::Particles => Box::new(particles::Particles::new()),
            EngineKind::Led => Box::new(led::Led::new()),
            EngineKind::SplitFlap => Box::new(splitflap::SplitFlap::new()),
            EngineKind::Laser => Box::new(laser::Laser::new()),
            EngineKind::Blocks => Box::new(blocks::Blocks::new()),
            EngineKind::Blueprint => Box::new(blueprint::Blueprint::new()),
            EngineKind::Sketch => Box::new(sketch::Sketch::new()),
            EngineKind::Chalkboard => Box::new(chalkboard::Chalkboard::new()),
            EngineKind::Watercolour => Box::new(watercolour::Watercolour::new()),
            EngineKind::Darkroom => Box::new(darkroom::Darkroom::new()),
        }
    }

    /// The medium an art engine draws in (`None`: the engine draws no art).
    pub fn medium(self) -> Option<&'static crate::render::art::Medium> {
        match self {
            EngineKind::Blueprint => Some(&blueprint::MEDIUM),
            EngineKind::Sketch => Some(&sketch::MEDIUM),
            EngineKind::Chalkboard => Some(&chalkboard::MEDIUM),
            EngineKind::Watercolour => Some(&watercolour::MEDIUM),
            EngineKind::Darkroom => Some(&darkroom::MEDIUM),
            _ => None,
        }
    }

    /// Prints the slide number itself, so the editorial counter is left out.
    pub fn numbers_slides(self) -> bool {
        self == EngineKind::Blueprint
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
        match self {
            EngineKind::Particles => particles::END_CAPTION_DELAY,
            EngineKind::Led => led::END_CAPTION_DELAY,
            EngineKind::SplitFlap => splitflap::END_CAPTION_DELAY,
            EngineKind::Laser => laser::END_CAPTION_DELAY,
            EngineKind::Blocks => blocks::END_CAPTION_DELAY,
            EngineKind::Blueprint => blueprint::END_CAPTION_DELAY,
            EngineKind::Sketch => sketch::END_CAPTION_DELAY,
            EngineKind::Chalkboard => chalkboard::END_CAPTION_DELAY,
            EngineKind::Watercolour => watercolour::END_CAPTION_DELAY,
            EngineKind::Darkroom => darkroom::END_CAPTION_DELAY,
            EngineKind::Plain => 0.0,
        }
    }
}

/// The engine a deck runs on: `--engine` on the command line, then the deck's
/// `@engine`, then the theme's own. An unknown `--engine` is an error (the
/// caller stops); an unknown or unavailable `@engine` is a warning and the
/// theme's engine is kept.
pub fn choose(
    cli: Option<&str>,
    deck: Option<&str>,
) -> Result<(Option<EngineKind>, Vec<String>), String> {
    let mut warnings = Vec::new();
    if let Some(name) = cli.map(str::trim) {
        let kind = EngineKind::from_name(&name.to_ascii_lowercase()).ok_or_else(|| {
            format!(
                "--engine: '{name}' is not an engine ({})",
                EngineKind::names()
            )
        })?;
        if !kind.available() {
            return Err(format!("--engine: {name} is not in this build of MDeck"));
        }
        return Ok((Some(kind), warnings));
    }
    let Some(name) = deck.map(str::trim).filter(|n| !n.is_empty()) else {
        return Ok((None, warnings));
    };
    match EngineKind::from_name(&name.to_ascii_lowercase()) {
        Some(kind) if kind.available() => Ok((Some(kind), warnings)),
        Some(_) => {
            warnings.push(format!(
                "@engine: {name} is not in this build of MDeck; using the theme's engine"
            ));
            Ok((None, warnings))
        }
        None => {
            warnings.push(format!(
                "@engine: '{name}' is not an engine ({}); using the theme's engine",
                EngineKind::names()
            ));
            Ok((None, warnings))
        }
    }
}

/// `theme` running on `kind` instead of its own engine (colours, fonts and
/// logo stay the theme's). A countdown the new engine cannot draw itself
/// becomes the plain one.
pub fn with_engine(
    mut theme: crate::theme::Theme,
    kind: Option<EngineKind>,
) -> crate::theme::Theme {
    let Some(kind) = kind else {
        return theme;
    };
    theme.engine = kind;
    if theme.countdown == crate::theme::Countdown::Burst && !kind.capabilities().countdown {
        theme.countdown = crate::theme::Countdown::Plain;
    }
    theme
}

/// What `kind` will not show on `slide`, one message per thing, for
/// `--check` and the summary line when presenting or exporting.
/// `has_story`: a story (sidecar or inline) exists for the slide.
pub fn unsupported(kind: EngineKind, slide: &Slide, has_story: bool) -> Vec<String> {
    let caps = kind.capabilities();
    let mut out = Vec::new();
    if let Some(name) = &slide.illustration
        && !caps.illustrations
    {
        out.push(format!(
            "@illustration: {name} is not shown by the {} engine",
            kind.name()
        ));
    }
    if caps.board {
        out.extend(splitflap::problems(slide));
    }
    if !caps.art
        && let Some(scene) = crate::render::art::slide_scene(slide)
    {
        let short: String = scene.chars().take(40).collect();
        let media: Vec<&str> = EngineKind::ALL
            .iter()
            .filter(|k| k.capabilities().art)
            .map(|k| k.name())
            .collect();
        out.push(format!(
            "@art: '{short}' is not drawn by the {} engine (engines that draw art: {})",
            kind.name(),
            media.join(", ")
        ));
    }
    if has_story && !caps.stories {
        out.push(format!(
            "the slide's story is not played by the {} engine",
            kind.name()
        ));
    }
    out
}

/// One line for stderr when a deck has content its engine will not show, or
/// `None` when everything shows.
pub fn unsupported_summary(
    kind: EngineKind,
    presentation: &crate::parser::Presentation,
    stories: &[bool],
) -> Option<String> {
    let n = presentation
        .slides
        .iter()
        .enumerate()
        .filter(|(i, s)| {
            !unsupported(kind, s, stories.get(*i).copied().unwrap_or(false)).is_empty()
        })
        .count();
    (n > 0).then(|| {
        format!(
            "the {} engine does not show everything on {n} slide{}; run `mdeck <deck> --check` for details",
            kind.name(),
            if n == 1 { "" } else { "s" }
        )
    })
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

    #[test]
    fn cli_beats_deck_beats_theme() {
        assert_eq!(
            choose(Some("plain"), Some("particles")).unwrap().0,
            Some(EngineKind::Plain)
        );
        assert_eq!(
            choose(None, Some(" Particles ")).unwrap().0,
            Some(EngineKind::Particles)
        );
        assert_eq!(choose(None, None).unwrap().0, None);
        // a bad --engine stops; a bad @engine warns and keeps the theme's
        assert!(
            choose(Some("lasers"), None)
                .unwrap_err()
                .contains("plain, particles")
        );
        let (kind, warnings) = choose(None, Some("fireworks")).unwrap();
        assert_eq!(kind, None);
        assert!(warnings[0].contains("fireworks"), "{warnings:?}");
    }

    #[test]
    fn with_engine_keeps_the_look_and_fixes_the_countdown() {
        let ember = crate::theme::Theme::ember();
        let plain = with_engine(ember.clone(), Some(EngineKind::Plain));
        assert_eq!(plain.engine, EngineKind::Plain);
        assert_eq!(plain.accent, ember.accent);
        assert_eq!(plain.countdown, crate::theme::Countdown::Plain);
        let dark = with_engine(crate::theme::Theme::dark(), Some(EngineKind::Particles));
        assert_eq!(dark.engine, EngineKind::Particles);
        assert_eq!(with_engine(ember.clone(), None).engine, ember.engine);
    }

    #[test]
    fn unsupported_names_illustrations_and_stories() {
        let pres = crate::parser::parse("# A\n@illustration: server\n\n- one\n\n# B\n\n- two\n");
        let a = &pres.slides[0];
        assert!(unsupported(EngineKind::Particles, a, true).is_empty());
        let plain = unsupported(EngineKind::Plain, a, true);
        assert_eq!(plain.len(), 2, "{plain:?}");
        assert!(plain[0].contains("server") && plain[1].contains("story"));
        assert!(unsupported(EngineKind::Plain, &pres.slides[1], false).is_empty());
        let line = unsupported_summary(EngineKind::Plain, &pres, &[false, false]).unwrap();
        assert!(line.contains("1 slide;"), "{line}");
        assert!(unsupported_summary(EngineKind::Particles, &pres, &[]).is_none());
    }

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
