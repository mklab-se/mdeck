//! Scene description: declarative data saying what every particle should
//! be doing on a slide. The field itself knows nothing about markdown.

#![cfg_attr(
    not(all_engines),
    allow(
        dead_code,
        reason = "story staging builds scenes in every build; only the particles engine reads all of them"
    )
)]

use std::sync::Arc;

use eframe::egui::Color32;

use super::Rng;

/// Which colour a particle glows. Mixes are chosen per group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tint {
    Ember,
    Flame,
    White,
    Candle,
    Pale,
}

impl Tint {
    pub(super) fn index(self) -> usize {
        match self {
            Tint::Ember => 0,
            Tint::Flame => 1,
            Tint::White => 2,
            Tint::Candle => 3,
            Tint::Pale => 4,
        }
    }
}

/// The colour of each [`Tint`], in `Tint::index` order. The theme sets them
/// (accent, soft accent, particle light, secondary, particle cool); these are
/// Ember's.
pub const DEFAULT_TINTS: [Color32; 5] = [
    Color32::from_rgb(0xFF, 0x4D, 0x1C),
    Color32::from_rgb(0xFF, 0x8A, 0x66),
    Color32::from_rgb(0xD7, 0xD7, 0xE1),
    Color32::from_rgb(0xF5, 0xA6, 0x23),
    Color32::from_rgb(0xAF, 0xC3, 0xF0),
];

/// Colour mix of a group.
#[derive(Clone, Copy, Debug)]
pub enum Palette {
    /// The site's default mix: mostly white dust with ember and a little candle.
    Site,
    /// Ember, flame and candle only.
    Warm,
    /// White and pale only.
    Cold,
    /// One colour.
    Solid(Tint),
}

impl Palette {
    pub(super) fn pick(self, rng: &mut Rng) -> Tint {
        let r = rng.unit();
        match self {
            Palette::Site => {
                if r < 0.40 {
                    Tint::Ember
                } else if r < 0.52 {
                    Tint::Flame
                } else if r < 0.94 {
                    Tint::White
                } else {
                    Tint::Candle
                }
            }
            Palette::Warm => {
                if r < 0.55 {
                    Tint::Ember
                } else if r < 0.85 {
                    Tint::Flame
                } else {
                    Tint::Candle
                }
            }
            Palette::Cold => {
                if r < 0.7 {
                    Tint::White
                } else {
                    Tint::Pale
                }
            }
            Palette::Solid(t) => t,
        }
    }
}

/// Where a group's particles live. Coordinates are fractions of the slide
/// rect (`u` across, `v` down); radii are fractions of `min(width, height)`.
#[derive(Clone, Debug)]
pub enum Home {
    /// A soft round cluster; `falloff` < 1 packs particles toward the centre.
    Cluster {
        u: f32,
        v: f32,
        r: f32,
        falloff: f32,
    },
    /// Uniformly scattered over a region.
    Field { u0: f32, v0: f32, u1: f32, v1: f32 },
    /// Sampled from a set of normalised points (a logo, a glyph) fitted into
    /// the box `(u, v, w, h)`.
    Mask {
        points: Arc<Vec<[f32; 2]>>,
        u: f32,
        v: f32,
        w: f32,
        h: f32,
    },
    /// Along a polyline in slide fractions; `spread` (fraction of min side)
    /// scatters particles across the line.
    Path { points: Vec<[f32; 2]>, spread: f32 },
    /// Straight out from `(u, v)` along each particle's own direction, to a
    /// distance of `r` (fraction of min side): an explosion.
    Radial { u: f32, v: f32, r: f32 },
    /// A thin ring of radius `r` and thickness `width` (fractions of min side).
    Ring { u: f32, v: f32, r: f32, width: f32 },
}

/// Per-frame motion applied on top of the home.
#[derive(Clone, Copy, Debug)]
pub enum Drift {
    /// Slow Lissajous wander; `amp` in fractions of min(w, h).
    Breathe { amp: f32, speed: f32 },
    /// Falls through its field and wraps to the top.
    Fall { speed: f32 },
    /// Rises through its field and wraps to the bottom.
    Rise { speed: f32 },
    /// Holds still apart from a faint shimmer.
    Still,
    /// Runs along its [`Home::Path`] and wraps; `speed` is passes per 4 s.
    Flow { speed: f32 },
    /// Circles the slide centre at its home's radius; `speed` in radians per
    /// second, scaled per particle so the field swirls rather than rotates.
    Orbit { speed: f32 },
    /// Moving forward through space: each particle slides away from the
    /// vanishing point `(u, v)` along the line through its home, gaining
    /// speed as it nears the edge (parallax: bigger, brighter ones run
    /// faster), fades out, and is reborn near the centre. `speed` is
    /// crossings per second for an average particle.
    Forward { u: f32, v: f32, speed: f32 },
}

#[derive(Clone, Debug)]
pub struct Group {
    /// Fraction of the pool assigned to this group (shares are normalised).
    pub share: f32,
    pub home: Home,
    pub drift: Drift,
    pub palette: Palette,
    /// Base brightness range.
    pub alpha: (f32, f32),
    /// Sprite diameter multiplier range (1.0 ≈ the site's default dust).
    pub size: (f32, f32),
    /// Reveal step that lights this group; `None` is always lit.
    pub step: Option<usize>,
    /// Connect each particle to its nearest neighbours in the group with a
    /// hairline (only meaningful for clusters).
    pub links: u8,
    /// Brightness while the group's step has not been reached (0 hides it).
    pub dim: f32,
    /// Reveal step from which the group glows hot (brighter, ember-tinted).
    pub hot_step: Option<usize>,
}

impl Group {
    pub fn new(share: f32, home: Home) -> Self {
        Self {
            share,
            home,
            drift: Drift::Breathe {
                amp: 0.010,
                speed: 1.15,
            },
            palette: Palette::Site,
            alpha: (0.10, 0.35),
            size: (0.4, 0.9),
            step: None,
            links: 0,
            dim: 0.10,
            hot_step: None,
        }
    }
    pub fn dim(mut self, dim: f32) -> Self {
        self.dim = dim;
        self
    }
    pub fn hot(mut self, step: usize) -> Self {
        self.hot_step = Some(step);
        self
    }
    pub fn drift(mut self, d: Drift) -> Self {
        self.drift = d;
        self
    }
    pub fn palette(mut self, p: Palette) -> Self {
        self.palette = p;
        self
    }
    pub fn alpha(mut self, lo: f32, hi: f32) -> Self {
        self.alpha = (lo, hi);
        self
    }
    pub fn size(mut self, lo: f32, hi: f32) -> Self {
        self.size = (lo, hi);
        self
    }
    pub fn step(mut self, step: usize) -> Self {
        self.step = Some(step);
        self
    }
    pub fn links(mut self, n: u8) -> Self {
        self.links = n;
        self
    }
}

/// A complete scene: what every particle should be doing on this slide.
#[derive(Clone, Debug, Default)]
pub struct Scene {
    pub groups: Vec<Group>,
    /// Tint of the hairlines.
    pub link_alpha: f32,
    /// How fast group brightness eases toward its target (per second).
    pub life_rate: f32,
}

impl Scene {
    pub fn new(groups: Vec<Group>) -> Self {
        Self {
            groups,
            link_alpha: 0.10,
            life_rate: LIFE_RATE,
        }
    }
}

/// Easing of a group's brightness toward its reveal target.
pub(super) const LIFE_RATE: f32 = 3.2;
