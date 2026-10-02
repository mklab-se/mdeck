//! Formations: how a bullet slide's item clusters are arranged.

use crate::engines::particles::Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Formation {
    Arc,
    Lazy,
    Ring,
    Diagonal,
    Scatter,
    Column,
}

const FORMATIONS: [Formation; 6] = [
    Formation::Arc,
    Formation::Lazy,
    Formation::Ring,
    Formation::Diagonal,
    Formation::Scatter,
    Formation::Column,
];

/// The formation for a slide (`seed` is the 1-based slide number).
pub fn formation_for(seed: u64) -> Formation {
    FORMATIONS[(seed as usize) % FORMATIONS.len()]
}

/// Centres for `n` item clusters in the right half, in slide fractions.
pub(super) fn formation_points(formation: Formation, n: usize, rng: &mut Rng) -> Vec<(f32, f32)> {
    use std::f32::consts::PI;
    let f_of = |i: usize| {
        if n == 1 {
            0.5
        } else {
            i as f32 / (n - 1) as f32
        }
    };
    let jitter = |rng: &mut Rng, (u, v): (f32, f32)| {
        (u + rng.range(-0.02, 0.02), v + rng.range(-0.015, 0.015))
    };
    let pts: Vec<(f32, f32)> = match formation {
        Formation::Lazy => (0..n)
            .map(|i| {
                let wobble = ((i as f32) * 1.7).sin() * 0.08;
                (0.70 + wobble, 0.16 + f_of(i) * 0.68)
            })
            .collect(),
        Formation::Arc => (0..n)
            .map(|i| {
                let f = f_of(i);
                (0.62 + 0.20 * (PI * f).sin(), 0.16 + f * 0.68)
            })
            .collect(),
        Formation::Diagonal => (0..n)
            .map(|i| {
                let f = f_of(i);
                (0.58 + 0.30 * f, 0.18 + f * 0.64)
            })
            .collect(),
        Formation::Column => (0..n).map(|i| (0.75, 0.16 + f_of(i) * 0.68)).collect(),
        Formation::Ring if n >= 3 => (0..n)
            .map(|i| {
                let a = -PI / 2.0 + 2.0 * PI * i as f32 / n as f32;
                (0.74 + 0.15 * a.cos(), 0.50 + 0.30 * a.sin())
            })
            .collect(),
        Formation::Ring => (0..n).map(|i| (0.75, 0.16 + f_of(i) * 0.68)).collect(),
        Formation::Scatter => {
            // golden-ratio strides across, evenly spaced down in a shuffled
            // order, so no two items share a column or a row
            let mut rows: Vec<usize> = (0..n).collect();
            for i in (1..n).rev() {
                let j = (rng.unit() * (i + 1) as f32) as usize;
                rows.swap(i, j.min(i));
            }
            (0..n)
                .map(|i| {
                    let u = 0.58 + 0.32 * ((i as f32 * 0.618_034 + 0.3) % 1.0);
                    (u, 0.16 + f_of(rows[i]) * 0.68)
                })
                .collect()
        }
    };
    pts.into_iter().map(|p| jitter(rng, p)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_formation_keeps_items_in_the_right_half_and_apart() {
        for f in FORMATIONS {
            for n in [1, 2, 3, 5, 9] {
                let mut rng = Rng::new(3);
                let pts = formation_points(f, n, &mut rng);
                assert_eq!(pts.len(), n);
                for (u, v) in &pts {
                    assert!(*u > 0.52 && *u < 0.96, "{f:?} n={n}: u {u}");
                    assert!(*v > 0.08 && *v < 0.92, "{f:?} n={n}: v {v}");
                }
                for i in 0..n {
                    for j in i + 1..n {
                        let d =
                            ((pts[i].0 - pts[j].0).powi(2) + (pts[i].1 - pts[j].1).powi(2)).sqrt();
                        assert!(d > 0.06, "{f:?} n={n}: items {i} and {j} overlap ({d})");
                    }
                }
            }
        }
    }
}
