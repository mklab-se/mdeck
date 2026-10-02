//! Pen strokes: points ordered into strokes a pen can follow.
//! A nearest-neighbour tour, untangled with 2-opt and rounded with Chaikin's
//! corner cutting, then timed so the pen lifts on long jumps. Used by the
//! art engines' reveal order and their line drawings.

use mdeck_sdk::paint::{Pos2, Rect};
use mdeck_sdk::stage::Place;

/// Most points a picture is traced through. Clouds are in importance order
/// (the first points sketch the silhouette) and glyph masks are shuffled, so
/// the first points are the right ones either way; the tour is quadratic.
pub const MAX_POINTS: usize = 560;

/// A pen drawing: the path through its points and when the pen reaches each.
#[derive(Clone, Debug)]
pub struct Strokes {
    /// Points in slide fractions.
    pub points: Vec<Pos2>,
    /// `pen[i]`: the move from point `i - 1` to `i` is drawn (else the
    /// pen is lifted and jumps).
    pub pen: Vec<bool>,
    /// When the pen reaches each point, from the start of the drawing.
    pub at: Vec<f32>,
    pub duration: f32,
    /// Brightness of the finished drawing (a title's backdrop is dimmer).
    pub weight: f32,
    /// Engine clock when the pen started.
    pub born: f32,
}

impl Strokes {
    /// The pen's position `t` seconds into the drawing, and whether it is down.
    pub fn tip(&self, t: f32) -> Option<(Pos2, bool)> {
        if self.points.is_empty() || t < 0.0 || t >= self.duration {
            return None;
        }
        let i = self
            .at
            .partition_point(|a| *a <= t)
            .clamp(1, self.points.len() - 1);
        let (a0, a1) = (self.at[i - 1], self.at[i]);
        let f = if a1 > a0 { (t - a0) / (a1 - a0) } else { 1.0 };
        let p = self.points[i - 1] + (self.points[i] - self.points[i - 1]) * f.clamp(0.0, 1.0);
        Some((p, self.pen[i]))
    }
}

/// A point in slide fractions, on screen inside `rect`.
pub fn to_screen(p: Pos2, rect: Rect) -> Pos2 {
    Pos2::new(
        rect.left() + p.x * rect.width(),
        rect.top() + p.y * rect.height(),
    )
}

/// `points` (unit square, importance order) placed in `place` and ordered
/// into a drawing path, in slide fractions.
pub fn toured(points: &[[f32; 2]], place: Place, aspect: f32) -> Vec<Pos2> {
    let pts: Vec<Pos2> = points
        .iter()
        .take(MAX_POINTS)
        .map(|p| Pos2::new(place.u + p[0] * place.w, place.v + p[1] * place.h))
        .collect();
    let mut order = tour(&pts, aspect);
    untangle(&pts, &mut order, aspect);
    order.into_iter().map(|i| pts[i]).collect()
}

/// 2-opt: reverse any stretch of the tour whose ends cross or double back,
/// until a pass finds nothing to improve (at most a few passes), so the pen
/// draws strokes instead of zig-zags.
pub fn untangle(pts: &[Pos2], order: &mut [usize], aspect: f32) {
    let d = |a: usize, b: usize| {
        let (p, q) = (pts[a], pts[b]);
        (((p.x - q.x) * aspect).powi(2) + (p.y - q.y).powi(2)).sqrt()
    };
    let n = order.len();
    if n < 4 {
        return;
    }
    for _ in 0..4 {
        let mut improved = false;
        for i in 0..n - 2 {
            for j in i + 2..n - 1 {
                let (a, b, c, e) = (order[i], order[i + 1], order[j], order[j + 1]);
                if d(a, c) + d(b, e) + 1e-6 < d(a, b) + d(c, e) {
                    order[i + 1..=j].reverse();
                    improved = true;
                }
            }
        }
        if !improved {
            break;
        }
    }
}

/// Round each pen-down run of the picture with two passes of Chaikin's
/// corner cutting; the pen state and run ends are kept.
pub fn smooth(points: Vec<Pos2>, pen: Vec<bool>) -> (Vec<Pos2>, Vec<bool>) {
    let mut out_p = Vec::with_capacity(points.len() * 2);
    let mut out_pen = Vec::with_capacity(points.len() * 2);
    let mut i = 0;
    while i < points.len() {
        // a run: point i with pen up (or the first), then pen-down points
        let mut j = i + 1;
        while j < points.len() && pen[j] {
            j += 1;
        }
        let mut run: Vec<Pos2> = points[i..j].to_vec();
        for _ in 0..2 {
            if run.len() < 3 {
                break;
            }
            let mut next = vec![run[0]];
            for w in run.windows(2) {
                next.push(w[0] + (w[1] - w[0]) * 0.25);
                next.push(w[0] + (w[1] - w[0]) * 0.75);
            }
            next.push(*run.last().expect("run"));
            run = next;
        }
        for (k, p) in run.into_iter().enumerate() {
            out_p.push(p);
            out_pen.push(k > 0 || pen[i]);
        }
        i = j;
    }
    (out_p, out_pen)
}

/// Nearest-neighbour tour from the top-left point. Distances are measured
/// in screen proportions (`aspect` is width over height).
pub fn tour(pts: &[Pos2], aspect: f32) -> Vec<usize> {
    let n = pts.len();
    if n == 0 {
        return Vec::new();
    }
    let d2 = |a: Pos2, b: Pos2| ((a.x - b.x) * aspect).powi(2) + (a.y - b.y).powi(2);
    let start = (0..n)
        .min_by(|&a, &b| {
            let ka = pts[a].x * aspect + pts[a].y;
            let kb = pts[b].x * aspect + pts[b].y;
            ka.total_cmp(&kb)
        })
        .unwrap_or(0);
    let mut seen = vec![false; n];
    let mut order = Vec::with_capacity(n);
    let mut cur = start;
    seen[cur] = true;
    order.push(cur);
    for _ in 1..n {
        let mut best = usize::MAX;
        let mut best_d = f32::MAX;
        for j in 0..n {
            if !seen[j] {
                let d = d2(pts[cur], pts[j]);
                if d < best_d {
                    best_d = d;
                    best = j;
                }
            }
        }
        seen[best] = true;
        order.push(best);
        cur = best;
    }
    order
}

/// Time the pen along `strokes`: long jumps inside a stroke and the moves
/// between strokes lift the pen and are nearly instant; the whole
/// drawing takes `duration`.
pub fn plan(
    strokes: Vec<Vec<Pos2>>,
    duration: f32,
    weight: f32,
    aspect: f32,
    born: f32,
) -> Strokes {
    let dist = |a: Pos2, b: Pos2| (((a.x - b.x) * aspect).powi(2) + (a.y - b.y).powi(2)).sqrt();
    let mut points = Vec::new();
    let mut pen = Vec::new();
    for stroke in strokes {
        if stroke.is_empty() {
            continue;
        }
        let mut steps: Vec<f32> = stroke.windows(2).map(|w| dist(w[0], w[1])).collect();
        let lift = {
            steps.sort_by(f32::total_cmp);
            steps.get(steps.len() / 2).copied().unwrap_or(0.0) * 3.2
        };
        for (k, p) in stroke.iter().enumerate() {
            let on = k > 0 && dist(stroke[k - 1], *p) <= lift.max(1e-4);
            points.push(*p);
            pen.push(on);
        }
    }
    let (points, pen) = smooth(points, pen);
    let mut cost = vec![0.0f32; points.len()];
    for i in 1..points.len() {
        let d = dist(points[i - 1], points[i]);
        cost[i] = if pen[i] { d } else { d * 0.06 };
    }
    let total: f32 = cost.iter().sum::<f32>().max(1e-5);
    let mut at = Vec::with_capacity(points.len());
    let mut acc = 0.0;
    for c in &cost {
        acc += c;
        at.push(acc / total * duration);
    }
    Strokes {
        points,
        pen,
        at,
        duration,
        weight,
        born,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::hash01;

    #[test]
    fn the_tour_visits_every_point_once_starting_top_left() {
        let pts: Vec<Pos2> = (0..50)
            .map(|k| Pos2::new(hash01(k) * 0.5 + 0.2, hash01(k + 99) * 0.5 + 0.2))
            .collect();
        let order = tour(&pts, 16.0 / 9.0);
        let mut sorted = order.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..50).collect::<Vec<_>>());
        let first = pts[order[0]];
        assert!(
            pts.iter()
                .all(|p| p.x * 16.0 / 9.0 + p.y >= first.x * 16.0 / 9.0 + first.y)
        );
    }

    #[test]
    fn long_jumps_lift_the_pen_and_the_drawing_finishes_on_time() {
        // two short strokes far apart
        let a: Vec<Pos2> = (0..20)
            .map(|k| Pos2::new(0.1 + k as f32 * 0.005, 0.2))
            .collect();
        let b: Vec<Pos2> = (0..20)
            .map(|k| Pos2::new(0.7 + k as f32 * 0.005, 0.8))
            .collect();
        let mut both = a.clone();
        both.extend(b);
        let pic = plan(vec![both], 2.0, 1.0, 16.0 / 9.0, 0.0);
        assert_eq!(
            pic.pen.iter().filter(|p| !**p).count(),
            2,
            "start, and the jump"
        );
        assert!((pic.at.last().unwrap() - 2.0).abs() < 1e-4);
        assert!(pic.tip(1.0).is_some());
        assert!(pic.tip(2.5).is_none(), "done after its duration");
        // the jump is fast: most of the time goes to the strokes
        let jump = pic
            .pen
            .iter()
            .position(|p| !p)
            .filter(|i| *i > 0)
            .unwrap_or(20);
        let at_jump = pic.at[jump] - pic.at[jump - 1];
        assert!(at_jump < 0.5, "{at_jump}");
    }
}
