//! The `aurora` engine: a few slow glows breathing under every slide.
//!
//! Made with `mdeck sdk new engine aurora`. Change anything: this is a
//! starting point that already keeps the engine contract (scales with the
//! slide, takes its colours from the theme, settles for stills and reduced
//! motion). The SDK guides are at
//! <https://github.com/mklab-se/mdeck/tree/main/docs/sdk>.

use mdeck_sdk::engine::{Capabilities, Engine, EngineDef, Needs, SettingKind, SettingSpec};
use mdeck_sdk::paint::{Painter, Sprite, SpriteBlend, SpriteLayer, mix};
use mdeck_sdk::registry::{Registry, RegistryError};
use mdeck_sdk::stage::{Frame, Stage};
use mdeck_sdk::tokens::EngineSettings;

/// The settings this engine reads from the theme's `engine:` block. mdeck
/// uses this list to check themes and to show the settings to authors.
pub const SETTINGS: &[SettingSpec] = &[
    SettingSpec::new(
        "speed",
        SettingKind::Number,
        "How fast the curtain moves, from 0 to 3 (default 1).",
    ),
    SettingSpec::new(
        "height",
        SettingKind::Number,
        "How tall the curtain is, from 0.5 to 2 (default 1).",
    ),
];

/// The settings, read once when the engine starts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    pub speed: f32,
    pub height: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            speed: 1.0,
            height: 1.0,
        }
    }
}

impl Settings {
    /// Read the theme's values. A bad value is reported (it shows up in
    /// `mdeck --check`) and replaced by something sensible: never fail.
    pub fn read(s: &EngineSettings) -> Self {
        let d = Settings::default();
        let mut speed = s.f32_or("speed", d.speed);
        if !(0.0..=3.0).contains(&speed) {
            s.report("speed", format!("should be between 0 and 3, not {speed}"));
            speed = speed.clamp(0.0, 3.0);
        }
        let mut height = s.f32_or("height", d.height);
        if !(0.5..=2.0).contains(&height) {
            s.report(
                "height",
                format!("should be between 0.5 and 2, not {height}"),
            );
            height = height.clamp(0.5, 2.0);
        }
        Settings { speed, height }
    }
}

/// The engine as mdeck sees it. A theme selects it with `engine: aurora`.
pub static DEF: EngineDef = EngineDef::new(
    "aurora",
    "Northern lights ripple across the top of every slide.",
    // Makes a running engine; it may read its settings here.
    |settings: &EngineSettings| {
        Box::new(Aurora {
            settings: Settings::read(settings),
            ..Aurora::default()
        })
    },
)
// What the core must do for this engine. NONE: it only decorates. Use
// `Capabilities::NONE.with_picture()` (and `with_countdown`, `with_ending`)
// to receive the slide's picture, draw the countdown, or play the end act
// (see the SDK tutorial).
.with_capabilities(Capabilities::NONE)
// The settings this engine reads from the theme's `engine:` block.
.with_settings(SETTINGS)
// What the engine needs from the theme (`Needs::NONE.with_page()` for a sheet).
.with_needs(Needs::NONE)
// Seconds into the end slide before the "made with mdeck" caption.
.with_ending_caption_delay(1.0);
// Only board engines (which draw whole slides) add `.with_board(&SET)`.

/// The entry point `mdeck build --with` calls: register what this crate brings.
pub fn register(r: &mut Registry) -> Result<(), RegistryError> {
    r.engine(&DEF)?;
    r.theme("aurora", include_str!("../theme.yaml"))
}

/// The running engine. One is made per presentation (and per export).
#[derive(Default)]
pub struct Aurora {
    /// Seconds of motion so far.
    clock: f32,
    /// Whether this frame is a still or under reduced motion.
    settled: bool,
    /// GPU state for the glow sprites, kept between frames.
    layer: SpriteLayer,
    /// The theme's settings.
    settings: Settings,
}

impl Engine for Aurora {
    /// Called first, every frame: bring the state up to date. Never paint here.
    fn update(&mut self, frame: &Frame, stage: &Stage) {
        self.settled = frame.settled();
        if self.settled {
            // Stills (export) and reduced motion: a settled pose that depends
            // only on the slide number, never on the wall clock.
            self.clock = stage.index as f32 * 2.0;
        } else {
            // Clamp the step so a stall never makes the motion jump.
            self.clock += frame.dt.clamp(0.0, 0.1) * self.settings.speed;
        }
    }

    /// Called second, every frame: draw under the slide's content.
    fn paint(&mut self, painter: &mut Painter, frame: &Frame, _stage: &Stage) {
        let tokens = frame.tokens;
        let lights = 120;
        let rays = 24;
        let mut sprites = Vec::with_capacity(lights * rays);
        for i in 0..lights {
            let u = -0.05 + 1.1 * i as f32 / (lights - 1) as f32;
            let base = 0.25 + 0.06 * (u * 7.0 + self.clock * 0.4).sin();
            // How tall the curtain is here: it ripples slowly along the ribbon.
            let height =
                (0.12 + 0.16 * (u * 13.0 + self.clock * 0.7).sin().abs()) * self.settings.height;
            for r in 0..rays {
                // t = 0 at the bright lower edge, 1 at the faint top of the ray.
                let t = r as f32 / (rays - 1) as f32;
                let v = base - height * t;
                // Accent at the edge, fading into the theme's cool colour.
                let c = mix(tokens.accent, tokens.particle_cool, t).to_f32();
                let alpha = 0.045 * (1.0 - t) * frame.opacity;
                sprites.push(Sprite {
                    center: frame.rect.lerp_inside(u, v),
                    size: 110.0 * frame.scale,
                    rgba: [c[0], c[1], c[2], alpha],
                });
            }
        }
        // Added light disappears on a light page: use ink there.
        let blend = if tokens.light {
            SpriteBlend::Normal
        } else {
            SpriteBlend::Additive
        };
        painter.sprites(&self.layer, sprites, blend);
    }

    /// Whether another frame is needed soon. Return false once nothing moves,
    /// so mdeck stops repainting.
    fn animating(&self) -> bool {
        !self.settled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mdeck_sdk::paint::Rect;
    use mdeck_sdk::stage::Moment;
    use mdeck_sdk::tokens::Tokens;

    #[test]
    fn registers_engine_and_theme() {
        let mut r = Registry::new();
        r.set_origin("aurora");
        register(&mut r).unwrap();
        assert!(r.engine_def("aurora").is_some());
        assert!(r.theme_source("aurora").is_some());
    }

    #[test]
    fn settles_under_reduced_motion() {
        let (tokens, settings) = (Tokens::default(), EngineSettings::new());
        let mut frame = Frame::new(Rect::ZERO, &tokens, &settings);
        frame.reduced_motion = true;
        let mut e = Aurora::default();
        e.update(&frame, &Stage::new(Moment::Slide));
        assert!(!e.animating());
    }

    #[test]
    fn bad_settings_are_reported_and_clamped() {
        use mdeck_sdk::tokens::Value;
        let theme = EngineSettings::from_pairs([
            ("speed", Value::Number(9.0)),
            ("height", Value::Number(1.5)),
        ]);
        let s = Settings::read(&theme);
        assert_eq!(
            s,
            Settings {
                speed: 3.0,
                height: 1.5
            }
        );
        let problems = theme.problems();
        assert_eq!(problems.len(), 1);
        assert!(problems[0].message.contains("between 0 and 3"));
    }
}
