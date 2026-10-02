//! Scenes the engine picks over a slide's own layout: an illustration on
//! the stage or behind a title, a countdown digit and its burst, and the
//! three acts of the end.

use std::sync::Arc;

use super::dust;
use crate::render::particles::{Drift, Group, Home, Palette, Scene};

/// A point cloud illustration on the stage (the right half of a copy
/// slide): fitted into the stage box with breathing room, warm, and lit from
/// the first step. `cloud_aspect` is the cloud's height over width.
pub fn illustration_stage(
    points: Arc<Vec<[f32; 2]>>,
    place: crate::engines::stage::Place,
) -> Scene {
    let crate::engines::stage::Place { u, v, w, h } = place;
    let mut scene = Scene::new(vec![
        Group::new(0.70, Home::Mask { points, u, v, w, h })
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
    place: crate::engines::stage::Place,
) -> Scene {
    let crate::engines::stage::Place { u, v, w, h } = place;
    let mut scene = Scene::new(vec![
        Group::new(0.72, Home::Mask { points, u, v, w, h })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Layout;

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
            let scene = illustration_stage(
                ring_points(),
                crate::engines::stage::figure_box(aspect, layout, 16.0 / 9.0, false),
            );
            let stage = crate::engines::stage::stage_box(layout);
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
        let scene = illustration_backdrop(
            ring_points(),
            crate::engines::stage::figure_box(1.2, Layout::Title, 16.0 / 9.0, true),
        );
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
        let wide = illustration_backdrop(
            ring_points(),
            crate::engines::stage::figure_box(0.2, Layout::Title, 16.0 / 9.0, true),
        );
        let Home::Mask { w, .. } = &wide.groups[0].home else {
            panic!()
        };
        assert!(*w <= 0.72 + 1e-4);
    }
}
