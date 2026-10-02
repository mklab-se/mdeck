//! The particles engine: a living field of particles under every slide. Each
//! frame it picks a scene from the stage (countdown digit, end act, the
//! slide's picture, the geometry its visuals drew, or the scene inferred
//! from the slide's design), ticks the field toward it and paints it.
//!
//! A fixed pool of glowing particles morphs from scene to scene: each scene
//! assigns every particle a *home* (a point it drifts around), and particles
//! ease toward their homes, so a slide change is a migration rather than a
//! cut. Scenes are declarative data ([`Scene`]) built from slide content in
//! [`scenes`]; the field itself knows nothing about markdown. The field draws
//! hairline links between neighbours, then the glow as additive sprites
//! (with wakes while live, ink-like on a light page).

pub mod field;
pub mod scene;
pub mod scenes;

pub use crate::engines::rng::Rng;
pub use field::Field;
pub use scene::{DEFAULT_TINTS, Drift, Group, Home, Palette, Scene, Tint};

use std::sync::Arc;

use mdeck_sdk::content::{Block, Slide};
use mdeck_sdk::engine::{Capabilities, Engine, EngineDef, Needs};
use mdeck_sdk::paint::Painter;
use mdeck_sdk::stage::{Frame, Moment, PictureSource, Stage};

/// Particles in the presentation window (the site uses 520 on desktop).
pub const DEFAULT_COUNT: usize = 900;

/// Seconds into the end slide when the caption fades in: after the bang has
/// faded to black.
pub const END_CAPTION_DELAY: f32 = 7.4;

pub static DEF: EngineDef = EngineDef {
    name: "particles",
    summary: "A living field of glowing particles that takes the shape of each slide.",
    capabilities: Capabilities {
        picture: true,
        countdown: true,
        ending: true,
        board: false,
        transition: false,
        medium: None,
    },
    settings: &[],
    needs: Needs { page: false },
    ending_caption_delay: END_CAPTION_DELAY,
    create: |_| Box::new(Particles::new()),
    board: None,
};

/// Where the opening countdown is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CountPhase {
    /// Showing this digit.
    Digit(u8),
    /// The last digit leaves and the first slide arrives.
    Burst,
}

/// The countdown phase of a moment and its progress, if it is one.
fn count_phase(moment: &Moment) -> Option<(CountPhase, f32)> {
    match moment {
        Moment::Countdown {
            digit, progress, ..
        } => Some((CountPhase::Digit(*digit), *progress)),
        Moment::Burst { progress } => Some((CountPhase::Burst, *progress)),
        _ => None,
    }
}

/// The end slide's choreography: the words, a swirl, a bang, then black.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum EndPhase {
    Words,
    Dance,
    Bang,
    Black,
}

const END_WORDS: f32 = 3.2;
const END_DANCE: f32 = 2.8;
const END_BANG: f32 = 1.2;

impl EndPhase {
    fn at(elapsed: f32) -> (EndPhase, f32) {
        if elapsed < END_WORDS {
            (EndPhase::Words, elapsed / END_WORDS)
        } else if elapsed < END_WORDS + END_DANCE {
            (EndPhase::Dance, (elapsed - END_WORDS) / END_DANCE)
        } else if elapsed < END_WORDS + END_DANCE + END_BANG {
            (EndPhase::Bang, (elapsed - END_WORDS - END_DANCE) / END_BANG)
        } else {
            (EndPhase::Black, 1.0)
        }
    }
}

/// Everything a scene choice depends on: slide index, reveal step, end
/// phase, countdown phase and the geometry's fingerprint.
type SceneKey = (usize, usize, Option<EndPhase>, Option<CountPhase>, u64);

pub struct Particles {
    field: Option<Field>,
    /// What the current scene was built for.
    key: Option<SceneKey>,
    /// Field speed for this frame's phase.
    speed: f32,
}

impl Particles {
    pub fn new() -> Self {
        Self {
            field: None,
            key: None,
            speed: 1.0,
        }
    }
}

impl Default for Particles {
    fn default() -> Self {
        Self::new()
    }
}

/// The scene for the stage's moment and slide.
fn scene_for(stage: &Stage, frame: &Frame, end_phase: Option<EndPhase>, seed: u64) -> Scene {
    let rect = frame.rect;
    let rect_aspect = rect.width() / rect.height();
    match (&stage.moment, end_phase) {
        (Moment::Countdown { mask, .. }, _) => {
            scenes::digit(Arc::clone(&mask.points), mask.aspect, rect_aspect)
        }
        (Moment::Burst { .. }, _) => scenes::burst(),
        (Moment::End { words, .. }, Some(phase)) => match phase {
            EndPhase::Words => {
                scenes::end_words(Arc::clone(&words.points), words.aspect, rect_aspect)
            }
            EndPhase::Dance => scenes::end_dance(),
            EndPhase::Bang | EndPhase::Black => scenes::end_bang(),
        },
        _ => {
            // Only point clouds are drawn in particles; an image or an
            // artwork picture leaves the slide's own scene.
            let cloud = stage.picture.as_ref().and_then(|p| match &p.source {
                PictureSource::Cloud(cloud) => Some((cloud, p.backdrop, p.place)),
                _ => None,
            });
            if let Some((cloud, backdrop, place)) = cloud {
                let points = Arc::new(cloud.points.clone());
                if backdrop {
                    scenes::illustration_backdrop(points, place)
                } else {
                    scenes::illustration_stage(points, place)
                }
            } else if let Some(slide) = stage.slide
                && !stage.geometry.is_empty()
                && uses_geometry(slide)
            {
                scenes::from_hints(stage.geometry, rect, seed)
            } else if let Some(slide) = stage.slide {
                scenes::for_slide(slide, seed, stage.title)
            } else {
                scenes::constellation(seed)
            }
        }
    }
}

impl Engine for Particles {
    fn update(&mut self, frame: &Frame, stage: &Stage) {
        let rect = frame.rect;
        let needs_new_field = match &self.field {
            None => true,
            Some(f) => (f.rect().size() - rect.size()).length() > 1.0,
        };
        if needs_new_field {
            let mut f = Field::new(DEFAULT_COUNT, 11);
            f.scatter(rect);
            self.field = Some(f);
            self.key = None;
        }

        let end_phase = match &stage.moment {
            Moment::End { elapsed, .. } => Some(EndPhase::at(*elapsed)),
            _ => None,
        };
        let countdown = count_phase(&stage.moment);
        let key = (
            stage.index,
            stage.step,
            end_phase.map(|(p, _)| p),
            countdown.map(|(p, _)| p),
            stage.geometry_key,
        );
        let seed = stage.index as u64 + 1;
        let still = frame.settled();
        if self.key != Some(key) {
            let scene = scene_for(stage, frame, end_phase.map(|(p, _)| p), seed);
            let field = self.field.as_mut().expect("field created above");
            field.set_scene(scene, rect, seed);
            if still {
                field.settle(stage.step);
            }
            self.key = Some(key);
        }
        let t = frame.tokens;
        let field = self.field.as_mut().expect("field created above");
        field.set_tints([
            t.accent,
            t.accent_soft,
            t.particle_light,
            t.secondary,
            t.particle_cool,
        ]);
        field.set_light(t.light);
        if still {
            return;
        }
        // Digits assemble briskly, the burst accelerates outward, slides
        // take their time.
        self.speed = match (countdown, end_phase) {
            (Some((CountPhase::Digit(_), _)), _) => 1.8,
            (Some((CountPhase::Burst, progress)), _) => 1.0 + 3.0 * progress,
            (None, Some((EndPhase::Words, _))) => 1.6,
            (None, Some((EndPhase::Dance, _))) => 1.3,
            (None, Some((EndPhase::Bang, progress))) => 1.2 + 3.0 * progress,
            (None, Some((EndPhase::Black, _))) => 2.0,
            (None, None) => 1.0,
        };
        field.tick(frame.dt * self.speed, stage.step);
    }

    fn paint(&mut self, painter: &mut Painter, frame: &Frame, _stage: &Stage) {
        let Some(field) = self.field.as_ref() else {
            return;
        };
        field.paint(painter, frame.rect, frame.opacity, !frame.settled());
    }
}

/// Designs whose field follows the drawn content rather than a fixed scene.
fn uses_geometry(slide: &Slide) -> bool {
    matches!(slide.design.as_str(), "visual" | "media" | "gallery")
        || slide
            .blocks
            .iter()
            .any(|b| matches!(b, Block::Image { .. }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mdeck_sdk::tokens::{EngineSettings, Tokens};

    #[test]
    fn end_phases_follow_the_clock() {
        assert_eq!(EndPhase::at(0.0).0, EndPhase::Words);
        assert_eq!(EndPhase::at(4.0).0, EndPhase::Dance);
        assert_eq!(EndPhase::at(6.5).0, EndPhase::Bang);
        assert_eq!(EndPhase::at(END_CAPTION_DELAY).0, EndPhase::Black);
    }

    #[test]
    fn charts_and_images_follow_their_geometry() {
        let mut s = Slide {
            design: "visual".into(),
            ..Default::default()
        };
        assert!(uses_geometry(&s));
        s.design = "points".into();
        assert!(!uses_geometry(&s));
        s.blocks.push(Block::Image {
            alt: String::new(),
            path: "a.png".into(),
            directives: Default::default(),
        });
        assert!(uses_geometry(&s));
    }

    /// A still settles the field at once: the first frame is already lit.
    #[test]
    fn a_still_frame_settles_the_field() {
        let tokens = Tokens::default();
        let settings = EngineSettings::new();
        let rect = mdeck_sdk::paint::Rect::from_min_size(
            mdeck_sdk::paint::Pos2::ZERO,
            mdeck_sdk::paint::Vec2::new(1920.0, 1080.0),
        );
        let mut frame = Frame::new(rect, &tokens, &settings);
        frame.still = true;
        let slide = Slide {
            design: "title".into(),
            ..Default::default()
        };
        let mut stage = Stage::new(Moment::Slide);
        stage.slide = Some(&slide);
        let mut engine = Particles::new();
        engine.update(&frame, &stage);
        assert!(engine.key.is_some());
        assert!(engine.field.is_some());
    }
}
