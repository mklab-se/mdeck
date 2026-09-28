use eframe::egui::{Pos2, Vec2};

// ─── Polyline geometry ───────────────────────────────────────────────────────

/// Apply rounded corners to an orthogonal polyline.
/// Returns a new polyline with arcs at each bend.
pub(super) fn apply_rounded_corners(waypoints: &[Pos2], radius: f32) -> Vec<Pos2> {
    if waypoints.len() < 3 {
        return waypoints.to_vec();
    }

    let mut result = Vec::new();
    result.push(waypoints[0]);

    for i in 1..waypoints.len() - 1 {
        let prev = waypoints[i - 1];
        let curr = waypoints[i];
        let next = waypoints[i + 1];

        // Compute available lengths on incoming and outgoing segments
        let in_len = (curr - prev).length();
        let out_len = (next - curr).length();

        // Clamp radius to half the shorter adjacent segment
        let r = radius.min(in_len / 2.0).min(out_len / 2.0);
        if r < 1.0 {
            result.push(curr);
            continue;
        }

        // Direction vectors
        let in_dir = (curr - prev).normalized();
        let out_dir = (next - curr).normalized();

        // Points where the arc starts and ends
        let arc_start = curr - in_dir * r;
        let arc_end = curr + out_dir * r;

        // Generate arc points (8-point approximation of quarter circle)
        let n_arc_points = 8;
        for j in 0..=n_arc_points {
            let t = j as f32 / n_arc_points as f32;
            let x = arc_start.x * (1.0 - t) * (1.0 - t)
                + curr.x * 2.0 * (1.0 - t) * t
                + arc_end.x * t * t;
            let y = arc_start.y * (1.0 - t) * (1.0 - t)
                + curr.y * 2.0 * (1.0 - t) * t
                + arc_end.y * t * t;
            result.push(Pos2::new(x, y));
        }
    }

    result.push(*waypoints.last().unwrap());
    result
}

/// Compute the total length of a polyline.
pub(super) fn polyline_length(points: &[Pos2]) -> f32 {
    let mut total = 0.0;
    for i in 0..points.len().saturating_sub(1) {
        total += (points[i + 1] - points[i]).length();
    }
    total
}

/// Find the point at a given distance along a polyline.
pub(super) fn polyline_point_at_distance(points: &[Pos2], distance: f32) -> Pos2 {
    let mut remaining = distance;
    for i in 0..points.len().saturating_sub(1) {
        let seg_len = (points[i + 1] - points[i]).length();
        if remaining <= seg_len {
            let t = remaining / seg_len.max(0.001);
            return Pos2::new(
                points[i].x + (points[i + 1].x - points[i].x) * t,
                points[i].y + (points[i + 1].y - points[i].y) * t,
            );
        }
        remaining -= seg_len;
    }
    *points.last().unwrap_or(&Pos2::ZERO)
}

/// The part of a polyline between two distances along it, with points closer
/// than half a pixel merged.
pub(super) fn clip_polyline(points: &[Pos2], start_d: f32, end_d: f32) -> Vec<Pos2> {
    let mut clipped = vec![polyline_point_at_distance(points, start_d)];
    let mut cumulative = 0.0;
    for i in 0..points.len().saturating_sub(1) {
        let seg_len = (points[i + 1] - points[i]).length();
        let next_cumulative = cumulative + seg_len;
        if next_cumulative > start_d && cumulative < end_d {
            if cumulative > start_d {
                clipped.push(points[i]);
            }
            if next_cumulative < end_d {
                clipped.push(points[i + 1]);
            }
        }
        cumulative = next_cumulative;
    }
    clipped.push(polyline_point_at_distance(points, end_d));
    clipped.dedup_by(|a, b| (*a - *b).length() < 0.5);
    clipped
}

/// The dashes of a dashed polyline, continuous across its bends.
pub(super) fn dash_segments(points: &[Pos2], dash_len: f32, gap_len: f32) -> Vec<[Pos2; 2]> {
    let total_len = polyline_length(points);
    let mut dashes = Vec::new();
    let mut d = 0.0;
    let mut drawing = true;
    while d < total_len {
        if drawing {
            let seg_end_d = (d + dash_len).min(total_len);
            dashes.push([
                polyline_point_at_distance(points, d),
                polyline_point_at_distance(points, seg_end_d),
            ]);
            d += dash_len;
        } else {
            d += gap_len;
        }
        drawing = !drawing;
    }
    dashes
}

/// Where a settled arrowhead sits at the `tip` end of a route whose last
/// points are `before`, `prev`, `tip`, and which way it points. On a final
/// segment too short to hold the head, it moves back to the last bend and
/// points along the segment before it.
pub(super) fn arrowhead_at(
    tip: Pos2,
    prev: Pos2,
    before: Option<Pos2>,
    arrow_size: f32,
) -> (Pos2, Vec2) {
    let last = tip - prev;
    match before {
        Some(before) if last.length() < arrow_size * 1.2 => (prev, prev - before),
        _ => (tip, last),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_rounded_corners() {
        let pts = vec![
            Pos2::new(0.0, 0.0),
            Pos2::new(100.0, 0.0),
            Pos2::new(100.0, 100.0),
        ];
        let result = apply_rounded_corners(&pts, 10.0);
        assert!(result.len() > 3);
        assert_eq!(result[0], Pos2::new(0.0, 0.0));
        assert_eq!(*result.last().unwrap(), Pos2::new(100.0, 100.0));
    }

    #[test]
    fn test_rounded_corners_straight_line_unchanged() {
        let pts = vec![Pos2::new(0.0, 0.0), Pos2::new(100.0, 0.0)];
        let result = apply_rounded_corners(&pts, 10.0);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_rounded_corners_preserves_endpoints() {
        let pts = vec![
            Pos2::new(10.0, 20.0),
            Pos2::new(100.0, 20.0),
            Pos2::new(100.0, 100.0),
            Pos2::new(200.0, 100.0),
        ];
        let result = apply_rounded_corners(&pts, 8.0);
        assert_eq!(result[0], pts[0]);
        assert_eq!(*result.last().unwrap(), *pts.last().unwrap());
    }

    #[test]
    fn test_rounded_corners_zero_radius() {
        let pts = vec![
            Pos2::new(0.0, 0.0),
            Pos2::new(100.0, 0.0),
            Pos2::new(100.0, 100.0),
        ];
        let result = apply_rounded_corners(&pts, 0.0);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_polyline_length() {
        let pts = vec![
            Pos2::new(0.0, 0.0),
            Pos2::new(100.0, 0.0),
            Pos2::new(100.0, 50.0),
        ];
        assert!((polyline_length(&pts) - 150.0).abs() < 0.1);
    }

    #[test]
    fn test_polyline_length_single_point() {
        assert_eq!(polyline_length(&[Pos2::new(5.0, 5.0)]), 0.0);
    }

    #[test]
    fn test_polyline_length_two_points() {
        let pts = [Pos2::new(0.0, 0.0), Pos2::new(3.0, 4.0)];
        assert!((polyline_length(&pts) - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_polyline_length_multi_segment() {
        let pts = [
            Pos2::new(0.0, 0.0),
            Pos2::new(10.0, 0.0),
            Pos2::new(10.0, 10.0),
            Pos2::new(0.0, 10.0),
        ];
        assert!((polyline_length(&pts) - 30.0).abs() < 0.01);
    }

    fn l_shape() -> [Pos2; 3] {
        [
            Pos2::new(0.0, 0.0),
            Pos2::new(100.0, 0.0),
            Pos2::new(100.0, 50.0),
        ]
    }

    #[test]
    fn point_at_distance_walks_segments_and_clamps() {
        let pts = l_shape();
        assert_eq!(polyline_point_at_distance(&pts, 0.0), pts[0]);
        assert_eq!(polyline_point_at_distance(&pts, 40.0), Pos2::new(40.0, 0.0));
        assert_eq!(
            polyline_point_at_distance(&pts, 120.0),
            Pos2::new(100.0, 20.0)
        );
        assert_eq!(polyline_point_at_distance(&pts, 999.0), pts[2]);
        assert_eq!(polyline_point_at_distance(&[], 5.0), Pos2::ZERO);
    }

    #[test]
    fn clip_polyline_keeps_bends_inside_the_span() {
        let pts = l_shape();
        let near = |got: Vec<Pos2>, want: &[Pos2]| {
            assert_eq!(got.len(), want.len(), "{got:?}");
            for (g, w) in got.iter().zip(want) {
                assert!((*g - *w).length() < 1e-3, "{got:?} vs {want:?}");
            }
        };
        near(
            clip_polyline(&pts, 20.0, 130.0),
            &[
                Pos2::new(20.0, 0.0),
                Pos2::new(100.0, 0.0),
                Pos2::new(100.0, 30.0),
            ],
        );
        // A span on one segment is a single straight piece
        near(
            clip_polyline(&pts, 10.0, 60.0),
            &[Pos2::new(10.0, 0.0), Pos2::new(60.0, 0.0)],
        );
        // The full length keeps every point exactly once
        near(clip_polyline(&pts, 0.0, 150.0), &pts);
    }

    #[test]
    fn dash_segments_alternate_and_cross_bends() {
        let pts = l_shape();
        let dashes = dash_segments(&pts, 8.0, 5.0);
        // 150 px of 13 px periods: 11 whole dashes and a 7 px tail dash
        assert_eq!(dashes.len(), 12);
        assert_eq!(dashes[0], [Pos2::new(0.0, 0.0), Pos2::new(8.0, 0.0)]);
        assert_eq!(dashes[1][0], Pos2::new(13.0, 0.0));
        // The period continues past the bend at 100
        assert_eq!(dashes[7], [Pos2::new(91.0, 0.0), Pos2::new(99.0, 0.0)]);
        assert_eq!(dashes[8], [Pos2::new(100.0, 4.0), Pos2::new(100.0, 12.0)]);
        assert_eq!(*dashes.last().unwrap(), [Pos2::new(100.0, 43.0), pts[2]]);
        assert!(dash_segments(&[Pos2::ZERO], 8.0, 5.0).is_empty());
    }

    #[test]
    fn arrowhead_moves_back_from_a_short_final_segment() {
        let tip = Pos2::new(100.0, 50.0);
        let prev = Pos2::new(100.0, 0.0);
        let before = Some(Pos2::new(0.0, 0.0));
        // A long final segment holds the head at the tip
        assert_eq!(
            arrowhead_at(tip, prev, before, 20.0),
            (tip, Vec2::new(0.0, 50.0))
        );
        // Too short: the head sits on the bend, along the segment before it
        assert_eq!(
            arrowhead_at(tip, prev, before, 50.0),
            (prev, Vec2::new(100.0, 0.0))
        );
        // Without an earlier segment it stays at the tip
        assert_eq!(
            arrowhead_at(tip, prev, None, 50.0),
            (tip, Vec2::new(0.0, 50.0))
        );
    }
}
