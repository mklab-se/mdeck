//! The Ember theme's live state inside the presentation window: the particle
//! field, which scene it is showing, and the opening countdown's digits.

use std::time::Instant;

use eframe::egui;

use crate::parser::Slide;
use crate::render::particles::{self, Field, scenes};
use crate::render::story::{self, Script};
use crate::theme::Theme;

static LOGO_BYTES: &[u8] = include_bytes!("../../media/logo-small.png");

/// Where the opening countdown is, as seen by the particle field.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum CountPhase {
    /// Showing this digit (3, 2 or 1).
    Digit(u8),
    /// The last digit bursts into black.
    Burst,
}

/// How far the countdown is through its current phase (0..1), for pacing.
pub(super) type CountProgress = f32;

/// Mask points in the unit square, with the mask's width / height.
type Mask = (std::sync::Arc<Vec<[f32; 2]>>, f32);

pub(super) struct EmberState {
    field: Option<Field>,
    /// (slide index, reveal step, end-slide flag, story version, countdown
    /// phase) the current scene was built for.
    key: Option<(usize, usize, bool, u64, Option<CountPhase>)>,
    /// Labels of the staged story, if the current scene is one.
    labels: Vec<story::Label>,
    logo: Option<Mask>,
    /// Glyph masks for the countdown digits, sampled from egui's font atlas.
    digits: Vec<(u8, Mask)>,
    last_tick: Option<Instant>,
}

impl EmberState {
    pub(super) fn new() -> Self {
        Self {
            field: None,
            key: None,
            labels: Vec::new(),
            logo: None,
            digits: Vec::new(),
            last_tick: None,
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
        countdown: Option<(CountPhase, CountProgress)>,
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

        let key = (index, reveal, end, story_version, countdown.map(|(p, _)| p));
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
            } else if end {
                let (points, aspect) = self
                    .logo
                    .get_or_insert_with(|| particles::mask_points_from_png(LOGO_BYTES))
                    .clone();
                scenes::mask(points, aspect, rect_aspect, 0.30)
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
            let field = self.field.as_mut().expect("field created above");
            field.set_scene(scene, rect, index as u64 + 1);
            self.key = Some(key);
        }
        let field = self.field.as_mut().expect("field created above");
        // Digits assemble briskly, the burst accelerates outward, slides
        // take their time.
        let speed = match countdown {
            Some((CountPhase::Digit(_), _)) => 1.8,
            Some((CountPhase::Burst, progress)) => 1.0 + 3.0 * progress,
            None => 1.0,
        };
        field.tick(dt * speed, reveal);
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
    let font = egui::FontId::new(220.0, theme.display_family());
    let (rect_px, image) = ui.fonts_mut(|f| {
        let galley = f.layout_no_wrap(ch.to_string(), font, egui::Color32::WHITE);
        let uv = galley
            .rows
            .first()
            .and_then(|r| r.glyphs.first())
            .map(|g| (g.uv_rect.min, g.uv_rect.max));
        (uv, f.image())
    });
    let Some((min, max)) = rect_px else {
        return (std::sync::Arc::new(Vec::new()), 0.6);
    };
    let (x0, y0, x1, y1) = (
        min[0] as usize,
        min[1] as usize,
        max[0] as usize,
        max[1] as usize,
    );
    let (gw, gh) = ((x1 - x0).max(1), (y1 - y0).max(1));
    let step = (gw.max(gh) / 90).max(1);
    let mut pts = Vec::new();
    for y in (y0..y1).step_by(step) {
        for x in (x0..x1).step_by(step) {
            if image[(x, y)].a() > 110 {
                pts.push([(x - x0) as f32 / gw as f32, (y - y0) as f32 / gh as f32]);
            }
        }
    }
    (std::sync::Arc::new(pts), gw as f32 / gh as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

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
