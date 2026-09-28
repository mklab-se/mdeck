//! Staging: a script placed on the slide's stage as a particle scene plus
//! the labels to draw over it.

use std::sync::Arc;

use eframe::egui::{Pos2, Rect};

use super::{Fill, FlowColor, Member, Script, is_figure};
use crate::parser::Layout;
use crate::render::illustration::Library;
use crate::render::particles::{Drift, Group, Home, Palette, Scene, Tint};

/// A label to draw over the field, in slide fractions, tied to a group so it
/// fades with it.
#[derive(Clone, Debug)]
pub struct Label {
    pub text: String,
    /// Anchor (top-centre of the label) as slide fractions.
    pub u: f32,
    pub v: f32,
    pub group: usize,
    pub figure: bool,
}

/// A staged script: the particle scene plus the labels to draw over it.
#[derive(Clone, Debug)]
pub struct Staged {
    pub scene: Scene,
    pub labels: Vec<Label>,
}

/// Whether a story can play on this slide at all. A story needs a stage:
/// the right half of a slide whose copy Ember lays out on the left (bullets,
/// content, quotes, section dividers). Slides whose content fills the frame
/// (code, charts, diagrams, tables, images, two columns) and title slides
/// (centred copy) never play a story; their field stays content-aware and
/// quiet, whatever a sidecar says.
pub fn allowed(slide: &crate::parser::Slide, index: usize) -> bool {
    crate::render::ember::handles(slide) && !crate::render::ember::is_title(slide, index)
}

/// The part of the slide the story plays on, as slide fractions.
pub fn stage_box(layout: Layout) -> Rect {
    match layout {
        // Quotes run wider, so the stage is narrower.
        Layout::Quote => Rect::from_min_max(Pos2::new(0.64, 0.10), Pos2::new(0.96, 0.90)),
        // Copy sits left; the stage is the right half.
        _ => Rect::from_min_max(Pos2::new(0.52, 0.10), Pos2::new(0.96, 0.90)),
    }
}

/// Geometry of a staged cast member, as slide fractions.
struct Placed<'a> {
    member: &'a Member,
    cloud: Option<Arc<crate::render::illustration::Cloud>>,
    /// Centre.
    u: f32,
    v: f32,
    /// Half extents.
    hw: f32,
    hh: f32,
}

impl Placed<'_> {
    /// Point on this member's bounding box edge in the direction of `(tu, tv)`.
    fn edge_toward(&self, tu: f32, tv: f32, aspect: f32) -> [f32; 2] {
        let dx = tu - self.u;
        let dy = (tv - self.v) / aspect;
        let len = (dx * dx + dy * dy).sqrt().max(1e-4);
        let (nx, ny) = (dx / len, dy / len);
        // scale to the box edge (ellipse approximation, slightly outside)
        let k = 1.0
            / ((nx / (self.hw * 1.15)).powi(2) + (ny / (self.hh * 1.15 / aspect)).powi(2)).sqrt();
        [self.u + nx * k, self.v + ny * k * aspect]
    }
}

/// Turn a script into a particle scene for a slide of `layout`, drawn in a
/// rect with the given width/height `aspect`. Cast looks come from `lib`; a
/// kind that does not resolve is drawn as a faint ring so its label still
/// stands (`mdeck --check` reports it).
pub fn stage(script: &Script, layout: Layout, aspect: f32, lib: &mut Library) -> Staged {
    let stage = stage_box(layout);
    let sw = stage.width();
    let sh = stage.height();
    // sizes as fractions of the slide width; heights are corrected by aspect
    let fig_w = 0.092;
    let prop_w = 0.096;
    // no member may be taller than this (fraction of slide height), so a
    // 3×3 grid with labels never collides
    let max_hh = 0.105;

    let placed: Vec<Placed> = script
        .cast
        .iter()
        .map(|m| {
            let (cu, cv) = m.cell.uv();
            let cloud = lib.get(&m.kind);
            let w = if is_figure(&m.kind) { fig_w } else { prop_w };
            let ratio = cloud.as_ref().map(|c| c.aspect).unwrap_or(1.0); // height / width
            let mut hw = w / 2.0;
            let mut hh = w * ratio / 2.0 * aspect;
            if hh > max_hh {
                let k = max_hh / hh;
                hh = max_hh;
                hw *= k;
            }
            Placed {
                member: m,
                cloud,
                u: stage.left() + cu * sw,
                v: stage.top() + cv * sh,
                hw,
                hh,
            }
        })
        .collect();

    let cast_n = placed.len().max(1) as f32;
    let mut groups: Vec<Group> = Vec::new();
    let mut labels: Vec<Label> = Vec::new();

    for p in &placed {
        let m = p.member;
        let step = script.show_step(&m.id);
        let home = match &p.cloud {
            Some(c) => Home::Mask {
                points: Arc::clone(&c.points),
                u: p.u - p.hw,
                v: p.v - p.hh,
                w: p.hw * 2.0,
                h: p.hh * 2.0,
            },
            None => Home::Ring {
                u: p.u,
                v: p.v,
                r: p.hw * aspect * 0.8,
                width: 0.01,
            },
        };
        let mut g = Group::new(0.60 / cast_n, home)
            .drift(Drift::Breathe {
                amp: 0.0015,
                speed: 0.6,
            })
            .size(0.42, 0.78)
            .dim(0.0)
            .step(step);
        g = match m.kind.as_str() {
            "hooded" => g.palette(Palette::Solid(Tint::Pale)).alpha(0.45, 0.9),
            k if is_figure(k) => g.palette(Palette::Site).alpha(0.6, 1.0),
            _ => g.palette(Palette::Cold).alpha(0.5, 0.95),
        };
        if let Some(h) = script.hot_step(&m.id) {
            g = g.hot(h);
        }
        labels.push(Label {
            text: m.label.clone().unwrap_or_else(|| m.id.clone()),
            u: p.u,
            v: p.v + p.hh + 0.012 * aspect,
            group: groups.len(),
            figure: is_figure(&m.kind),
        });
        groups.push(g);

        // Fills
        if let Some(fill) = m.fill.filter(|f| *f != Fill::Outline) {
            let inner = Group::new(
                0.10 / cast_n,
                Home::Cluster {
                    u: p.u,
                    v: p.v,
                    r: p.hw * 0.55 / aspect.max(0.01) * aspect,
                    falloff: 0.7,
                },
            )
            .size(0.25, 0.5)
            .dim(0.0)
            .step(step);
            let inner = match fill {
                Fill::Brain => inner
                    .palette(Palette::Warm)
                    .alpha(0.25, 0.8)
                    .drift(Drift::Breathe {
                        amp: 0.006,
                        speed: 2.2,
                    })
                    .links(2),
                Fill::Hot => inner.palette(Palette::Solid(Tint::Ember)).alpha(0.2, 0.6),
                Fill::Cold => inner.palette(Palette::Cold).alpha(0.15, 0.5),
                Fill::Outline => inner,
            };
            let inner = match script.hot_step(&m.id) {
                Some(h) => inner.hot(h),
                None => inner,
            };
            groups.push(inner);
        }
    }

    // Flows: bezier from the edge of `from` to the edge of `to`.
    for (fi, f) in script.flows.iter().enumerate() {
        let (Some(a), Some(b)) = (
            placed.iter().find(|p| p.member.id == f.from),
            placed.iter().find(|p| p.member.id == f.to),
        ) else {
            continue;
        };
        let p0 = a.edge_toward(b.u, b.v, aspect);
        let p1 = b.edge_toward(a.u, a.v, aspect);
        let points = bezier_points(p0, p1, if fi % 2 == 0 { 0.10 } else { -0.10 }, aspect);
        // Runners per flow scale with its length so a short hop is a few
        // sparks and a long one is a stream, never a pile.
        let length: f32 = points
            .windows(2)
            .map(|w| ((w[1][0] - w[0][0]).powi(2) + ((w[1][1] - w[0][1]) / aspect).powi(2)).sqrt())
            .sum();
        let share = (0.012 + length * 0.22).min(0.08);
        let tint = match f.color.unwrap_or(FlowColor::White) {
            FlowColor::White => Tint::White,
            FlowColor::Ember => Tint::Ember,
            FlowColor::Candle => Tint::Candle,
            FlowColor::Pale => Tint::Pale,
        };
        groups.push(
            Group::new(
                share,
                Home::Path {
                    points,
                    spread: 0.006,
                },
            )
            .palette(Palette::Solid(tint))
            .alpha(0.35, 0.9)
            .size(0.35, 0.7)
            .dim(0.0)
            .step(f.at)
            .drift(Drift::Flow { speed: 1.0 }),
        );
    }

    // The rest is quiet dust so the stage never sits in dead black.
    let used: f32 = groups.iter().map(|g| g.share).sum();
    groups.push(
        Group::new(
            (1.0 - used).max(0.12),
            Home::Field {
                u0: 0.02,
                v0: 0.04,
                u1: 0.98,
                v1: 0.96,
            },
        )
        .alpha(0.04, 0.16)
        .size(0.4, 0.8),
    );

    let mut scene = Scene::new(groups);
    scene.link_alpha = 0.10;
    Staged { scene, labels }
}

/// A cubic bezier from `p0` to `p1` bent sideways by `bend` (fraction of the
/// chord), sampled into a polyline in slide fractions.
fn bezier_points(p0: [f32; 2], p1: [f32; 2], bend: f32, aspect: f32) -> Vec<[f32; 2]> {
    let dx = p1[0] - p0[0];
    let dy = (p1[1] - p0[1]) / aspect;
    let dist = (dx * dx + dy * dy).sqrt().max(1e-4);
    let (nx, ny) = (-dy / dist, dx / dist);
    let b = dist * bend;
    let c0 = [
        p0[0] + dx * 0.3 + nx * b * 0.5,
        p0[1] + (dy * 0.3 + ny * b * 0.5) * aspect,
    ];
    let c1 = [
        p0[0] + dx * 0.72 + nx * b,
        p0[1] + (dy * 0.72 + ny * b) * aspect,
    ];
    (0..=24)
        .map(|i| {
            let t = i as f32 / 24.0;
            let u = 1.0 - t;
            [
                u * u * u * p0[0]
                    + 3.0 * u * u * t * c0[0]
                    + 3.0 * u * t * t * c1[0]
                    + t * t * t * p1[0],
                u * u * u * p0[1]
                    + 3.0 * u * u * t * c0[1]
                    + 3.0 * u * t * t * c1[1]
                    + t * t * t * p1[1],
            ]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::story::fixtures::{SAMPLE, test_library};

    #[test]
    fn unknown_kinds_are_reported_and_staged_as_rings() {
        let s = Script::parse("cast:\n  - { id: a, kind: person, cell: left }\n  - { id: b, kind: zeppelin, cell: right }\n").unwrap();
        let mut lib = test_library();
        assert_eq!(s.unknown_kinds(&mut lib), vec!["zeppelin".to_string()]);
        let staged = stage(&s, Layout::Bullet, 16.0 / 9.0, &mut lib);
        assert_eq!(staged.labels.len(), 2);
        assert!(matches!(staged.scene.groups[1].home, Home::Ring { .. }));
        assert!(matches!(staged.scene.groups[0].home, Home::Mask { .. }));
    }

    #[test]
    fn staging_makes_a_group_per_member_plus_flows_and_labels() {
        let s = Script::parse(SAMPLE).unwrap();
        let mut lib = test_library();
        let staged = stage(&s, Layout::Bullet, 16.0 / 9.0, &mut lib);
        assert_eq!(staged.labels.len(), 3);
        // 3 outlines + 1 brain fill + 2 flows + dust
        assert_eq!(staged.scene.groups.len(), 7);
        let model_group = staged
            .labels
            .iter()
            .find(|l| l.text == "The model")
            .unwrap()
            .group;
        assert_eq!(staged.scene.groups[model_group].step, Some(1));
        assert_eq!(staged.scene.groups[model_group].hot_step, Some(2));
        // every label sits inside the stage
        let sb = stage_box(Layout::Bullet);
        for l in &staged.labels {
            assert!(l.u > sb.left() && l.u < sb.right(), "{}", l.text);
        }
    }
}
