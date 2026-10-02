//! Content-aware scenes: the geometry a slide's renderers published, turned
//! into particles that serve it.

use eframe::egui::{Pos2, Rect};

use super::dust;
use crate::render::hints::Hint;
use crate::render::particles::{Drift, Group, Home, Palette, Rng, Scene, Tint};

/// Turn the geometry a slide's renderers published into a scene that serves
/// it: embers off bar tops, runners along paths, sparks circling rings,
/// glints on points, and dust that keeps out of every frame.
pub fn from_hints(hints: &[Hint], rect: Rect, seed: u64) -> Scene {
    let mut rng = Rng::new(seed);
    let frac = Frac::new(rect);
    let mut groups: Vec<Group> = Vec::new();
    groups.extend(bar_groups(hints, &frac));
    groups.extend(path_groups(hints, &frac));
    groups.extend(circle_groups(hints, &frac));
    groups.extend(point_groups(hints, &frac));
    let used: f32 = groups.iter().map(|g| g.share).sum();
    groups.extend(dust_groups(hints, &frac, used, &mut rng));

    let mut scene = Scene::new(groups);
    scene.link_alpha = 0.06;
    scene
}

/// Screen points to slide fractions.
struct Frac {
    rect: Rect,
    min_side: f32,
}

impl Frac {
    fn new(rect: Rect) -> Self {
        Self {
            rect,
            min_side: rect.width().min(rect.height()),
        }
    }

    fn u(&self, x: f32) -> f32 {
        ((x - self.rect.left()) / self.rect.width()).clamp(0.0, 1.0)
    }

    fn v(&self, y: f32) -> f32 {
        ((y - self.rect.top()) / self.rect.height()).clamp(0.0, 1.0)
    }
}

/// Embers rising off bar tops, or heat drifting off the end of a horizontal
/// bar. Segments that share a column (stacked bars) merge into one top.
fn bar_groups(hints: &[Hint], f: &Frac) -> Vec<Group> {
    let mut bars: Vec<Rect> = Vec::new();
    for h in hints {
        if let Hint::Bar(r) = h
            && r.width() > 2.0
            && r.height() > 2.0
        {
            if let Some(existing) = bars
                .iter_mut()
                .find(|b| (b.left() - r.left()).abs() < 2.0 && (b.right() - r.right()).abs() < 2.0)
            {
                *existing = existing.union(*r);
            } else {
                bars.push(*r);
            }
        }
    }
    if bars.is_empty() {
        return Vec::new();
    }
    let rect = f.rect;
    let total_w: f32 = bars.iter().map(|b| b.width()).sum::<f32>().max(1.0);
    bars.iter()
        .map(|b| {
            let horizontal =
                b.width() > b.height() * 2.5 && b.left() < rect.left() + rect.width() * 0.45;
            let share =
                0.30 * b.width().min(b.height()).max(8.0) / total_w.max(bars.len() as f32 * 8.0);
            let group = if horizontal {
                // heat drifts off the right end of a horizontal bar
                Group::new(
                    share.max(0.02),
                    Home::Field {
                        u0: f.u(b.right()),
                        v0: f.v(b.top()),
                        u1: f.u(b.right() + b.height() * 1.8),
                        v1: f.v(b.bottom()),
                    },
                )
                .drift(Drift::Breathe {
                    amp: 0.004,
                    speed: 2.0,
                })
            } else {
                Group::new(
                    share.max(0.02),
                    Home::Field {
                        u0: f.u(b.left() + b.width() * 0.08),
                        v0: f.v(b.top() - b.width().min(b.height()) * 0.9),
                        u1: f.u(b.right() - b.width() * 0.08),
                        v1: f.v(b.top()),
                    },
                )
                .drift(Drift::Rise { speed: 0.55 })
            };
            group
                .palette(Palette::Warm)
                .alpha(0.25, 0.65)
                .size(0.35, 0.7)
        })
        .collect()
}

/// Runners along each path, in the drawing direction.
fn path_groups(hints: &[Hint], f: &Frac) -> Vec<Group> {
    let mut groups = Vec::new();
    let mut path_i = 0usize;
    for h in hints {
        if let Hint::Path(pts) = h
            && pts.len() >= 2
        {
            let points: Vec<[f32; 2]> = pts.iter().map(|p| [f.u(p.x), f.v(p.y)]).collect();
            let length: f32 = pts
                .windows(2)
                .map(|w| ((w[1].x - w[0].x).powi(2) + (w[1].y - w[0].y).powi(2)).sqrt())
                .sum::<f32>()
                / f.min_side;
            let tint = [Tint::White, Tint::Ember, Tint::Candle, Tint::Pale][path_i % 4];
            path_i += 1;
            groups.push(
                Group::new(
                    (0.01 + length * 0.05).min(0.06),
                    Home::Path {
                        points,
                        spread: 0.004,
                    },
                )
                .palette(Palette::Solid(tint))
                .alpha(0.3, 0.8)
                .size(0.3, 0.6)
                .drift(Drift::Flow { speed: 0.7 }),
            );
        }
    }
    groups
}

/// Sparks circling just outside each circle, keeping the largest per centre.
fn circle_groups(hints: &[Hint], f: &Frac) -> Vec<Group> {
    let mut circles: Vec<(Pos2, f32)> = Vec::new();
    for h in hints {
        if let Hint::Circle { center, radius } = h {
            if let Some(c) = circles.iter_mut().find(|(c, _)| c.distance(*center) < 4.0) {
                c.1 = c.1.max(*radius);
            } else {
                circles.push((*center, *radius));
            }
        }
    }
    circles
        .iter()
        .take(4)
        .enumerate()
        .map(|(i, (c, r))| {
            Group::new(
                0.14,
                Home::Ring {
                    u: f.u(c.x),
                    v: f.v(c.y),
                    r: r / f.min_side * 1.09,
                    width: 0.022,
                },
            )
            .palette(if i % 2 == 0 {
                Palette::Warm
            } else {
                Palette::Cold
            })
            .alpha(0.3, 0.75)
            .size(0.35, 0.65)
            .drift(Drift::Orbit {
                speed: if i % 2 == 0 { 0.35 } else { -0.28 },
            })
        })
        .collect()
}

/// A glint on each point.
fn point_groups(hints: &[Hint], f: &Frac) -> Vec<Group> {
    let points: Vec<Pos2> = hints
        .iter()
        .filter_map(|h| {
            if let Hint::Point(p) = h {
                Some(*p)
            } else {
                None
            }
        })
        .collect();
    if points.is_empty() {
        return Vec::new();
    }
    let share = (0.20 / points.len() as f32).min(0.03);
    points
        .iter()
        .take(40)
        .map(|p| {
            Group::new(
                share,
                Home::Cluster {
                    u: f.u(p.x),
                    v: f.v(p.y),
                    r: 0.012,
                    falloff: 0.5,
                },
            )
            .palette(Palette::Site)
            .alpha(0.25, 0.7)
            .size(0.3, 0.55)
            .drift(Drift::Breathe {
                amp: 0.003,
                speed: 2.4,
            })
        })
        .collect()
}

/// Dust everywhere the content is not: bands around the union of frames,
/// filling what the other groups (`used`) left of the pool.
fn dust_groups(hints: &[Hint], f: &Frac, used: f32, rng: &mut Rng) -> Vec<Group> {
    let frame = hints
        .iter()
        .filter_map(|h| {
            if let Hint::Frame(r) = h {
                Some(*r)
            } else {
                None
            }
        })
        .reduce(|a, b| a.union(b));
    let dust_share = (1.0 - used).max(0.25);
    let Some(frame) = frame else {
        return vec![dust(dust_share)];
    };
    // a very faint haze over everything, so the content never floats
    // in dead black, plus slightly denser dust in the margins
    let mut groups = vec![dust(dust_share * 0.45).alpha(0.03, 0.10)];
    groups.extend(margin_bands(frame, f, dust_share * 0.45));
    // a few soft lights in the corners, always outside the frame
    // never the top-left, where the heading lives
    let corners = [(0.95, 0.90), (0.06, 0.92), (0.95, 0.12)];
    let (u, v) = corners[(rng.unit() * 2.99) as usize];
    groups.push(
        Group::new(
            0.06,
            Home::Cluster {
                u,
                v,
                r: 0.10,
                falloff: 0.6,
            },
        )
        .alpha(0.12, 0.4)
        .size(0.6, 1.1)
        .links(2),
    );
    groups
}

/// Dust in the margins above, below, left and right of `frame`, sharing
/// `share` of the pool by area.
fn margin_bands(frame: Rect, f: &Frac, share: f32) -> Vec<Group> {
    let (l, t, r, b) = (
        f.u(frame.left()),
        f.v(frame.top()),
        f.u(frame.right()),
        f.v(frame.bottom()),
    );
    let bands = [
        (0.02, 0.04, 0.98, t), // above
        (0.02, b, 0.98, 0.96), // below
        (0.02, t, l, b),       // left
        (r, t, 0.98, b),       // right
    ];
    // slivers thinner than 5% of the slide get no dust of their own
    let mut areas: Vec<f32> = bands
        .iter()
        .map(|(u0, v0, u1, v1)| {
            if u1 - u0 < 0.05 || v1 - v0 < 0.05 {
                0.0
            } else {
                (u1 - u0) * (v1 - v0)
            }
        })
        .collect();
    let total: f32 = areas.iter().sum::<f32>().max(1e-3);
    for a in &mut areas {
        *a /= total;
    }
    bands
        .iter()
        .zip(areas)
        .filter(|(_, a)| *a > 0.02)
        .map(|((u0, v0, u1, v1), a)| {
            Group::new(
                share * a,
                Home::Field {
                    u0: *u0,
                    v0: *v0,
                    u1: *u1,
                    v1: *v1,
                },
            )
            .alpha(0.06, 0.20)
            .size(0.45, 0.9)
            .drift(Drift::Breathe {
                amp: 0.011,
                speed: 1.1,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bars_paths_circles_and_frames_each_get_groups() {
        let rect = Rect::from_min_size(Pos2::ZERO, eframe::egui::vec2(1920.0, 1080.0));
        let hints = vec![
            Hint::Frame(Rect::from_min_max(
                Pos2::new(100.0, 200.0),
                Pos2::new(1800.0, 1000.0),
            )),
            Hint::Bar(Rect::from_min_max(
                Pos2::new(300.0, 600.0),
                Pos2::new(400.0, 900.0),
            )),
            // a stacked segment on the same column merges into the same bar
            Hint::Bar(Rect::from_min_max(
                Pos2::new(300.0, 400.0),
                Pos2::new(400.0, 600.0),
            )),
            Hint::Path(vec![
                Pos2::new(500.0, 800.0),
                Pos2::new(900.0, 500.0),
                Pos2::new(1300.0, 700.0),
            ]),
            Hint::Circle {
                center: Pos2::new(1500.0, 600.0),
                radius: 150.0,
            },
            Hint::Point(Pos2::new(700.0, 700.0)),
        ];
        let scene = from_hints(&hints, rect, 1);
        let bars = scene
            .groups
            .iter()
            .filter(|g| matches!(g.drift, Drift::Rise { .. }))
            .count();
        let paths = scene
            .groups
            .iter()
            .filter(|g| matches!(g.home, Home::Path { .. }))
            .count();
        let rings = scene
            .groups
            .iter()
            .filter(|g| matches!(g.home, Home::Ring { .. }))
            .count();
        assert_eq!(bars, 1, "stacked segments merge into one bar");
        assert_eq!(paths, 1);
        assert_eq!(rings, 1);
        // dust bands exist above and below the frame
        let fields = scene
            .groups
            .iter()
            .filter(|g| matches!(g.home, Home::Field { .. }))
            .count();
        assert!(fields >= 3);
        let total: f32 = scene.groups.iter().map(|g| g.share).sum();
        assert!(total > 0.9 && total < 1.2, "shares sum to {total}");
    }
}
