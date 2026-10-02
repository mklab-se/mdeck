//! # Tutorial step 1: Ambience
//!
//! A calm animated ground: a dozen soft lights drift slowly under every
//! slide. The engine shows the whole frame lifecycle and the contract every
//! engine keeps:
//!
//! - `update` advances the clock, then `paint` draws (the host calls both
//!   every frame, under the slide's content);
//! - every size is multiplied by `frame.scale`, so the ground looks the same
//!   at any resolution;
//! - every colour comes from the theme's tokens;
//! - under reduced motion, and in stills, the engine shows a settled pose
//!   that depends only on the slide number, so exports are reproducible.
//!
//! The walkthrough is `docs/sdk/tutorial-1-ambience.md`.

use mdeck_sdk::engine::{Capabilities, Engine, EngineDef, Needs};
use mdeck_sdk::paint::{Color, Painter, Sprite, SpriteBlend, SpriteLayer, smoothstep};
use mdeck_sdk::registry::{Registry, RegistryError};
use mdeck_sdk::stage::{Frame, Stage};
use mdeck_sdk::tokens::{EngineSettings, Tokens};

/// The engine as mdeck registers it: `engine: ambience` in a theme.
pub static DEF: EngineDef = EngineDef {
    name: "ambience",
    summary: "Soft lights drift slowly under every slide.",
    // It only decorates: no pictures, no countdown, no end act.
    capabilities: Capabilities::NONE,
    // It reads no settings from the theme's `engine:` block.
    settings: &[],
    needs: Needs { page: false },
    ending_caption_delay: 1.0,
    create,
    board: None,
};

/// The showcase theme: a deep blue night for the soft lights.
pub const THEME: &str = include_str!("../themes/dusk.yaml");

/// Register the engine and its showcase theme.
pub fn register(r: &mut Registry) -> Result<(), RegistryError> {
    r.engine(&DEF)?;
    r.theme("dusk", THEME)
}

fn create(_settings: &EngineSettings) -> Box<dyn Engine> {
    Box::new(Ambience::new())
}

/// How many lights drift under a slide.
pub const LIGHTS: usize = 12;

/// Seconds the ground takes to fade in when the deck opens.
const FADE_IN: f32 = 1.5;

/// Seconds of motion between one slide's still and the next: stills sit
/// at `index * STILL_SPACING` on the clock, so neighbours differ.
const STILL_SPACING: f32 = 7.0;

/// The running engine.
pub struct Ambience {
    lights: Vec<Light>,
    /// Seconds of motion so far (the clock every light reads).
    clock: f32,
    /// Seconds since the engine started, for the fade-in.
    age: f32,
    /// Whether the last frame was a still or under reduced motion.
    settled: bool,
    /// The GPU state behind the glow sprites, kept from frame to frame.
    layer: SpriteLayer,
}

/// One soft light: where it circles, how far, how fast, how big.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Light {
    /// Centre of its orbit in slide fractions (0..1).
    pub home: [f32; 2],
    /// Orbit radii in slide fractions.
    pub reach: [f32; 2],
    /// Radians per second.
    pub speed: f32,
    /// Starting angle.
    pub phase: f32,
    /// Diameter in points on a 1920 by 1080 slide.
    pub size: f32,
    /// Brightness, 0..1.
    pub glow: f32,
    /// Which token colour: 0 accent, 1 secondary, 2 the cool particle light.
    pub hue: u8,
}

impl Light {
    /// Where the light is at `t` seconds, in slide fractions.
    pub fn at(&self, t: f32) -> [f32; 2] {
        let a = self.phase + self.speed * t;
        [
            self.home[0] + self.reach[0] * a.sin(),
            self.home[1] + self.reach[1] * (a * 0.7 + 1.3).cos(),
        ]
    }

    /// The light's colour from the theme.
    pub fn color(&self, tokens: &Tokens) -> Color {
        match self.hue {
            0 => tokens.accent,
            1 => tokens.secondary,
            _ => tokens.particle_cool,
        }
    }
}

impl Default for Ambience {
    fn default() -> Self {
        Self::new()
    }
}

impl Ambience {
    /// A new engine. The lights are fixed for the whole deck, so nothing
    /// jumps between slides; only the clock moves.
    pub fn new() -> Self {
        Self {
            lights: (0..LIGHTS).map(light).collect(),
            clock: 0.0,
            age: 0.0,
            settled: false,
            layer: SpriteLayer::new(),
        }
    }

    /// The lights.
    pub fn lights(&self) -> &[Light] {
        &self.lights
    }

    /// The clock the lights read.
    pub fn clock(&self) -> f32 {
        self.clock
    }
}

impl Engine for Ambience {
    fn update(&mut self, frame: &Frame, stage: &Stage) {
        self.settled = frame.settled();
        if self.settled {
            // A still (export) or reduced motion: no motion at all, and a
            // pose that depends only on the slide, never on the wall clock.
            self.clock = stage.index as f32 * STILL_SPACING;
            self.age = FADE_IN;
        } else {
            // Clamp dt: after a stall (a laptop waking up) the lights must
            // not leap across the slide.
            let dt = frame.dt.clamp(0.0, 0.1);
            self.clock += dt;
            self.age += dt;
        }
    }

    fn paint(&mut self, painter: &mut Painter, frame: &Frame, _stage: &Stage) {
        let fade = smoothstep(0.0, FADE_IN, self.age) * frame.opacity;
        if fade <= 0.0 {
            return;
        }
        let tokens = frame.tokens;
        // Added light vanishes on a light page: draw ink there instead,
        // fainter, so the ground stays a ground.
        let (blend, strength) = if tokens.light {
            (SpriteBlend::Normal, 0.35)
        } else {
            (SpriteBlend::Additive, 1.0)
        };
        let sprites = self
            .lights
            .iter()
            .map(|l| {
                let [u, v] = l.at(self.clock);
                let c = l.color(tokens).to_f32();
                Sprite {
                    center: frame.rect.lerp_inside(u, v),
                    size: l.size * frame.scale,
                    rgba: [c[0], c[1], c[2], l.glow * strength * fade],
                }
            })
            .collect();
        painter.sprites(&self.layer, sprites, blend);
    }

    fn animating(&self) -> bool {
        // A settled ground never needs another frame.
        !self.settled
    }
}

/// Light `i`, derived from its number only: the same deck always gets the
/// same lights.
fn light(i: usize) -> Light {
    let r = |k: u64| unit(hash(i as u64 * 16 + k));
    Light {
        home: [0.15 + 0.70 * r(0), 0.12 + 0.76 * r(1)],
        reach: [0.04 + 0.08 * r(2), 0.03 + 0.06 * r(3)],
        speed: 0.05 + 0.10 * r(4),
        phase: std::f32::consts::TAU * r(5),
        size: 260.0 + 420.0 * r(6),
        glow: 0.10 + 0.16 * r(7),
        hue: (r(8) * 3.0) as u8,
    }
}

/// A 64-bit mix (splitmix64): deterministic pseudo-randomness without a
/// random number generator or the wall clock.
fn hash(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

/// A hash as a number in 0..1.
fn unit(h: u64) -> f32 {
    (h >> 40) as f32 / (1u64 << 24) as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use mdeck_sdk::stage::Moment;
    use mdeck_sdk::testing::{Headless, mean_difference};

    fn still(index: usize, w: usize, h: usize) -> mdeck_sdk::paint::ImageData {
        let mut headless = Headless::new(w, h);
        let (tokens, settings) = (Tokens::default(), EngineSettings::new());
        let mut frame = Frame::new(headless.rect(), &tokens, &settings);
        frame.still = true;
        let mut stage = Stage::new(Moment::Slide);
        stage.index = index;
        headless.render_engine(&mut Ambience::new(), &frame, &stage)
    }

    #[test]
    fn registers_engine_and_theme() {
        let mut r = Registry::new();
        r.set_origin("engine-ambience");
        register(&mut r).unwrap();
        assert!(r.engine_def("ambience").is_some());
        assert!(r.theme_source("dusk").is_some());
    }

    #[test]
    fn lights_stay_on_the_slide() {
        for l in Ambience::new().lights() {
            for t in [0.0, 10.0, 100.0, 1000.0] {
                let [u, v] = l.at(t);
                assert!(
                    (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v),
                    "{u},{v}"
                );
            }
        }
    }

    #[test]
    fn stills_are_deterministic_and_differ_by_slide() {
        let a = still(3, 192, 108);
        let b = still(3, 192, 108);
        assert_eq!(a, b);
        let c = still(4, 192, 108);
        assert!(mean_difference(&a, &c).unwrap() > 0.1);
    }

    #[test]
    fn reduced_motion_settles_and_stops_asking_for_frames() {
        let (tokens, settings) = (Tokens::default(), EngineSettings::new());
        let mut frame = Frame::new(mdeck_sdk::paint::Rect::ZERO, &tokens, &settings);
        let stage = Stage::new(Moment::Slide);
        let mut e = Ambience::new();
        e.update(&frame, &stage);
        assert!(e.animating());
        frame.reduced_motion = true;
        e.update(&frame, &stage);
        e.update(&frame, &stage);
        assert!(!e.animating());
        assert_eq!(e.clock(), 0.0, "no motion under reduced motion");
    }

    #[test]
    fn the_clock_survives_a_stall() {
        let (tokens, settings) = (Tokens::default(), EngineSettings::new());
        let mut frame = Frame::new(mdeck_sdk::paint::Rect::ZERO, &tokens, &settings);
        frame.dt = 30.0;
        let mut e = Ambience::new();
        e.update(&frame, &Stage::new(Moment::Slide));
        assert!(e.clock() <= 0.1);
    }

    #[test]
    fn the_ground_scales_with_the_slide() {
        // Half the resolution, half the light: the small render matches the
        // large one sampled at every other pixel.
        let small = still(0, 192, 108);
        let large = still(0, 384, 216);
        let mut sampled = small.clone();
        for y in 0..108 {
            for x in 0..192 {
                sampled.pixels[y * 192 + x] = large.get(x * 2, y * 2).unwrap();
            }
        }
        assert!(mean_difference(&small, &sampled).unwrap() < 2.0);
    }
}
