//! Particle silhouettes for the story cast.
//!
//! Each kind is a set of polylines in a unit square (x 0..1, y 0..1 with the
//! kind's own aspect), sampled uniformly by length into points the particle
//! field can use as a mask. Outline first, detail never: at deck scale a head
//! and a pair of shoulders read as a person, and anything finer reads as noise.

use std::sync::{Arc, Mutex, OnceLock};

use super::Kind;

type Poly = Vec<[f32; 2]>;

/// Height / width of the kind's bounding box.
pub fn aspect(kind: Kind) -> f32 {
    match kind {
        Kind::Person | Kind::Hooded => 1.35,
        Kind::Orb => 1.0,
        Kind::Doc => 1.25,
        Kind::Docs => 1.15,
        Kind::Inbox => 0.6,
        Kind::Db => 1.1,
        Kind::Cloud => 0.62,
        Kind::Laptop => 0.7,
        Kind::Folder => 0.8,
        Kind::Mail => 0.68,
        Kind::Gate => 1.7,
        Kind::Box => 0.85,
    }
}

fn circle(cx: f32, cy: f32, r: f32, from: f32, to: f32, n: usize) -> Poly {
    (0..=n)
        .map(|i| {
            let a = from + (to - from) * i as f32 / n as f32;
            [cx + a.cos() * r, cy + a.sin() * r]
        })
        .collect()
}

fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> Poly {
    vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1], [x0, y0]]
}

/// Polylines for a kind, in a box of width 1 and height [`aspect`].
fn polylines(kind: Kind) -> Vec<Poly> {
    use std::f32::consts::PI;
    let h = aspect(kind);
    match kind {
        Kind::Person => vec![
            // head
            circle(0.5, 0.26, 0.20, 0.0, 2.0 * PI, 40),
            // shoulders: a wide cap
            circle(0.5, h, 0.50, PI, 2.0 * PI, 40),
            // shoulder base line
            vec![[0.0, h], [1.0, h]],
        ],
        Kind::Hooded => vec![
            // hood: a pointed arch over the head
            vec![
                [0.16, 0.62],
                [0.30, 0.22],
                [0.50, 0.04],
                [0.70, 0.22],
                [0.84, 0.62],
            ],
            // dark face opening (a small arc)
            circle(0.5, 0.40, 0.12, 0.2 * PI, 0.8 * PI, 12),
            circle(0.5, h, 0.50, PI, 2.0 * PI, 40),
            vec![[0.0, h], [1.0, h]],
        ],
        Kind::Box => vec![rect(0.02, 0.02, 0.98, h - 0.02)],
        Kind::Orb => vec![
            circle(0.5, 0.5, 0.48, 0.0, 2.0 * PI, 56),
            circle(0.5, 0.5, 0.30, 0.0, 2.0 * PI, 30),
        ],
        Kind::Doc => vec![
            // sheet with a folded corner
            vec![
                [0.05, 0.02],
                [0.70, 0.02],
                [0.95, 0.27],
                [0.95, h - 0.02],
                [0.05, h - 0.02],
                [0.05, 0.02],
            ],
            vec![[0.70, 0.02], [0.70, 0.27], [0.95, 0.27]],
            vec![[0.22, 0.55], [0.78, 0.55]],
            vec![[0.22, 0.75], [0.78, 0.75]],
            vec![[0.22, 0.95], [0.60, 0.95]],
        ],
        Kind::Docs => vec![
            rect(0.18, 0.02, 0.98, h - 0.24),
            rect(0.10, 0.12, 0.90, h - 0.13),
            rect(0.02, 0.22, 0.82, h - 0.02),
        ],
        Kind::Inbox => vec![
            // tray: open top, slot front
            vec![
                [0.02, 0.10],
                [0.02, h - 0.02],
                [0.98, h - 0.02],
                [0.98, 0.10],
            ],
            vec![
                [0.02, 0.32],
                [0.30, 0.32],
                [0.36, 0.44],
                [0.64, 0.44],
                [0.70, 0.32],
                [0.98, 0.32],
            ],
        ],
        Kind::Db => vec![
            circle(0.5, 0.16, 0.48, 0.0, 2.0 * PI, 40)
                .into_iter()
                .map(|[x, y]| [x, 0.16 + (y - 0.16) * 0.32])
                .collect(),
            vec![[0.02, 0.16], [0.02, h - 0.16]],
            vec![[0.98, 0.16], [0.98, h - 0.16]],
            circle(0.5, h - 0.16, 0.48, 0.0, PI, 24)
                .into_iter()
                .map(|[x, y]| [x, h - 0.16 + (y - (h - 0.16)) * 0.32])
                .collect(),
            circle(0.5, 0.50, 0.48, 0.0, PI, 24)
                .into_iter()
                .map(|[x, y]| [x, 0.50 + (y - 0.50) * 0.32])
                .collect(),
        ],
        Kind::Cloud => vec![
            circle(0.30, 0.42, 0.20, 0.55 * PI, 1.85 * PI, 24),
            circle(0.52, 0.30, 0.24, 0.95 * PI, 2.05 * PI, 28),
            circle(0.76, 0.42, 0.18, 1.15 * PI, 2.45 * PI, 24),
            vec![[0.12, 0.60], [0.90, 0.60]],
        ],
        Kind::Laptop => vec![
            rect(0.14, 0.02, 0.86, 0.46),
            vec![[0.02, 0.62], [0.14, 0.46]],
            vec![[0.98, 0.62], [0.86, 0.46]],
            vec![
                [0.02, 0.62],
                [0.98, 0.62],
                [0.98, h - 0.02],
                [0.02, h - 0.02],
                [0.02, 0.62],
            ],
        ],
        Kind::Folder => vec![
            vec![
                [0.02, 0.14],
                [0.36, 0.14],
                [0.44, 0.04],
                [0.98, 0.04],
                [0.98, h - 0.02],
                [0.02, h - 0.02],
                [0.02, 0.14],
            ],
            vec![[0.02, 0.30], [0.98, 0.30]],
        ],
        Kind::Mail => vec![
            rect(0.02, 0.02, 0.98, h - 0.02),
            vec![[0.02, 0.02], [0.50, 0.40], [0.98, 0.02]],
        ],
        Kind::Gate => vec![
            vec![[0.35, 0.0], [0.35, h]],
            vec![[0.65, 0.0], [0.65, h]],
            vec![[0.0, 0.5], [1.0, 0.5]],
            vec![[0.0, h - 0.5], [1.0, h - 0.5]],
        ],
    }
}

/// Sample every polyline of a kind uniformly by length into points in the
/// unit square (y scaled by 1 / aspect so the mask fits a `w × h` box).
fn sample(kind: Kind) -> Arc<Vec<[f32; 2]>> {
    let h = aspect(kind);
    let mut pts = Vec::new();
    let step = 0.012; // spacing in box units
    for poly in polylines(kind) {
        for w in poly.windows(2) {
            let (a, b) = (w[0], w[1]);
            let len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
            let n = (len / step).ceil().max(1.0) as usize;
            for i in 0..n {
                let t = i as f32 / n as f32;
                pts.push([a[0] + (b[0] - a[0]) * t, (a[1] + (b[1] - a[1]) * t) / h]);
            }
        }
    }
    Arc::new(pts)
}

type Points = Arc<Vec<[f32; 2]>>;

/// Cached mask points for a kind.
pub fn outline(kind: Kind) -> Points {
    static CACHE: OnceLock<Mutex<Vec<(Kind, Points)>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(Vec::new()));
    let mut guard = cache.lock().unwrap_or_else(|p| p.into_inner());
    if let Some((_, pts)) = guard.iter().find(|(k, _)| *k == kind) {
        return Arc::clone(pts);
    }
    let pts = sample(kind);
    guard.push((kind, Arc::clone(&pts)));
    pts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_samples_inside_the_unit_square() {
        for kind in Kind::ALL {
            let pts = outline(kind);
            assert!(pts.len() > 40, "{kind:?} has too few points");
            for p in pts.iter() {
                assert!(
                    p[0] >= -0.01 && p[0] <= 1.01,
                    "{kind:?} x out of range: {}",
                    p[0]
                );
                assert!(
                    p[1] >= -0.01 && p[1] <= 1.01,
                    "{kind:?} y out of range: {}",
                    p[1]
                );
            }
        }
    }
}
