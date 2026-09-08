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
    .alpha(0.06, 0.22)
    .size(0.4, 0.8)
    .drift(Drift::Breathe {
        amp: 0.006,
        speed: 0.8,
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
    .alpha(0.05, 0.14)
    .size(2.6, 4.2)
    .drift(Drift::Breathe {
        amp: 0.01,
        speed: 0.5,
    })
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

/// Particles assemble into a mask (the logo).
pub fn mask(points: Arc<Vec<[f32; 2]>>, aspect: f32, rect_aspect: f32, size: f32) -> Scene {
    // `size` is the mask's width as a fraction of the slide width; the height
    // follows from the image aspect and the slide aspect.
    let w = size;
    let h = size / aspect * rect_aspect;
    let mut scene = Scene::new(vec![
        Group::new(
            0.9,
            Home::Mask {
                points,
                u: 0.5 - w / 2.0,
                v: 0.46 - h / 2.0,
                w,
                h,
            },
        )
        .alpha(0.6, 1.0)
        .size(0.36, 0.6)
        .drift(Drift::Still),
        dust(0.1),
    ]);
    scene.link_alpha = 0.0;
    scene
}

/// A countdown digit: the glyph mask, bright and tight, with a little dust.
pub fn digit(points: Arc<Vec<[f32; 2]>>, glyph_aspect: f32, rect_aspect: f32) -> Scene {
    // The digit stands about half the slide tall.
    let h = 0.52;
    let w = h * glyph_aspect / rect_aspect;
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

/// The digit bursts: every particle flies straight out from the centre and
/// fades to black (the group is born dimmed, so its life eases to zero).
pub fn burst() -> Scene {
    let mut scene = Scene::new(vec![
        Group::new(
            1.0,
            Home::Radial {
                u: 0.5,
                v: 0.47,
                r: 1.6,
            },
        )
        .palette(Palette::Warm)
        .alpha(0.7, 1.0)
        .size(0.5, 0.9)
        .drift(Drift::Still)
        .dim(0.0)
        .step(1),
    ]);
    scene.link_alpha = 0.0;
    scene
}

/// A section divider: one warm mass on the right and slow rising embers.
fn section() -> Scene {
    let mut scene = Scene::new(vec![
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
        dust(0.32),
        bokeh(0.10),
    ]);
    scene.link_alpha = 0.07;
    scene
}

/// A quote: a candle pool low in the frame with embers drifting up.
fn candle() -> Scene {
    let mut scene = Scene::new(vec![
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
        dust(0.38).palette(Palette::Cold),
        bokeh(0.10),
    ]);
    scene.link_alpha = 0.0;
    scene
}

/// A code slide: pale rain on the right, dust elsewhere.
fn rain() -> Scene {
    let mut scene = Scene::new(vec![
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
        dust(0.40),
        bokeh(0.10),
    ]);
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
    groups.push(dust(0.66).alpha(0.04, 0.16));
    groups.push(bokeh(0.10).alpha(0.03, 0.09));
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
    let mut groups = Vec::with_capacity(n + 3);
    // Items arranged on a lazy S through the right half, top to bottom.
    for (i, &step) in steps.iter().take(n).enumerate() {
        let f = if n == 1 {
            0.5
        } else {
            i as f32 / (n - 1) as f32
        };
        let wobble = ((i as f32) * 1.7).sin() * 0.08;
        let u = 0.70 + wobble + rng.range(-0.03, 0.03);
        let v = 0.16 + f * 0.68 + rng.range(-0.02, 0.02);
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
    groups.push(dust(0.38));
    groups.push(bokeh(0.10));
    Scene::new(groups)
}

/// The scene for a slide. `seed` keeps a slide's scene stable between frames
/// and reloads; different slides get different seeds.
pub fn for_slide(slide: &Slide, seed: u64) -> Scene {
    if seed == 1 && crate::render::ember::is_title(slide, 0) {
        return constellation(seed);
    }
    match slide.layout {
        Layout::Title => constellation(seed),
        Layout::Section => section(),
        Layout::Quote => candle(),
        Layout::Code => rain(),
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
        };
        let scene = for_slide(&slide, 3);
        let steps: Vec<Option<usize>> = scene
            .groups
            .iter()
            .filter(|g| matches!(g.home, Home::Cluster { .. }))
            .map(|g| g.step)
            .collect();
        assert_eq!(steps, vec![Some(0), Some(1), Some(1), Some(2)]);
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
