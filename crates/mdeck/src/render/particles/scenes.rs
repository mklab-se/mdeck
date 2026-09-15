//! Scenes inferred from slide content.
//!
//! Every layout gets a default choreography that carries its meaning: a
//! title slide opens on a constellation, a bullet slide lights one cluster
//! per item as they reveal, a quote slide burns like a candle, a code slide
//! rains. Nothing here is authored per deck.

use std::sync::Arc;

use crate::parser::{Block, Layout, ListMarker, Slide};

use super::{Drift, Group, Home, Palette, Rng, Scene, Tint};

/// Faint dust everywhere, so no part of the slide is ever dead black.
fn dust(share: f32) -> Group {
    Group::new(
        share,
        Home::Field {
            u0: 0.02,
            v0: 0.04,
            u1: 0.98,
            v1: 0.96,
        },
    )
    .alpha(0.10, 0.30)
    .size(0.45, 0.9)
    .drift(Drift::Breathe {
        amp: 0.011,
        speed: 1.1,
    })
}

/// A few big, very soft lights out of focus.
fn bokeh(share: f32) -> Group {
    Group::new(
        share,
        Home::Field {
            u0: 0.02,
            v0: 0.05,
            u1: 0.98,
            v1: 0.95,
        },
    )
    .alpha(0.07, 0.18)
    .size(2.6, 4.2)
    .drift(Drift::Breathe {
        amp: 0.016,
        speed: 0.7,
    })
}

// ---------------------------------------------------------------------------
// Backdrops: what fills the dark on a slide without an illustration
// ---------------------------------------------------------------------------

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
fn backdrop(seed: u64, share: f32) -> Vec<Group> {
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

// ---------------------------------------------------------------------------
// Formations: how a bullet slide's item clusters are arranged
// ---------------------------------------------------------------------------

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
fn formation_points(formation: Formation, n: usize, rng: &mut Rng) -> Vec<(f32, f32)> {
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

/// The site's homepage: clusters hugging the flanks, copy in the dark middle.
pub fn constellation(seed: u64) -> Scene {
    let mut rng = Rng::new(seed);
    let mut groups = Vec::new();
    for c in 0..7 {
        let u = if c % 2 == 0 {
            rng.range(0.05, 0.30)
        } else {
            rng.range(0.62, 0.94)
        };
        let bright = c == 1;
        groups.push(
            Group::new(
                0.55 / 7.0,
                Home::Cluster {
                    u,
                    v: rng.range(0.10, 0.90),
                    r: rng.range(0.06, 0.13),
                    falloff: 0.6,
                },
            )
            .alpha(
                if bright { 0.7 } else { 0.4 },
                if bright { 1.0 } else { 0.9 },
            )
            .size(0.8, 1.3)
            .links(3),
        );
    }
    groups.push(dust(0.35));
    groups.push(bokeh(0.10));
    Scene::new(groups)
}

/// A point cloud illustration on the story stage (the right half of a copy
/// slide): fitted into the stage box with breathing room, warm, and lit from
/// the first step. `cloud_aspect` is the cloud's height over width.
pub fn illustration_stage(
    points: Arc<Vec<[f32; 2]>>,
    cloud_aspect: f32,
    layout: Layout,
    rect_aspect: f32,
) -> Scene {
    let stage = crate::render::story::stage_box(layout);
    let avail_w = stage.width() * 0.82;
    let avail_h = stage.height() * 0.82;
    // heights are slide-height fractions: h = w * aspect * (W / H)
    let mut w = avail_w;
    let mut h = w * cloud_aspect * rect_aspect;
    if h > avail_h {
        h = avail_h;
        w = h / (cloud_aspect * rect_aspect);
    }
    let (cu, cv) = (stage.center().x, stage.center().y);
    let mut scene = Scene::new(vec![
        Group::new(
            0.70,
            Home::Mask {
                points,
                u: cu - w / 2.0,
                v: cv - h / 2.0,
                w,
                h,
            },
        )
        .palette(Palette::Warm)
        .alpha(0.45, 0.95)
        .size(0.42, 0.8)
        .drift(Drift::Breathe {
            amp: 0.0018,
            speed: 0.7,
        }),
        dust(0.30).alpha(0.04, 0.16),
    ]);
    scene.link_alpha = 0.0;
    scene
}

/// A point cloud illustration behind a title: large, dim and soft, breathing
/// slowly under the centred copy, the way a backdrop is out of focus.
pub fn illustration_backdrop(
    points: Arc<Vec<[f32; 2]>>,
    cloud_aspect: f32,
    rect_aspect: f32,
) -> Scene {
    let mut h = 0.80;
    let mut w = h / (cloud_aspect * rect_aspect);
    if w > 0.72 {
        w = 0.72;
        h = w * cloud_aspect * rect_aspect;
    }
    let mut scene = Scene::new(vec![
        Group::new(
            0.72,
            Home::Mask {
                points,
                u: 0.5 - w / 2.0,
                v: 0.5 - h / 2.0,
                w,
                h,
            },
        )
        .palette(Palette::Warm)
        .alpha(0.12, 0.34)
        .size(0.5, 1.0)
        .drift(Drift::Breathe {
            amp: 0.004,
            speed: 0.5,
        }),
        dust(0.28).alpha(0.03, 0.12),
    ]);
    scene.link_alpha = 0.0;
    scene
}

/// A countdown digit: the glyph mask, bright and tight, with a little dust.
pub fn digit(points: Arc<Vec<[f32; 2]>>, glyph_aspect: f32, rect_aspect: f32) -> Scene {
    // The digit stands about half the slide tall and is stretched a quarter
    // wider than the face draws it: particles read better with more room.
    let h = 0.52;
    let w = h * glyph_aspect / rect_aspect * 1.25;
    let mut scene = Scene::new(vec![
        Group::new(
            0.86,
            Home::Mask {
                points,
                u: 0.5 - w / 2.0,
                v: 0.47 - h / 2.0,
                w,
                h,
            },
        )
        .palette(Palette::Site)
        .alpha(0.7, 1.0)
        .size(0.42, 0.7)
        .drift(Drift::Breathe {
            amp: 0.0012,
            speed: 1.4,
        }),
        dust(0.14).alpha(0.03, 0.10),
    ]);
    scene.link_alpha = 0.0;
    scene
}

/// The digit bursts: every particle flashes ember, flies straight out from
/// the centre and fades to black. The group is born dimmed, so its life eases
/// to zero, slowly enough to be seen on the way out.
pub fn burst() -> Scene {
    let mut scene = Scene::new(vec![
        Group::new(
            1.0,
            Home::Radial {
                u: 0.5,
                v: 0.47,
                r: 2.2,
            },
        )
        .palette(Palette::Warm)
        .alpha(0.8, 1.0)
        .size(0.6, 1.1)
        .drift(Drift::Still)
        .dim(0.0)
        .step(1)
        .hot(0),
    ]);
    scene.link_alpha = 0.0;
    scene.life_rate = 1.7;
    scene
}

/// The end, act one: the particles spell the words.
pub fn end_words(points: Arc<Vec<[f32; 2]>>, text_aspect: f32, rect_aspect: f32) -> Scene {
    // the words span about 56% of the slide width
    let w = 0.56;
    let h = w / text_aspect * rect_aspect;
    let mut scene = Scene::new(vec![
        Group::new(
            0.84,
            Home::Mask {
                points,
                u: 0.5 - w / 2.0,
                v: 0.47 - h / 2.0,
                w,
                h,
            },
        )
        .palette(Palette::Site)
        .alpha(0.7, 1.0)
        .size(0.42, 0.72)
        .drift(Drift::Breathe {
            amp: 0.0012,
            speed: 1.6,
        }),
        dust(0.16),
    ]);
    scene.link_alpha = 0.0;
    scene
}

/// The end, act two: the words let go and everything swirls around the
/// centre like sparks over a fire.
pub fn end_dance() -> Scene {
    let mut scene = Scene::new(vec![
        Group::new(
            0.55,
            Home::Cluster {
                u: 0.5,
                v: 0.47,
                r: 0.34,
                falloff: 0.5,
            },
        )
        .palette(Palette::Warm)
        .alpha(0.6, 1.0)
        .size(0.5, 0.9)
        .drift(Drift::Orbit { speed: 1.9 }),
        Group::new(
            0.30,
            Home::Cluster {
                u: 0.5,
                v: 0.47,
                r: 0.16,
                falloff: 0.7,
            },
        )
        .palette(Palette::Site)
        .alpha(0.7, 1.0)
        .size(0.4, 0.7)
        .drift(Drift::Orbit { speed: -2.8 }),
        dust(0.15),
    ]);
    scene.link_alpha = 0.0;
    scene
}

/// The end, act three: the bang. Everything flies out and fades to black.
pub fn end_bang() -> Scene {
    let mut scene = burst();
    if let Some(g) = scene.groups.first_mut() {
        g.home = Home::Radial {
            u: 0.5,
            v: 0.47,
            r: 2.6,
        };
    }
    scene.life_rate = 1.5;
    scene
}

/// A section divider: one warm mass on the right and slow rising embers.
fn section(seed: u64) -> Scene {
    let mut groups = vec![
        Group::new(
            0.40,
            Home::Cluster {
                u: 0.72,
                v: 0.40,
                r: 0.22,
                falloff: 0.45,
            },
        )
        .palette(Palette::Warm)
        .alpha(0.25, 0.85)
        .size(0.7, 1.4)
        .links(2),
        Group::new(
            0.18,
            Home::Field {
                u0: 0.50,
                v0: 0.05,
                u1: 0.98,
                v1: 0.98,
            },
        )
        .palette(Palette::Warm)
        .alpha(0.15, 0.5)
        .size(0.4, 0.9)
        .drift(Drift::Rise { speed: 0.6 }),
    ];
    groups.extend(backdrop(seed, 0.42));
    let mut scene = Scene::new(groups);
    scene.link_alpha = 0.07;
    scene
}

/// A quote: a candle pool low in the frame with embers drifting up.
fn candle(seed: u64) -> Scene {
    let mut groups = vec![
        Group::new(
            0.30,
            Home::Cluster {
                u: 0.62,
                v: 0.92,
                r: 0.28,
                falloff: 0.35,
            },
        )
        .palette(Palette::Warm)
        .alpha(0.12, 0.55)
        .size(0.8, 1.6),
        Group::new(
            0.22,
            Home::Field {
                u0: 0.40,
                v0: 0.10,
                u1: 0.85,
                v1: 1.0,
            },
        )
        .palette(Palette::Warm)
        .alpha(0.10, 0.45)
        .size(0.35, 0.8)
        .drift(Drift::Rise { speed: 0.45 }),
    ];
    groups.extend(backdrop(seed, 0.48));
    let mut scene = Scene::new(groups);
    scene.link_alpha = 0.0;
    scene
}

/// A code slide: pale rain on the right, dust elsewhere.
fn rain(seed: u64) -> Scene {
    let mut groups = vec![
        Group::new(
            0.42,
            Home::Field {
                u0: 0.56,
                v0: 0.0,
                u1: 0.98,
                v1: 1.0,
            },
        )
        .palette(Palette::Cold)
        .alpha(0.10, 0.42)
        .size(0.3, 0.7)
        .drift(Drift::Fall { speed: 1.0 }),
        Group::new(
            0.08,
            Home::Field {
                u0: 0.56,
                v0: 0.0,
                u1: 0.98,
                v1: 1.0,
            },
        )
        .palette(Palette::Solid(Tint::Ember))
        .alpha(0.15, 0.5)
        .size(0.3, 0.6)
        .drift(Drift::Fall { speed: 1.6 }),
    ];
    groups.extend(backdrop(seed, 0.50));
    let mut scene = Scene::new(groups);
    scene.link_alpha = 0.0;
    scene
}

/// A slide whose content fills the frame (charts, diagrams, images, tables):
/// keep the field quiet so the content reads.
fn quiet(seed: u64) -> Scene {
    let mut rng = Rng::new(seed);
    let corners = [(0.06, 0.12), (0.94, 0.88), (0.92, 0.10), (0.08, 0.90)];
    let mut groups = Vec::new();
    for _ in 0..2 {
        let (u, v) = corners[(rng.unit() * 3.99) as usize];
        groups.push(
            Group::new(
                0.12,
                Home::Cluster {
                    u,
                    v,
                    r: 0.12,
                    falloff: 0.6,
                },
            )
            .alpha(0.15, 0.45)
            .size(0.6, 1.1)
            .links(2),
        );
    }
    groups.extend(backdrop(seed, 0.76).into_iter().map(|g| {
        let (lo, hi) = g.alpha;
        g.alpha(lo * 0.6, hi * 0.6)
    }));
    let mut scene = Scene::new(groups);
    scene.link_alpha = 0.06;
    scene
}

/// Reveal step of each top-level list item, in order, using the same rule as
/// the text renderer (`+` starts a step, `*` joins the previous one).
fn item_steps(items: &[crate::parser::ListItem]) -> Vec<usize> {
    let mut counter = 0usize;
    items
        .iter()
        .map(|item| match item.marker {
            ListMarker::Static | ListMarker::Ordered => 0,
            ListMarker::NextStep => {
                counter += 1;
                counter
            }
            ListMarker::WithPrev => counter,
        })
        .collect()
}

/// Bullet and content slides: one cluster per item on the right flank, each
/// lighting with its reveal step. Copy sits on the left in the dark.
fn clusters(slide: &Slide, seed: u64) -> Scene {
    let mut rng = Rng::new(seed);
    let steps: Vec<usize> = slide
        .blocks
        .iter()
        .find_map(|b| match b {
            Block::List { items, .. } => Some(item_steps(items)),
            _ => None,
        })
        .unwrap_or_else(|| vec![0, 0, 0]);
    let n = steps.len().clamp(1, 9);
    let mut groups = Vec::with_capacity(n + 4);
    let points = formation_points(formation_for(seed), n, &mut rng);
    for (i, &step) in steps.iter().take(n).enumerate() {
        let (u, v) = points[i];
        let r = (0.075 - 0.004 * n as f32).max(0.045);
        groups.push(
            Group::new(
                0.52 / n as f32,
                Home::Cluster {
                    u,
                    v,
                    r,
                    falloff: 0.55,
                },
            )
            .alpha(0.45, 1.0)
            .size(0.7, 1.25)
            .links(3)
            .step(step),
        );
    }
    groups.extend(backdrop(seed, 0.48));
    Scene::new(groups)
}

pub fn for_slide(slide: &Slide, seed: u64) -> Scene {
    if seed == 1 && crate::render::ember::is_title(slide, 0) {
        return constellation(seed);
    }
    match slide.layout {
        Layout::Title => constellation(seed),
        Layout::Section => section(seed),
        Layout::Quote => candle(seed),
        Layout::Code => rain(seed),
        Layout::Bullet | Layout::Content | Layout::TwoColumn => clusters(slide, seed),
        Layout::Image | Layout::Gallery | Layout::Diagram | Layout::Visualization => quiet(seed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{Inline, ListItem};

    fn item(marker: ListMarker) -> ListItem {
        ListItem {
            marker,
            inlines: vec![Inline::Text("x".into())],
            children: vec![],
        }
    }

    fn ring_points() -> Arc<Vec<[f32; 2]>> {
        Arc::new(
            (0..32)
                .map(|i| {
                    let a = i as f32 / 32.0 * std::f32::consts::TAU;
                    [0.5 + 0.5 * a.cos(), 0.5 + 0.5 * a.sin()]
                })
                .collect(),
        )
    }

    #[test]
    fn stage_illustration_fits_inside_the_stage_box() {
        for (aspect, layout) in [
            (0.5, Layout::Bullet),
            (1.0, Layout::Quote),
            (2.5, Layout::Section),
        ] {
            let scene = illustration_stage(ring_points(), aspect, layout, 16.0 / 9.0);
            let stage = crate::render::story::stage_box(layout);
            let Home::Mask { u, v, w, h, .. } = &scene.groups[0].home else {
                panic!("first group is the mask");
            };
            assert!(
                *u >= stage.left() - 1e-4 && u + w <= stage.right() + 1e-4,
                "{layout:?} {aspect}: u {u} w {w}"
            );
            assert!(
                *v >= stage.top() - 1e-4 && v + h <= stage.bottom() + 1e-4,
                "{layout:?} {aspect}: v {v} h {h}"
            );
            // the cloud keeps its aspect on a 16:9 slide
            let px_aspect = (h * 9.0) / (w * 16.0);
            assert!(
                (px_aspect - aspect).abs() < 0.02,
                "{layout:?}: drawn aspect {px_aspect} vs {aspect}"
            );
        }
    }

    #[test]
    fn backdrop_illustration_is_centred_and_dim() {
        let scene = illustration_backdrop(ring_points(), 1.2, 16.0 / 9.0);
        let Home::Mask { u, v, w, h, .. } = &scene.groups[0].home else {
            panic!("first group is the mask");
        };
        assert!((u + w / 2.0 - 0.5).abs() < 1e-4 && (v + h / 2.0 - 0.5).abs() < 1e-4);
        assert!(
            scene.groups[0].alpha.1 < 0.5,
            "backdrop is bright: {:?}",
            scene.groups[0].alpha
        );
        // a very wide cloud is capped by width
        let wide = illustration_backdrop(ring_points(), 0.2, 16.0 / 9.0);
        let Home::Mask { w, .. } = &wide.groups[0].home else {
            panic!()
        };
        assert!(*w <= 0.72 + 1e-4);
    }

    #[test]
    fn bullet_scene_has_one_cluster_per_item_with_its_step() {
        let slide = Slide {
            directives: vec![],
            blocks: vec![Block::List {
                ordered: false,
                items: vec![
                    item(ListMarker::Static),
                    item(ListMarker::NextStep),
                    item(ListMarker::WithPrev),
                    item(ListMarker::NextStep),
                ],
            }],
            layout: Layout::Bullet,
            raw_source: String::new(),
            notes: None,
            story_hint: None,
            scene_script: None,
            illustration: None,
        };
        let scene = for_slide(&slide, 3);
        let steps: Vec<Option<usize>> = scene
            .groups
            .iter()
            // item clusters carry a reveal step; backdrop clusters never do
            .filter(|g| matches!(g.home, Home::Cluster { .. }) && g.step.is_some())
            .map(|g| g.step)
            .collect();
        assert_eq!(steps, vec![Some(0), Some(1), Some(1), Some(2)]);
    }

    fn bullets(n: usize) -> Slide {
        Slide {
            directives: vec![],
            blocks: vec![Block::List {
                ordered: false,
                items: (0..n).map(|_| item(ListMarker::Static)).collect(),
            }],
            layout: Layout::Bullet,
            raw_source: String::new(),
            notes: None,
            story_hint: None,
            scene_script: None,
            illustration: None,
        }
    }

    fn cluster_centres(scene: &Scene) -> Vec<(f32, f32)> {
        scene
            .groups
            .iter()
            .filter(|g| g.step.is_some())
            .filter_map(|g| match g.home {
                Home::Cluster { u, v, .. } => Some((u, v)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn consecutive_bullet_slides_differ_in_formation_and_backdrop() {
        for i in 1..=12u64 {
            assert_ne!(formation_for(i), formation_for(i + 1));
            assert_ne!(backdrop_for(i), backdrop_for(i + 1));
        }
        let a = cluster_centres(&for_slide(&bullets(3), 1));
        let b = cluster_centres(&for_slide(&bullets(3), 2));
        let moved: f32 = a
            .iter()
            .zip(&b)
            .map(|(p, q)| ((p.0 - q.0).powi(2) + (p.1 - q.1).powi(2)).sqrt())
            .sum();
        assert!(
            moved > 0.15,
            "identical slides look the same: {a:?} vs {b:?}"
        );
    }

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

    #[test]
    fn every_layout_yields_a_scene_with_dust() {
        for layout in [
            Layout::Title,
            Layout::Section,
            Layout::Quote,
            Layout::Code,
            Layout::Bullet,
            Layout::Content,
            Layout::TwoColumn,
            Layout::Image,
            Layout::Gallery,
            Layout::Diagram,
            Layout::Visualization,
        ] {
            let slide = Slide {
                directives: vec![],
                blocks: vec![],
                layout,
                raw_source: String::new(),
                notes: None,
                story_hint: None,
                scene_script: None,
                illustration: None,
            };
            let scene = for_slide(&slide, 1);
            assert!(scene.groups.len() >= 2, "{layout:?} has too few groups");
            let total: f32 = scene.groups.iter().map(|g| g.share).sum();
            assert!(
                total > 0.9 && total < 1.1,
                "{layout:?} shares sum to {total}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Content-aware scenes
// ---------------------------------------------------------------------------

use crate::render::hints::Hint;
use eframe::egui::{Pos2, Rect};

/// Turn the geometry a slide's renderers published into a scene that serves
/// it: embers off bar tops, runners along paths, sparks circling rings,
/// glints on points, and dust that keeps out of every frame.
pub fn from_hints(hints: &[Hint], rect: Rect, seed: u64) -> Scene {
    let mut rng = Rng::new(seed);
    let to_u = |x: f32| ((x - rect.left()) / rect.width()).clamp(0.0, 1.0);
    let to_v = |y: f32| ((y - rect.top()) / rect.height()).clamp(0.0, 1.0);
    let min_side = rect.width().min(rect.height());

    let mut groups: Vec<Group> = Vec::new();

    // Bars: merge segments that share a column (stacked bars) into one top.
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
    if !bars.is_empty() {
        let total_w: f32 = bars.iter().map(|b| b.width()).sum::<f32>().max(1.0);
        for b in &bars {
            let horizontal =
                b.width() > b.height() * 2.5 && b.left() < rect.left() + rect.width() * 0.45;
            let share =
                0.30 * b.width().min(b.height()).max(8.0) / total_w.max(bars.len() as f32 * 8.0);
            let group = if horizontal {
                // heat drifts off the right end of a horizontal bar
                Group::new(
                    share.max(0.02),
                    Home::Field {
                        u0: to_u(b.right()),
                        v0: to_v(b.top()),
                        u1: to_u(b.right() + b.height() * 1.8),
                        v1: to_v(b.bottom()),
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
                        u0: to_u(b.left() + b.width() * 0.08),
                        v0: to_v(b.top() - b.width().min(b.height()) * 0.9),
                        u1: to_u(b.right() - b.width() * 0.08),
                        v1: to_v(b.top()),
                    },
                )
                .drift(Drift::Rise { speed: 0.55 })
            };
            groups.push(
                group
                    .palette(Palette::Warm)
                    .alpha(0.25, 0.65)
                    .size(0.35, 0.7),
            );
        }
    }

    // Paths: runners in the drawing direction.
    let mut path_i = 0usize;
    for h in hints {
        if let Hint::Path(pts) = h
            && pts.len() >= 2
        {
            let points: Vec<[f32; 2]> = pts.iter().map(|p| [to_u(p.x), to_v(p.y)]).collect();
            let length: f32 = pts
                .windows(2)
                .map(|w| ((w[1].x - w[0].x).powi(2) + (w[1].y - w[0].y).powi(2)).sqrt())
                .sum::<f32>()
                / min_side;
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

    // Circles: keep the largest per centre, sparks circle just outside it.
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
    for (i, (c, r)) in circles.iter().take(4).enumerate() {
        groups.push(
            Group::new(
                0.14,
                Home::Ring {
                    u: to_u(c.x),
                    v: to_v(c.y),
                    r: r / min_side * 1.09,
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
            }),
        );
    }

    // Points: a glint on each.
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
    if !points.is_empty() {
        let share = (0.20 / points.len() as f32).min(0.03);
        for p in points.iter().take(40) {
            groups.push(
                Group::new(
                    share,
                    Home::Cluster {
                        u: to_u(p.x),
                        v: to_v(p.y),
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
                }),
            );
        }
    }

    // Dust everywhere the content is not: bands around the union of frames.
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
    let used: f32 = groups.iter().map(|g| g.share).sum();
    let dust_share = (1.0 - used).max(0.25);
    match frame {
        Some(f) => {
            // a very faint haze over everything, so the content never floats
            // in dead black, plus slightly denser dust in the margins
            groups.push(dust(dust_share * 0.45).alpha(0.03, 0.10));
            let (l, t, r, b) = (
                to_u(f.left()),
                to_v(f.top()),
                to_u(f.right()),
                to_v(f.bottom()),
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
            for ((u0, v0, u1, v1), a) in bands.iter().zip(areas) {
                if a > 0.02 {
                    groups.push(
                        Group::new(
                            dust_share * 0.45 * a,
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
                        }),
                    );
                }
            }
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
        }
        None => groups.push(dust(dust_share)),
    }

    let mut scene = Scene::new(groups);
    scene.link_alpha = 0.06;
    scene
}

#[cfg(test)]
mod hint_tests {
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
