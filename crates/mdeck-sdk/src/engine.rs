//! Engines: the code that brings a slide to life around its content
//! (particles, LED walls, chalk, heat). See [`Engine`] and [`EngineDef`].

use crate::design::DesignSet;
use crate::paint::{Color, Painter, Pos2};
use crate::problem::Problem;
use crate::stage::{Frame, Stage, Strategy};
use crate::tokens::EngineSettings;

/// A running engine. The host calls [`Engine::update`] and then
/// [`Engine::paint`] every frame, under the slide's content.
///
/// ```
/// use mdeck_sdk::engine::Engine;
/// use mdeck_sdk::paint::{premul, Painter};
/// use mdeck_sdk::stage::{Frame, Stage};
///
/// /// A slow pulse of the accent colour in the slide's corner.
/// struct Pulse { t: f32 }
///
/// impl Engine for Pulse {
///     fn update(&mut self, frame: &Frame, _stage: &Stage) {
///         self.t = if frame.settled() { 0.0 } else { self.t + frame.dt };
///     }
///     fn paint(&mut self, p: &mut Painter, frame: &Frame, _stage: &Stage) {
///         let glow = 0.5 + 0.5 * (self.t * 2.0).sin();
///         let r = 80.0 * frame.scale;
///         p.circle_filled(frame.rect.min, r, premul(frame.tokens.accent, glow * frame.opacity));
///     }
/// }
/// ```
pub trait Engine {
    /// Bring the state up to date with `stage` and advance the clock by
    /// `frame.dt`. With `frame.still` set, settle at once (ENG-07); with
    /// `frame.reduced_motion`, show the settled state (ENG-09).
    fn update(&mut self, frame: &Frame, stage: &Stage);

    /// Paint the engine's layer into `frame.rect`.
    fn paint(&mut self, painter: &mut Painter, frame: &Frame, stage: &Stage);

    /// Draw the presenter's pen strokes in the engine's own way (the thermal
    /// engine's heat trace). Return `true` when the engine drew them, so
    /// the host leaves out its plain ink. The default draws nothing.
    fn annotate(
        &mut self,
        _painter: &mut Painter,
        _frame: &Frame,
        _strokes: &[Annotation],
    ) -> bool {
        false
    }

    /// Whether the engine is still moving and needs another frame soon.
    /// The host stops repainting a settled slide when this is `false`.
    fn animating(&self) -> bool {
        true
    }
}

/// One pen stroke the presenter drew on the slide.
///
/// ```
/// use mdeck_sdk::engine::Annotation;
/// use mdeck_sdk::paint::{Color, Pos2};
/// let a = Annotation { points: vec![Pos2::ZERO], color: Color::WHITE, width: 4.0, age: 0.0 };
/// assert_eq!(a.points.len(), 1);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Annotation {
    /// The stroke's points in order, in points on the slide.
    pub points: Vec<Pos2>,
    /// The pen's colour.
    pub color: Color,
    /// The pen's width in points.
    pub width: f32,
    /// Seconds since the stroke was finished (0 while drawing).
    pub age: f32,
}

/// What the core must do differently for an engine (ENG-04).
///
/// ```
/// use mdeck_sdk::engine::Capabilities;
/// let c = Capabilities { countdown: true, ..Capabilities::NONE };
/// assert!(c.countdown && !c.board);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Capabilities {
    /// Shows the slide's picture (a point cloud, an artwork or an image).
    pub picture: bool,
    /// Draws the opening countdown itself.
    pub countdown: bool,
    /// Plays an act of its own on the end slide.
    pub ending: bool,
    /// Draws every slide itself, text included, through
    /// [`EngineDef::board`] (ENG-03).
    pub board: bool,
    /// Owns the transitions between slides.
    pub transition: bool,
    /// The medium an art engine draws generated pictures in.
    pub medium: Option<Medium>,
}

impl Capabilities {
    /// No capabilities: an engine that only decorates.
    pub const NONE: Capabilities = Capabilities {
        picture: false,
        countdown: false,
        ending: false,
        board: false,
        transition: false,
        medium: None,
    };
}

/// The medium of an art engine: how generated pictures are prepared for it.
///
/// ```
/// use mdeck_sdk::engine::{Medium, MediumKind};
/// use mdeck_sdk::stage::Strategy;
/// let m = Medium { name: "watercolour", kind: MediumKind::Tonal, strategy: Strategy::Bloom };
/// assert_eq!(m.kind, MediumKind::Tonal);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Medium {
    /// The medium's name (`line`, `watercolour`, `darkroom`, ...).
    pub name: &'static str,
    /// Line art or tonal pictures.
    pub kind: MediumKind,
    /// How the prepared picture is revealed.
    pub strategy: Strategy,
}

/// Whether a medium draws line art or full tone.
///
/// See [`Medium`] for an example.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MediumKind {
    /// Ink lines on a surface: pictures become white with ink as alpha.
    Line,
    /// Full-tone pictures.
    Tonal,
}

/// What an engine needs from the theme (ENG-12).
///
/// ```
/// use mdeck_sdk::engine::Needs;
/// assert!(!Needs::default().page);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Needs {
    /// The slide must be a sheet on a surface (the theme's `page:` block).
    pub page: bool,
}

/// The type of an engine setting.
///
/// See [`SettingSpec`] for an example.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingKind {
    /// Text.
    Text,
    /// A number.
    Number,
    /// `true` or `false`.
    Bool,
    /// A colour like `#ff4d1c`.
    Color,
    /// One of the listed words.
    OneOf(&'static [&'static str]),
}

/// One setting an engine reads from the theme's `engine:` block.
///
/// ```
/// use mdeck_sdk::engine::{SettingKind, SettingSpec};
/// const SURFACE: SettingSpec = SettingSpec {
///     key: "surface",
///     kind: SettingKind::OneOf(&["sheet", "slate"]),
///     summary: "Paper or a blackboard.",
/// };
/// assert_eq!(SURFACE.key, "surface");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingSpec {
    /// The key in the `engine:` block.
    pub key: &'static str,
    /// Its type.
    pub kind: SettingKind,
    /// One sentence for `mdeck theme` help and the docs.
    pub summary: &'static str,
}

/// An engine as registered: its name, what it can do and how to make one.
///
/// ```
/// use mdeck_sdk::engine::{Capabilities, Engine, EngineDef, Needs, SettingKind, SettingSpec};
/// use mdeck_sdk::paint::Painter;
/// use mdeck_sdk::stage::{Frame, Stage};
/// use mdeck_sdk::tokens::{EngineSettings, Value};
///
/// struct Calm;
/// impl Engine for Calm {
///     fn update(&mut self, _: &Frame, _: &Stage) {}
///     fn paint(&mut self, _: &mut Painter, _: &Frame, _: &Stage) {}
///     fn animating(&self) -> bool { false }
/// }
///
/// static CALM: EngineDef = EngineDef {
///     name: "calm",
///     summary: "Nothing moves.",
///     capabilities: Capabilities::NONE,
///     settings: &[SettingSpec { key: "depth", kind: SettingKind::Number, summary: "How deep." }],
///     needs: Needs { page: false },
///     ending_caption_delay: 1.0,
///     create: |_| Box::new(Calm),
///     board: None,
/// };
///
/// let s = EngineSettings::from_pairs([("depth", Value::Bool(true)), ("hue", Value::Null)]);
/// assert_eq!(CALM.check_settings(&s).len(), 2);
/// ```
pub struct EngineDef {
    /// The name themes and decks use (`engine: calm`).
    pub name: &'static str,
    /// One sentence for `mdeck theme list` and the docs.
    pub summary: &'static str,
    /// What the core must do differently.
    pub capabilities: Capabilities,
    /// The settings the engine reads from the theme's `engine:` block.
    pub settings: &'static [SettingSpec],
    /// What the engine needs from the theme.
    pub needs: Needs,
    /// Seconds into the end slide when the "powered by" caption fades in.
    pub ending_caption_delay: f32,
    /// A new runtime, reading its settings.
    pub create: fn(&EngineSettings) -> Box<dyn Engine>,
    /// A board engine's design set: it draws every slide (see
    /// [`Capabilities::board`]).
    pub board: Option<&'static dyn DesignSet>,
}

impl std::fmt::Debug for EngineDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineDef")
            .field("name", &self.name)
            .field("capabilities", &self.capabilities)
            .field("settings", &self.settings)
            .field("needs", &self.needs)
            .finish_non_exhaustive()
    }
}

impl EngineDef {
    /// Check `settings` against [`EngineDef::settings`]: every value of the
    /// wrong type and every key the engine does not declare is a problem.
    /// Runs without creating the engine.
    ///
    /// See [`EngineDef`] for an example.
    pub fn check_settings(&self, settings: &EngineSettings) -> Vec<Problem> {
        let probe = settings.clone();
        for spec in self.settings {
            match spec.kind {
                SettingKind::Text => {
                    probe.str(spec.key);
                }
                SettingKind::Number => {
                    probe.f32(spec.key);
                }
                SettingKind::Bool => {
                    probe.bool(spec.key);
                }
                SettingKind::Color => {
                    probe.color(spec.key);
                }
                SettingKind::OneOf(options) => {
                    probe.one_of(spec.key, options);
                }
            }
        }
        probe.problems()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::Value;

    struct Still;
    impl Engine for Still {
        fn update(&mut self, _: &Frame, _: &Stage) {}
        fn paint(&mut self, _: &mut Painter, _: &Frame, _: &Stage) {}
    }

    static DEF: EngineDef = EngineDef {
        name: "still",
        summary: "",
        capabilities: Capabilities::NONE,
        settings: &[
            SettingSpec {
                key: "surface",
                kind: SettingKind::OneOf(&["sheet", "slate"]),
                summary: "",
            },
            SettingSpec {
                key: "ink",
                kind: SettingKind::Color,
                summary: "",
            },
        ],
        needs: Needs { page: true },
        ending_caption_delay: 0.0,
        create: |_| Box::new(Still),
        board: None,
    };

    #[test]
    fn valid_settings_pass_and_the_original_is_untouched() {
        let s = EngineSettings::from_pairs([
            ("surface", Value::String("slate".into())),
            ("ink", Value::String("#123456".into())),
        ]);
        assert!(DEF.check_settings(&s).is_empty());
        // checking reads a clone, so the engine's own reads still count
        assert_eq!(s.problems().len(), 2);
    }

    #[test]
    fn invalid_choices_are_reported() {
        let s = EngineSettings::from_pairs([("surface", Value::String("paper".into()))]);
        let p = DEF.check_settings(&s);
        assert_eq!(p.len(), 1);
        assert!(p[0].message.contains("one of sheet, slate"), "{}", p[0]);
    }

    #[test]
    fn default_hooks() {
        let mut e = Still;
        assert!(e.animating());
        let _ = (DEF.create)(&EngineSettings::new());
        let _ = &mut e;
    }
}
