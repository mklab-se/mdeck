//! The engine host: the engine-neutral half of drawing an engine. It keeps
//! the clock, follows the geometry renderers publish, times the end slide,
//! draws the countdown digits and end words as masks, resolves the slide's
//! figure, and hands all of it to the engine as a [`Stage`].

use std::time::Instant;

use eframe::egui;

use super::masks::{glyph_mask, text_mask, trim_flag};
use super::stage::{Art, CountPhase, Figure, FrameCx, Mask, Moment, Stage, figure_box};
use super::{Engine, EngineKind};
use crate::parser::Slide;
use crate::render::hints::{self, Hint};
use crate::render::illustration::Library;
use crate::render::story::Script;
use crate::theme::Theme;

/// One frame's worth of input from the presentation window or the export.
#[derive(Clone, Copy)]
pub struct Shot<'a> {
    pub rect: egui::Rect,
    /// The slide to show (the target during a transition); `None` on the end slide.
    pub slide: Option<&'a Slide>,
    pub story: Option<&'a Script>,
    pub story_version: u64,
    /// The slide's generated picture, when it has one and it is loaded.
    pub art: Option<&'a std::sync::Arc<crate::render::art::prepare::Prepared>>,
    pub index: usize,
    pub reveal: usize,
    /// On the end slide.
    pub end: bool,
    pub countdown: Option<(CountPhase, f32)>,
    pub theme: &'a Theme,
    pub scale: f32,
    pub opacity: f32,
    /// Export: settle at once.
    pub still: bool,
    pub deck_title: Option<&'a str>,
    /// Slides in the deck.
    pub count: usize,
}

/// One step of the engine's clock: seconds since the last, seconds into
/// the end slide, whether to settle at once (export) and whether to paint
/// (a rehearsal paints only its last step).
#[derive(Clone, Copy)]
struct Tick {
    dt: f32,
    end_elapsed: f32,
    still: bool,
    paint: bool,
}

pub struct Host {
    kind: EngineKind,
    engine: Box<dyn Engine>,
    /// When the end slide was entered, for its choreography.
    end_started: Option<Instant>,
    /// Geometry the current slide's renderers published, and its fingerprint.
    hints: Vec<Hint>,
    hints_key: u64,
    /// Slide shown last frame (hints belong to one slide).
    last_index: Option<usize>,
    /// "THE END" as a mask, rasterised once per display face.
    end_words: Option<Mask>,
    /// Glyph masks for the countdown digits.
    digits: Vec<(u8, Mask)>,
    /// The display face the masks were drawn in.
    mask_face: Option<egui::FontFamily>,
    last_tick: Option<Instant>,
}

impl Host {
    pub fn new(kind: EngineKind) -> Self {
        Self {
            kind,
            engine: kind.create(),
            end_started: None,
            hints: Vec::new(),
            hints_key: 0,
            last_index: None,
            end_words: None,
            digits: Vec::new(),
            mask_face: None,
            last_tick: None,
        }
    }

    /// Seconds since the end slide was entered (0 when not on it).
    pub fn end_elapsed(&self) -> f32 {
        self.end_started
            .map(|t| t.elapsed().as_secs_f32())
            .unwrap_or(0.0)
    }

    /// Advance the engine one frame and paint it into `shot.rect`. A theme
    /// on another engine swaps the runtime first.
    pub fn frame(&mut self, ui: &egui::Ui, shot: Shot, lib: &mut Library) {
        if shot.theme.engine != self.kind {
            *self = Host::new(shot.theme.engine);
        }
        if !self.kind.paints() {
            return;
        }
        let now = Instant::now();
        let dt = self
            .last_tick
            .map(|t| now.duration_since(t).as_secs_f32())
            .unwrap_or(1.0 / 60.0);
        self.last_tick = Some(now);
        // The end slide runs its own clock from the moment it is entered.
        let end_elapsed = if shot.end {
            let started = *self.end_started.get_or_insert(now);
            now.duration_since(started).as_secs_f32()
        } else {
            self.end_started = None;
            0.0
        };
        let tick = Tick {
            dt,
            end_elapsed,
            still: shot.still,
            paint: true,
        };
        self.step(ui, &shot, lib, tick);
    }

    /// Run the engine from a cold start through `seconds` of simulated time
    /// at 60 frames a second, then paint that frame: a still of the motion
    /// (`MDECK_EXPORT_AT`, for looking at animations in export). A burst
    /// runs its progress over the rehearsal.
    pub fn rehearse(&mut self, ui: &egui::Ui, shot: Shot, lib: &mut Library, seconds: f32) {
        *self = Host::new(shot.theme.engine);
        if !self.kind.paints() {
            return;
        }
        let dt = 1.0 / 60.0;
        let steps = (seconds.max(0.0) / dt).round() as usize;
        for k in 0..=steps {
            let t = k as f32 * dt;
            let mut s = shot;
            if let Some((CountPhase::Burst, _)) = s.countdown {
                s.countdown = Some((CountPhase::Burst, (t / 1.2).min(1.0)));
            }
            let tick = Tick {
                dt,
                end_elapsed: t,
                still: false,
                paint: k == steps,
            };
            self.step(ui, &s, lib, tick);
        }
    }

    fn step(&mut self, ui: &egui::Ui, shot: &Shot, lib: &mut Library, tick: Tick) {
        let caps = self.kind.capabilities();
        self.follow_hints(ui, shot.index);
        let moment = self.moment(ui, shot, tick.end_elapsed);

        let rect_aspect = shot.rect.width() / shot.rect.height();
        let title = shot
            .slide
            .is_some_and(|s| crate::render::ember::is_title(s, shot.index));
        let figure = if caps.illustrations {
            shot.slide.and_then(|slide| {
                let cloud = illustration_for(slide, lib)?;
                let place = figure_box(cloud.aspect, slide.layout, rect_aspect, title);
                Some(Figure {
                    cloud,
                    backdrop: title,
                    place,
                })
            })
        } else {
            None
        };
        let art = if caps.art {
            shot.slide.zip(shot.art).map(|(slide, picture)| Art {
                place: figure_box(picture.aspect(), slide.layout, rect_aspect, title),
                picture: picture.clone(),
                backdrop: title,
            })
        } else {
            None
        };
        let stage = Stage {
            moment,
            index: shot.index,
            reveal: shot.reveal,
            slide: shot.slide,
            title,
            story: if caps.stories { shot.story } else { None },
            story_version: shot.story_version,
            figure,
            art,
            hints: &self.hints,
            hints_key: self.hints_key,
            deck_title: shot.deck_title,
            count: shot.count,
        };
        let cx = FrameCx {
            rect: shot.rect,
            scale: shot.scale,
            opacity: shot.opacity,
            dt: tick.dt,
            still: tick.still,
            theme: shot.theme,
        };
        self.engine.prepare(ui, &cx, &stage);
        self.engine.update(&cx, &stage, lib);
        if tick.paint {
            self.engine.paint(ui, &cx, &stage);
        }
        self.last_index = Some(shot.index);
    }

    /// Take the geometry the renderers published. They publish while the
    /// slide draws (after the engine), so what arrives is last frame's;
    /// collection stays on only while an engine is drawing.
    fn follow_hints(&mut self, ui: &egui::Ui, index: usize) {
        hints::set_enabled(ui.ctx(), true);
        let fresh = hints::take(ui.ctx());
        let changed = self.last_index.is_some_and(|i| i != index);
        adopt_hints(&mut self.hints, &mut self.hints_key, fresh, changed);
    }

    /// What the frame shows: a countdown digit or its burst, the end slide
    /// with its words, or a slide. Masks are drawn in the theme's display
    /// face, again when the face changes.
    fn moment(&mut self, ui: &egui::Ui, shot: &Shot, end_elapsed: f32) -> Moment {
        let face = shot.theme.display_family();
        if self.mask_face.as_ref() != Some(&face) {
            self.digits.clear();
            self.end_words = None;
            self.mask_face = Some(face);
        }
        if let Some((phase, progress)) = shot.countdown {
            match phase {
                CountPhase::Digit(digit) => Moment::Countdown {
                    digit,
                    mask: self.digit_mask(ui, shot.theme, digit),
                    progress,
                },
                CountPhase::Burst => Moment::Burst { progress },
            }
        } else if shot.end {
            let words = self
                .end_words
                .get_or_insert_with(|| text_mask(ui, shot.theme, "THE END"))
                .clone();
            Moment::End {
                elapsed: end_elapsed,
                words,
            }
        } else {
            Moment::Slide
        }
    }

    /// Mask points and aspect (width / height) for a digit, rasterised once
    /// through egui in the theme's display face.
    fn digit_mask(&mut self, ui: &egui::Ui, theme: &Theme, digit: u8) -> Mask {
        if let Some((_, mask)) = self.digits.iter().find(|(d, _)| *d == digit) {
            return mask.clone();
        }
        let mut mask = glyph_mask(ui, theme, char::from(b'0' + digit));
        if digit == 1 {
            mask = trim_flag(mask);
        }
        self.digits.push((digit, mask.clone()));
        mask
    }
}

/// The slide's illustration, when it asks for one, the layout can show it
/// (an editorial layout draws the slide) and the name resolves.
/// Fold the hints taken this frame into the held set. On the first frame of
/// a new slide, `fresh` is what the previous slide drew last frame, so it is
/// dropped with the held set instead of adopted (a slide without visuals
/// would otherwise keep the previous chart's geometry, D24).
fn adopt_hints(held: &mut Vec<Hint>, key: &mut u64, fresh: Vec<Hint>, slide_changed: bool) {
    if slide_changed {
        held.clear();
        *key = 0;
        return;
    }
    if !fresh.is_empty() {
        let fp = hints::fingerprint(&fresh);
        if fp != *key {
            *held = fresh;
            *key = fp;
        }
    }
}

pub fn illustration_for(
    slide: &Slide,
    lib: &mut Library,
) -> Option<std::sync::Arc<crate::render::illustration::Cloud>> {
    let name = slide.illustration.as_deref()?;
    if !crate::render::ember::handles(slide) {
        return None;
    }
    lib.get(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar() -> Vec<Hint> {
        vec![Hint::Bar(egui::Rect::from_min_size(
            egui::pos2(10.0, 20.0),
            egui::vec2(40.0, 80.0),
        ))]
    }

    #[test]
    fn hints_taken_on_a_slide_change_belong_to_the_previous_slide() {
        let mut held = bar();
        let mut key = hints::fingerprint(&held);
        // First frame of the next slide: what arrives is the chart's.
        adopt_hints(&mut held, &mut key, bar(), true);
        assert!(held.is_empty(), "the previous slide's chart leaked");
        assert_eq!(key, 0);
        // A slide without visuals publishes nothing afterwards.
        adopt_hints(&mut held, &mut key, Vec::new(), false);
        assert!(held.is_empty());
    }

    #[test]
    fn hints_on_the_same_slide_are_adopted_and_kept() {
        let mut held = Vec::new();
        let mut key = 0;
        adopt_hints(&mut held, &mut key, bar(), false);
        assert_eq!(held.len(), 1);
        assert_ne!(key, 0);
        // A frame that published nothing keeps the held geometry.
        adopt_hints(&mut held, &mut key, Vec::new(), false);
        assert_eq!(held.len(), 1);
    }
}
