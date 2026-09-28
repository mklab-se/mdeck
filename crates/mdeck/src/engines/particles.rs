//! The particles engine: a living field of particles under every slide. Each
//! frame it picks a scene from the stage (countdown digit, end act, story,
//! illustration, hints from renderers, or the scene inferred from the slide's
//! layout), ticks the field toward it and paints it.

use eframe::egui;

use super::stage::{CountPhase, FrameCx, Moment, Stage};
use super::{Capabilities, Engine, EngineDef};
use crate::parser::Slide;
use crate::render::illustration::Library;
use crate::render::particles::{self, Field, scenes};
use crate::render::story;

/// Seconds into the end slide when the caption fades in: after the bang has
/// faded to black.
pub const END_CAPTION_DELAY: f32 = 7.4;

pub static DEF: EngineDef = EngineDef {
    capabilities: Capabilities {
        stories: true,
        ..Capabilities::PICTURES
    },
    create: || Box::new(Particles::new()),
    end_caption_delay: END_CAPTION_DELAY,
    medium: None,
    render_slide: None,
    problems: None,
};

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
/// phase, story version, countdown phase and the hints' fingerprint.
type SceneKey = (usize, usize, Option<EndPhase>, u64, Option<CountPhase>, u64);

pub struct Particles {
    field: Option<Field>,
    /// What the current scene was built for.
    key: Option<SceneKey>,
    /// Labels of the staged story, if the current scene is one.
    labels: Vec<story::Label>,
    /// Field speed for this frame's phase.
    speed: f32,
}

impl Particles {
    pub fn new() -> Self {
        Self {
            field: None,
            key: None,
            labels: Vec::new(),
            speed: 1.0,
        }
    }
}

impl Default for Particles {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for Particles {
    fn update(&mut self, cx: &FrameCx, stage: &Stage, lib: &mut Library) {
        let rect = cx.rect;
        let needs_new_field = match &self.field {
            None => true,
            Some(f) => (f.rect().size() - rect.size()).length() > 1.0,
        };
        if needs_new_field {
            let mut f = Field::new(particles::DEFAULT_COUNT, 11);
            f.scatter(rect);
            self.field = Some(f);
            self.key = None;
        }

        let end_phase = match &stage.moment {
            Moment::End { elapsed, .. } => Some(EndPhase::at(*elapsed)),
            _ => None,
        };
        let countdown = stage.moment.count_phase();
        let key = (
            stage.index,
            stage.reveal,
            end_phase.map(|(p, _)| p),
            stage.story_version,
            countdown.map(|(p, _)| p),
            stage.hints_key,
        );
        let rect_aspect = rect.width() / rect.height();
        let seed = stage.index as u64 + 1;
        if self.key != Some(key) {
            self.labels.clear();
            let scene = match (&stage.moment, end_phase) {
                (Moment::Countdown { mask, .. }, _) => {
                    scenes::digit(mask.0.clone(), mask.1, rect_aspect)
                }
                (Moment::Burst { .. }, _) => scenes::burst(),
                (Moment::End { words, .. }, Some((phase, _))) => match phase {
                    EndPhase::Words => scenes::end_words(words.0.clone(), words.1, rect_aspect),
                    EndPhase::Dance => scenes::end_dance(),
                    EndPhase::Bang | EndPhase::Black => scenes::end_bang(),
                },
                _ => {
                    if let (Some(slide), Some(script)) = (stage.slide, stage.story)
                        && !stage.title
                    {
                        let staged = story::stage(script, slide.layout, rect_aspect, lib);
                        self.labels = staged.labels;
                        staged.scene
                    } else if let Some(fig) = &stage.figure {
                        let points = std::sync::Arc::clone(&fig.cloud.points);
                        if fig.backdrop {
                            scenes::illustration_backdrop(points, fig.place)
                        } else {
                            scenes::illustration_stage(points, fig.place)
                        }
                    } else if let Some(slide) = stage.slide
                        && !stage.hints.is_empty()
                        && uses_hints(slide)
                    {
                        scenes::from_hints(stage.hints, rect, seed)
                    } else if let Some(slide) = stage.slide {
                        scenes::for_slide(slide, seed)
                    } else {
                        scenes::constellation(seed)
                    }
                }
            };
            let field = self.field.as_mut().expect("field created above");
            field.set_scene(scene, rect, seed);
            if cx.still {
                field.settle(stage.reveal);
            }
            self.key = Some(key);
        }
        let theme = cx.theme;
        let field = self.field.as_mut().expect("field created above");
        field.set_tints([
            theme.accent,
            theme.accent_soft,
            theme.particle_light,
            theme.secondary,
            theme.particle_cool,
        ]);
        field.set_light(theme.is_light());
        if cx.still {
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
        field.tick(cx.dt * self.speed, stage.reveal);
    }

    fn paint(&mut self, ui: &egui::Ui, cx: &FrameCx, _stage: &Stage) {
        let Some(field) = self.field.as_ref() else {
            return;
        };
        field.paint(ui.painter(), cx.rect, cx.opacity, !cx.still);
        if !self.labels.is_empty() {
            story::draw_labels(
                ui.painter(),
                &self.labels,
                field,
                cx.rect,
                cx.theme,
                cx.scale,
                cx.opacity,
            );
        }
        if !cx.still {
            ui.ctx().request_repaint();
        }
    }
}

/// Layouts whose field follows the drawn content rather than a fixed scene.
fn uses_hints(slide: &Slide) -> bool {
    use crate::parser::Layout;
    matches!(
        slide.layout,
        Layout::Visualization | Layout::Diagram | Layout::Image | Layout::Gallery
    ) || slide
        .blocks
        .iter()
        .any(|b| matches!(b, crate::parser::Block::Image { .. }))
}
