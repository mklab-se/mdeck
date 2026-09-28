//! Casting a scene: which particles join which group, where each one rests,
//! and which neighbours are joined by hairlines.

use eframe::egui::Rect;

use super::Field;
use super::motion::path_point;
use crate::render::particles::{Group, Home, Rng};

/// Contiguous index ranges of an `n`-particle pool, one per group, sized by
/// share (the last group takes the remainder).
pub(super) fn share_ranges(groups: &[Group], n: usize) -> Vec<(usize, usize)> {
    let total_share: f32 = groups.iter().map(|g| g.share.max(0.0)).sum();
    let total_share = if total_share <= 0.0 { 1.0 } else { total_share };
    let mut start = 0usize;
    let mut ranges: Vec<(usize, usize)> = Vec::with_capacity(groups.len());
    for (gi, g) in groups.iter().enumerate() {
        let count = if gi + 1 == groups.len() {
            n - start
        } else {
            ((g.share.max(0.0) / total_share) * n as f32).round() as usize
        };
        let end = (start + count).min(n);
        ranges.push((start, end));
        start = end;
    }
    ranges
}

impl Field {
    /// Shuffle which particles land in which group so a group is not always
    /// the same physical particles (keeps morphs lively).
    pub(super) fn shuffled_order(&mut self) -> Vec<usize> {
        let n = self.particles.len();
        let mut order: Vec<usize> = (0..n).collect();
        for i in (1..n).rev() {
            let j = (self.rng.unit() * (i + 1) as f32) as usize;
            order.swap(i, j.min(i));
        }
        order
    }

    /// Give every particle its group's look and a home inside its group.
    pub(super) fn place_homes(
        &mut self,
        groups: &[Group],
        ranges: &[(usize, usize)],
        order: &[usize],
        rect: Rect,
    ) {
        let min_side = rect.width().min(rect.height());
        for (gi, g) in groups.iter().enumerate() {
            let (s, e) = ranges[gi];
            for (j, &pi) in order[s..e].iter().enumerate() {
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
                    // Mask points are in importance order: a group of n
                    // particles takes the first n, so a small group is a
                    // sketch of the whole shape rather than a random speckle.
                    Home::Mask { points, u, v, w, h } if !points.is_empty() => {
                        let q = points[j % points.len()];
                        let jitter = 0.0015 * min_side;
                        (
                            rect.left()
                                + (u + q[0] * w) * rect.width()
                                + self.rng.range(-jitter, jitter),
                            rect.top()
                                + (v + q[1] * h) * rect.height()
                                + self.rng.range(-jitter, jitter),
                        )
                    }
                    home => home_point(home, rect, min_side, &mut self.rng),
                };
                p.hx = hx;
                p.hy = hy;
            }
        }
    }

    /// Hairlines: the k nearest neighbours within each linked group.
    pub(super) fn neighbour_links(
        &self,
        groups: &[Group],
        ranges: &[(usize, usize)],
        order: &[usize],
    ) -> Vec<(usize, usize)> {
        let mut links = Vec::new();
        for (gi, g) in groups.iter().enumerate() {
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
                        links.push((i, j));
                    }
                }
            }
        }
        links
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
        Home::Ring { u, v, r, width } => {
            let a = rng.range(0.0, std::f32::consts::TAU);
            let d = (r + rng.range(-0.5, 0.5) * width) * min_side;
            (
                rect.left() + u * rect.width() + a.cos() * d,
                rect.top() + v * rect.height() + a.sin() * d,
            )
        }
        // Radial homes depend on the particle's position and are resolved in
        // `place_homes`; a bare call lands on the centre.
        Home::Radial { u, v, .. } => (
            rect.left() + u * rect.width(),
            rect.top() + v * rect.height(),
        ),
        Home::Path { points, spread } => {
            let (px, py, nx, ny) = path_point(points, rng.unit(), rect);
            let off = rng.range(-1.0, 1.0) * spread * min_side;
            (px + nx * off, py + ny * off)
        }
        // Masks are assigned in `place_homes` (first-n rule); an empty mask
        // collapses to the centre of its box.
        Home::Mask { u, v, w, h, .. } => (
            rect.left() + (u + w / 2.0) * rect.width(),
            rect.top() + (v + h / 2.0) * rect.height(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use eframe::egui::{self, Pos2};

    use super::*;
    use crate::render::particles::Scene;

    #[test]
    fn a_mask_group_takes_the_first_points_in_order() {
        let mut field = Field::new(4, 1);
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(1000.0, 1000.0));
        field.scatter(rect);
        let points: Vec<[f32; 2]> = (0..10).map(|i| [i as f32 / 10.0, 0.5]).collect();
        let scene = Scene::new(vec![Group::new(
            1.0,
            Home::Mask {
                points: Arc::new(points),
                u: 0.0,
                v: 0.0,
                w: 1.0,
                h: 1.0,
            },
        )]);
        field.set_scene(scene, rect, 3);
        let mut xs: Vec<f32> = field.particles.iter().map(|p| p.hx).collect();
        xs.sort_by(f32::total_cmp);
        // four particles, so the first four points (x = 0, 100, 200, 300)
        for (x, want) in xs.iter().zip([0.0, 100.0, 200.0, 300.0]) {
            assert!((x - want).abs() < 3.0, "homes {xs:?}");
        }
    }
}
