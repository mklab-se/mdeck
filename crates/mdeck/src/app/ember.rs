//! The Ember theme's live state inside the presentation window: the particle
//! field, which scene it is showing, and the opening countdown's digits.

use std::time::Instant;

use eframe::egui;

use crate::parser::Slide;
use crate::render::hints::{self, Hint};
use crate::render::illustration::Library;
use crate::render::particles::{self, Field, scenes};
use crate::render::story::{self, Script};
use crate::theme::Theme;

/// Where the opening countdown is, as seen by the particle field.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum CountPhase {
    /// Showing this digit (3, 2 or 1).
    Digit(u8),
    /// The last digit bursts into black.
    Burst,
}

/// How far the countdown is through its current phase (0..1), for pacing.
pub(crate) type CountProgress = f32;

/// The end slide's choreography: the words, a swirl, a bang, then black.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum EndPhase {
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

/// Mask points in the unit square, with the mask's width / height.
type Mask = (std::sync::Arc<Vec<[f32; 2]>>, f32);

/// Everything a scene choice depends on: slide index, reveal step, end
/// phase, story version and countdown phase.
type SceneKey = (usize, usize, Option<EndPhase>, u64, Option<CountPhase>);

pub(crate) struct EmberState {
    field: Option<Field>,
    /// What the current scene was built for.
    key: Option<SceneKey>,
    /// When the end slide was entered, for its choreography.
    end_started: Option<Instant>,
    /// Geometry the current slide's renderers published, and its fingerprint.
    hints: Vec<Hint>,
    hints_key: u64,
    /// "THE END" as a mask, rasterised once.
    end_words: Option<Mask>,
    /// Labels of the staged story, if the current scene is one.
    labels: Vec<story::Label>,
    /// Glyph masks for the countdown digits, sampled from egui's font atlas.
    digits: Vec<(u8, Mask)>,
    last_tick: Option<Instant>,
}

impl EmberState {
    pub(crate) fn new() -> Self {
        Self {
            field: None,
            key: None,
            labels: Vec::new(),
            digits: Vec::new(),
            end_started: None,
            end_words: None,
            hints: Vec::new(),
            hints_key: 0,
            last_tick: None,
        }
    }

    /// Seconds since the end slide was entered (0 when not on it).
    pub(crate) fn end_elapsed(&self) -> f32 {
        self.end_started
            .map(|t| t.elapsed().as_secs_f32())
            .unwrap_or(0.0)
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

    /// Advance the field one frame for the slide about to be drawn and paint
    /// it into `rect`. `end` selects the end choreography. With `still` the
    /// field settles instantly instead of animating (export).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn frame(
        &mut self,
        ui: &egui::Ui,
        rect: egui::Rect,
        slide: Option<&Slide>,
        story: Option<&Script>,
        story_version: u64,
        index: usize,
        reveal: usize,
        end: bool,
        countdown: Option<(CountPhase, CountProgress)>,
        theme: &Theme,
        scale: f32,
        opacity: f32,
        still: bool,
        lib: &mut Library,
    ) {
        let now = Instant::now();
        // Renderers publish geometry while the slide draws (after this call),
        // so what we read here is last frame's. Collection stays on only
        // while Ember is drawing.
        hints::set_enabled(ui.ctx(), true);
        let fresh = hints::take(ui.ctx());
        let slide_changed = self.key.is_some_and(|k| k.0 != index);
        if slide_changed {
            self.hints.clear();
            self.hints_key = 0;
        }
        if !fresh.is_empty() {
            let fp = hints::fingerprint(&fresh);
            if fp != self.hints_key {
                self.hints = fresh;
                self.hints_key = fp;
                // force a scene rebuild for hint-driven layouts
                self.key = None;
            }
        }
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

        // The end slide runs its own clock from the moment it is entered.
        let end_phase = if end {
            let started = *self.end_started.get_or_insert(now);
            Some(EndPhase::at(now.duration_since(started).as_secs_f32()))
        } else {
            self.end_started = None;
            None
        };
        let key = (
            index,
            reveal,
            end_phase.map(|(p, _)| p),
            story_version,
            countdown.map(|(p, _)| p),
        );
        let rect_aspect = rect.width() / rect.height();
        if self.key != Some(key) {
            self.labels.clear();
            let scene = if let Some((phase, _)) = countdown {
                match phase {
                    CountPhase::Digit(d) => {
                        let (points, aspect) = self.digit_mask(ui, theme, d);
                        scenes::digit(points, aspect, rect_aspect)
                    }
                    CountPhase::Burst => scenes::burst(),
                }
            } else if let Some((phase, _)) = end_phase {
                match phase {
                    EndPhase::Words => {
                        let (points, aspect) = self
                            .end_words
                            .get_or_insert_with(|| text_mask(ui, theme, "THE END"))
                            .clone();
                        scenes::end_words(points, aspect, rect_aspect)
                    }
                    EndPhase::Dance => scenes::end_dance(),
                    EndPhase::Bang | EndPhase::Black => scenes::end_bang(),
                }
            } else if let (Some(slide), Some(script)) = (slide, story)
                && !crate::render::ember::is_title(slide, index)
            {
                let staged = story::stage(script, slide.layout, rect_aspect, lib);
                self.labels = staged.labels;
                staged.scene
            } else if let Some(slide) = slide
                && let Some(cloud) = illustration_for(slide, lib)
            {
                if crate::render::ember::is_title(slide, index) {
                    scenes::illustration_backdrop(
                        std::sync::Arc::clone(&cloud.points),
                        cloud.aspect,
                        rect_aspect,
                    )
                } else {
                    scenes::illustration_stage(
                        std::sync::Arc::clone(&cloud.points),
                        cloud.aspect,
                        slide.layout,
                        rect_aspect,
                    )
                }
            } else if let Some(slide) = slide
                && !self.hints.is_empty()
                && uses_hints(slide)
            {
                scenes::from_hints(&self.hints, rect, index as u64 + 1)
            } else if let Some(slide) = slide {
                scenes::for_slide(slide, index as u64 + 1)
            } else {
                scenes::constellation(index as u64 + 1)
            };
            let field = self.field.as_mut().expect("field created above");
            field.set_scene(scene, rect, index as u64 + 1);
            if still {
                field.settle(reveal);
            }
            self.key = Some(key);
        }
        let field = self.field.as_mut().expect("field created above");
        if still {
            field.paint(ui.painter(), rect, opacity, false);
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
            return;
        }
        // Digits assemble briskly, the burst accelerates outward, slides
        // take their time.
        let speed = match (countdown, end_phase) {
            (Some((CountPhase::Digit(_), _)), _) => 1.8,
            (Some((CountPhase::Burst, progress)), _) => 1.0 + 3.0 * progress,
            (None, Some((EndPhase::Words, _))) => 1.6,
            (None, Some((EndPhase::Dance, _))) => 1.3,
            (None, Some((EndPhase::Bang, progress))) => 1.2 + 3.0 * progress,
            (None, Some((EndPhase::Black, _))) => 2.0,
            (None, None) => 1.0,
        };
        field.tick(dt * speed, reveal);
        field.paint(ui.painter(), rect, opacity, true);
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
}

/// The slide's illustration, when it asks for one, the layout can show it
/// (Ember draws the slide itself) and the name resolves.
pub(crate) fn illustration_for(
    slide: &Slide,
    lib: &mut Library,
) -> Option<std::sync::Arc<crate::render::illustration::Cloud>> {
    let name = slide.illustration.as_deref()?;
    if !crate::render::ember::handles(slide) {
        return None;
    }
    lib.get(name)
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

/// Spectral's 1 wears a long flag. Keep only the half of it nearest the stem,
/// then renormalise the mask to its new width.
fn trim_flag((pts, aspect): Mask) -> Mask {
    // The flag is the part left of the stem in the top third of the glyph;
    // the stem starts around 45% of the width in this face.
    let flag_cut = 0.24;
    let kept: Vec<[f32; 2]> = pts
        .iter()
        .copied()
        .filter(|p| !(p[1] < 0.34 && p[0] < flag_cut))
        .collect();
    let min_x = kept.iter().map(|p| p[0]).fold(1.0, f32::min);
    let width = (1.0 - min_x).max(1e-3);
    let renormalised = kept
        .into_iter()
        .map(|p| [(p[0] - min_x) / width, p[1]])
        .collect();
    (std::sync::Arc::new(renormalised), aspect * width)
}

/// Sample a glyph's coverage out of egui's font atlas into mask points in the
/// unit square, returning them with the glyph's width / height.
fn glyph_mask(ui: &egui::Ui, theme: &Theme, ch: char) -> Mask {
    text_mask(ui, theme, &ch.to_string())
}

/// Sample a whole string's coverage out of egui's font atlas into mask points
/// in the unit square, with the text's width / height. Glyphs keep their
/// layout positions, so spacing and kerning come from the face.
fn text_mask(ui: &egui::Ui, theme: &Theme, text: &str) -> Mask {
    let font = egui::FontId::new(220.0, theme.display_family());
    let (glyphs, image) = ui.fonts_mut(|f| {
        let galley = f.layout_no_wrap(text.to_string(), font, egui::Color32::WHITE);
        let glyphs: Vec<(egui::Pos2, egui::Vec2, [u16; 2], [u16; 2])> = galley
            .rows
            .iter()
            .flat_map(|r| r.glyphs.iter())
            .filter(|g| g.uv_rect.max[0] > g.uv_rect.min[0])
            .map(|g| {
                (
                    g.pos + g.uv_rect.offset,
                    g.uv_rect.size,
                    g.uv_rect.min,
                    g.uv_rect.max,
                )
            })
            .collect();
        (glyphs, f.image())
    });
    if glyphs.is_empty() {
        return (std::sync::Arc::new(Vec::new()), 0.6);
    }
    // bounding box of the ink in layout points
    let (mut bx0, mut by0, mut bx1, mut by1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for (pos, size, _, _) in &glyphs {
        bx0 = bx0.min(pos.x);
        by0 = by0.min(pos.y);
        bx1 = bx1.max(pos.x + size.x);
        by1 = by1.max(pos.y + size.y);
    }
    let (bw, bh) = ((bx1 - bx0).max(1.0), (by1 - by0).max(1.0));
    let mut pts = Vec::new();
    for (pos, size, min, max) in &glyphs {
        let (x0, y0, x1, y1) = (
            min[0] as usize,
            min[1] as usize,
            max[0] as usize,
            max[1] as usize,
        );
        let (gw, gh) = ((x1 - x0).max(1), (y1 - y0).max(1));
        let step = (bh as usize / 90).max(1);
        for y in (y0..y1).step_by(step) {
            for x in (x0..x1).step_by(step) {
                if image[(x, y)].a() > 110 {
                    let px = pos.x + (x - x0) as f32 / gw as f32 * size.x;
                    let py = pos.y + (y - y0) as f32 / gh as f32 * size.y;
                    pts.push([(px - bx0) / bw, (py - by0) / bh]);
                }
            }
        }
    }
    // Masks are consumed in order (a group of n particles takes the first n
    // points), and scanline order would light the top of the glyph first.
    shuffle(&mut pts, 0x6C7F);
    (std::sync::Arc::new(pts), bw / bh)
}

/// Deterministic Fisher-Yates.
fn shuffle(pts: &mut [[f32; 2]], seed: u64) {
    let mut rng = particles::Rng::new(seed);
    for i in (1..pts.len()).rev() {
        let j = (rng.unit() * (i + 1) as f32) as usize;
        pts.swap(i, j.min(i));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn end_words_mask_is_wide_and_dense() {
        let ctx = egui::Context::default();
        crate::render::fonts::install(&ctx);
        let theme = Theme::ember();
        let mut output = ctx.run_ui(Default::default(), |ui| {
            let (pts, aspect) = text_mask(ui, &theme, "THE END");
            assert!(pts.len() > 1500, "only {} points", pts.len());
            assert!(aspect > 4.0 && aspect < 9.0, "aspect {aspect}");
            let (w, h) = (((14.0 * aspect) * 2.0) as usize, 14usize);
            let mut grid = vec![vec![' '; w + 1]; h + 1];
            for p in pts.iter() {
                let x = (p[0] * w as f32) as usize;
                let y = (p[1] * h as f32) as usize;
                grid[y.min(h)][x.min(w)] = '#';
            }
            eprintln!("--- THE END (aspect {aspect:.2}, {} points)", pts.len());
            for row in grid {
                eprintln!("{}", row.into_iter().collect::<String>());
            }
        });
        output.textures_delta.clear();
    }

    /// The digit masks come out of egui's font atlas; make sure the sampled
    /// coverage is a real glyph (dense, right aspect) and print it so a
    /// reviewer can eyeball the shape in the test output.
    #[test]
    fn digit_masks_are_glyph_shaped() {
        let ctx = egui::Context::default();
        crate::render::fonts::install(&ctx);
        let theme = Theme::ember();
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1920.0, 1080.0),
                )),
                ..Default::default()
            },
            |ui| {
                for ch in ['3', '2', '1'] {
                    let mut mask = glyph_mask(ui, &theme, ch);
                    if ch == '1' {
                        mask = trim_flag(mask);
                    }
                    let (pts, aspect) = mask;
                    assert!(pts.len() > 300, "{ch}: only {} points", pts.len());
                    assert!(aspect > 0.35 && aspect < 0.9, "{ch}: aspect {aspect}");
                    // ASCII dump, 24 rows
                    let (w, h) = (((24.0 * aspect) * 2.0) as usize, 24usize);
                    let mut grid = vec![vec![' '; w + 1]; h + 1];
                    for p in pts.iter() {
                        let x = (p[0] * w as f32) as usize;
                        let y = (p[1] * h as f32) as usize;
                        grid[y.min(h)][x.min(w)] = '#';
                    }
                    eprintln!("--- {ch} (aspect {aspect:.2}, {} points)", pts.len());
                    for row in grid {
                        eprintln!("{}", row.into_iter().collect::<String>());
                    }
                }
            },
        );
        output.textures_delta.clear();
    }
}
