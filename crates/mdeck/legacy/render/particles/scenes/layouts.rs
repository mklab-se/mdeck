//! The default scene for each layout, chosen from the slide alone.

use super::backdrop::backdrop;
use super::formation::{formation_for, formation_points};
use super::{bokeh, dust};
use crate::parser::{Block, Layout, Slide};
use crate::render::particles::{Drift, Group, Home, Palette, Rng, Scene, Tint};

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

/// Reveal step of each top-level list item, in order.
fn item_steps(items: &[crate::parser::ListItem]) -> Vec<usize> {
    items.iter().map(|item| item.step).collect()
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
    use crate::parser::{Inline, ListItem, ListMarker};
    use crate::render::particles::scenes::backdrop::backdrop_for;

    fn item(marker: ListMarker) -> ListItem {
        ListItem::new(marker, vec![Inline::Text("x".into())], vec![])
    }

    #[test]
    fn bullet_scene_has_one_cluster_per_item_with_its_step() {
        let slide = crate::parser::parse("# A\n\n- a\n+ b\n  - b1\n+ c\n- d")
            .slides
            .remove(0);
        let scene = for_slide(&slide, 3);
        let steps: Vec<Option<usize>> = scene
            .groups
            .iter()
            // item clusters carry a reveal step; backdrop clusters never do
            .filter(|g| matches!(g.home, Home::Cluster { .. }) && g.step.is_some())
            .map(|g| g.step)
            .collect();
        assert_eq!(steps, vec![Some(0), Some(1), Some(2), Some(0)]);
    }

    fn bullets(n: usize) -> Slide {
        Slide {
            blocks: vec![Block::List {
                ordered: false,
                start: 1,
                items: (0..n).map(|_| item(ListMarker::Static)).collect(),
            }],
            layout: Layout::Bullet,
            raw_source: String::new(),
            notes: None,
            ..Default::default()
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
                blocks: vec![],
                layout,
                raw_source: String::new(),
                notes: None,
                ..Default::default()
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
