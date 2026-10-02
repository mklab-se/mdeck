//! The thermal engine: the deck seen through a thermal instrument. A heat
//! field lies under the slides, drawn in the `palette` setting's colours in
//! contour bands and transparent where it is cold.
//!
//! - **The cold opening.** On title and section slides the heading forms in
//!   heat: points glow inside the letters, spread and join into contours,
//!   and the words are readable within a second. Then the crisp type rises
//!   into it (the core holds the copy back [`COLD_OPEN_HOLD`], see
//!   [`Engine::copy_hold`]) and the heat settles into a faint contour halo
//!   that stays. The heading comes from the slide's
//!   [`Hint::Text`] geometry, rasterised with [`Painter::glyph_ink`].
//! - **Heat signatures.** A picture glows like a warm body; the countdown
//!   digits heat up and cool off; the end words glow and fade.
//! - **Calm evidence.** Where a slide shows charts, images or diagrams the
//!   field stays dark. With `drift: true` a few embers drift through the
//!   dark on ordinary slides, cooling as they rise.
//! - **Cooling between slides.** Nothing is cleared on a slide change: the
//!   old heat cools while the new heat builds.
//! - **The heat trace.** The presenter's pen strokes arrive white-hot and
//!   cool through the palette ([`Engine::annotate`], `trace.rs`).
//!
//! Exports and reduced motion show the settled field.

mod field;
mod trace;

use mdeck_sdk::cloud::Mask;
use mdeck_sdk::engine::{
    Annotation, Capabilities, Engine, EngineDef, Needs, SettingKind, SettingSpec,
};
use mdeck_sdk::geometry::Hint;
use mdeck_sdk::paint::{Color, Painter, Pos2, Rect, Texture, TextureFilter};
use mdeck_sdk::stage::{Frame, Look, Moment, PictureSource, Stage};
use mdeck_sdk::tokens::EngineSettings;

use crate::engines::hash01;
use crate::engines::heat_palette::Palette;
use field::{Field, smooth};

/// Seconds into the end slide when the caption fades in: the words have
/// glowed and cooled.
pub const END_CAPTION_DELAY: f32 = 4.4;
/// Seconds the core holds the copy of a title or section slide back while
/// the heading forms in heat.
pub const COLD_OPEN_HOLD: f32 = 1.5;
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
/// Atlas coverage a glyph pixel needs to heat its cell.
const INK_ALPHA: u8 = 100;
/// The simulation step a late heading is replayed at (see
/// [`Thermal::replay`]).
const REPLAY_DT: f32 = 1.0 / 60.0;
/// A heading that arrives later than this into its moment is not replayed
/// (the replay's cost is bounded); the next frames heat it as they go.
const REPLAY_MAX: f32 = 10.0;

const PALETTES: &[&str] = &[
    "iron",
    "white-hot",
    "black-hot",
    "rainbow",
    "arctic",
    "lava",
];

pub static DEF: EngineDef = EngineDef {
    name: "thermal",
    summary: "A heat field under the slides: headings form in heat, pictures glow like warm bodies.",
    capabilities: Capabilities {
        picture: true,
        countdown: true,
        ending: true,
        ..Capabilities::NONE
    },
    settings: &[
        SettingSpec {
            key: "palette",
            kind: SettingKind::OneOf(PALETTES),
            summary: "The palette the heat glows in (default iron).",
        },
        SettingSpec {
            key: "drift",
            kind: SettingKind::Bool,
            summary: "Embers drift through the dark on ordinary slides (default false).",
        },
    ],
    needs: Needs { page: false },
    ending_caption_delay: END_CAPTION_DELAY,
    create: |s| Box::new(Thermal::new(s)),
    board: None,
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
    palette: Palette,
    drift: bool,
    field: Option<Field>,
    key: Option<(usize, Look)>,
    /// Seconds since the current moment began.
    age: f32,
    /// The field and embers as the current moment began, so a heading that
    /// arrives late (its glyphs are sampled in `paint`) can be replayed
    /// from the start.
    start: Option<(Vec<f32>, Vec<Ember>)>,
    /// The moment has not had its heading sampled yet.
    fresh: bool,
    heading: Option<HeadingMask>,
    embers: Vec<Ember>,
    texture: Option<Texture>,
    /// The field changed since the texture was last uploaded.
    dirty: bool,
    /// The heat still moves: ask for another frame.
    moving: bool,
}

impl Thermal {
    pub fn new(settings: &EngineSettings) -> Self {
        let palette = settings
            .one_of("palette", PALETTES)
            .and_then(Palette::from_name)
            .unwrap_or(Palette::DEFAULT);
        Self {
            palette,
            drift: settings.bool_or("drift", false),
            field: None,
            key: None,
            age: 0.0,
            start: None,
            fresh: false,
            heading: None,
            embers: (0..EMBERS).map(|i| ember(i as u32, 0)).collect(),
            texture: None,
            dirty: true,
            moving: true,
        }
    }

    /// Set the sources for this moment, `age` seconds into it.
    fn sources(&mut self, frame: &Frame, stage: &Stage) {
        let Some(field) = self.field.as_mut() else {
            return;
        };
        field.clear_sources();
        let rect = frame.rect;
        let (fw, fh) = (field.w as f32, field.h as f32);
        let to_cell = |p: Pos2| {
            (
                (p.x - rect.left()) / rect.width() * fw,
                (p.y - rect.top()) / rect.height() * fh,
            )
        };
        // charts, images and diagrams stay dark
        for hint in stage.geometry {
            if let Hint::Frame(r) = hint {
                let (x0, y0) = to_cell(r.min);
                let (x1, y1) = to_cell(r.max);
                field.keep_calm(x0, y0, x1, y1);
            }
        }
        let age = self.age;
        // a mask in the unit square, `height` of the slide tall, centred
        let stamp_mask = |field: &mut Field, mask: &Mask, height: f32, v: f32| {
            let h = height * fh;
            let w = h * mask.aspect * rect.height() / rect.width() * fw / fh;
            let (x0, y0) = ((fw - w) / 2.0, (fh - h) / 2.0);
            for p in mask.points.iter() {
                field.disc(x0 + p[0] * w, y0 + p[1] * h, 1.1, v);
            }
        };
        match &stage.moment {
            Moment::Countdown { mask, progress, .. } => {
                let v = smooth(0.0, 0.3, *progress) * (1.0 - 0.85 * smooth(0.75, 1.0, *progress));
                stamp_mask(field, mask, 0.5, v);
            }
            Moment::Burst { .. } => {}
            Moment::End { words, elapsed } => {
                let v = end_heat(*elapsed, frame.settled());
                if v > 0.0 {
                    stamp_mask(field, words, 0.16, v);
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
                if let Some(pic) = &stage.picture
                    && let PictureSource::Cloud(cloud) = &pic.source
                {
                    let p = pic.place;
                    let v = if pic.backdrop { 0.32 } else { 0.72 };
                    let n = cloud.points.len().min(1400);
                    for pt in &cloud.points[..n] {
                        let x = (p.u + pt[0] * p.w) * fw;
                        let y = (p.v + pt[1] * p.h) * fh;
                        field.disc(x, y, 0.9, v);
                    }
                }
                if self.drift {
                    for e in &self.embers {
                        field.disc(e.x * fw, e.y * fh, 1.3, e.heat * 0.55);
                    }
                }
            }
        }
    }

    /// Build the heading's cells from the slide's text geometry, sampling
    /// the glyphs' ink. Returns whether the mask changed.
    fn heading_mask(&mut self, painter: &Painter, frame: &Frame, stage: &Stage) -> bool {
        let text: Vec<_> = stage
            .geometry
            .iter()
            .filter_map(|h| match h {
                Hint::Text {
                    text,
                    font,
                    pos,
                    slide,
                    ..
                } if *slide == stage.index => Some((text, *font, *pos)),
                _ => None,
            })
            .collect();
        if text.is_empty() {
            return self.heading.take().is_some();
        }
        let key = stage.geometry_key ^ (stage.index as u64).wrapping_mul(0x9E37_79B9);
        if self.heading.as_ref().is_some_and(|h| h.key == key) {
            return false;
        }
        let Some(field) = self.field.as_ref() else {
            return false;
        };
        let rect = frame.rect;
        let (fw, fh) = (field.w, field.h);
        let mut on = vec![false; fw * fh];
        for (text, font, pos) in text {
            // every pixel of the glyphs' ink, into its field cell
            for p in painter.glyph_ink(text, font, INK_ALPHA) {
                let (px, py) = (pos.x + p.x, pos.y + p.y);
                let cx = ((px - rect.left()) / rect.width() * fw as f32) as isize;
                let cy = ((py - rect.top()) / rect.height() * fh as f32) as isize;
                if cx >= 0 && cy >= 0 && (cx as usize) < fw && (cy as usize) < fh {
                    on[cy as usize * fw + cx as usize] = true;
                }
            }
        }
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
    fn ensure_field(&mut self, rect: Rect) {
        let aspect = rect.width() / rect.height().max(1.0);
        if self
            .field
            .as_ref()
            .is_none_or(|f| ((f.w as f32 / f.h as f32) - aspect).abs() > 0.05)
        {
            self.field = Some(Field::new(aspect));
            self.heading = None;
            self.start = None;
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

    /// How fast heat spreads now.
    fn spread(&self, stage: &Stage) -> f32 {
        let forming = self.heading.is_some()
            && matches!(stage.moment, Moment::Slide)
            && self.age < COLD_OPEN_HOLD + 1.2;
        if forming {
            let t = smooth(COLD_OPEN_HOLD - 0.2, COLD_OPEN_HOLD + 1.2, self.age);
            SPREAD_FORMING + (SPREAD_SETTLED - SPREAD_FORMING) * t
        } else if self.heading.is_some() {
            SPREAD_SETTLED
        } else if stage
            .picture
            .as_ref()
            .is_some_and(|p| matches!(p.source, PictureSource::Cloud(_)))
        {
            // a body reads as one warm shape, its detail glowing through
            SPREAD_FIGURE
        } else {
            field::SPREAD
        }
    }

    /// Heat the field from this moment's sources and let it move by `dt`
    /// (settle it at once for a still).
    fn advance(&mut self, frame: &Frame, stage: &Stage, dt: f32) {
        self.sources(frame, stage);
        let spread = self.spread(stage);
        if let Some(f) = self.field.as_mut() {
            f.spread = spread;
            if frame.settled() {
                f.settle();
                self.dirty = true;
            } else if f.step(dt) > 1e-4 {
                self.dirty = true;
            }
        }
    }

    /// The heading's glyphs are sampled in `paint` (they need the painter),
    /// a frame after the slide arrived, or after the whole run of an
    /// `export --at` rehearsal: run the moment again from its start with
    /// the heading, so the cold opening is where it would have been.
    fn replay(&mut self, frame: &Frame, stage: &Stage) {
        let Some((heat, embers)) = self.start.clone() else {
            return;
        };
        let elapsed = self.age;
        if let Some(f) = self.field.as_mut() {
            f.heat.copy_from_slice(&heat);
        }
        self.embers = embers;
        let steps = (elapsed / REPLAY_DT).round() as usize;
        let dt = if steps > 0 {
            elapsed / steps as f32
        } else {
            0.0
        };
        self.age = 0.0;
        for _ in 0..steps {
            self.age += dt;
            if self.drift {
                self.drift(dt, stage.index as u32);
            }
            self.advance(frame, stage, dt);
        }
        self.age = elapsed;
        self.dirty = true;
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

/// The heat the end words are stamped with. They warm up over half a
/// second and cool after [`END_WORDS`]; a still shows them at full glow, so
/// an exported end slide looks finished instead of black (D26).
fn end_heat(elapsed: f32, still: bool) -> f32 {
    if elapsed >= END_WORDS {
        0.0
    } else if still {
        1.0
    } else {
        smooth(0.0, 0.5, elapsed)
    }
}

impl Engine for Thermal {
    fn update(&mut self, frame: &Frame, stage: &Stage) {
        self.ensure_field(frame.rect);
        let key = (stage.index, stage.moment.look(END_WORDS));
        if self.key != Some(key) {
            self.key = Some(key);
            self.age = 0.0;
            self.start = self
                .field
                .as_ref()
                .map(|f| (f.heat.clone(), self.embers.clone()));
            self.fresh = true;
        }
        if frame.settled() {
            // the settled look: the opening long over
            self.age = 60.0;
        } else {
            self.age += frame.dt;
            if self.drift {
                self.drift(frame.dt, stage.index as u32);
            }
        }
        self.advance(frame, stage, frame.dt);
    }

    fn paint(&mut self, painter: &mut Painter, frame: &Frame, stage: &Stage) {
        self.ensure_field(frame.rect);
        if self.heading_mask(painter, frame, stage) {
            if frame.settled() {
                self.advance(frame, stage, 0.0);
            } else if self.fresh && self.age <= REPLAY_MAX {
                self.replay(frame, stage);
            }
        }
        if self.heading.is_some() {
            self.fresh = false;
        }
        let Some(field) = self.field.as_ref() else {
            return;
        };
        let changed = self.dirty;
        if self.dirty || self.texture.is_none() {
            let image = field.image(&self.palette.lut(), BANDS);
            match &mut self.texture {
                Some(t) => painter.update_texture(t, &image, TextureFilter::Linear),
                None => {
                    self.texture = Some(painter.load_texture(
                        "mdeck-thermal-field",
                        &image,
                        TextureFilter::Linear,
                    ))
                }
            }
            self.dirty = false;
        }
        if let Some(t) = &self.texture {
            let a = (frame.opacity.clamp(0.0, 1.0) * 255.0) as u8;
            painter.with_clip(frame.rect).image(
                t,
                frame.rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color::from_rgba_premultiplied(a, a, a, a),
            );
        }
        // ask for frames while the heat moves; a settled field stays put
        self.moving = !frame.settled() && (changed || self.drift || self.age < FORMED);
    }

    fn annotate(&mut self, painter: &mut Painter, _frame: &Frame, strokes: &[Annotation]) -> bool {
        trace::draw(painter, strokes, &self.palette.lut());
        true
    }

    fn copy_hold(&self) -> f32 {
        COLD_OPEN_HOLD
    }

    fn animating(&self) -> bool {
        self.moving
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mdeck_sdk::tokens::{Tokens, Value};

    fn thermal() -> Thermal {
        Thermal::new(&EngineSettings::new())
    }

    #[test]
    fn a_still_of_the_end_shows_the_words_glowing() {
        // export enters the end slide and takes the still at once
        assert_eq!(end_heat(0.0, true), 1.0);
        assert!(end_heat(0.0, false) < 0.01, "live, the words warm up");
        assert!((end_heat(1.0, false) - 1.0).abs() < 1e-6);
        assert_eq!(end_heat(END_WORDS + 0.1, false), 0.0, "then they cool");
    }

    #[test]
    fn embers_start_low_and_warm_and_are_reproducible() {
        let a = ember(3, 7);
        let b = ember(3, 7);
        assert_eq!((a.x, a.y, a.heat), (b.x, b.y, b.heat));
        assert!(a.y >= 0.55 && a.heat > 0.2 && a.vy < 0.0);
        let mut t = thermal();
        let before = t.embers[0].y;
        t.drift(1.0, 0);
        assert!(
            t.embers[0].y < before || t.embers[0].heat > 0.2,
            "rises or is reborn"
        );
    }

    #[test]
    fn the_engine_forms_headings_and_shows_pictures() {
        assert!(DEF.capabilities.picture && DEF.capabilities.countdown);
        assert!(DEF.capabilities.ending && !DEF.capabilities.board);
        assert_eq!(thermal().copy_hold(), COLD_OPEN_HOLD);
    }

    #[test]
    fn settings_pick_the_palette_and_the_drift() {
        let s = EngineSettings::from_pairs([
            ("palette", Value::String("Lava".into())),
            ("drift", Value::Bool(true)),
        ]);
        let t = Thermal::new(&s);
        assert_eq!(t.palette, Palette::Lava);
        assert!(t.drift);
        assert!(s.problems().is_empty());
        let t = thermal();
        assert_eq!((t.palette, t.drift), (Palette::Iron, false));
        let bad = EngineSettings::from_pairs([("palette", Value::String("plasma".into()))]);
        assert!(DEF.check_settings(&bad).len() == 1);
    }

    #[test]
    fn a_still_settles_and_stops_asking_for_frames() {
        let tokens = Tokens::default();
        let settings = EngineSettings::new();
        let rect = Rect::from_min_size(Pos2::ZERO, mdeck_sdk::paint::Vec2::new(1920.0, 1080.0));
        let mut f = Frame::new(rect, &tokens, &settings);
        f.still = true;
        let mut t = thermal();
        let points = (0..400)
            .map(|k| [(k % 20) as f32 / 19.0, (k / 20) as f32 / 19.0])
            .collect();
        let words = Mask::new(points, 4.0);
        let stage = Stage::new(Moment::End {
            elapsed: 0.0,
            words,
        });
        t.update(&f, &stage);
        assert!(t.field.as_ref().unwrap().peak() > 0.5, "the end words glow");
        assert_eq!(t.age, 60.0);
    }

    #[test]
    fn a_late_heading_is_replayed_from_the_moment_start() {
        let tokens = Tokens::default();
        let settings = EngineSettings::new();
        let rect = Rect::from_min_size(Pos2::ZERO, mdeck_sdk::paint::Vec2::new(1920.0, 1080.0));
        let f = Frame::new(rect, &tokens, &settings);
        let stage = Stage::new(Moment::Slide);
        let mut t = thermal();
        for _ in 0..30 {
            t.update(&f, &stage);
        }
        assert!(t.field.as_ref().unwrap().peak() < 1e-6, "no heading yet");
        // the heading arrives half a second in
        t.heading = Some(HeadingMask {
            key: 1,
            cells: (100..140)
                .flat_map(|x| (80..90).map(move |y| (x, y, 0.5)))
                .collect(),
        });
        t.replay(&f, &stage);
        assert!((t.age - 0.5).abs() < 1e-3, "the clock is kept");
        assert!(t.field.as_ref().unwrap().peak() > 0.3, "the letters glow");
    }
}
