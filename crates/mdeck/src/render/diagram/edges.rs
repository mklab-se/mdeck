use std::collections::HashMap;

use eframe::egui::{self, Color32, FontId, Pos2, Stroke};

use super::routing;
use super::types::*;
use crate::theme::Theme;

/// Compute a point on a node face, offset from center by `port_offset`.
/// For Left/Right faces, offset shifts Y. For Top/Bottom faces, offset shifts X.
pub(super) fn face_point_with_port(rect: &egui::Rect, face: Face, port_offset: f32) -> Pos2 {
    let c = rect.center();
    match face {
        Face::Right => Pos2::new(rect.right(), c.y + port_offset),
        Face::Left => Pos2::new(rect.left(), c.y + port_offset),
        Face::Bottom => Pos2::new(c.x + port_offset, rect.bottom()),
        Face::Top => Pos2::new(c.x + port_offset, rect.top()),
    }
}

/// Choose the best exit face from `from_rect` toward `to_rect` center.
pub(super) fn choose_exit_face(from_rect: &egui::Rect, to_center: Pos2) -> Face {
    let c = from_rect.center();
    let dx = to_center.x - c.x;
    let dy = to_center.y - c.y;
    if dx.abs() >= dy.abs() {
        if dx >= 0.0 { Face::Right } else { Face::Left }
    } else if dy >= 0.0 {
        Face::Bottom
    } else {
        Face::Top
    }
}

/// Choose the best entry face on `to_rect` coming from `from_center`.
pub(super) fn choose_entry_face(to_rect: &egui::Rect, from_center: Pos2) -> Face {
    let c = to_rect.center();
    let dx = from_center.x - c.x;
    let dy = from_center.y - c.y;
    if dx.abs() >= dy.abs() {
        if dx >= 0.0 { Face::Right } else { Face::Left }
    } else if dy >= 0.0 {
        Face::Bottom
    } else {
        Face::Top
    }
}

/// Compute a ramp point offset from a face point.
pub(super) fn ramp_from_face(
    rect: &egui::Rect,
    face: Face,
    port_offset: f32,
    node_margin: f32,
) -> Pos2 {
    let fp = face_point_with_port(rect, face, port_offset);
    match face {
        Face::Right => Pos2::new(fp.x + node_margin, fp.y),
        Face::Left => Pos2::new(fp.x - node_margin, fp.y),
        Face::Bottom => Pos2::new(fp.x, fp.y + node_margin),
        Face::Top => Pos2::new(fp.x, fp.y - node_margin),
    }
}

/// Map a routing::Direction to the corresponding Face.
pub(super) fn direction_to_face(dir: routing::types::Direction) -> Face {
    match dir {
        routing::types::Direction::North => Face::Top,
        routing::types::Direction::South => Face::Bottom,
        routing::types::Direction::East => Face::Right,
        routing::types::Direction::West => Face::Left,
    }
}

// ─── New routing engine integration ─────────────────────────────────────────

/// Compute lane capacity for horizontal corridors (edges travel left/right).
/// The gap available is the vertical space between node edges.
pub(super) fn compute_h_capacity(
    grid: &GridInfo,
    node_rects: &HashMap<String, egui::Rect>,
    lane_spacing: f32,
) -> i32 {
    // Find the minimum vertical gap between any two adjacent rows
    let mut min_gap = f32::MAX;
    for row in 0..grid.rows {
        for col in 0..grid.cols {
            // Check if this cell has a node
            if let Some(rect) = find_rect_at(grid, node_rects, col, row) {
                let node_h = rect.height();
                let gap = grid.cell_h - node_h;
                min_gap = min_gap.min(gap);
            }
        }
    }
    if !min_gap.is_finite() || min_gap <= 0.0 || lane_spacing <= 0.0 {
        return 3; // sensible default
    }
    let capacity = (min_gap / lane_spacing).floor() as i32;
    capacity.max(1)
}

/// Compute lane capacity for vertical corridors (edges travel up/down).
/// The gap available is the horizontal space between node edges.
pub(super) fn compute_v_capacity(
    grid: &GridInfo,
    node_rects: &HashMap<String, egui::Rect>,
    lane_spacing: f32,
) -> i32 {
    let mut min_gap = f32::MAX;
    for row in 0..grid.rows {
        for col in 0..grid.cols {
            if let Some(rect) = find_rect_at(grid, node_rects, col, row) {
                let node_w = rect.width();
                let gap = grid.cell_w - node_w;
                min_gap = min_gap.min(gap);
            }
        }
    }
    if !min_gap.is_finite() || min_gap <= 0.0 || lane_spacing <= 0.0 {
        return 3;
    }
    let capacity = (min_gap / lane_spacing).floor() as i32;
    capacity.max(1)
}

/// Find the rect of a node at a specific 0-indexed grid cell.
fn find_rect_at(
    grid: &GridInfo,
    node_rects: &HashMap<String, egui::Rect>,
    col: usize,
    row: usize,
) -> Option<egui::Rect> {
    if !grid.occupied.contains(&(col, row)) {
        return None;
    }
    // Find rect whose center falls in this cell
    let cell_center_x = grid.origin_x + (col as f32 + 0.5) * grid.cell_w;
    let cell_center_y = grid.origin_y + (row as f32 + 0.5) * grid.cell_h;
    let cell_center = Pos2::new(cell_center_x, cell_center_y);
    node_rects
        .values()
        .find(|r| {
            let c = r.center();
            (c.x - cell_center.x).abs() < grid.cell_w * 0.5
                && (c.y - cell_center.y).abs() < grid.cell_h * 0.5
        })
        .copied()
}

/// Convert a routing engine Route to pixel waypoints for drawing.
///
/// The routing engine works in 1-based integer grid coordinates.
/// This function converts those to pixel positions using the GridInfo geometry,
/// and adds face connection points (ramps) at the start and end.
#[allow(clippy::too_many_arguments)]
pub(super) fn waypoints_to_pixels(
    route: &routing::types::Route,
    grid: &GridInfo,
    from_rect: &egui::Rect,
    to_rect: &egui::Rect,
    node_margin: f32,
    lane_spacing: f32,
    port_offset_start: f32,
    port_offset_end: f32,
) -> Vec<Pos2> {
    if route.waypoints.len() < 2 {
        return Vec::new();
    }

    let mut pixels = Vec::new();

    // Determine exit direction from the first two waypoints
    let first = &route.waypoints[0];
    let second = &route.waypoints[1];
    let exit_dir = coord_direction(first.coord, second.coord);
    let exit_face = direction_to_face(exit_dir);

    // Determine entry direction from the last two waypoints
    let n = route.waypoints.len();
    let penult = &route.waypoints[n - 2];
    let last = &route.waypoints[n - 1];
    let entry_dir = coord_direction(penult.coord, last.coord);
    let entry_face = direction_to_face(entry_dir.opposite());

    // Start: face point and ramp on the source node
    let fp_start = face_point_with_port(from_rect, exit_face, port_offset_start);
    let ramp_start = ramp_from_face(from_rect, exit_face, port_offset_start, node_margin);
    pixels.push(fp_start);
    pixels.push(ramp_start);

    // Intermediate waypoints (skip first = source center, skip last = target center)
    for i in 1..n - 1 {
        let wp = &route.waypoints[i];
        let px = coord_to_pixel_x(wp.coord, grid);
        let py = coord_to_pixel_y(wp.coord, grid);

        // Compute incoming offset (from previous segment's direction and lane)
        let prev_wp = &route.waypoints[i - 1];
        let in_dir = coord_direction(prev_wp.coord, wp.coord);
        let in_lane = prev_wp.lane;
        let (in_ox, in_oy) = lane_offset(in_dir, in_lane, lane_spacing);

        // Compute outgoing offset (from this waypoint's direction and lane to next)
        let (out_ox, out_oy) = if i + 1 < n {
            let next_wp = &route.waypoints[i + 1];
            let out_dir = coord_direction(wp.coord, next_wp.coord);
            lane_offset(out_dir, wp.lane, lane_spacing)
        } else {
            (0.0, 0.0)
        };

        // At a turn (horizontal↔vertical), compute a single combined corner point
        // that keeps both the incoming and outgoing segments straight:
        //   - Horizontal segments offset Y → keep incoming Y at the corner
        //   - Vertical segments offset X → keep outgoing X at the corner
        let is_turn = in_dir.is_horizontal() != {
            if i + 1 < n {
                let next_wp = &route.waypoints[i + 1];
                coord_direction(wp.coord, next_wp.coord).is_horizontal()
            } else {
                in_dir.is_horizontal()
            }
        };

        let pt = if is_turn {
            let (cx, cy) = if in_dir.is_horizontal() {
                // Horizontal → Vertical: keep incoming Y, use outgoing X
                (out_ox, in_oy)
            } else {
                // Vertical → Horizontal: keep incoming X, use outgoing Y
                (in_ox, out_oy)
            };
            Pos2::new(px + cx, py + cy)
        } else {
            // Straight segment: use outgoing offset (matches next segment)
            Pos2::new(px + out_ox, py + out_oy)
        };

        if let Some(prev) = pixels.last()
            && (*prev - pt).length() < 1.0
        {
            continue;
        }
        pixels.push(pt);
    }

    // End: ramp and face point on the target node
    let ramp_end = ramp_from_face(to_rect, entry_face, port_offset_end, node_margin);
    let fp_end = face_point_with_port(to_rect, entry_face, port_offset_end);
    if let Some(prev) = pixels.last() {
        if (*prev - ramp_end).length() >= 1.0 {
            pixels.push(ramp_end);
        }
    } else {
        pixels.push(ramp_end);
    }
    pixels.push(fp_end);

    // Ensure all segments are orthogonal by inserting corner points
    ensure_orthogonal(&mut pixels);

    pixels
}

/// Convert a grid coordinate's column to pixel X.
/// The routing engine uses 1-based integer coords. A coord with col=1 means
/// the center of column 1 → pixel x = (1 - 0.5) * cell_w + origin_x = 0.5 * cell_w + origin_x.
/// Half-integer coords (junction between columns) are handled naturally via col_f64().
pub(super) fn coord_to_pixel_x(coord: routing::types::GridCoord, grid: &GridInfo) -> f32 {
    // The routing engine uses 1-based coords. Grid layout uses: center_x = (col - 0.5) * cell_w
    // But origin_x is already in the GridInfo, so: px = (col_f64 - 0.5) * cell_w + origin_x
    // However, the routing engine coords are 1-based integers stored as doubled.
    // col_f64() gives the actual column number (1.0, 1.5, 2.0, etc.)
    // The grid layout places col 1 at (1 - 0.5) * cell_w = 0.5 * cell_w from origin.
    (coord.col_f64() as f32 - 0.5) * grid.cell_w + grid.origin_x
}

/// Convert a grid coordinate's row to pixel Y.
pub(super) fn coord_to_pixel_y(coord: routing::types::GridCoord, grid: &GridInfo) -> f32 {
    (coord.row_f64() as f32 - 0.5) * grid.cell_h + grid.origin_y
}

/// Determine the direction from one grid coord to another.
pub(super) fn coord_direction(
    from: routing::types::GridCoord,
    to: routing::types::GridCoord,
) -> routing::types::Direction {
    let dc = to.col2 - from.col2;
    let dr = to.row2 - from.row2;
    if dc.abs() >= dr.abs() {
        if dc >= 0 {
            routing::types::Direction::East
        } else {
            routing::types::Direction::West
        }
    } else if dr >= 0 {
        routing::types::Direction::South
    } else {
        routing::types::Direction::North
    }
}

/// Compute pixel offset for a lane perpendicular to the travel direction.
/// Lane 0 = center (no offset). Uses absolute convention:
///   - Horizontal segments: positive lanes offset south (+Y), negative offset north (-Y)
///   - Vertical segments: positive lanes offset east (+X), negative offset west (-X)
///
/// This ensures lane numbers map to the same physical position on a segment
/// regardless of travel direction.
pub(super) fn lane_offset(
    dir: routing::types::Direction,
    lane: i32,
    lane_spacing: f32,
) -> (f32, f32) {
    if lane == 0 {
        return (0.0, 0.0);
    }
    let offset = lane as f32 * lane_spacing;
    if dir.is_horizontal() {
        // Horizontal travel: lane offset in Y. Positive lane = south.
        (0.0, offset)
    } else {
        // Vertical travel: lane offset in X. Positive lane = east.
        (offset, 0.0)
    }
}

/// Post-process waypoints to ensure every consecutive pair is axis-aligned.
pub(super) fn ensure_orthogonal(waypoints: &mut Vec<Pos2>) {
    let mut i = 0;
    while i + 1 < waypoints.len() {
        let a = waypoints[i];
        let b = waypoints[i + 1];
        let dx = (a.x - b.x).abs();
        let dy = (a.y - b.y).abs();
        if dx > 1.0 && dy > 1.0 {
            let was_horizontal = if i > 0 {
                let prev = waypoints[i - 1];
                (prev.y - a.y).abs() < (prev.x - a.x).abs()
            } else {
                dx > dy
            };
            let corner = if was_horizontal {
                Pos2::new(b.x, a.y)
            } else {
                Pos2::new(a.x, b.y)
            };
            waypoints.insert(i + 1, corner);
        }
        i += 1;
    }
}

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

/// Draw a dashed polyline across multiple segments with continuity.
fn draw_dashed_polyline(
    painter: &egui::Painter,
    points: &[Pos2],
    width: f32,
    color: Color32,
    scale: f32,
) {
    let dash_len = 8.0 * scale;
    let gap_len = 5.0 * scale;
    let total_len = polyline_length(points);
    let stroke = Stroke::new(width, color);

    let mut d = 0.0;
    let mut drawing = true;
    while d < total_len {
        if drawing {
            let seg_end_d = (d + dash_len).min(total_len);
            let p1 = polyline_point_at_distance(points, d);
            let p2 = polyline_point_at_distance(points, seg_end_d);
            painter.line_segment([p1, p2], stroke);
            d += dash_len;
        } else {
            d += gap_len;
        }
        drawing = !drawing;
    }
}

/// Draw a routed edge with rounded corners, arrowheads, and optional label.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_routed_edge(
    painter: &egui::Painter,
    waypoints: &[Pos2],
    arrow: ArrowKind,
    label: &str,
    edge_color: Color32,
    label_bg: Color32,
    label_text_color: Color32,
    line_width: f32,
    arrow_size: f32,
    corner_radius: f32,
    theme: &Theme,
    scale: f32,
    _opacity: f32,
    anim_progress: f32,
) {
    if waypoints.len() < 2 {
        return;
    }

    let is_dashed = matches!(arrow, ArrowKind::DashedLine | ArrowKind::DashedArrow);
    let start = waypoints[0];
    let end = *waypoints.last().unwrap();

    // Apply rounded corners to get smooth polyline
    let smooth_points = apply_rounded_corners(waypoints, corner_radius);

    // Determine arrowhead status
    let has_end_arrow = matches!(
        arrow,
        ArrowKind::Forward | ArrowKind::DashedArrow | ArrowKind::Bidirectional
    );
    let has_start_arrow = matches!(arrow, ArrowKind::Reverse | ArrowKind::Bidirectional);

    // Total length for animation clipping
    let total_len = polyline_length(&smooth_points);

    // Clip the effective drawing length by animation progress
    let effective_len = total_len * anim_progress;

    // Shorten the polyline for arrowheads. While animating, the end arrowhead
    // rides on the growing tip, so the line stops one arrow length before it.
    let draw_start_d = if has_start_arrow { arrow_size } else { 0.0 };
    let draw_end_d = if has_end_arrow {
        (effective_len - arrow_size).max(0.0)
    } else {
        effective_len
    };

    // Build shortened polyline for line drawing
    if draw_end_d > draw_start_d + 1.0 {
        let line_start = polyline_point_at_distance(&smooth_points, draw_start_d);
        let line_end = polyline_point_at_distance(&smooth_points, draw_end_d);

        // Collect intermediate points
        let mut draw_points = vec![line_start];
        let mut cumulative = 0.0;
        for i in 0..smooth_points.len().saturating_sub(1) {
            let seg_len = (smooth_points[i + 1] - smooth_points[i]).length();
            let next_cumulative = cumulative + seg_len;
            if next_cumulative > draw_start_d && cumulative < draw_end_d {
                if cumulative > draw_start_d {
                    draw_points.push(smooth_points[i]);
                }
                if next_cumulative < draw_end_d {
                    draw_points.push(smooth_points[i + 1]);
                }
            }
            cumulative = next_cumulative;
        }
        draw_points.push(line_end);
        draw_points.dedup_by(|a, b| (*a - *b).length() < 0.5);

        if draw_points.len() >= 2 {
            if is_dashed {
                draw_dashed_polyline(painter, &draw_points, line_width, edge_color, scale);
            } else {
                painter.add(egui::Shape::line(
                    draw_points,
                    Stroke::new(line_width, edge_color),
                ));
            }
        }
    }

    // Arrowheads: at the animated tip while growing, snapped to the true end once complete
    let draw_arrowhead = |tip: Pos2, direction: egui::Vec2| {
        let d = direction.normalized();
        let p = egui::vec2(-d.y, d.x);
        let p1 = tip - d * arrow_size + p * arrow_size * 0.4;
        let p2 = tip - d * arrow_size - p * arrow_size * 0.4;
        painter.add(egui::Shape::convex_polygon(
            vec![tip, p1, p2],
            edge_color,
            Stroke::NONE,
        ));
    };

    if has_end_arrow && anim_progress < 1.0 {
        if effective_len > arrow_size * 1.2 {
            let tip = polyline_point_at_distance(&smooth_points, effective_len);
            let tail = polyline_point_at_distance(
                &smooth_points,
                (effective_len - arrow_size * 0.5).max(0.0),
            );
            draw_arrowhead(tip, tip - tail);
        }
    } else if has_end_arrow {
        let n = waypoints.len();
        let last_seg_len = if n >= 2 {
            (waypoints[n - 1] - waypoints[n - 2]).length()
        } else {
            total_len
        };
        if last_seg_len >= arrow_size * 1.2 {
            let last_dir = if n >= 2 {
                waypoints[n - 1] - waypoints[n - 2]
            } else {
                end - start
            };
            draw_arrowhead(end, last_dir);
        } else if n >= 3 {
            let pre_turn_dir = waypoints[n - 2] - waypoints[n - 3];
            let arrowhead_tip = waypoints[n - 2];
            draw_arrowhead(arrowhead_tip, pre_turn_dir);
        } else {
            let last_dir = end - start;
            draw_arrowhead(end, last_dir);
        }
    }

    if has_start_arrow && effective_len > arrow_size * 1.2 {
        let first_seg_len = if waypoints.len() >= 2 {
            (waypoints[1] - waypoints[0]).length()
        } else {
            total_len
        };
        if first_seg_len >= arrow_size * 1.2 {
            let first_dir = if waypoints.len() >= 2 {
                waypoints[0] - waypoints[1]
            } else {
                start - end
            };
            draw_arrowhead(start, first_dir);
        } else if waypoints.len() >= 3 {
            let post_turn_dir = waypoints[1] - waypoints[2];
            let arrowhead_tip = waypoints[1];
            draw_arrowhead(arrowhead_tip, post_turn_dir);
        } else {
            let first_dir = start - end;
            draw_arrowhead(start, first_dir);
        }
    }

    // Edge label only when animation is complete
    if !label.is_empty() && anim_progress >= 1.0 {
        let mid_distance = total_len * 0.20;
        let mid = polyline_point_at_distance(&smooth_points, mid_distance);
        let label_font_size = theme.body_size * 0.65 * scale;
        let label_pad_h = 10.0 * scale;
        let label_pad_v = 5.0 * scale;
        let galley = painter.layout_no_wrap(
            label.to_string(),
            FontId::proportional(label_font_size),
            label_text_color,
        );
        let label_w = galley.rect.width() + label_pad_h * 2.0;
        let label_h = galley.rect.height() + label_pad_v * 2.0;
        let label_rect = egui::Rect::from_center_size(mid, egui::vec2(label_w, label_h));
        painter.rect_filled(label_rect, label_h / 2.0, label_bg);
        painter.galley(
            egui::pos2(
                label_rect.left() + label_pad_h,
                label_rect.top() + label_pad_v,
            ),
            galley,
            label_text_color,
        );
    }
}
