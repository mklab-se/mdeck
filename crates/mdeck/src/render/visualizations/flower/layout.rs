//! Where the centre and the petals go. The centre and the bulbs are sized
//! for their text first ([`Want`]); then the whole flower is scaled down
//! until it fits the rect, with the bulbs on the tightest ellipse around the
//! centre that keeps them clear of it and of each other.

use std::f32::consts::{PI, TAU};

use eframe::egui::{Pos2, Rect, Vec2};

/// The sizes the text asks for, at full font size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Want {
    /// Radius of the centre circle.
    pub radius: f32,
    /// Half width and half height of every bulb.
    pub half: Vec2,
}

/// The flower in a rect.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    pub center: Pos2,
    /// Radius of the centre circle.
    pub radius: f32,
    pub petals: Vec<PetalBox>,
    /// How much the wanted sizes (and so the fonts) were scaled to fit.
    pub scale: f32,
}

/// One petal's bulb: an axis-aligned ellipse around its text.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PetalBox {
    pub bulb: Pos2,
    /// Half width and half height of the bulb.
    pub half: Vec2,
}

impl PetalBox {
    /// Where the bulb's outline meets a ray from its centre along `dir`.
    pub fn edge_toward(&self, dir: Vec2) -> Pos2 {
        self.bulb + dir.normalized() * ellipse_radius(self.half, dir)
    }
}

/// Distance from an ellipse's centre to its outline along `dir`.
pub fn ellipse_radius(half: Vec2, dir: Vec2) -> f32 {
    let d = dir.normalized();
    if !d.is_finite() || d == Vec2::ZERO || half.x <= 0.0 || half.y <= 0.0 {
        return 0.0;
    }
    1.0 / ((d.x / half.x).powi(2) + (d.y / half.y).powi(2)).sqrt()
}

/// Space kept between bulbs, between a bulb and the centre (where its stem
/// runs), and at the rect's edges, as fractions of the rect's height.
const GAP: f32 = 0.03;
const STEM: f32 = 0.055;
const EDGE: f32 = 0.01;

/// Lay out `n` petals of `want` around a centre in `rect`: the first petal
/// on top, the rest clockwise.
pub fn layout(n: usize, rect: Rect, want: Want) -> Layout {
    let center = rect.center();
    let angles: Vec<f32> = (0..n)
        .map(|i| -PI / 2.0 + TAU * i as f32 / n.max(1) as f32)
        .collect();
    let mut scale = 1.0_f32;
    let petals = loop {
        if let Some(petals) = place(&angles, rect, want, scale) {
            break petals;
        }
        if scale < 0.05 {
            break place_at(&angles, rect, want.half * scale, 1.0, 1.0);
        }
        scale *= 0.97;
    };
    Layout {
        center,
        radius: want.radius * scale,
        petals,
        scale,
    }
}

/// The bulbs at `scale` on the tightest ring that keeps them clear, or
/// `None` when no ring inside the rect does. The ring starts about as much
/// wider than tall as the bulbs are (so every stem comes out about as long)
/// and stretches wider when the rect is short.
fn place(angles: &[f32], rect: Rect, want: Want, scale: f32) -> Option<Vec<PetalBox>> {
    let h = rect.height();
    let radius = want.radius * scale;
    let half = want.half * scale;
    if radius > h / 2.0 - EDGE * h || half.y * 2.0 > h {
        return None;
    }
    let base = ((half.x + radius) / (half.y + radius)).max(1.0);
    let clear = |petals: &[PetalBox]| {
        let clear_of_center = petals.iter().all(|p| {
            let to_center = rect.center() - p.bulb;
            to_center.length() - ellipse_radius(half, to_center) >= radius + STEM * h
        });
        clear_of_center
            && (0..petals.len()).all(|i| {
                (i + 1..petals.len()).all(|j| {
                    let d = petals[j].bulb - petals[i].bulb;
                    d.length() >= ellipse_radius(half, d) * 2.0 + GAP * h
                })
            })
    };
    (1..=40).map(|i| i as f32 / 40.0).find_map(|t| {
        [1.0, 1.3, 1.7, 2.2, 3.0].iter().find_map(|stretch| {
            let petals = place_at(angles, rect, half, t, base * stretch);
            clear(&petals).then_some(petals)
        })
    })
}

/// The bulbs of `half` size on a ring `t` of the way out to the rect's
/// top and bottom, `aspect` times as wide as tall (as far as the rect
/// allows).
fn place_at(angles: &[f32], rect: Rect, half: Vec2, t: f32, aspect: f32) -> Vec<PetalBox> {
    let (w, h) = (rect.width(), rect.height());
    let ry_max = (h / 2.0 - half.y - EDGE * h).max(0.0);
    let rx_max = (w / 2.0 - half.x - EDGE * h).max(0.0);
    let ry = (ry_max * t).min(rx_max / aspect);
    let rx = (ry * aspect).min(rx_max);
    angles
        .iter()
        .map(|&a| PetalBox {
            bulb: rect.center() + Vec2::new(rx * a.cos(), ry * a.sin()),
            half,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect() -> Rect {
        Rect::from_min_size(Pos2::ZERO, Vec2::new(1600.0, 800.0))
    }

    const WANT: Want = Want {
        radius: 130.0,
        half: Vec2::new(150.0, 100.0),
    };

    fn assert_clear(l: &Layout, r: Rect) {
        for (i, p) in l.petals.iter().enumerate() {
            let bounds = Rect::from_center_size(p.bulb, p.half * 2.0);
            assert!(
                r.expand(0.5).contains_rect(bounds),
                "petal {i} leaves the rect"
            );
            let to_center = l.center - p.bulb;
            assert!(
                to_center.length() - ellipse_radius(p.half, to_center) > l.radius,
                "petal {i} covers the centre"
            );
            for q in &l.petals[i + 1..] {
                let d = q.bulb - p.bulb;
                assert!(
                    d.length() > ellipse_radius(p.half, d) * 2.0,
                    "petals overlap"
                );
            }
        }
    }

    #[test]
    fn petals_start_on_top_and_go_clockwise() {
        let l = layout(4, rect(), WANT);
        assert_eq!(l.center, Pos2::new(800.0, 400.0));
        assert!(l.petals[0].bulb.y < l.center.y, "first on top");
        assert!((l.petals[0].bulb.x - l.center.x).abs() < 1e-3);
        assert!(l.petals[1].bulb.x > l.center.x, "second on the right");
        assert!(l.petals[2].bulb.y > l.center.y, "third below");
        assert_clear(&l, rect());
    }

    #[test]
    fn any_number_of_petals_fits_without_touching() {
        for n in 1..=12 {
            let l = layout(n, rect(), WANT);
            assert_eq!(l.petals.len(), n);
            assert_clear(&l, rect());
        }
        // more petals, a smaller flower
        assert!(layout(10, rect(), WANT).scale < layout(4, rect(), WANT).scale);
    }

    #[test]
    fn a_flower_that_fits_keeps_its_size_and_sits_close() {
        let small = Want {
            radius: 60.0,
            half: Vec2::new(60.0, 40.0),
        };
        let l = layout(4, rect(), small);
        assert_eq!(l.scale, 1.0);
        assert_eq!(l.radius, 60.0);
        // the ring is as tight as the stems allow, not out at the edges
        assert!(l.petals[0].bulb.y > 100.0, "{:?}", l.petals[0]);
    }

    #[test]
    fn ellipse_radius_along_the_axes() {
        let half = Vec2::new(30.0, 10.0);
        assert!((ellipse_radius(half, Vec2::X) - 30.0).abs() < 1e-4);
        assert!((ellipse_radius(half, -Vec2::Y) - 10.0).abs() < 1e-4);
        assert_eq!(ellipse_radius(half, Vec2::ZERO), 0.0);
    }
}
