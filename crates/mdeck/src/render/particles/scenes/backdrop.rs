//! Backdrops: what fills the dark on a slide without an illustration.

use super::{bokeh, dust};
use crate::render::particles::{Drift, Group, Home, Palette, Rng};

/// The field behind a slide's own scene. Slides walk this list by number, so
/// neighbours never share one and a run of look-alike bullet slides still
/// differs from one to the next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backdrop {
    /// A star field drifting slowly forward.
    Stars,
    /// The original soft dust and bokeh.
    Dust,
    /// Two spiral arms turning about the centre.
    Galaxy,
    /// A few large clouds, out of focus.
    Nebula,
}

const BACKDROPS: [Backdrop; 4] = [
    Backdrop::Stars,
    Backdrop::Dust,
    Backdrop::Galaxy,
    Backdrop::Nebula,
];

/// The backdrop for a slide (`seed` is the 1-based slide number).
pub fn backdrop_for(seed: u64) -> Backdrop {
    BACKDROPS[(seed as usize) % BACKDROPS.len()]
}

/// Groups that fill `share` of the pool with the slide's backdrop.
pub(super) fn backdrop(seed: u64, share: f32) -> Vec<Group> {
    match backdrop_for(seed) {
        Backdrop::Dust => vec![dust(share * 0.80), bokeh(share * 0.20)],
        Backdrop::Stars => vec![
            Group::new(
                share * 0.85,
                Home::Field {
                    u0: 0.0,
                    v0: 0.0,
                    u1: 1.0,
                    v1: 1.0,
                },
            )
            .alpha(0.06, 0.34)
            .size(0.3, 1.0)
            .drift(Drift::Forward {
                u: 0.5,
                v: 0.5,
                speed: 0.02,
            }),
            bokeh(share * 0.15).alpha(0.03, 0.08),
        ],
        Backdrop::Galaxy => {
            let mut groups: Vec<Group> = (0..2)
                .map(|arm| {
                    Group::new(
                        share * 0.36,
                        Home::Path {
                            points: spiral_arm(arm as f32 * std::f32::consts::PI),
                            spread: 0.032,
                        },
                    )
                    .alpha(0.16, 0.5)
                    .size(0.4, 1.1)
                    .drift(Drift::Orbit { speed: 0.012 })
                })
                .collect();
            groups.push(
                Group::new(
                    share * 0.15,
                    Home::Cluster {
                        u: 0.5,
                        v: 0.5,
                        r: 0.07,
                        falloff: 0.5,
                    },
                )
                .palette(Palette::Warm)
                .alpha(0.12, 0.34)
                .size(0.5, 1.2)
                .drift(Drift::Orbit { speed: 0.02 }),
            );
            groups.push(dust(share * 0.13).alpha(0.03, 0.12));
            groups
        }
        Backdrop::Nebula => {
            let mut rng = Rng::new(seed ^ 0x4E45_4255);
            let mut groups: Vec<Group> = (0..3)
                .map(|_| {
                    Group::new(
                        share * 0.22,
                        Home::Cluster {
                            u: rng.range(0.15, 0.85),
                            v: rng.range(0.2, 0.8),
                            r: rng.range(0.18, 0.30),
                            falloff: 0.5,
                        },
                    )
                    .palette(if rng.unit() < 0.5 {
                        Palette::Warm
                    } else {
                        Palette::Cold
                    })
                    .alpha(0.03, 0.11)
                    .size(2.0, 3.6)
                    .drift(Drift::Breathe {
                        amp: 0.02,
                        speed: 0.4,
                    })
                })
                .collect();
            groups.push(dust(share * 0.34).alpha(0.04, 0.16));
            groups
        }
    }
}

/// One arm of an Archimedean spiral about the slide centre, as slide
/// fractions (heights stretched for a wide slide), starting at `phase`.
fn spiral_arm(phase: f32) -> Vec<[f32; 2]> {
    (0..48)
        .map(|i| {
            let t = i as f32 / 47.0;
            let theta = 0.5 + t * 2.2 * std::f32::consts::PI;
            let r = 0.015 + 0.046 * theta;
            let a = theta + phase;
            [0.5 + r * a.cos() * 1.15, 0.5 + r * a.sin() * 1.45]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_backdrop_fills_its_share() {
        for (i, kind) in BACKDROPS.iter().enumerate() {
            let groups = backdrop(i as u64, 0.5);
            assert_eq!(backdrop_for(i as u64), *kind);
            let total: f32 = groups.iter().map(|g| g.share).sum();
            assert!((total - 0.5).abs() < 0.01, "{kind:?} fills {total}");
            if *kind == Backdrop::Stars {
                assert!(
                    groups
                        .iter()
                        .any(|g| matches!(g.drift, Drift::Forward { .. }))
                );
            }
        }
    }
}
