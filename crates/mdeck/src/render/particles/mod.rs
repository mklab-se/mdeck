//! The particle field behind Ember slides.
//!
//! A fixed pool of glowing particles morphs from scene to scene: each scene
//! assigns every particle a *home* (a point it drifts around), and particles
//! ease toward their homes, so a slide change is a migration rather than a
//! cut. Scenes are declarative data ([`Scene`]) built from slide content in
//! [`scenes`]; the field itself knows nothing about markdown.
//!
//! Rendering happens in two layers: hairline links between neighbouring
//! particles are drawn with the egui painter, and the glow sprites are drawn
//! additively by [`gl::GlowRenderer`].

pub mod gl;
pub mod scenes;

use std::sync::Arc;

use eframe::egui::{self, Color32, Pos2, Rect};

use gl::Sprite;

// ---------------------------------------------------------------------------
// Deterministic randomness (exports must be reproducible)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    fn next_u64(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.unit()
    }
}

// ---------------------------------------------------------------------------
// Scene description
// ---------------------------------------------------------------------------

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
    fn rgb(self) -> [f32; 3] {
        match self {
            Tint::Ember => [1.0, 0.302, 0.110],    // #ff4d1c
            Tint::Flame => [1.0, 0.541, 0.400],    // #ff8a66
            Tint::White => [0.843, 0.843, 0.882],  // #d7d7e1
            Tint::Candle => [0.961, 0.651, 0.137], // #f5a623
            Tint::Pale => [0.686, 0.765, 0.941],   // #afc3f0
        }
    }
}

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
    fn pick(self, rng: &mut Rng) -> Tint {
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
                amp: 0.008,
                speed: 1.0,
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

// ---------------------------------------------------------------------------
// The field
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct Particle {
    x: f32,
    y: f32,
    /// Home (rest position) in points.
    hx: f32,
    hy: f32,
    /// Current target including drift.
    tx: f32,
    ty: f32,
    /// Easing factor toward the target (per 60 Hz frame).
    k: f32,
    /// Base diameter in points at reference scale, before the group multiplier.
    base_size: f32,
    size_mul: f32,
    phase: f32,
    speed: f32,
    base_alpha: f32,
    alpha: f32,
    tint: Tint,
    group: usize,
    /// Position through a field or along a path, 0..1.
    along: f32,
    /// Lateral offset across a path, -1..1.
    lateral: f32,
}

/// Easing of a group's brightness toward its reveal target.
const LIFE_RATE: f32 = 3.2;
/// Particles in the presentation window (the site uses 520 on desktop).
pub const DEFAULT_COUNT: usize = 900;
/// Reference slide width the site's pixel sizes were tuned for.
const REF_WIDTH: f32 = 1440.0;
/// Gain over the site's values: decks are watched on projectors that are
/// dimmer than a laptop screen, so sprites are larger and brighter.
const SIZE_GAIN: f32 = 1.4;
const ALPHA_GAIN: f32 = 1.35;

pub struct Field {
    particles: Vec<Particle>,
    rng: Rng,
    scene: Scene,
    rect: Rect,
    /// Brightness of each group (eases toward 1 or the dimmed level).
    life: Vec<f32>,
    /// Heat of each group (0 normal, 1 glowing hot), eased.
    heat: Vec<f32>,
    /// Neighbour hairlines as particle index pairs.
    links: Vec<(usize, usize)>,
    /// 0 right after a scene change, easing to 1: fades links in.
    scene_fade: f32,
    time: f32,
    renderer: gl::GlowRenderer,
}

impl Field {
    pub fn new(count: usize, seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let particles = (0..count)
            .map(|_| Particle {
                x: 0.0,
                y: 0.0,
                hx: 0.0,
                hy: 0.0,
                tx: 0.0,
                ty: 0.0,
                k: rng.range(0.03, 0.08),
                base_size: rng.range(0.9, 2.4) * 4.6,
                size_mul: 1.0,
                phase: rng.range(0.0, std::f32::consts::TAU),
                speed: rng.range(0.4, 1.0),
                base_alpha: 0.0,
                alpha: 0.0,
                tint: Tint::White,
                group: 0,
                along: rng.unit(),
                lateral: rng.range(-1.0, 1.0),
            })
            .collect();
        Self {
            particles,
            rng,
            scene: Scene::default(),
            rect: Rect::NOTHING,
            life: Vec::new(),
            heat: Vec::new(),
            links: Vec::new(),
            scene_fade: 1.0,
            time: 0.0,
            renderer: gl::GlowRenderer::default(),
        }
    }

    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// Current brightness (0..1) of group `gi`, for drawing things that
    /// belong to it (labels) with the same fade.
    pub fn group_life(&self, gi: usize) -> f32 {
        self.life.get(gi).copied().unwrap_or(0.0)
    }

    /// Current heat (0..1) of group `gi`.
    pub fn group_heat(&self, gi: usize) -> f32 {
        self.heat.get(gi).copied().unwrap_or(0.0)
    }

    /// Scatter every particle randomly over `rect` (initial state, so the
    /// first scene assembles out of dust instead of out of one corner).
    pub fn scatter(&mut self, rect: Rect) {
        self.rect = rect;
        for p in &mut self.particles {
            p.x = rect.left() + self.rng.unit() * rect.width();
            p.y = rect.top() + self.rng.unit() * rect.height();
            p.tx = p.x;
            p.ty = p.y;
        }
    }

    /// Install a new scene for a slide drawn in `rect`. `seed` makes the
    /// particle assignment reproducible per slide.
    pub fn set_scene(&mut self, scene: Scene, rect: Rect, seed: u64) {
        self.rect = rect;
        self.rng = Rng::new(seed ^ 0xA5A5_5A5A);
        let n = self.particles.len();
        let total_share: f32 = scene.groups.iter().map(|g| g.share.max(0.0)).sum();
        let total_share = if total_share <= 0.0 { 1.0 } else { total_share };

        // Assign contiguous index ranges to groups by share.
        let mut start = 0usize;
        let mut ranges: Vec<(usize, usize)> = Vec::with_capacity(scene.groups.len());
        for (gi, g) in scene.groups.iter().enumerate() {
            let count = if gi + 1 == scene.groups.len() {
                n - start
            } else {
                ((g.share.max(0.0) / total_share) * n as f32).round() as usize
            };
            let end = (start + count).min(n);
            ranges.push((start, end));
            start = end;
        }

        // Shuffle which particles land in which group so a group is not always
        // the same physical particles (keeps morphs lively).
        let mut order: Vec<usize> = (0..n).collect();
        for i in (1..n).rev() {
            let j = (self.rng.unit() * (i + 1) as f32) as usize;
            order.swap(i, j.min(i));
        }

        let min_side = rect.width().min(rect.height());
        for (gi, g) in scene.groups.iter().enumerate() {
            let (s, e) = ranges[gi];
            for &pi in &order[s..e] {
                let p = &mut self.particles[pi];
                p.group = gi;
                p.tint = g.palette.pick(&mut self.rng);
                p.base_alpha = self.rng.range(g.alpha.0, g.alpha.1);
                p.size_mul = self.rng.range(g.size.0, g.size.1);
                p.along = self.rng.unit();
                p.lateral = self.rng.range(-1.0, 1.0);
                let (hx, hy) = match &g.home {
                    Home::Radial { u, v, r } => {
                        let cx = rect.left() + u * rect.width();
                        let cy = rect.top() + v * rect.height();
                        let (dx, dy) = (p.x - cx, p.y - cy);
                        let len = (dx * dx + dy * dy).sqrt().max(1.0);
                        let d = r * min_side * self.rng.range(0.7, 1.3);
                        (cx + dx / len * d, cy + dy / len * d)
                    }
                    home => home_point(home, rect, min_side, &mut self.rng),
                };
                p.hx = hx;
                p.hy = hy;
            }
        }

        // Hairlines: k nearest neighbours within each linked group.
        self.links.clear();
        for (gi, g) in scene.groups.iter().enumerate() {
            if g.links == 0 {
                continue;
            }
            let (s, e) = ranges[gi];
            let members = &order[s..e];
            for &i in members {
                let (ix, iy) = (self.particles[i].hx, self.particles[i].hy);
                let mut near: Vec<(f32, usize)> = members
                    .iter()
                    .filter(|&&j| j != i)
                    .map(|&j| {
                        let dx = ix - self.particles[j].hx;
                        let dy = iy - self.particles[j].hy;
                        (dx * dx + dy * dy, j)
                    })
                    .collect();
                near.sort_by(|a, b| a.0.total_cmp(&b.0));
                for &(_, j) in near.iter().take(g.links as usize) {
                    if i < j {
                        self.links.push((i, j));
                    }
                }
            }
        }

        // Groups that were already lit stay lit; new groups start dark and
        // fade in, so a scene change breathes instead of popping.
        let old_life = std::mem::take(&mut self.life);
        self.life = (0..scene.groups.len())
            .map(|gi| old_life.get(gi).copied().unwrap_or(0.0))
            .collect();
        self.heat = vec![0.0; scene.groups.len()];
        self.scene = scene;
        self.scene_fade = 0.0;
    }

    /// Jump every particle to its target and light every revealed group
    /// (used for export, where there is no animation).
    pub fn settle(&mut self, reveal_step: usize) {
        for (gi, g) in self.scene.groups.iter().enumerate() {
            self.life[gi] = life_target(g, reveal_step);
            self.heat[gi] = heat_target(g, reveal_step);
        }
        self.scene_fade = 1.0;
        // One tick computes targets and alphas with the final brightness.
        self.tick(1.0 / 60.0, reveal_step);
        for p in &mut self.particles {
            p.x = p.tx;
            p.y = p.ty;
        }
    }

    /// Advance the simulation by `dt` seconds.
    pub fn tick(&mut self, dt: f32, reveal_step: usize) {
        let dt = dt.clamp(0.0, 0.1);
        self.time += dt;
        let t = self.time;
        let rect = self.rect;
        let min_side = rect.width().min(rect.height());
        let frames = dt * 60.0;

        self.scene_fade += (1.0 - self.scene_fade) * (1.0 - (1.0 - 0.035f32).powf(frames));

        for (gi, g) in self.scene.groups.iter().enumerate() {
            let target = life_target(g, reveal_step);
            let l = &mut self.life[gi];
            *l += (target - *l) * (1.0 - (-self.scene.life_rate * dt).exp());
            let ht = heat_target(g, reveal_step);
            let h = &mut self.heat[gi];
            *h += (ht - *h) * (1.0 - (-LIFE_RATE * 0.8 * dt).exp());
        }

        for p in &mut self.particles {
            let Some(g) = self.scene.groups.get(p.group) else {
                continue;
            };
            let life = self.life[p.group];
            match g.drift {
                Drift::Breathe { amp, speed } => {
                    let a = amp * min_side;
                    p.tx = p.hx + (t * 0.30 * speed * p.speed + p.phase).sin() * a;
                    p.ty = p.hy + (t * 0.24 * speed * p.speed + p.phase * 1.7).cos() * a * 0.8;
                    p.alpha = p.base_alpha * (0.8 + 0.2 * (t * 1.2 + p.phase).sin());
                }
                Drift::Still => {
                    p.tx = p.hx;
                    p.ty = p.hy;
                    p.alpha = p.base_alpha * (0.88 + 0.12 * (t * 2.0 + p.phase).sin());
                }
                Drift::Flow { speed } => {
                    if let Home::Path { points, spread } = &g.home {
                        p.along = (p.along + speed * p.speed * dt / 4.0).rem_euclid(1.0);
                        let (px, py, nx, ny) = path_point(points, p.along, rect);
                        let off = p.lateral * spread * min_side;
                        p.tx = px + nx * off;
                        p.ty = py + ny * off;
                        let edge = (p.along * (1.0 - p.along) * 6.0).clamp(0.0, 1.0);
                        p.alpha = p.base_alpha * edge;
                        // wrapped: snap so the runner does not streak back
                        if ((p.tx - p.x).powi(2) + (p.ty - p.y).powi(2)).sqrt() > min_side * 0.2 {
                            p.x = p.tx;
                            p.y = p.ty;
                        }
                    } else {
                        p.tx = p.hx;
                        p.ty = p.hy;
                        p.alpha = p.base_alpha;
                    }
                }
                Drift::Fall { speed } | Drift::Rise { speed } => {
                    let (v0, v1) = field_v_range(&g.home);
                    let span = (v1 - v0) * rect.height();
                    let dir = if matches!(g.drift, Drift::Fall { .. }) {
                        1.0
                    } else {
                        -1.0
                    };
                    p.along = (p.along + dir * speed * p.speed * dt / 6.0).rem_euclid(1.0);
                    p.tx = p.hx + (t * 0.5 * p.speed + p.phase).sin() * 0.004 * min_side;
                    p.ty = rect.top() + v0 * rect.height() + p.along * span;
                    // fade at both ends so wrap-around is invisible
                    let edge = (p.along * (1.0 - p.along) * 4.0).clamp(0.0, 1.0);
                    p.alpha = p.base_alpha * edge * (0.7 + 0.3 * (t * 1.5 + p.phase).sin());
                    // when the particle wraps, snap so it does not streak back
                    if (p.ty - p.y).abs() > span * 0.5 {
                        p.y = p.ty;
                        p.x = p.tx;
                    }
                }
            }
            let heat = self.heat[p.group];
            p.alpha *= life * (1.0 + heat * 0.9);
            let k = 1.0 - (1.0 - p.k).powf(frames);
            p.x += (p.tx - p.x) * k;
            p.y += (p.ty - p.y) * k;
        }
    }

    /// Draw the field: hairlines through the egui painter, glow through GL.
    pub fn paint(&self, painter: &egui::Painter, rect: Rect, opacity: f32) {
        let scale = rect.width() / REF_WIDTH;

        if !self.links.is_empty() && self.scene.link_alpha > 0.0 {
            let a = self.scene.link_alpha * self.scene_fade * opacity;
            for &(i, j) in &self.links {
                let (p, q) = (&self.particles[i], &self.particles[j]);
                let la = a * ((p.alpha + q.alpha) * 0.5).min(1.0);
                if la < 0.01 {
                    continue;
                }
                let color = Color32::from_rgba_unmultiplied(255, 255, 255, (la * 255.0) as u8);
                painter.line_segment(
                    [Pos2::new(p.x, p.y), Pos2::new(q.x, q.y)],
                    egui::Stroke::new(1.0, color),
                );
            }
        }

        let sprites: Vec<Sprite> = self
            .particles
            .iter()
            .filter(|p| p.alpha > 0.003)
            .map(|p| {
                let [r, g, b] = p.tint.rgb();
                let heat = self.heat.get(p.group).copied().unwrap_or(0.0) * 0.75;
                let [er, eg, eb] = Tint::Ember.rgb();
                Sprite {
                    x: p.x,
                    y: p.y,
                    size: p.base_size * p.size_mul * scale * SIZE_GAIN * (1.0 + heat * 0.4),
                    rgba: [
                        r + (er - r) * heat,
                        g + (eg - g) * heat,
                        b + (eb - b) * heat,
                        (p.alpha * ALPHA_GAIN * opacity).clamp(0.0, 1.0),
                    ],
                }
            })
            .collect();
        self.renderer.paint(painter, rect, sprites);
    }
}

fn life_target(g: &Group, reveal_step: usize) -> f32 {
    match g.step {
        Some(s) if s > reveal_step => g.dim,
        _ => 1.0,
    }
}

fn heat_target(g: &Group, reveal_step: usize) -> f32 {
    match g.hot_step {
        Some(s) if s <= reveal_step => 1.0,
        _ => 0.0,
    }
}

/// Point at parameter `s` (0..1) along a normalised polyline, with its unit
/// normal, in points.
fn path_point(points: &[[f32; 2]], s: f32, rect: Rect) -> (f32, f32, f32, f32) {
    if points.len() < 2 {
        let c = rect.center();
        return (c.x, c.y, 0.0, 1.0);
    }
    let to_px = |p: [f32; 2]| {
        (
            rect.left() + p[0] * rect.width(),
            rect.top() + p[1] * rect.height(),
        )
    };
    let pts: Vec<(f32, f32)> = points.iter().map(|&p| to_px(p)).collect();
    let mut total = 0.0;
    let lens: Vec<f32> = pts
        .windows(2)
        .map(|w| {
            let l = ((w[1].0 - w[0].0).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt();
            total += l;
            l
        })
        .collect();
    let mut d = s.clamp(0.0, 1.0) * total;
    for (i, l) in lens.iter().enumerate() {
        if d <= *l || i + 1 == lens.len() {
            let f = if *l > 0.0 {
                (d / l).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let (a, b) = (pts[i], pts[i + 1]);
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let len = (dx * dx + dy * dy).sqrt().max(1e-3);
            return (a.0 + dx * f, a.1 + dy * f, -dy / len, dx / len);
        }
        d -= l;
    }
    let last = pts[pts.len() - 1];
    (last.0, last.1, 0.0, 1.0)
}

fn field_v_range(home: &Home) -> (f32, f32) {
    match home {
        Home::Field { v0, v1, .. } => (*v0, *v1),
        _ => (0.0, 1.0),
    }
}

/// Pick a rest position for one particle of a group.
fn home_point(home: &Home, rect: Rect, min_side: f32, rng: &mut Rng) -> (f32, f32) {
    match home {
        Home::Cluster { u, v, r, falloff } => {
            let a = rng.range(0.0, std::f32::consts::TAU);
            let d = rng.unit().powf(*falloff) * r * min_side;
            (
                rect.left() + u * rect.width() + a.cos() * d,
                rect.top() + v * rect.height() + a.sin() * d,
            )
        }
        Home::Field { u0, v0, u1, v1 } => (
            rect.left() + rng.range(*u0, *u1) * rect.width(),
            rect.top() + rng.range(*v0, *v1) * rect.height(),
        ),
        // Radial homes depend on the particle's position and are resolved in
        // `set_scene`; a bare call lands on the centre.
        Home::Radial { u, v, .. } => (
            rect.left() + u * rect.width(),
            rect.top() + v * rect.height(),
        ),
        Home::Path { points, spread } => {
            let (px, py, nx, ny) = path_point(points, rng.unit(), rect);
            let off = rng.range(-1.0, 1.0) * spread * min_side;
            (px + nx * off, py + ny * off)
        }
        Home::Mask { points, u, v, w, h } => {
            if points.is_empty() {
                return (rect.center().x, rect.center().y);
            }
            let q = points[(rng.unit() * (points.len() - 1) as f32) as usize];
            let jitter = 0.0015 * min_side;
            (
                rect.left() + (u + q[0] * w) * rect.width() + rng.range(-jitter, jitter),
                rect.top() + (v + q[1] * h) * rect.height() + rng.range(-jitter, jitter),
            )
        }
    }
}

/// Sample the opaque pixels of a PNG into points normalised to the unit
/// square (x/width, y/height). Callers fit the mask into a box with the
/// image's aspect ratio. Used for the logo intro and the end slide.
pub fn mask_points_from_png(bytes: &[u8]) -> (Arc<Vec<[f32; 2]>>, f32) {
    let Ok(img) = image::load_from_memory(bytes) else {
        return (Arc::new(Vec::new()), 1.0);
    };
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
    let step = (w.max(h) / 110).max(1);
    let mut pts = Vec::new();
    for y in (0..h).step_by(step) {
        for x in (0..w).step_by(step) {
            if rgba.get_pixel(x as u32, y as u32)[3] > 120 {
                pts.push([x as f32 / w as f32, y as f32 / h as f32]);
            }
        }
    }
    (Arc::new(pts), w as f32 / h.max(1) as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shares_cover_every_particle_and_groups_light_by_step() {
        let mut field = Field::new(200, 7);
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(1920.0, 1080.0));
        field.scatter(rect);
        let scene = Scene::new(vec![
            Group::new(
                1.0,
                Home::Cluster {
                    u: 0.2,
                    v: 0.5,
                    r: 0.1,
                    falloff: 0.6,
                },
            )
            .alpha(0.6, 1.0)
            .links(2),
            Group::new(
                1.0,
                Home::Field {
                    u0: 0.0,
                    v0: 0.0,
                    u1: 1.0,
                    v1: 1.0,
                },
            )
            .step(2),
        ]);
        field.set_scene(scene, rect, 1);
        let in_first = field.particles.iter().filter(|p| p.group == 0).count();
        assert_eq!(in_first, 100);
        assert!(!field.links.is_empty());
        field.settle(1);
        assert!(field.life[0] > 0.99);
        // A freshly settled field is at full brightness (exports are stills).
        let max_alpha = field
            .particles
            .iter()
            .filter(|p| p.group == 0)
            .map(|p| p.alpha)
            .fold(0.0, f32::max);
        assert!(max_alpha > 0.7, "settled cluster is dim: {max_alpha}");
        assert!(field.life[1] < 0.2, "unrevealed group must be dimmed");
        field.settle(2);
        assert!(field.life[1] > 0.99);
        // every particle rests inside the slide (clusters may poke out slightly)
        for p in &field.particles {
            assert!(p.x > -200.0 && p.x < 2120.0 && p.y > -200.0 && p.y < 1280.0);
        }
    }

    #[test]
    fn rng_is_deterministic() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..10 {
            assert_eq!(a.unit(), b.unit());
        }
    }
}
