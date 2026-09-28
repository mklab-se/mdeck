use std::collections::HashMap;

use eframe::egui::{self, Pos2};

use super::routing::types::{Direction, GridCoord, Route};
use super::types::*;

// ─── Node faces and ports ────────────────────────────────────────────────────

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

/// The face of `rect` that looks most directly at `point`: the exit face
/// toward a target, or the entry face from a source.
pub(super) fn face_toward(rect: &egui::Rect, point: Pos2) -> Face {
    let c = rect.center();
    let dx = point.x - c.x;
    let dy = point.y - c.y;
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
pub(super) fn direction_to_face(dir: Direction) -> Face {
    match dir {
        Direction::North => Face::Top,
        Direction::South => Face::Bottom,
        Direction::East => Face::Right,
        Direction::West => Face::Left,
    }
}

/// Spreads connections that share a node face so they do not overlap. Only
/// fallback (unrouted) edges claim ports; routed edges take their port from
/// the lane they leave in.
pub(super) struct PortClaims {
    counts: HashMap<(String, Face), usize>,
    spacing: f32,
}

impl PortClaims {
    pub(super) fn new(spacing: f32) -> Self {
        Self {
            counts: HashMap::new(),
            spacing,
        }
    }

    /// The offset along `face` of the next connection to `node`: centred for
    /// the first, then alternating either side, within 30% of the face.
    pub(super) fn claim(&mut self, node: &str, face: Face, rect: &egui::Rect) -> f32 {
        let idx = self.counts.entry((node.to_string(), face)).or_insert(0);
        let current = *idx;
        *idx += 1;

        if current == 0 {
            return 0.0;
        }

        let face_length = match face {
            Face::Left | Face::Right => rect.height(),
            Face::Top | Face::Bottom => rect.width(),
        };
        let max_offset = face_length * 0.3;
        let level = current.div_ceil(2);
        let sign = if current % 2 == 1 { 1.0 } else { -1.0 };
        let offset = sign * level as f32 * self.spacing;
        offset.clamp(-max_offset, max_offset)
    }
}

/// A direct connection for an edge the router could not place: out of the
/// face toward the target, a short ramp at each end.
pub(super) fn direct_waypoints(
    edge: &DiagramEdge,
    from_rect: &egui::Rect,
    to_rect: &egui::Rect,
    claims: &mut PortClaims,
    node_margin: f32,
) -> Vec<Pos2> {
    let exit_face = face_toward(from_rect, to_rect.center());
    let entry_face = face_toward(to_rect, from_rect.center());
    let port_start = claims.claim(&edge.from, exit_face, from_rect);
    let port_end = claims.claim(&edge.to, entry_face, to_rect);
    let fp_start = face_point_with_port(from_rect, exit_face, port_start);
    let fp_end = face_point_with_port(to_rect, entry_face, port_end);
    let ramp_start = ramp_from_face(from_rect, exit_face, port_start, node_margin);
    let ramp_end = ramp_from_face(to_rect, entry_face, port_end, node_margin);
    vec![fp_start, ramp_start, ramp_end, fp_end]
}

// ─── Routes to pixels ────────────────────────────────────────────────────────

/// Which way a route leaves its source and enters its target.
struct RouteEnds {
    exit_dir: Direction,
    exit_face: Face,
    entry_dir: Direction,
    entry_face: Face,
}

/// Read the exit and entry directions off a route of at least two waypoints.
fn route_ends(route: &Route) -> RouteEnds {
    let n = route.waypoints.len();
    let exit_dir = coord_direction(route.waypoints[0].coord, route.waypoints[1].coord);
    let entry_dir = coord_direction(route.waypoints[n - 2].coord, route.waypoints[n - 1].coord);
    RouteEnds {
        exit_dir,
        exit_face: direction_to_face(exit_dir),
        entry_dir,
        entry_face: direction_to_face(entry_dir.opposite()),
    }
}

/// Port offsets (start, end) of a routed edge, taken from the lanes of its
/// first and last segments so the face points line up with the corridor.
pub(super) fn route_ports(route: &Route, lane_spacing: f32) -> (f32, f32) {
    let ends = route_ends(route);
    let n = route.waypoints.len();

    let (elx, ely) = lane_offset(ends.exit_dir, route.waypoints[0].lane, lane_spacing);
    let port_start = match ends.exit_face {
        Face::Left | Face::Right => ely,
        Face::Top | Face::Bottom => elx,
    };

    let (nlx, nly) = lane_offset(ends.entry_dir, route.waypoints[n - 2].lane, lane_spacing);
    let port_end = match ends.entry_face {
        Face::Left | Face::Right => nly,
        Face::Top | Face::Bottom => nlx,
    };
    (port_start, port_end)
}

/// Convert a routing engine Route to pixel waypoints for drawing.
///
/// The routing engine works in 1-based integer grid coordinates.
/// This function converts those to pixel positions using the GridInfo geometry,
/// and adds face connection points (ramps) at the start and end.
pub(super) fn waypoints_to_pixels(
    route: &Route,
    grid: &GridInfo,
    (from_rect, to_rect): (&egui::Rect, &egui::Rect),
    (port_offset_start, port_offset_end): (f32, f32),
    metrics: &EdgeMetrics,
) -> Vec<Pos2> {
    if route.waypoints.len() < 2 {
        return Vec::new();
    }
    let ends = route_ends(route);
    let node_margin = metrics.node_margin;

    // Start: face point and ramp on the source node
    let mut pixels = vec![
        face_point_with_port(from_rect, ends.exit_face, port_offset_start),
        ramp_from_face(from_rect, ends.exit_face, port_offset_start, node_margin),
    ];

    // Intermediate waypoints (skip first = source center, skip last = target center)
    for i in 1..route.waypoints.len() - 1 {
        let pt = bend_point(route, i, grid, metrics.lane_spacing);
        if let Some(prev) = pixels.last()
            && (*prev - pt).length() < 1.0
        {
            continue;
        }
        pixels.push(pt);
    }

    // End: ramp and face point on the target node
    let ramp_end = ramp_from_face(to_rect, ends.entry_face, port_offset_end, node_margin);
    let fp_end = face_point_with_port(to_rect, ends.entry_face, port_offset_end);
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

/// The pixel position of intermediate waypoint `i`, shifted into the lanes of
/// the segments either side of it.
fn bend_point(route: &Route, i: usize, grid: &GridInfo, lane_spacing: f32) -> Pos2 {
    let wp = &route.waypoints[i];
    let px = coord_to_pixel_x(wp.coord, grid);
    let py = coord_to_pixel_y(wp.coord, grid);

    // Incoming offset (from the previous segment's direction and lane)
    let prev_wp = &route.waypoints[i - 1];
    let in_dir = coord_direction(prev_wp.coord, wp.coord);
    let (in_ox, in_oy) = lane_offset(in_dir, prev_wp.lane, lane_spacing);

    // Outgoing offset (from this waypoint's direction and lane to the next)
    let out_dir = coord_direction(wp.coord, route.waypoints[i + 1].coord);
    let (out_ox, out_oy) = lane_offset(out_dir, wp.lane, lane_spacing);

    // At a turn (horizontal↔vertical), compute a single combined corner point
    // that keeps both the incoming and outgoing segments straight:
    //   - Horizontal segments offset Y → keep incoming Y at the corner
    //   - Vertical segments offset X → keep outgoing X at the corner
    if in_dir.is_horizontal() != out_dir.is_horizontal() {
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
    }
}

/// Convert a grid coordinate's column to pixel X.
/// The routing engine uses 1-based coords, and col_f64() gives the actual
/// column number (1.0, 1.5, 2.0, etc.), so half-integer junctions between
/// columns fall out naturally. The grid layout places col 1 at
/// (1 - 0.5) * cell_w from the origin.
pub(super) fn coord_to_pixel_x(coord: GridCoord, grid: &GridInfo) -> f32 {
    (coord.col_f64() as f32 - 0.5) * grid.cell_w + grid.origin_x
}

/// Convert a grid coordinate's row to pixel Y.
pub(super) fn coord_to_pixel_y(coord: GridCoord, grid: &GridInfo) -> f32 {
    (coord.row_f64() as f32 - 0.5) * grid.cell_h + grid.origin_y
}

/// Determine the direction from one grid coord to another.
pub(super) fn coord_direction(from: GridCoord, to: GridCoord) -> Direction {
    let dc = to.col2 - from.col2;
    let dr = to.row2 - from.row2;
    if dc.abs() >= dr.abs() {
        if dc >= 0 {
            Direction::East
        } else {
            Direction::West
        }
    } else if dr >= 0 {
        Direction::South
    } else {
        Direction::North
    }
}

/// Compute pixel offset for a lane perpendicular to the travel direction.
/// Lane 0 = center (no offset). Uses absolute convention:
///   - Horizontal segments: positive lanes offset south (+Y), negative offset north (-Y)
///   - Vertical segments: positive lanes offset east (+X), negative offset west (-X)
///
/// This ensures lane numbers map to the same physical position on a segment
/// regardless of travel direction.
pub(super) fn lane_offset(dir: Direction, lane: i32, lane_spacing: f32) -> (f32, f32) {
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
