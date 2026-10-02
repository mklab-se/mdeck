//! The lens of a `@thermal` block: where the thermal layer shows at a step
//! (everywhere, through a lens that glides, or opening up), and the
//! textured circle that draws it.

use eframe::egui::{self, Color32, Pos2, Rect, Vec2};

use super::draw::{ease, progress};
use super::state::{LensAt, State};
use crate::render::BlockCx;

/// Seconds for the lens to glide or open.
const GLIDE: f32 = 0.7;
const OPEN: f32 = 0.6;

/// What the thermal layer covers.
#[derive(Debug, Clone, Copy)]
pub(super) enum Shape {
    Full,
    Circle {
        center: Pos2,
        radius: f32,
        ring: f32,
    },
}

/// How strongly and where the thermal layer shows.
pub(super) fn coverage(
    cx: &BlockCx,
    state: &State,
    img: Rect,
    from_start: bool,
) -> (f32, Option<Shape>) {
    if from_start {
        return (1.0, Some(Shape::Full));
    }
    let place = |l: LensAt| {
        (
            Pos2::new(
                img.left() + l.x * img.width(),
                img.top() + l.y * img.height(),
            ),
            l.r * img.width(),
        )
    };
    let diag = img.size().length();
    if let Some(step) = state.revealed {
        let p = ease(progress(cx, step, OPEN));
        if p >= 1.0 {
            return (1.0, Some(Shape::Full));
        }
        let (center, r0) = match state.lens {
            Some((l, _, _)) => place(l),
            None => (img.center(), 0.0),
        };
        // open from the lens until the circle covers the whole image
        let reach = [
            img.left_top(),
            img.right_top(),
            img.left_bottom(),
            img.right_bottom(),
        ]
        .iter()
        .map(|c| c.distance(center))
        .fold(0.0, f32::max)
        .max(diag * 0.5);
        let a = if state.lens.is_some() { 1.0 } else { p };
        return (
            a,
            Some(Shape::Circle {
                center,
                radius: r0 + (reach - r0) * p,
                ring: 1.0 - p,
            }),
        );
    }
    if let Some((now, from, step)) = state.lens {
        let p = ease(progress(cx, step, GLIDE));
        let (to_c, to_r) = place(now);
        let (from_c, from_r, a) = match from {
            Some(f) => {
                let (c, r) = place(f);
                (c, r, 1.0)
            }
            // the first lens glides in from the lower left
            None => (
                Pos2::new(
                    img.left() + img.width() * 0.12,
                    img.bottom() - img.height() * 0.15,
                ),
                to_r,
                p,
            ),
        };
        let center = from_c + (to_c - from_c) * p;
        return (
            a,
            Some(Shape::Circle {
                center,
                radius: from_r + (to_r - from_r) * p,
                ring: 1.0,
            }),
        );
    }
    (0.0, None)
}

/// A circle of `tex` (mapped onto `img`) as a fan mesh.
pub(super) fn textured_circle(
    painter: &egui::Painter,
    tex: egui::TextureId,
    img: Rect,
    center: Pos2,
    radius: f32,
    tint: Color32,
) {
    let mut mesh = egui::Mesh::with_texture(tex);
    let uv = |p: Pos2| {
        Pos2::new(
            (p.x - img.left()) / img.width(),
            (p.y - img.top()) / img.height(),
        )
    };
    mesh.vertices.push(egui::epaint::Vertex {
        pos: center,
        uv: uv(center),
        color: tint,
    });
    let n = 96;
    for i in 0..=n {
        let a = std::f32::consts::TAU * i as f32 / n as f32;
        let p = center + Vec2::angled(a) * radius;
        mesh.vertices.push(egui::epaint::Vertex {
            pos: p,
            uv: uv(p),
            color: tint,
        });
        if i > 0 {
            mesh.add_triangle(0, i as u32, i as u32 + 1);
        }
    }
    painter.add(egui::Shape::mesh(mesh));
}
