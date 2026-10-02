//! Smooth links between nodes: cubic curves sampled to polylines, drawn on
//! progressively when they are revealed, ending in a filled arrowhead.

use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};

/// A point on the cubic Bézier curve `c` at `t` (0 to 1).
pub fn bezier(c: [Pos2; 4], t: f32) -> Pos2 {
    let u = 1.0 - t;
    let w = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
    Pos2::new(
        c.iter().zip(w).map(|(p, w)| p.x * w).sum(),
        c.iter().zip(w).map(|(p, w)| p.y * w).sum(),
    )
}

/// The cubic curve `c` as `n` segments.
pub fn sample(c: [Pos2; 4], n: usize) -> Vec<Pos2> {
    let n = n.max(1);
    (0..=n).map(|i| bezier(c, i as f32 / n as f32)).collect()
}

/// A curve leaving `from` and arriving at `to` horizontally (an S between
/// columns).
pub fn horizontal_s(from: Pos2, to: Pos2) -> [Pos2; 4] {
    let dx = (to.x - from.x) * 0.5;
    [from, from + Vec2::new(dx, 0.0), to - Vec2::new(dx, 0.0), to]
}

/// Total length of a polyline.
pub fn length(points: &[Pos2]) -> f32 {
    points.windows(2).map(|w| w[0].distance(w[1])).sum()
}

/// The first `frac` (0 to 1) of a polyline by length.
pub fn trim(points: &[Pos2], frac: f32) -> Vec<Pos2> {
    if frac >= 1.0 || points.len() < 2 {
        return points.to_vec();
    }
    let mut left = length(points) * frac.max(0.0);
    let mut out = vec![points[0]];
    for w in points.windows(2) {
        let d = w[0].distance(w[1]);
        if d >= left {
            if d > 0.0 {
                out.push(w[0] + (w[1] - w[0]) * (left / d));
            }
            break;
        }
        left -= d;
        out.push(w[1]);
    }
    out
}

/// The point `frac` (0 to 1) of the way along a polyline, by length.
pub fn point_at(points: &[Pos2], frac: f32) -> Pos2 {
    trim(points, frac).last().copied().unwrap_or(Pos2::ZERO)
}

/// Draw a polyline that ends in a filled arrowhead `head` long, pointing the
/// way the line runs at its end. The line stops at the head's base so the
/// tip stays sharp.
pub fn draw_arrow(painter: &egui::Painter, points: &[Pos2], stroke: Stroke, head: f32) {
    if points.len() < 2 || length(points) < 1.0 {
        return;
    }
    let tip = points[points.len() - 1];
    // the direction over the last `head` of the line, not just its last segment
    let back = point_at(points, 1.0 - (head / length(points)).min(1.0));
    let dir = (tip - back).normalized();
    if !dir.is_finite() || dir == Vec2::ZERO {
        painter.add(egui::Shape::line(points.to_vec(), stroke));
        return;
    }
    let len = length(points);
    let body = trim(points, ((len - head * 0.8) / len).max(0.0));
    painter.add(egui::Shape::line(body, stroke));
    painter.add(arrowhead(tip, dir, head, stroke.color));
}

/// A filled arrowhead with its tip at `tip`, pointing along `dir`.
pub fn arrowhead(tip: Pos2, dir: Vec2, head: f32, color: Color32) -> egui::Shape {
    let normal = Vec2::new(-dir.y, dir.x);
    let base = tip - dir * head;
    egui::Shape::convex_polygon(
        vec![tip, base + normal * head * 0.5, base - normal * head * 0.5],
        color,
        Stroke::NONE,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bezier_runs_from_end_to_end() {
        let c = horizontal_s(Pos2::new(0.0, 0.0), Pos2::new(100.0, 50.0));
        assert_eq!(bezier(c, 0.0), c[0]);
        assert_eq!(bezier(c, 1.0), c[3]);
        // an S is symmetric: its midpoint is halfway
        let mid = bezier(c, 0.5);
        assert!((mid.x - 50.0).abs() < 1e-4 && (mid.y - 25.0).abs() < 1e-4);
        assert_eq!(sample(c, 8).len(), 9);
    }

    #[test]
    fn trim_cuts_by_length() {
        let line = [
            Pos2::new(0.0, 0.0),
            Pos2::new(10.0, 0.0),
            Pos2::new(10.0, 10.0),
        ];
        assert_eq!(length(&line), 20.0);
        assert_eq!(trim(&line, 0.25), vec![line[0], Pos2::new(5.0, 0.0)]);
        assert_eq!(point_at(&line, 0.75), Pos2::new(10.0, 5.0));
        assert_eq!(trim(&line, 1.0), line.to_vec());
        assert_eq!(trim(&line, 0.0), vec![line[0], line[0]]);
    }
}
