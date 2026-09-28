//! The soft dark pillow behind the copy.

use eframe::egui::{self, Pos2, Rect};

use super::fade;
use crate::theme::Theme;

/// The soft dark ellipse that keeps copy readable over the lights: a radial
/// gradient from 72% black at the centre to transparent, drawn as a fan of
/// concentric rings so egui's linear vertex interpolation follows the curve.
pub fn pillow(painter: &egui::Painter, copy: Rect, alpha: f32, theme: &Theme) {
    let center = copy.center();
    let rx = copy.width() * 0.5 + copy.width() * 0.34;
    let ry = copy.height() * 0.5 + copy.height() * 0.42 + 40.0;
    let stops: [(f32, f32); 5] = [
        (0.0, 0.72),
        (0.30, 0.62),
        (0.55, 0.38),
        (0.78, 0.0),
        (1.0, 0.0),
    ];
    let segments = 48;
    let mut mesh = egui::Mesh::default();
    let color_at = |a: f32| fade(theme.background, a * alpha);
    // centre vertex
    mesh.colored_vertex(center, color_at(stops[0].1));
    for (ri, (r, a)) in stops.iter().enumerate().skip(1) {
        for s in 0..segments {
            let ang = s as f32 / segments as f32 * std::f32::consts::TAU;
            let p = Pos2::new(center.x + ang.cos() * rx * r, center.y + ang.sin() * ry * r);
            mesh.colored_vertex(p, color_at(*a));
        }
        let ring_start = 1 + (ri - 1) * segments;
        if ri == 1 {
            for s in 0..segments {
                let a = ring_start + s;
                let b = ring_start + (s + 1) % segments;
                mesh.add_triangle(0, a as u32, b as u32);
            }
        } else {
            let prev = ring_start - segments;
            for s in 0..segments {
                let a0 = prev + s;
                let a1 = prev + (s + 1) % segments;
                let b0 = ring_start + s;
                let b1 = ring_start + (s + 1) % segments;
                mesh.add_triangle(a0 as u32, b0 as u32, b1 as u32);
                mesh.add_triangle(a0 as u32, b1 as u32, a1 as u32);
            }
        }
    }
    painter.add(egui::Shape::mesh(mesh));
}
