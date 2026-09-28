//! The simulation step: groups ease toward their reveal brightness, each
//! particle's drift moves its target, and the particle eases after it.

use eframe::egui::{Pos2, Rect};

use super::{Field, Particle};
use crate::render::particles::scene::LIFE_RATE;
use crate::render::particles::{Drift, Group, Home};

/// What every drift needs to know about the frame being simulated.
struct Clock {
    /// Seconds since the field was created.
    t: f32,
    /// This frame's step in seconds.
    dt: f32,
    rect: Rect,
    min_side: f32,
}

impl Field {
    /// Advance the simulation by `dt` seconds.
    pub fn tick(&mut self, dt: f32, reveal_step: usize) {
        let dt = dt.clamp(0.0, 0.1);
        self.time += dt;
        let frames = dt * 60.0;
        self.scene_fade += (1.0 - self.scene_fade) * (1.0 - (1.0 - 0.035f32).powf(frames));
        self.ease_groups(dt, reveal_step);
        self.move_particles(dt, frames);
    }

    /// Ease each group's brightness and heat toward the reveal step's target.
    fn ease_groups(&mut self, dt: f32, reveal_step: usize) {
        for (gi, g) in self.scene.groups.iter().enumerate() {
            let target = life_target(g, reveal_step);
            let l = &mut self.life[gi];
            *l += (target - *l) * (1.0 - (-self.scene.life_rate * dt).exp());
            let ht = heat_target(g, reveal_step);
            let h = &mut self.heat[gi];
            *h += (ht - *h) * (1.0 - (-LIFE_RATE * 0.8 * dt).exp());
        }
    }

    /// Drift every particle's target, light it by its group, and ease it
    /// toward the target.
    fn move_particles(&mut self, dt: f32, frames: f32) {
        let rect = self.rect;
        let clock = Clock {
            t: self.time,
            dt,
            rect,
            min_side: rect.width().min(rect.height()),
        };
        for p in &mut self.particles {
            let Some(g) = self.scene.groups.get(p.group) else {
                continue;
            };
            let life = self.life[p.group];
            drift(p, g, &clock);
            let heat = self.heat[p.group];
            p.alpha *= life * (1.0 + heat * 0.9);
            let k = 1.0 - (1.0 - p.k).powf(frames);
            p.x += (p.tx - p.x) * k;
            p.y += (p.ty - p.y) * k;
        }
    }
}

/// Set a particle's target and base brightness for this frame from its
/// group's drift.
fn drift(p: &mut Particle, g: &Group, c: &Clock) {
    match g.drift {
        Drift::Breathe { amp, speed } => breathe(p, amp, speed, c),
        Drift::Still => {
            p.tx = p.hx;
            p.ty = p.hy;
            p.alpha = p.base_alpha * (0.88 + 0.12 * (c.t * 2.0 + p.phase).sin());
        }
        Drift::Orbit { speed } => orbit(p, &g.home, speed, c),
        Drift::Flow { speed } => flow(p, &g.home, speed, c),
        Drift::Forward { u, v, speed } => forward(p, (u, v), speed, c),
        Drift::Fall { speed } | Drift::Rise { speed } => {
            let dir = if matches!(g.drift, Drift::Fall { .. }) {
                1.0
            } else {
                -1.0
            };
            fall(p, &g.home, dir * speed, c);
        }
    }
}

fn breathe(p: &mut Particle, amp: f32, speed: f32, c: &Clock) {
    let t = c.t;
    let a = amp * c.min_side;
    // two incommensurate frequencies so the wander never
    // settles into a visible loop
    p.tx = p.hx
        + (t * 0.42 * speed * p.speed + p.phase).sin() * a
        + (t * 0.17 * speed + p.phase * 2.3).cos() * a * 0.5;
    p.ty = p.hy
        + (t * 0.34 * speed * p.speed + p.phase * 1.7).cos() * a * 0.8
        + (t * 0.21 * speed + p.phase * 3.1).sin() * a * 0.45;
    p.alpha = p.base_alpha * (0.78 + 0.22 * (t * 1.4 + p.phase).sin());
}

fn orbit(p: &mut Particle, home: &Home, speed: f32, c: &Clock) {
    let rect = c.rect;
    let centre = match home {
        Home::Ring { u, v, .. } | Home::Cluster { u, v, .. } => Pos2::new(
            rect.left() + u * rect.width(),
            rect.top() + v * rect.height(),
        ),
        _ => rect.center(),
    };
    let (dx, dy) = (p.hx - centre.x, p.hy - centre.y);
    let a = c.t * speed * (0.6 + 0.8 * p.speed) + p.phase * 0.05;
    let (s, co) = a.sin_cos();
    p.tx = centre.x + dx * co - dy * s;
    p.ty = centre.y + dx * s + dy * co;
    p.alpha = p.base_alpha * (0.75 + 0.25 * (c.t * 3.0 + p.phase).sin());
}

fn flow(p: &mut Particle, home: &Home, speed: f32, c: &Clock) {
    let Home::Path { points, spread } = home else {
        p.tx = p.hx;
        p.ty = p.hy;
        p.alpha = p.base_alpha;
        return;
    };
    let min_side = c.min_side;
    p.along = (p.along + speed * p.speed * c.dt / 4.0).rem_euclid(1.0);
    let (px, py, nx, ny) = path_point(points, p.along, c.rect);
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
}

fn forward(p: &mut Particle, (u, v): (f32, f32), speed: f32, c: &Clock) {
    let rect = c.rect;
    let cx = rect.left() + u * rect.width();
    let cy = rect.top() + v * rect.height();
    let before = p.along;
    let rate = speed * (0.5 + p.speed) * (0.5 + p.size_mul);
    p.along = (p.along + rate * c.dt).rem_euclid(1.0);
    // distance grows with the square of progress: slow near
    // the vanishing point, quick past the edge
    let f = 0.06 + p.along * p.along * 1.5;
    p.tx = cx + (p.hx - cx) * f;
    p.ty = cy + (p.hy - cy) * f;
    let fade_in = (p.along / 0.15).min(1.0);
    let fade_out = ((1.0 - p.along) / 0.2).min(1.0);
    p.alpha = p.base_alpha * fade_in * fade_out;
    // reborn: snap so it does not streak back to the centre
    if p.along < before {
        p.x = p.tx;
        p.y = p.ty;
    }
}

/// Fall (positive `speed`) or rise (negative) through the group's field,
/// wrapping at the ends.
fn fall(p: &mut Particle, home: &Home, speed: f32, c: &Clock) {
    let (t, rect) = (c.t, c.rect);
    let (v0, v1) = field_v_range(home);
    let span = (v1 - v0) * rect.height();
    p.along = (p.along + speed * p.speed * c.dt / 6.0).rem_euclid(1.0);
    p.tx = p.hx + (t * 0.5 * p.speed + p.phase).sin() * 0.004 * c.min_side;
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

pub(super) fn life_target(g: &Group, reveal_step: usize) -> f32 {
    match g.step {
        Some(s) if s > reveal_step => g.dim,
        _ => 1.0,
    }
}

pub(super) fn heat_target(g: &Group, reveal_step: usize) -> f32 {
    match g.hot_step {
        Some(s) if s <= reveal_step => 1.0,
        _ => 0.0,
    }
}

/// Point at parameter `s` (0..1) along a normalised polyline, with its unit
/// normal, in points.
pub(super) fn path_point(points: &[[f32; 2]], s: f32, rect: Rect) -> (f32, f32, f32, f32) {
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

#[cfg(test)]
mod tests {
    use eframe::egui;

    use super::*;
    use crate::render::particles::Scene;

    #[test]
    fn forward_drift_moves_outward_and_is_reborn_near_the_centre() {
        let mut field = Field::new(50, 5);
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(1600.0, 900.0));
        field.scatter(rect);
        let scene = Scene::new(vec![
            Group::new(
                1.0,
                Home::Field {
                    u0: 0.0,
                    v0: 0.0,
                    u1: 1.0,
                    v1: 1.0,
                },
            )
            .drift(Drift::Forward {
                u: 0.5,
                v: 0.5,
                speed: 0.05,
            }),
        ]);
        field.set_scene(scene, rect, 1);
        field.settle(0);
        let c = rect.center();
        let dist = |p: &Particle| ((p.tx - c.x).powi(2) + (p.ty - c.y).powi(2)).sqrt();
        let start: Vec<f32> = field.particles.iter().map(dist).collect();
        // one second later every particle that did not wrap is farther out
        for _ in 0..60 {
            field.tick(1.0 / 60.0, 0);
        }
        let mut farther = 0;
        for (p, s0) in field.particles.iter().zip(&start) {
            assert!(p.tx.is_finite() && p.ty.is_finite());
            assert!(p.alpha >= 0.0 && p.alpha <= 1.0);
            if dist(p) > *s0 {
                farther += 1;
            }
        }
        assert!(farther > 40, "only {farther} of 50 moved outward");
        // after a long while everything has wrapped at least once and is
        // still inside a sane radius
        for _ in 0..60 * 60 {
            field.tick(1.0 / 60.0, 0);
        }
        for p in &field.particles {
            assert!(
                dist(p) < rect.width() * 2.0,
                "particle ran away: {}",
                dist(p)
            );
        }
    }
}
