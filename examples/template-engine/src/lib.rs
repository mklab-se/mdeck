//! The `template-engine` engine: a few slow glows breathing under every slide.
//!
//! Made with `mdeck sdk new engine template-engine`. Change anything: this is a
//! starting point that already keeps the engine contract (scales with the
//! slide, takes its colours from the theme, settles for stills and reduced
//! motion). The SDK guides are at
//! <https://github.com/mklab-se/mdeck/tree/main/docs/sdk>.

use mdeck_sdk::engine::{Capabilities, Engine, EngineDef, Needs};
use mdeck_sdk::paint::{Painter, Sprite, SpriteBlend, SpriteLayer};
use mdeck_sdk::registry::{Registry, RegistryError};
use mdeck_sdk::stage::{Frame, Stage};
use mdeck_sdk::tokens::EngineSettings;

/// The engine as mdeck sees it. A theme selects it with `engine: template-engine`.
pub static DEF: EngineDef = EngineDef::new(
    "template-engine",
    "A few slow glows breathing under every slide.",
    // Makes a running engine; it may read its settings here.
    |_settings: &EngineSettings| Box::new(Glow::default()),
)
// What the core must do for this engine. NONE: it only decorates. Use
// `Capabilities::NONE.with_picture()` (and `with_countdown`, `with_ending`)
// to receive the slide's picture, draw the countdown, or play the end act
// (see the SDK tutorial).
.with_capabilities(Capabilities::NONE)
// The settings this engine reads from the theme's `engine:` block.
.with_settings(&[])
// What the engine needs from the theme (`Needs::NONE.with_page()` for a sheet).
.with_needs(Needs::NONE)
// Seconds into the end slide before the "made with mdeck" caption.
.with_ending_caption_delay(1.0);
// Only board engines (which draw whole slides) add `.with_board(&SET)`.

/// The entry point `mdeck build --with` calls: register what this crate brings.
pub fn register(r: &mut Registry) -> Result<(), RegistryError> {
    r.engine(&DEF)?;
    r.theme("template-engine", include_str!("../theme.yaml"))
}

/// The running engine. One is made per presentation (and per export).
#[derive(Default)]
pub struct Glow {
    /// Seconds of motion so far.
    clock: f32,
    /// Whether this frame is a still or under reduced motion.
    settled: bool,
    /// GPU state for the glow sprites, kept between frames.
    layer: SpriteLayer,
}

impl Engine for Glow {
    /// Called first, every frame: bring the state up to date. Never paint here.
    fn update(&mut self, frame: &Frame, stage: &Stage) {
        self.settled = frame.settled();
        if self.settled {
            // Stills (export) and reduced motion: a settled pose that depends
            // only on the slide number, never on the wall clock.
            self.clock = stage.index as f32 * 2.0;
        } else {
            // Clamp the step so a stall never makes the motion jump.
            self.clock += frame.dt.clamp(0.0, 0.1);
        }
    }

    /// Called second, every frame: draw under the slide's content.
    fn paint(&mut self, painter: &mut Painter, frame: &Frame, _stage: &Stage) {
        let tokens = frame.tokens;
        let colors = [tokens.accent, tokens.secondary, tokens.particle_cool];
        let sprites = (0..3)
            .map(|i| {
                let k = i as f32;
                // Positions in slide fractions, sizes in 1920x1080 points
                // times frame.scale: the same look at any resolution.
                let u = 0.25 + 0.25 * k + 0.03 * (self.clock * 0.3 + k).sin();
                let v = 0.6 + 0.05 * (self.clock * 0.2 + k * 2.0).cos();
                let breath = 0.75 + 0.25 * (self.clock * 0.5 + k).sin();
                let c = colors[i].to_f32();
                Sprite {
                    center: frame.rect.lerp_inside(u, v),
                    size: 640.0 * frame.scale,
                    rgba: [c[0], c[1], c[2], 0.22 * breath * frame.opacity],
                }
            })
            .collect();
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
        r.set_origin("template-engine");
        register(&mut r).unwrap();
        assert!(r.engine_def("template-engine").is_some());
        assert!(r.theme_source("template-engine").is_some());
    }

    #[test]
    fn settles_under_reduced_motion() {
        let (tokens, settings) = (Tokens::default(), EngineSettings::new());
        let mut frame = Frame::new(Rect::ZERO, &tokens, &settings);
        frame.reduced_motion = true;
        let mut e = Glow::default();
        e.update(&frame, &Stage::new(Moment::Slide));
        assert!(!e.animating());
    }
}
