//! The thermal engine: the deck seen through a thermal instrument. A heat
//! field lies under the slides, drawn in the theme's heat palette in
//! contour bands and transparent where it is cold.
//!
//! - **The cold opening.** On title and section slides the heading forms in
//!   heat: points glow inside the letters, spread and join into contours,
//!   and the words are readable within a second. Then the crisp type rises
//!   into it (the copy waits [`COLD_OPEN_HOLD`]) and the heat settles into a
//!   faint contour halo that stays.
//! - **Heat signatures.** An `picture` glows like a warm body; the
//!   countdown digits heat up and cool off; the end words glow and fade.
//! - **Calm evidence.** Where a slide shows charts, images or diagrams the
//!   field stays dark. With `heat: { drift: true }` a few embers drift
//!   through the dark on ordinary slides, cooling as they rise.
//! - **Cooling between slides.** Nothing is cleared on a slide change: the
//!   old heat cools while the new heat builds.
//!
//! Exports and reduced motion show the settled field.

mod field;

use eframe::egui::{self, Pos2, Rect};

use super::stage::{FrameCx, Look, Moment, Stage};
use super::{Capabilities, Engine, EngineDef, hash01};
use crate::render::ember::COLD_OPEN_HOLD;
use crate::render::hints::Hint;
use crate::render::illustration::Library;
use field::{Field, smooth};

/// Seconds into the end slide when the caption fades in: the words have
/// glowed and cooled.
pub const END_CAPTION_DELAY: f32 = 4.4;
/// The end words glow this long, then cool.
const END_WORDS: f32 = 3.0;
/// The heading reaches full heat this long after the slide is entered:
/// readable words in the field.
const FORMED: f32 = 0.8;
/// The heat a settled heading keeps: the faint contour halo.
const HALO: f32 = 0.38;
/// The first share of a heading's cells (by rank) glow as small discs: the
/// points of heat the opening starts from.
const SEEDS: f32 = 0.05;
/// How fast heat spreads while a heading forms (tight, so the letters read)
/// and once it has settled (wide, a halo of contours).
const SPREAD_FORMING: f32 = 9.0;
const SPREAD_SETTLED: f32 = 70.0;
const SPREAD_FIGURE: f32 = 90.0;
/// Contour bands the field is drawn in.
const BANDS: usize = 9;
const EMBERS: usize = 120;

pub static DEF: EngineDef = EngineDef {
    capabilities: Capabilities {
        cold_open: true,
        heat_trace: true,
        ..Capabilities::PICTURES
    },
    create: || Box::new(Thermal::new()),
    end_caption_delay: END_CAPTION_DELAY,
    medium: None,
    render_slide: None,
    problems: None,
};

/// A heading's glyphs in field cells, with a rank per cell that decides
/// when it starts to glow.
struct HeadingMask {
    key: u64,
    cells: Vec<(u16, u16, f32)>,
}

#[derive(Clone, Copy)]
struct Ember {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    heat: f32,
}

pub struct Thermal {
    field: Option<Field>,
    key: Option<(usize, Look)>,
    /// Seconds since the current moment began.
    age: f32,
    heading: Option<HeadingMask>,
    embers: Vec<Ember>,
    texture: Option<egui::TextureHandle>,
    /// The field changed since the texture was last uploaded.
    dirty: bool,
}

impl Thermal {
    pub fn new() -> Self {
        Self {
            field: None,
            key: None,
            age: 0.0,
            heading: None,
            embers: (0..EMBERS).map(|i| ember(i as u32, 0)).collect(),
            texture: None,
            dirty: true,
        }
    }

    /// Set the sources for this moment, `age` seconds into it.
    fn sources(&mut self, cx: &FrameCx, stage: &Stage) {
        let Some(field) = self.field.as_mut() else {
            return;
        };
        field.clear_sources();
        let rect = cx.rect;
        let (fw, fh) = (field.w as f32, field.h as f32);
        let to_cell = |p: Pos2| {
            (
                (p.x - rect.left()) / rect.width() * fw,
                (p.y - rect.top()) / rect.height() * fh,
            )
        };
        // charts, images and diagrams stay dark
        for hint in stage.hints {
            if let Hint::Frame(r) = hint {
                let (x0, y0) = to_cell(r.min);
                let (x1, y1) = to_cell(r.max);
                field.keep_calm(x0, y0, x1, y1);
            }
        }
        let age = self.age;
        // a mask in the unit square, `height` of the slide tall, centred
        let stamp_mask = |field: &mut Field, pts: &[[f32; 2]], aspect: f32, height: f32, v: f32| {
            let h = height * fh;
            let w = h * aspect * rect.height() / rect.width() * fw / fh;
            let (x0, y0) = ((fw - w) / 2.0, (fh - h) / 2.0);
            for p in pts {
                field.disc(x0 + p[0] * w, y0 + p[1] * h, 1.1, v);
            }
        };
        match &stage.moment {
            Moment::Countdown { mask, progress, .. } => {
                let v = smooth(0.0, 0.3, *progress) * (1.0 - 0.85 * smooth(0.75, 1.0, *progress));
                stamp_mask(field, &mask.0, mask.1, 0.5, v);
            }
            Moment::Burst { .. } => {}
            Moment::End { words, elapsed } => {
                if *elapsed < END_WORDS {
                    let v = smooth(0.0, 0.5, *elapsed);
                    stamp_mask(field, &words.0, words.1, 0.16, v);
                }
            }
            Moment::Slide => {
                if let Some(h) = &self.heading {
                    // seeds glow first, then the whole letters, then the
                    // heat sinks to the halo as the crisp type rises
                    let reach = smooth(0.0, FORMED, age);
                    let level =
                        1.0 - (1.0 - HALO) * smooth(COLD_OPEN_HOLD, COLD_OPEN_HOLD + 1.1, age);
                    for &(x, y, rank) in &h.cells {
                        if rank > reach {
                            continue;
                        }
                        if rank < SEEDS && age < FORMED {
                            // a seed: a point of heat that grows
                            let r = 1.0 + 2.0 * smooth(0.0, FORMED, age);
                            field.disc(x as f32 + 0.5, y as f32 + 0.5, r, level);
                        } else {
                            field.source(x as usize, y as usize, level);
                        }
                    }
                }
                if let Some(fig) = &stage.figure {
                    let p = fig.place;
                    let v = if fig.backdrop { 0.32 } else { 0.72 };
                    let n = fig.cloud.points.len().min(1400);
                    for pt in &fig.cloud.points[..n] {
                        let x = (p.u + pt[0] * p.w) * fw;
                        let y = (p.v + pt[1] * p.h) * fh;
                        field.disc(x, y, 0.9, v);
                    }
                }
                if cx.theme.heat.drift {
                    for e in &self.embers {
                        field.disc(e.x * fw, e.y * fh, 1.3, e.heat * 0.55);
                    }
                }
            }
        }
    }

    /// Build the heading's cells from the renderer's hint, sampling the
    /// glyphs' coverage out of the font atlas.
    fn heading_mask(&mut self, ui: &egui::Ui, cx: &FrameCx, stage: &Stage) -> bool {
        let text: Vec<(&std::sync::Arc<egui::Galley>, Pos2)> = stage
            .hints
            .iter()
            .filter_map(|h| match h {
                Hint::Text { galley, pos, slide } if *slide == stage.index => Some((galley, *pos)),
                _ => None,
            })
            .collect();
        if text.is_empty() {
            let had = self.heading.take().is_some();
            return had;
        }
        let key = stage.hints_key ^ (stage.index as u64).wrapping_mul(0x9E37_79B9);
        if self.heading.as_ref().is_some_and(|h| h.key == key) {
            return false;
        }
        let Some(field) = self.field.as_ref() else {
            return false;
        };
        let rect = cx.rect;
        let (fw, fh) = (field.w, field.h);
        let mut on = vec![false; fw * fh];
        ui.fonts_mut(|f| {
            let atlas = f.image();
            for (galley, pos) in &text {
                for row in &galley.rows {
                    for g in &row.glyphs {
                        let (min, max) = (g.uv_rect.min, g.uv_rect.max);
                        if max[0] <= min[0] || max[1] <= min[1] {
                            continue;
                        }
                        let origin = *pos + row.pos.to_vec2() + g.pos.to_vec2() + g.uv_rect.offset;
                        let size = g.uv_rect.size;
                        // every atlas pixel of the glyph, into its field cell
                        for ay in min[1]..max[1] {
                            for ax in min[0]..max[0] {
                                if atlas[(ax as usize, ay as usize)].a() < 100 {
                                    continue;
                                }
                                let px = origin.x
                                    + (ax - min[0]) as f32 / (max[0] - min[0]) as f32 * size.x;
                                let py = origin.y
                                    + (ay - min[1]) as f32 / (max[1] - min[1]) as f32 * size.y;
                                let cx_ = ((px - rect.left()) / rect.width() * fw as f32) as isize;
                                let cy_ = ((py - rect.top()) / rect.height() * fh as f32) as isize;
                                if cx_ >= 0
                                    && cy_ >= 0
                                    && (cx_ as usize) < fw
                                    && (cy_ as usize) < fh
                                {
                                    on[cy_ as usize * fw + cx_ as usize] = true;
                                }
                            }
                        }
                    }
                }
            }
        });
        let cells = on
            .iter()
            .enumerate()
            .filter(|(_, v)| **v)
            .map(|(i, _)| {
                let (x, y) = (i % fw, i / fw);
                // seeds are spread across the letters, not swept along them
                (x as u16, y as u16, hash01(i as u32 ^ 0x5bd1_e995))
            })
            .collect();
        self.heading = Some(HeadingMask { key, cells });
        true
    }

    /// A field over the slide's aspect (a new one when the aspect changes).
    fn ensure_field(&mut self, cx: &FrameCx) {
        let aspect = cx.rect.width() / cx.rect.height().max(1.0);
        if self
            .field
            .as_ref()
            .is_none_or(|f| ((f.w as f32 / f.h as f32) - aspect).abs() > 0.05)
        {
            self.field = Some(Field::new(aspect));
            self.heading = None;
            self.dirty = true;
        }
    }

    fn drift(&mut self, dt: f32, seed: u32) {
        for (i, e) in self.embers.iter_mut().enumerate() {
            e.x += e.vx * dt;
            e.y += e.vy * dt;
            e.heat -= dt * 0.06;
            if e.heat <= 0.02 || e.y < -0.02 || e.x < -0.02 || e.x > 1.02 {
                *e = ember(i as u32, seed.wrapping_add((e.x * 1000.0) as u32));
            }
        }
    }
}

impl Default for Thermal {
    fn default() -> Self {
        Self::new()
    }
}

/// A new ember near the bottom, from a seed (no wall-clock randomness).
fn ember(i: u32, seed: u32) -> Ember {
    let r = |k: u32| hash01(i.wrapping_mul(7919) ^ seed.wrapping_mul(31) ^ k);
    Ember {
        x: r(1),
        y: 0.55 + 0.5 * r(2),
        vx: (r(3) - 0.5) * 0.02,
        vy: -0.012 - 0.03 * r(4),
        heat: 0.25 + 0.45 * r(5),
    }
}

impl Engine for Thermal {
    fn prepare(&mut self, ui: &egui::Ui, cx: &FrameCx, stage: &Stage) {
        self.ensure_field(cx);
        if self.heading_mask(ui, cx, stage) {
            self.dirty = true;
        }
    }

    fn update(&mut self, cx: &FrameCx, stage: &Stage, _lib: &mut Library) {
        self.ensure_field(cx);
        let key = (stage.index, stage.moment.look(END_WORDS));
        if self.key != Some(key) {
            self.key = Some(key);
            self.age = 0.0;
        }
        if cx.still {
            // the settled look: the opening long over
            self.age = 60.0;
        } else {
            self.age += cx.dt;
            if cx.theme.heat.drift {
                self.drift(cx.dt, stage.index as u32);
            }
        }
        self.sources(cx, stage);
        let forming = self.heading.is_some()
            && matches!(stage.moment, Moment::Slide)
            && self.age < COLD_OPEN_HOLD + 1.2;
        let spread = if forming {
            let t = smooth(COLD_OPEN_HOLD - 0.2, COLD_OPEN_HOLD + 1.2, self.age);
            SPREAD_FORMING + (SPREAD_SETTLED - SPREAD_FORMING) * t
        } else if self.heading.is_some() {
            SPREAD_SETTLED
        } else if stage.figure.is_some() {
            // a body reads as one warm shape, its detail glowing through
            SPREAD_FIGURE
        } else {
            field::SPREAD
        };
        if let Some(f) = self.field.as_mut() {
            f.spread = spread;
            if cx.still {
                f.settle();
                self.dirty = true;
            } else if f.step(cx.dt) > 1e-4 {
                self.dirty = true;
            }
        }
    }

    fn paint(&mut self, ui: &egui::Ui, cx: &FrameCx, _stage: &Stage) {
        let Some(field) = self.field.as_ref() else {
            return;
        };
        let lut = cx.theme.heat.palette.lut();
        let changed = self.dirty;
        if self.dirty || self.texture.is_none() {
            let image = field.image(&lut, BANDS);
            let options = egui::TextureOptions::LINEAR;
            match &mut self.texture {
                Some(t) => t.set(image, options),
                None => {
                    self.texture =
                        Some(ui.ctx().load_texture("mdeck-thermal-field", image, options))
                }
            }
            self.dirty = false;
        }
        if let Some(t) = &self.texture {
            let tint = egui::Color32::from_white_alpha((cx.opacity.clamp(0.0, 1.0) * 255.0) as u8);
            ui.painter().with_clip_rect(cx.rect).image(
                t.id(),
                cx.rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                tint,
            );
        }
        // ask for frames while the heat moves; a settled field stays put
        let moving = !cx.still && (changed || cx.theme.heat.drift || self.age < FORMED);
        if moving {
            ui.ctx().request_repaint();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Not a check: writes the cold opening's field at a few moments to
    /// `$MDECK_THERMAL_DUMP` (a folder) as PNGs, for tuning by eye.
    /// `MDECK_THERMAL_DUMP=/tmp/x cargo test dump_cold_opening -- --ignored`
    #[test]
    #[ignore]
    fn dump_cold_opening() {
        let Some(dir) = std::env::var_os("MDECK_THERMAL_DUMP") else {
            return;
        };
        let ctx = egui::Context::default();
        crate::render::fonts::install(&ctx);
        let theme = crate::theme::lookup::load_builtin("thermal").expect("thermal theme");
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(1920.0, 1080.0));
        // two frames so the font atlas holds the glyphs
        for frame in 0..2 {
            let mut output = ctx.run_ui(Default::default(), |ui| {
                let galley = ui.painter().layout_no_wrap(
                    "What your eyes can't see".into(),
                    egui::FontId::new(104.0, theme.display_family()),
                    egui::Color32::WHITE,
                );
                let pos = Pos2::new(960.0 - galley.size().x / 2.0, 470.0);
                let hints = vec![Hint::Text {
                    galley,
                    pos,
                    slide: 0,
                }];
                if frame == 0 {
                    return;
                }
                let stage = Stage {
                    moment: Moment::Slide,
                    index: 0,
                    reveal: 0,
                    slide: None,
                    title: true,
                    figure: None,
                    art: None,
                    hints: &hints,
                    hints_key: 1,
                    deck_title: None,
                    count: 1,
                };
                let mut lib = Library::default();
                let mut t = Thermal::new();
                let dt = 1.0 / 60.0;
                let mut now = 0.0;
                for at in [0.2f32, 0.5, 0.8, 1.2, 1.8, 2.6, 4.0] {
                    while now < at {
                        let cx = FrameCx {
                            rect,
                            scale: 1.0,
                            opacity: 1.0,
                            dt,
                            still: false,
                            theme: &theme,
                        };
                        t.prepare(ui, &cx, &stage);
                        t.update(&cx, &stage, &mut lib);
                        now += dt;
                    }
                    let f = t.field.as_ref().unwrap();
                    let img = f.image(&theme.heat.palette.lut(), BANDS);
                    let rgba: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
                    let path = std::path::Path::new(&dir).join(format!("field-{at:.1}.png"));
                    image::RgbaImage::from_raw(f.w as u32, f.h as u32, rgba)
                        .unwrap()
                        .save(path)
                        .unwrap();
                }
            });
            output.textures_delta.clear();
        }
    }

    #[test]
    fn embers_start_low_and_warm_and_are_reproducible() {
        let a = ember(3, 7);
        let b = ember(3, 7);
        assert_eq!((a.x, a.y, a.heat), (b.x, b.y, b.heat));
        assert!(a.y >= 0.55 && a.heat > 0.2 && a.vy < 0.0);
        let mut t = Thermal::new();
        let before = t.embers[0].y;
        t.drift(1.0, 0);
        assert!(
            t.embers[0].y < before || t.embers[0].heat > 0.2,
            "rises or is reborn"
        );
    }

    #[test]
    fn the_engine_forms_headings_and_traces_heat() {
        assert!(DEF.capabilities.cold_open && DEF.capabilities.heat_trace);
        assert!(DEF.capabilities.editorial && DEF.capabilities.countdown);
    }
}
