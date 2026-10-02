//! The field: a fixed pool of particles, the scene they are acting out, and
//! the paint pass that draws them.

use mdeck_sdk::paint::{Color, Painter, Pos2, Rect, Sprite, SpriteBlend, SpriteLayer, Stroke};

use super::{DEFAULT_TINTS, Rng, Scene, Tint};

mod assign;
mod motion;

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
    /// GPU state of the glow sprites (wake buffers on GL).
    layer: SpriteLayer,
    /// Linear-ish RGB per tint, from the theme.
    tints: [[f32; 3]; 5],
    /// Drawing on a light page: ink instead of light.
    light: bool,
}

fn rgb(c: Color) -> [f32; 3] {
    [
        c.r() as f32 / 255.0,
        c.g() as f32 / 255.0,
        c.b() as f32 / 255.0,
    ]
}

impl Field {
    /// Set the particle colours, in [`Tint`] order (see [`DEFAULT_TINTS`]).
    pub fn set_tints(&mut self, tints: [Color; 5]) {
        self.tints = tints.map(rgb);
    }

    /// On a light page the particles blend like ink (no trails, dark links).
    pub fn set_light(&mut self, light: bool) {
        self.light = light;
    }

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
            rect: Rect::ZERO,
            life: Vec::new(),
            heat: Vec::new(),
            links: Vec::new(),
            scene_fade: 1.0,
            time: 0.0,
            layer: SpriteLayer::new(),
            tints: DEFAULT_TINTS.map(rgb),
            light: false,
        }
    }

    pub fn rect(&self) -> Rect {
        self.rect
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
        let ranges = assign::share_ranges(&scene.groups, self.particles.len());
        let order = self.shuffled_order();
        self.place_homes(&scene.groups, &ranges, &order, rect);
        self.links = self.neighbour_links(&scene.groups, &ranges, &order);

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
            self.life[gi] = motion::life_target(g, reveal_step);
            self.heat[gi] = motion::heat_target(g, reveal_step);
        }
        self.scene_fade = 1.0;
        // One tick computes targets and alphas with the final brightness.
        self.tick(1.0 / 60.0, reveal_step);
        for p in &mut self.particles {
            p.x = p.tx;
            p.y = p.ty;
        }
    }

    /// Draw the field: hairlines, then the glow sprites. Live frames get
    /// wakes; stills (`wakes == false`) are drawn crisp; a light page blends
    /// like ink.
    pub fn paint(&self, painter: &Painter, rect: Rect, opacity: f32, wakes: bool) {
        self.paint_links(painter, opacity);
        let sprites = self.sprites(rect.width() / REF_WIDTH, opacity);
        let blend = if self.light {
            SpriteBlend::Normal
        } else if wakes {
            SpriteBlend::AdditiveWakes
        } else {
            SpriteBlend::Additive
        };
        // The sprites map onto `rect` (the GL viewport), as before the SDK.
        painter.with_clip(rect).sprites(&self.layer, sprites, blend);
    }

    /// The hairlines between linked neighbours, faded in after a scene change.
    fn paint_links(&self, painter: &Painter, opacity: f32) {
        if self.links.is_empty() || self.scene.link_alpha <= 0.0 {
            return;
        }
        let a = self.scene.link_alpha * self.scene_fade * opacity;
        for &(i, j) in &self.links {
            let (p, q) = (&self.particles[i], &self.particles[j]);
            let la = a * ((p.alpha + q.alpha) * 0.5).min(1.0);
            if la < 0.01 {
                continue;
            }
            let ink = if self.light { 40 } else { 255 };
            let color = Color::from_rgba_unmultiplied(ink, ink, ink, (la * 255.0) as u8);
            painter.line_segment(
                [Pos2::new(p.x, p.y), Pos2::new(q.x, q.y)],
                Stroke::new(1.0, color),
            );
        }
    }

    /// One glow sprite per visible particle, warmed toward ember by its
    /// group's heat.
    fn sprites(&self, scale: f32, opacity: f32) -> Vec<Sprite> {
        self.particles
            .iter()
            .filter(|p| p.alpha > 0.003)
            .map(|p| {
                let [r, g, b] = self.tints[p.tint.index()];
                let heat = self.heat.get(p.group).copied().unwrap_or(0.0) * 0.75;
                let [er, eg, eb] = self.tints[Tint::Ember.index()];
                Sprite {
                    center: Pos2::new(p.x, p.y),
                    size: p.base_size * p.size_mul * scale * SIZE_GAIN * (1.0 + heat * 0.4),
                    rgba: [
                        r + (er - r) * heat,
                        g + (eg - g) * heat,
                        b + (eb - b) * heat,
                        (p.alpha * ALPHA_GAIN * opacity).clamp(0.0, 1.0),
                    ],
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::particles::{Group, Home};
    use mdeck_sdk::paint::Vec2;

    #[test]
    fn shares_cover_every_particle_and_groups_light_by_step() {
        let mut field = Field::new(200, 7);
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(1920.0, 1080.0));
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
}
