//! # Tutorial step 3: Reacting to content
//!
//! An engine that serves the slide's visuals instead of decorating around
//! them. It reads the geometry visuals publish on the stage:
//!
//! - **bars**: embers rise from each bar's top;
//! - **paths** (line series, routed edges): bright runners travel along them;
//! - **circles** (pies, donuts, rings): sparks orbit the rim;
//! - **frames** (images, cards, the copy): the engine stays dark inside them.
//!
//! It also shows typed, validated settings from the theme's `engine:` block
//! (with a range check reported through [`EngineSettings::report`]), what
//! the engine needs from the theme, and an honest [`Engine::animating`]:
//! with nothing to react to, the engine settles and stops asking for frames.
//!
//! The walkthrough is `docs/sdk/tutorial-3-reactive.md`.

use mdeck_sdk::engine::{Capabilities, Engine, EngineDef, Needs, SettingKind, SettingSpec};
use mdeck_sdk::geometry::Hint;
use mdeck_sdk::paint::{Color, Painter, Pos2, Rect, Sprite, SpriteBlend, SpriteLayer, smoothstep};
use mdeck_sdk::registry::{Registry, RegistryError};
use mdeck_sdk::stage::{Frame, Stage};
use mdeck_sdk::tokens::{EngineSettings, Tokens};

/// The settings this engine reads, with their types: mdeck checks a theme
/// against them (`EngineDef::check_settings`) without running the engine.
pub const SETTINGS: &[SettingSpec] = &[
    SettingSpec {
        key: "glow",
        kind: SettingKind::Number,
        summary: "How bright the lights are, from 0 to 2 (default 1).",
    },
    SettingSpec {
        key: "runners",
        kind: SettingKind::Bool,
        summary: "Send runners along line series and edges (default true).",
    },
    SettingSpec {
        key: "palette",
        kind: SettingKind::OneOf(&["accent", "warm", "cool"]),
        summary: "Which theme colours the lights take (default accent).",
    },
    SettingSpec {
        key: "tint",
        kind: SettingKind::Color,
        summary: "One colour for every light, overriding the palette.",
    },
];

/// The engine as mdeck registers it: `engine: reactive` in a theme.
pub static DEF: EngineDef = EngineDef {
    name: "reactive",
    summary: "Embers rise from bars and runners trace lines; dark behind images.",
    capabilities: Capabilities::NONE,
    settings: SETTINGS,
    // Light on dark: no page needed. An ink engine would ask for one here
    // (`page: true`) and `mdeck theme check` would report a theme without it.
    needs: Needs { page: false },
    ending_caption_delay: 1.0,
    create,
    board: None,
};

/// The showcase theme.
pub const THEME: &str = include_str!("../themes/signal.yaml");

/// Register the engine and its showcase theme.
pub fn register(r: &mut Registry) -> Result<(), RegistryError> {
    r.engine(&DEF)?;
    r.theme("signal", THEME)
}

fn create(settings: &EngineSettings) -> Box<dyn Engine> {
    Box::new(Reactive::new(Settings::read(settings)))
}

/// Which theme colours the lights take.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Palette {
    /// The accent and the secondary accent.
    Accent,
    /// The warm particle light.
    Warm,
    /// The cool particle light.
    Cool,
}

/// The engine's settings, typed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    /// Brightness multiplier, 0..=2.
    pub glow: f32,
    /// Whether runners travel along paths.
    pub runners: bool,
    /// Colour choice.
    pub palette: Palette,
    /// A colour overriding the palette.
    pub tint: Option<Color>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            glow: 1.0,
            runners: true,
            palette: Palette::Accent,
            tint: None,
        }
    }
}

impl Settings {
    /// Read every setting. A value of the wrong type is recorded by
    /// `settings` and the default is used; a number out of range is
    /// reported and clamped. mdeck shows the problems in `--check`.
    pub fn read(settings: &EngineSettings) -> Self {
        let d = Settings::default();
        let mut glow = settings.f32_or("glow", d.glow);
        if !(0.0..=2.0).contains(&glow) {
            settings.report("glow", format!("should be between 0 and 2, not {glow}"));
            glow = glow.clamp(0.0, 2.0);
        }
        let palette = match settings.one_of("palette", &["accent", "warm", "cool"]) {
            Some("warm") => Palette::Warm,
            Some("cool") => Palette::Cool,
            _ => d.palette,
        };
        Settings {
            glow,
            runners: settings.bool_or("runners", d.runners),
            palette,
            tint: settings.color("tint"),
        }
    }

    /// The colour of light number `i`.
    pub fn color(&self, tokens: &Tokens, i: usize) -> Color {
        if let Some(c) = self.tint {
            return c;
        }
        match self.palette {
            Palette::Accent if i % 3 == 2 => tokens.secondary,
            Palette::Accent => tokens.accent,
            Palette::Warm => tokens.particle_light,
            Palette::Cool => tokens.particle_cool,
        }
    }
}

/// A path with its length measured once, so runners move at an even pace.
#[derive(Clone, Debug, PartialEq)]
pub struct Route {
    points: Vec<Pos2>,
    /// Distance from the start to each point.
    along: Vec<f32>,
}

impl Route {
    /// A route along `points`.
    pub fn new(points: Vec<Pos2>) -> Self {
        let mut along = Vec::with_capacity(points.len());
        let mut d = 0.0;
        for (i, p) in points.iter().enumerate() {
            if i > 0 {
                d += p.distance(points[i - 1]);
            }
            along.push(d);
        }
        Self { points, along }
    }

    /// Total length in points.
    pub fn length(&self) -> f32 {
        self.along.last().copied().unwrap_or(0.0)
    }

    /// The point at fraction `f` (0..1) of the length.
    pub fn at(&self, f: f32) -> Pos2 {
        let target = f.clamp(0.0, 1.0) * self.length();
        let i = self.along.partition_point(|&d| d < target).max(1);
        if i >= self.points.len() {
            return self.points.last().copied().unwrap_or(Pos2::ZERO);
        }
        let (d0, d1) = (self.along[i - 1], self.along[i]);
        let t = if d1 > d0 {
            (target - d0) / (d1 - d0)
        } else {
            0.0
        };
        self.points[i - 1].lerp(self.points[i], t)
    }
}

/// What the engine derived from the published geometry.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Scene {
    /// Tops of bars: centre x, top y, bar width.
    pub bars: Vec<(Pos2, f32)>,
    /// Paths with runners.
    pub routes: Vec<Route>,
    /// Circles: centre and radius.
    pub rings: Vec<(Pos2, f32)>,
    /// Boxes to stay dark in.
    pub frames: Vec<Rect>,
}

impl Scene {
    /// Derive a scene from `hints`.
    pub fn from_hints(hints: &[Hint]) -> Self {
        let mut s = Scene::default();
        for h in hints {
            match h {
                Hint::Bar(r) => s.bars.push((Pos2::new(r.center().x, r.top()), r.width())),
                Hint::Path(p) if p.len() > 1 => s.routes.push(Route::new(p.clone())),
                Hint::Circle { center, radius } => s.rings.push((*center, *radius)),
                Hint::Frame(r) => s.frames.push(*r),
                _ => {}
            }
        }
        s
    }

    /// Whether anything in the scene moves.
    pub fn moves(&self, settings: &Settings) -> bool {
        !self.bars.is_empty()
            || !self.rings.is_empty()
            || (settings.runners && !self.routes.is_empty())
    }

    /// Whether `p` is inside a frame (with `margin` points to spare).
    pub fn in_frame(&self, p: Pos2, margin: f32) -> bool {
        self.frames.iter().any(|f| f.expand(margin).contains(p))
    }
}

/// Embers per bar, runners per path, sparks per ring.
const EMBERS: usize = 7;
const RUNNERS: usize = 3;
const SPARKS: usize = 10;
/// Seconds for the ground to fade in.
const FADE_IN: f32 = 1.0;

/// The running engine.
pub struct Reactive {
    settings: Settings,
    scene: Scene,
    /// The geometry fingerprint the scene was derived from.
    key: Option<u64>,
    clock: f32,
    age: f32,
    settled: bool,
    layer: SpriteLayer,
}

impl Reactive {
    /// A new engine with `settings`.
    pub fn new(settings: Settings) -> Self {
        Self {
            settings,
            scene: Scene::default(),
            key: None,
            clock: 0.0,
            age: 0.0,
            settled: false,
            layer: SpriteLayer::new(),
        }
    }

    /// What the engine reacts to now.
    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    fn sprites(&self, frame: &Frame) -> Vec<Sprite> {
        let s = frame.scale;
        let t = self.clock;
        let fade = smoothstep(0.0, FADE_IN, self.age) * frame.opacity * self.settings.glow;
        let mut out = Vec::new();
        let mut light = |center: Pos2, size: f32, glow: f32, i: usize| {
            // Dark inside frames: never draw over an image or a card.
            if glow <= 0.0 || self.scene.in_frame(center, size * 0.25) {
                return;
            }
            let c = self.settings.color(frame.tokens, i).to_f32();
            out.push(Sprite {
                center,
                size,
                rgba: [c[0], c[1], c[2], (glow * fade).min(1.0)],
            });
        };
        // The ground: a low, wide, still glow along the bottom.
        for i in 0..5 {
            let u = 0.1 + 0.2 * i as f32;
            light(frame.rect.lerp_inside(u, 1.0), 700.0 * s, 0.12, i);
        }
        for (b, &(top, width)) in self.scene.bars.iter().enumerate() {
            for k in 0..EMBERS {
                let r = unit(hash((b * EMBERS + k) as u64));
                let age = (t * (0.25 + 0.2 * r) + r).fract();
                let x = top.x + (r - 0.5) * width * 0.8 + 12.0 * s * (t * 2.0 + r * 9.0).sin();
                let p = Pos2::new(x, top.y - age * 160.0 * s);
                light(
                    p,
                    (24.0 + 16.0 * r) * s,
                    (age * std::f32::consts::PI).sin() * 0.9,
                    k,
                );
            }
        }
        if self.settings.runners {
            for (n, route) in self.scene.routes.iter().enumerate() {
                let speed = 220.0 * s / route.length().max(1.0);
                for k in 0..RUNNERS {
                    let head = (t * speed + k as f32 / RUNNERS as f32).fract();
                    // a short comet: the head and a fading tail behind it
                    for tail in 0..6 {
                        let f = head - tail as f32 * 0.012;
                        if f < 0.0 {
                            break;
                        }
                        let glow = 0.9 * (1.0 - tail as f32 / 6.0);
                        light(route.at(f), (56.0 - 6.0 * tail as f32) * s, glow, n + k);
                    }
                }
            }
        }
        for (n, &(c, radius)) in self.scene.rings.iter().enumerate() {
            for k in 0..SPARKS {
                let a = t * 0.4 + k as f32 / SPARKS as f32 * std::f32::consts::TAU;
                let p = c + mdeck_sdk::paint::Vec2::angled(a) * (radius + 10.0 * s);
                light(p, 28.0 * s, 0.7, n + k);
            }
        }
        out
    }
}

impl Engine for Reactive {
    fn update(&mut self, frame: &Frame, stage: &Stage) {
        // Derive the scene only when the geometry changes: the fingerprint
        // ignores sub-pixel jitter, so this is rare.
        if self.key != Some(stage.geometry_key) {
            self.scene = Scene::from_hints(stage.geometry);
            self.key = Some(stage.geometry_key);
        }
        self.settled = frame.settled();
        if self.settled {
            // A fixed moment, rotated by slide number: deterministic stills.
            self.clock = 0.7 + stage.index as f32 * 1.3;
            self.age = FADE_IN;
        } else {
            let dt = frame.dt.clamp(0.0, 0.1);
            self.clock += dt;
            self.age += dt;
        }
    }

    fn paint(&mut self, painter: &mut Painter, frame: &Frame, _stage: &Stage) {
        let blend = if frame.tokens.light {
            SpriteBlend::Normal
        } else {
            SpriteBlend::Additive
        };
        painter.sprites(&self.layer, self.sprites(frame), blend);
    }

    fn animating(&self) -> bool {
        // Honest: only while something moves. A slide of plain text on
        // this engine costs nothing once the ground has faded in.
        !self.settled && (self.age < FADE_IN || self.scene.moves(&self.settings))
    }
}

fn hash(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

fn unit(h: u64) -> f32 {
    (h >> 40) as f32 / (1u64 << 24) as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use mdeck_sdk::geometry::fingerprint;
    use mdeck_sdk::paint::Vec2;
    use mdeck_sdk::stage::Moment;
    use mdeck_sdk::tokens::Value;

    fn text(s: &str) -> Value {
        Value::String(s.into())
    }

    #[test]
    fn defaults_without_settings() {
        let s = EngineSettings::new();
        assert_eq!(Settings::read(&s), Settings::default());
        assert!(s.problems().is_empty());
    }

    #[test]
    fn typed_settings_are_read() {
        let s = EngineSettings::from_pairs([
            ("glow", Value::Number(1.5)),
            ("runners", Value::Bool(false)),
            ("palette", text("cool")),
            ("tint", text("#ff0000")),
        ]);
        let read = Settings::read(&s);
        assert_eq!(read.glow, 1.5);
        assert!(!read.runners);
        assert_eq!(read.palette, Palette::Cool);
        assert_eq!(read.tint, Some(Color::from_rgb(255, 0, 0)));
        assert!(s.problems().is_empty(), "{:?}", s.problems());
    }

    #[test]
    fn bad_settings_are_reported_with_the_line() {
        let s = EngineSettings::from_pairs([
            ("glow", Value::Number(9.0)),
            ("palette", text("neon")),
            ("sparkle", Value::Bool(true)),
        ])
        .at_line(4);
        let read = Settings::read(&s);
        assert_eq!(read.glow, 2.0, "clamped");
        assert_eq!(read.palette, Palette::Accent, "default");
        let p = s.problems();
        let messages: Vec<&str> = p.iter().map(|p| p.message.as_str()).collect();
        assert_eq!(
            messages,
            [
                "`glow` should be between 0 and 2, not 9",
                "`palette` should be one of accent, warm, cool, not `neon`",
                "unknown setting `sparkle`",
            ]
        );
        assert!(p.iter().all(|p| p.line == Some(4)));
    }

    #[test]
    fn the_definition_checks_types_without_running() {
        let s = EngineSettings::from_pairs([("runners", text("often"))]);
        assert_eq!(DEF.check_settings(&s).len(), 1);
    }

    #[test]
    fn routes_measure_evenly() {
        let r = Route::new(vec![
            Pos2::new(0.0, 0.0),
            Pos2::new(10.0, 0.0),
            Pos2::new(10.0, 30.0),
        ]);
        assert_eq!(r.length(), 40.0);
        assert_eq!(r.at(0.25), Pos2::new(10.0, 0.0));
        assert_eq!(r.at(0.5), Pos2::new(10.0, 10.0));
        assert_eq!(r.at(1.0), Pos2::new(10.0, 30.0));
    }

    fn frame_with(hints: &[Hint], frames: usize, reduced: bool) -> Reactive {
        let (tokens, settings) = (Tokens::default(), EngineSettings::new());
        let mut f = Frame::new(
            Rect::from_min_size(Pos2::ZERO, Vec2::new(1920.0, 1080.0)),
            &tokens,
            &settings,
        );
        f.reduced_motion = reduced;
        let mut stage = Stage::new(Moment::Slide);
        stage.geometry = hints;
        stage.geometry_key = fingerprint(hints);
        let mut e = Reactive::new(Settings::default());
        for _ in 0..frames {
            e.update(&f, &stage);
        }
        e
    }

    #[test]
    fn animating_is_honest() {
        let bar = [Hint::Bar(Rect::from_min_size(
            Pos2::new(100.0, 500.0),
            Vec2::new(80.0, 300.0),
        ))];
        assert!(frame_with(&[], 1, false).animating(), "fading in");
        assert!(!frame_with(&[], 120, false).animating(), "nothing moves");
        assert!(frame_with(&bar, 120, false).animating(), "embers rise");
        assert!(!frame_with(&bar, 1, true).animating(), "reduced motion");
    }

    #[test]
    fn nothing_is_drawn_inside_a_frame() {
        let frame = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(1920.0, 1080.0));
        let hints = [
            Hint::Bar(Rect::from_min_size(
                Pos2::new(900.0, 500.0),
                Vec2::new(80.0, 300.0),
            )),
            Hint::Frame(frame),
        ];
        let e = frame_with(&hints, 1, true);
        let (tokens, settings) = (Tokens::default(), EngineSettings::new());
        let f = Frame::new(frame, &tokens, &settings);
        assert!(e.sprites(&f).is_empty());
    }

    #[test]
    fn the_scene_is_derived_from_hints() {
        let hints = [
            Hint::Path(vec![Pos2::ZERO, Pos2::new(1.0, 1.0)]),
            Hint::Path(vec![Pos2::ZERO]),
            Hint::Circle {
                center: Pos2::ZERO,
                radius: 5.0,
            },
            Hint::Point(Pos2::ZERO),
        ];
        let s = Scene::from_hints(&hints);
        assert_eq!((s.routes.len(), s.rings.len(), s.bars.len()), (1, 1, 0));
    }
}
