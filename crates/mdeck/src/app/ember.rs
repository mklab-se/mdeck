//! The Ember theme's live state inside the presentation window: the particle
//! field, which scene it is showing, and the logo intro.

use std::time::{Duration, Instant};

use eframe::egui;

use crate::parser::Slide;
use crate::render::particles::{self, Field, scenes};
use crate::render::story::{self, Script};
use crate::theme::Theme;

static LOGO_BYTES: &[u8] = include_bytes!("../../media/logo-small.png");

/// How long the particles hold the logo before dissolving into the first scene.
const INTRO_ASSEMBLE: Duration = Duration::from_millis(1500);
const INTRO_HOLD: Duration = Duration::from_millis(1300);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Intro {
    /// Not started yet (first title slide not shown).
    Pending,
    /// Particles are forming / holding the logo.
    Running(Instant),
    /// Finished or skipped.
    Done,
}

pub(super) struct EmberState {
    field: Option<Field>,
    /// (slide index, reveal step, end-slide flag, story version) the current scene was built for.
    key: Option<(usize, usize, bool, u64)>,
    /// Labels of the staged story, if the current scene is one.
    labels: Vec<story::Label>,
    intro: Intro,
    logo: Option<(std::sync::Arc<Vec<[f32; 2]>>, f32)>,
    last_tick: Option<Instant>,
}

impl EmberState {
    pub(super) fn new() -> Self {
        Self {
            field: None,
            key: None,
            labels: Vec::new(),
            intro: Intro::Pending,
            logo: None,
            last_tick: None,
        }
    }

    pub(super) fn intro_running(&self) -> bool {
        matches!(self.intro, Intro::Running(_))
    }

    /// Any navigation ends the intro early, as a key press does on the site.
    pub(super) fn skip_intro(&mut self) {
        if self.intro_running() {
            self.intro = Intro::Done;
            self.key = None;
        }
    }

    fn logo(&mut self) -> (std::sync::Arc<Vec<[f32; 2]>>, f32) {
        self.logo
            .get_or_insert_with(|| particles::mask_points_from_png(LOGO_BYTES))
            .clone()
    }

    /// Advance the field one frame for the slide about to be drawn and paint
    /// it into `rect`. `end` selects the logo scene for the virtual end slide.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn frame(
        &mut self,
        ui: &egui::Ui,
        rect: egui::Rect,
        slide: Option<&Slide>,
        story: Option<&Script>,
        story_version: u64,
        index: usize,
        reveal: usize,
        end: bool,
        theme: &Theme,
        scale: f32,
        opacity: f32,
    ) {
        let now = Instant::now();
        let dt = self
            .last_tick
            .map(|t| now.duration_since(t).as_secs_f32())
            .unwrap_or(1.0 / 60.0);
        self.last_tick = Some(now);

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

        // The logo intro runs once, on the first title slide.
        if self.intro == Intro::Pending {
            let is_title = slide.is_some_and(|s| crate::render::ember::is_title(s, index));
            if index == 0 && is_title && !end {
                self.intro = Intro::Running(now);
                self.key = None;
            } else {
                self.intro = Intro::Done;
            }
        }
        if let Intro::Running(start) = self.intro
            && now.duration_since(start) > INTRO_ASSEMBLE + INTRO_HOLD
        {
            self.intro = Intro::Done;
            self.key = None;
        }

        let key = (index, reveal, end, story_version);
        let show_logo = end || self.intro_running();
        let field = self.field.as_mut().expect("field created above");
        if self.key != Some(key) {
            self.labels.clear();
            let scene = if show_logo {
                let (points, aspect) = self
                    .logo
                    .get_or_insert_with(|| particles::mask_points_from_png(LOGO_BYTES))
                    .clone();
                scenes::mask(points, aspect, rect.width() / rect.height(), 0.30)
            } else if let (Some(slide), Some(script)) = (slide, story)
                && !crate::render::ember::is_title(slide, index)
            {
                let staged = story::stage(script, slide.layout, rect.width() / rect.height());
                self.labels = staged.labels;
                staged.scene
            } else if let Some(slide) = slide {
                scenes::for_slide(slide, index as u64 + 1)
            } else {
                scenes::constellation(index as u64 + 1)
            };
            field.set_scene(scene, rect, index as u64 + 1);
            self.key = Some(key);
        }
        // During the intro the particles rush to their marks a little faster.
        field.tick(if show_logo { dt * 1.6 } else { dt }, reveal);
        field.paint(ui.painter(), rect, opacity);
        if !self.labels.is_empty() {
            story::draw_labels(
                ui.painter(),
                &self.labels,
                field,
                rect,
                theme,
                scale,
                opacity,
            );
        }
        ui.ctx().request_repaint();
    }

    /// Ensure the logo mask is decoded (called lazily; kept for API symmetry).
    #[allow(dead_code)]
    pub(super) fn warm(&mut self) {
        let _ = self.logo();
    }
}
