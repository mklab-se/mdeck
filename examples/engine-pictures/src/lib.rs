//! # Tutorial step 2: Pictures and moments
//!
//! A field of glowing motes that gathers into whatever the slide shows:
//!
//! - the slide's picture (a point cloud), drawn in the engine's own medium
//!   inside the box the design gives it;
//! - the countdown digit and the end words, from the masks on the stage;
//! - a low band of resting motes when there is nothing to show, kept below
//!   the copy.
//!
//! Every mote eases toward its target, so changing slides is a smooth
//! migration instead of a cut. Stills skip the easing and show the settled
//! formation, so exports stay deterministic.
//!
//! The walkthrough is `docs/sdk/tutorial-2-pictures.md`.

use mdeck_sdk::cloud::Cloud;
use mdeck_sdk::engine::{Capabilities, Engine, EngineDef};
use mdeck_sdk::paint::{Painter, Pos2, Rect, Sprite, SpriteBlend, SpriteLayer, Vec2, mix};
use mdeck_sdk::registry::{Registry, RegistryError};
use mdeck_sdk::stage::{Frame, Look, Moment, Picture, PictureSource, Stage};
use mdeck_sdk::tokens::EngineSettings;

/// The engine as mdeck registers it: `engine: pictures` in a theme.
pub static DEF: EngineDef = EngineDef::new(
    "pictures",
    "Glowing motes gather into the slide's picture, the countdown and the end.",
    create,
)
// The core resolves the slide's picture for us, hands us the countdown
// digits and plays our end act instead of its plain "The End".
.with_capabilities(
    Capabilities::NONE
        .with_picture()
        .with_countdown()
        .with_ending(),
)
// The end words take about two seconds to gather; the caption follows.
.with_ending_caption_delay(2.5);

/// The showcase theme.
pub const THEME: &str = include_str!("../themes/fireflies.yaml");

/// Register the engine and its showcase theme.
pub fn register(r: &mut Registry) -> Result<(), RegistryError> {
    r.engine(&DEF)?;
    r.theme("fireflies", THEME)
}

fn create(_settings: &EngineSettings) -> Box<dyn Engine> {
    Box::new(Motes::new())
}

/// How many motes there are. Every formation uses all of them.
pub const MOTES: usize = 700;

/// Seconds the end words stay before the motes sink away.
pub const END_WORDS: f32 = 6.0;

/// One glowing mote, in slide fractions so a resized window keeps the
/// formation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mote {
    /// Where it is.
    pub pos: [f32; 2],
    /// Where it is going.
    pub target: [f32; 2],
    /// How bright it is (0..1), and how bright it is going to be.
    pub glow: f32,
    /// Target brightness.
    pub target_glow: f32,
    /// How fast it eases (per second): motes arrive at different times.
    pub pace: f32,
}

/// What the current formation was built for: rebuilt only when it changes.
#[derive(Clone, Debug, PartialEq)]
struct Key {
    look: Look,
    index: usize,
    picture: Option<String>,
    size: [i32; 2],
}

/// The running engine.
pub struct Motes {
    motes: Vec<Mote>,
    key: Option<Key>,
    clock: f32,
    settled: bool,
    layer: SpriteLayer,
}

impl Default for Motes {
    fn default() -> Self {
        Self::new()
    }
}

impl Motes {
    /// A new field, motes resting along the bottom of the slide.
    pub fn new() -> Self {
        let motes = (0..MOTES)
            .map(|i| {
                let rest = rest(i, 0);
                Mote {
                    pos: rest,
                    target: rest,
                    glow: 0.0,
                    target_glow: 0.35,
                    pace: 1.6 + 1.6 * unit(hash(i as u64 * 7 + 3)),
                }
            })
            .collect();
        Self {
            motes,
            key: None,
            clock: 0.0,
            settled: false,
            layer: SpriteLayer::new(),
        }
    }

    /// The motes.
    pub fn motes(&self) -> &[Mote] {
        &self.motes
    }

    /// Point every mote at the formation `stage` asks for in `rect`.
    fn form(&mut self, stage: &Stage, rect: Rect) {
        let place = |i: usize, p: Pos2| -> [f32; 2] {
            let (u, v) = rect.fraction_of(p);
            // a little jitter, so motes sharing a point do not stack
            let j = |k: u64| (unit(hash(i as u64 * 31 + k)) - 0.5) * 0.004;
            [u + j(1), v + j(2)]
        };
        match &stage.moment {
            Moment::Countdown { mask, .. } => {
                let box_ = fit(rect, rect.height() * 0.55, mask.aspect, 0.8);
                for (i, m) in self.motes.iter_mut().enumerate() {
                    let [x, y] = mask.points[i % mask.points.len().max(1)];
                    m.target = place(i, box_.lerp_inside(x, y));
                    m.target_glow = 0.9;
                }
                if mask.is_empty() {
                    self.rest(stage.index);
                }
            }
            Moment::End { words, elapsed, .. } if *elapsed < END_WORDS && !words.is_empty() => {
                let box_ = fit(rect, rect.height() * 0.2, words.aspect, 0.8);
                for (i, m) in self.motes.iter_mut().enumerate() {
                    let [x, y] = words.points[i % words.points.len()];
                    m.target = place(i, box_.lerp_inside(x, y));
                    m.target_glow = 0.85;
                }
            }
            Moment::End { .. } => {
                // After the words: the motes sink below the slide and fade.
                for (i, m) in self.motes.iter_mut().enumerate() {
                    m.target = [rest(i, 0)[0], 1.08];
                    m.target_glow = 0.0;
                }
            }
            Moment::Burst { .. } => {
                // The last digit flies apart: every mote heads outward.
                for (i, m) in self.motes.iter_mut().enumerate() {
                    let a = unit(hash(i as u64 * 11)) * std::f32::consts::TAU;
                    m.target = [0.5 + 0.8 * a.cos(), 0.5 + 0.8 * a.sin()];
                    m.target_glow = 0.0;
                }
            }
            Moment::Slide => match cloud(stage.picture.as_ref()) {
                Some((cloud, pic)) => self.draw_picture(stage.index, &cloud, pic, rect),
                // No picture, or one we cannot draw in our medium (an
                // artwork or an image): rest. A slide is never left empty.
                None => self.rest(stage.index),
            },
            // A moment newer than this engine: rest.
            _ => self.rest(stage.index),
        }
    }

    /// The picture's points inside the box the design placed it in, at its
    /// own aspect. Motes beyond the cloud's points rest.
    fn draw_picture(&mut self, index: usize, cloud: &Cloud, pic: &Picture, rect: Rect) {
        let area = pic.place.in_rect(rect);
        // Cloud::aspect is height over width.
        let box_ = fit(area, area.height(), 1.0 / cloud.aspect.max(0.01), 1.0);
        // On a title slide the picture is a large, dim backdrop.
        let glow = if pic.backdrop { 0.3 } else { 0.8 };
        for (i, m) in self.motes.iter_mut().enumerate() {
            match cloud.points.get(i) {
                Some(&[x, y]) => {
                    let (u, v) = rect.fraction_of(box_.lerp_inside(x, y));
                    m.target = [u, v];
                    m.target_glow = glow;
                }
                None => {
                    m.target = rest(i, index);
                    m.target_glow = 0.25;
                }
            }
        }
    }

    /// The resting band: low on the slide, clear of the copy above it.
    fn rest(&mut self, index: usize) {
        for (i, m) in self.motes.iter_mut().enumerate() {
            m.target = rest(i, index);
            m.target_glow = 0.35;
        }
    }
}

impl Engine for Motes {
    fn update(&mut self, frame: &Frame, stage: &Stage) {
        self.settled = frame.settled();
        let rect = frame.rect;
        // Rebuild targets only when what is shown changes, not every frame.
        let key = Key {
            look: stage.moment.look(END_WORDS),
            index: stage.index,
            picture: cloud(stage.picture.as_ref()).map(|(c, _)| c.name.clone()),
            size: [rect.width() as i32, rect.height() as i32],
        };
        if self.key.as_ref() != Some(&key) {
            self.form(stage, rect);
            self.key = Some(key);
        }
        if self.settled {
            // Stills and reduced motion: the finished formation, at once.
            for m in &mut self.motes {
                m.pos = m.target;
                m.glow = m.target_glow;
            }
            return;
        }
        let dt = frame.dt.clamp(0.0, 0.1);
        self.clock += dt;
        for m in &mut self.motes {
            // Exponential easing: frame-rate independent, never overshoots.
            let k = 1.0 - (-m.pace * dt).exp();
            m.pos[0] += (m.target[0] - m.pos[0]) * k;
            m.pos[1] += (m.target[1] - m.pos[1]) * k;
            m.glow += (m.target_glow - m.glow) * k;
        }
    }

    fn paint(&mut self, painter: &mut Painter, frame: &Frame, _: &Stage) {
        let t = frame.tokens;
        let (blend, strength) = if t.light {
            (SpriteBlend::Normal, 0.6)
        } else {
            (SpriteBlend::Additive, 1.0)
        };
        let size = 9.0 * frame.scale;
        let sprites = self
            .motes
            .iter()
            .enumerate()
            .filter(|(_, m)| m.glow > 0.01)
            .map(|(i, m)| {
                let r = unit(hash(i as u64));
                // A slow shimmer while live; none in stills.
                let shimmer = if self.settled {
                    1.0
                } else {
                    0.8 + 0.2 * (self.clock * (1.0 + r) + r * 40.0).sin()
                };
                let c = mix(t.particle_light, t.accent, r * 0.6).to_f32();
                Sprite {
                    center: frame.rect.lerp_inside(m.pos[0], m.pos[1]),
                    size: size * (0.7 + 0.6 * r),
                    rgba: [
                        c[0],
                        c[1],
                        c[2],
                        m.glow * shimmer * strength * frame.opacity,
                    ],
                }
            })
            .collect();
        painter.sprites(&self.layer, sprites, blend);
    }

    fn animating(&self) -> bool {
        // The motes shimmer while live: this engine always moves, and
        // says so.
        !self.settled
    }
}

/// The slide's picture when it is a point cloud.
fn cloud(picture: Option<&Picture>) -> Option<(std::sync::Arc<Cloud>, &Picture)> {
    let pic = picture?;
    match &pic.source {
        PictureSource::Cloud(c) if !c.points.is_empty() => Some((c.clone(), pic)),
        _ => None,
    }
}

/// A box of height `h` (at most `max_w` of `area`'s width) and width over
/// height `aspect`, centred in `area`.
fn fit(area: Rect, h: f32, aspect: f32, max_w: f32) -> Rect {
    let mut size = Vec2::new(h * aspect, h);
    let limit = area.width() * max_w;
    if size.x > limit {
        size = size * (limit / size.x);
    }
    if size.y > area.height() {
        size = size * (area.height() / size.y);
    }
    Rect::from_center_size(area.center(), size)
}

/// Mote `i`'s resting place on slide `index`: a low band, its shape
/// rotating by slide number so neighbouring slides differ.
fn rest(i: usize, index: usize) -> [f32; 2] {
    let r = |k: u64| unit(hash((i as u64) * 13 + k + index as u64 * 1_000_003));
    let u = r(1);
    let wave = 0.025 * (u * 9.0 + index as f32).sin();
    [u, 0.88 + wave + 0.07 * (r(2) - 0.5) * r(3)]
}

fn hash(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

fn unit(h: u64) -> f32 {
    (h >> 40) as f32 / (1u64 << 24) as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use mdeck_sdk::cloud::Mask;
    use mdeck_sdk::stage::Place;
    use mdeck_sdk::tokens::Tokens;
    use std::sync::Arc;

    fn rect() -> Rect {
        Rect::from_min_size(Pos2::ZERO, Vec2::new(1920.0, 1080.0))
    }

    fn ring() -> Arc<Cloud> {
        let pts = (0..400)
            .map(|i| {
                let a = i as f32 * 0.37;
                [0.5 + 0.5 * a.cos(), 0.5 + 0.5 * a.sin()]
            })
            .collect();
        Arc::new(Cloud::new("ring", pts, 1.0))
    }

    fn with_picture(backdrop: bool) -> Stage<'static> {
        let mut s = Stage::new(Moment::Slide);
        let mut p = Picture::new(
            PictureSource::Cloud(ring()),
            Place {
                u: 0.55,
                v: 0.1,
                w: 0.4,
                h: 0.8,
            },
        );
        p.backdrop = backdrop;
        s.picture = Some(p);
        s
    }

    fn step(e: &mut Motes, stage: &Stage, still: bool, seconds: f32) {
        let (tokens, settings) = (Tokens::default(), EngineSettings::new());
        let mut f = Frame::new(rect(), &tokens, &settings);
        f.still = still;
        let frames = (seconds * 60.0) as usize;
        for _ in 0..frames.max(1) {
            e.update(&f, stage);
        }
    }

    #[test]
    fn the_picture_sits_in_its_place() {
        let mut e = Motes::new();
        step(&mut e, &with_picture(false), true, 0.0);
        let on: Vec<_> = e.motes().iter().filter(|m| m.glow > 0.5).collect();
        assert_eq!(on.len(), 400, "one mote per cloud point");
        for m in on {
            let [u, v] = m.pos;
            assert!(
                (0.54..=0.96).contains(&u) && (0.09..=0.91).contains(&v),
                "{u},{v}"
            );
        }
    }

    #[test]
    fn a_backdrop_is_dim() {
        let mut e = Motes::new();
        step(&mut e, &with_picture(true), true, 0.0);
        assert!(e.motes().iter().all(|m| m.glow <= 0.31));
    }

    #[test]
    fn without_a_picture_the_motes_rest_below_the_copy() {
        let mut e = Motes::new();
        step(&mut e, &Stage::new(Moment::Slide), true, 0.0);
        assert!(e.motes().iter().all(|m| m.pos[1] > 0.8));
    }

    #[test]
    fn motes_ease_between_slides() {
        let mut e = Motes::new();
        step(&mut e, &Stage::new(Moment::Slide), true, 0.0);
        let stage = with_picture(false);
        step(&mut e, &stage, false, 0.1);
        let moving = e.motes().iter().filter(|m| m.pos != m.target).count();
        assert!(moving > 300, "still on their way after 0.1 s");
        step(&mut e, &stage, false, 6.0);
        let far = e
            .motes()
            .iter()
            .map(|m| (m.pos[0] - m.target[0]).abs() + (m.pos[1] - m.target[1]).abs())
            .fold(0.0, f32::max);
        assert!(far < 0.01, "arrived after 6 s, {far}");
    }

    #[test]
    fn the_countdown_digit_is_centred() {
        let mask = Mask::new(vec![[0.0, 0.0], [1.0, 1.0], [0.5, 0.5]], 0.6);
        let stage = Stage::new(Moment::countdown(3, mask, 0.0));
        let mut e = Motes::new();
        step(&mut e, &stage, true, 0.0);
        let xs: Vec<f32> = e.motes().iter().map(|m| m.pos[0]).collect();
        let (lo, hi) = xs
            .iter()
            .fold((1.0f32, 0.0f32), |(a, b), &x| (a.min(x), b.max(x)));
        assert!((lo + hi - 1.0).abs() < 0.02, "centred: {lo}..{hi}");
    }

    #[test]
    fn after_the_end_words_everything_fades() {
        let words = Mask::new(vec![[0.5, 0.5]], 4.0);
        let stage = Stage::new(Moment::end(END_WORDS + 1.0, words));
        let mut e = Motes::new();
        step(&mut e, &stage, true, 0.0);
        assert!(e.motes().iter().all(|m| m.glow == 0.0));
    }

    #[test]
    fn an_artwork_falls_back_to_resting() {
        let mut stage = with_picture(false);
        if let Some(p) = &mut stage.picture {
            p.source = PictureSource::Image(Arc::new(mdeck_sdk::paint::ImageData::default()));
        }
        let mut e = Motes::new();
        step(&mut e, &stage, true, 0.0);
        assert!(e.motes().iter().all(|m| m.pos[1] > 0.8));
    }

    #[test]
    fn registers() {
        let mut r = Registry::new();
        register(&mut r).unwrap();
        assert!(r.engine_def("pictures").unwrap().capabilities.picture);
    }
}
