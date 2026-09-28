use std::collections::HashSet;

use eframe::egui::{self, Pos2};

use super::super::parsing::parse_diagram;
use super::super::ports::*;
use super::super::routing;
use super::super::types::*;

// ── Face/ramp helper tests ───────────────────────────────────────────────

#[test]
fn test_face_selection_horizontal() {
    let r = egui::Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(100.0, 80.0));
    assert!(matches!(
        face_toward(&r, Pos2::new(200.0, 40.0)),
        Face::Right
    ));
    assert!(matches!(
        face_toward(&r, Pos2::new(-100.0, 40.0)),
        Face::Left
    ));
}

#[test]
fn test_face_selection_vertical() {
    let r = egui::Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(100.0, 80.0));
    assert!(matches!(
        face_toward(&r, Pos2::new(50.0, 200.0)),
        Face::Bottom
    ));
    assert!(matches!(
        face_toward(&r, Pos2::new(50.0, -100.0)),
        Face::Top
    ));
}

#[test]
fn test_face_selection_diagonal_right_down() {
    let r = egui::Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(100.0, 80.0));
    // When dx and dy are equal, dx.abs() >= dy.abs() is true, so Right
    let face = face_toward(&r, Pos2::new(200.0, 200.0));
    assert!(matches!(face, Face::Right | Face::Bottom));
}

#[test]
fn test_entry_face_matches_direction() {
    let r = egui::Rect::from_min_max(Pos2::new(100.0, 100.0), Pos2::new(200.0, 180.0));
    assert!(matches!(face_toward(&r, Pos2::new(0.0, 140.0)), Face::Left));
    assert!(matches!(
        face_toward(&r, Pos2::new(300.0, 140.0)),
        Face::Right
    ));
    assert!(matches!(face_toward(&r, Pos2::new(150.0, 0.0)), Face::Top));
    assert!(matches!(
        face_toward(&r, Pos2::new(150.0, 300.0)),
        Face::Bottom
    ));
}

#[test]
fn test_ramp_from_face_right() {
    let r = egui::Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(100.0, 80.0));
    let ramp = ramp_from_face(&r, Face::Right, 0.0, 10.0);
    assert_eq!(ramp.x, 110.0);
    assert_eq!(ramp.y, 40.0);
}

#[test]
fn test_ramp_from_face_left() {
    let r = egui::Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(100.0, 80.0));
    let ramp = ramp_from_face(&r, Face::Left, 0.0, 10.0);
    assert_eq!(ramp.x, -10.0);
    assert_eq!(ramp.y, 40.0);
}

#[test]
fn test_ramp_from_face_bottom() {
    let r = egui::Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(100.0, 80.0));
    let ramp = ramp_from_face(&r, Face::Bottom, 0.0, 10.0);
    assert_eq!(ramp.x, 50.0);
    assert_eq!(ramp.y, 90.0);
}

#[test]
fn test_ramp_from_face_top() {
    let r = egui::Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(100.0, 80.0));
    let ramp = ramp_from_face(&r, Face::Top, 0.0, 10.0);
    assert_eq!(ramp.x, 50.0);
    assert_eq!(ramp.y, -10.0);
}

#[test]
fn test_face_point_with_port_offset() {
    let r = egui::Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(100.0, 80.0));
    let pt = face_point_with_port(&r, Face::Top, 5.0);
    assert_eq!(pt.x, 55.0); // center.x + 5
    assert_eq!(pt.y, 0.0);
}

// ── Ensure orthogonal tests ──────────────────────────────────────────────

#[test]
fn test_ensure_orthogonal_inserts_corner() {
    let mut pts = vec![Pos2::new(0.0, 0.0), Pos2::new(100.0, 80.0)];
    ensure_orthogonal(&mut pts);
    assert!(pts.len() >= 3);
    // All consecutive pairs should be axis-aligned
    for w in pts.windows(2) {
        let dx = (w[0].x - w[1].x).abs();
        let dy = (w[0].y - w[1].y).abs();
        assert!(dx < 1.5 || dy < 1.5, "Non-orthogonal segment: {w:?}");
    }
}

#[test]
fn test_ensure_orthogonal_already_orthogonal() {
    let mut pts = vec![
        Pos2::new(0.0, 0.0),
        Pos2::new(100.0, 0.0),
        Pos2::new(100.0, 80.0),
    ];
    ensure_orthogonal(&mut pts);
    assert_eq!(pts.len(), 3); // no insertions needed
}

// ── Coordinate conversion tests ──────────────────────────────────────────

#[test]
fn test_coord_to_pixel_basic() {
    let grid = GridInfo {
        cols: 3,
        rows: 2,
        cell_w: 200.0,
        cell_h: 150.0,
        origin_x: 10.0,
        origin_y: 20.0,
        occupied: HashSet::new(),
    };
    // Col 1, Row 1 (1-based) → center of first cell
    let coord = routing::types::GridCoord::from_int(1, 1);
    let px = coord_to_pixel_x(coord, &grid);
    let py = coord_to_pixel_y(coord, &grid);
    // (1 - 0.5) * 200 + 10 = 110
    assert!((px - 110.0).abs() < 0.1);
    // (1 - 0.5) * 150 + 20 = 95
    assert!((py - 95.0).abs() < 0.1);
}

#[test]
fn test_coord_to_pixel_junction() {
    let grid = GridInfo {
        cols: 3,
        rows: 2,
        cell_w: 200.0,
        cell_h: 150.0,
        origin_x: 0.0,
        origin_y: 0.0,
        occupied: HashSet::new(),
    };
    // Junction at (1.5, 1.0), between columns 1 and 2
    let coord = routing::types::GridCoord::from_grid(1.5, 1.0);
    let px = coord_to_pixel_x(coord, &grid);
    let py = coord_to_pixel_y(coord, &grid);
    // (1.5 - 0.5) * 200 + 0 = 200
    assert!((px - 200.0).abs() < 0.1);
    // (1.0 - 0.5) * 150 + 0 = 75
    assert!((py - 75.0).abs() < 0.1);
}

#[test]
fn test_lane_offset_zero() {
    let (ox, oy) = lane_offset(routing::types::Direction::East, 0, 20.0);
    assert_eq!(ox, 0.0);
    assert_eq!(oy, 0.0);
}

#[test]
fn test_lane_offset_positive() {
    // Absolute convention: positive lane = south for horizontal, east for vertical.
    // Lane +1 traveling East → south (+Y)
    let (ox, oy) = lane_offset(routing::types::Direction::East, 1, 20.0);
    assert_eq!(ox, 0.0);
    assert_eq!(oy, 20.0);

    // Lane +1 traveling West → also south (+Y): absolute, not direction-relative
    let (ox, oy) = lane_offset(routing::types::Direction::West, 1, 20.0);
    assert_eq!(ox, 0.0);
    assert_eq!(oy, 20.0);

    // Lane +1 traveling South → east (+X)
    let (ox, oy) = lane_offset(routing::types::Direction::South, 1, 20.0);
    assert_eq!(ox, 20.0);
    assert_eq!(oy, 0.0);

    // Lane +1 traveling North → also east (+X)
    let (ox, oy) = lane_offset(routing::types::Direction::North, 1, 20.0);
    assert_eq!(ox, 20.0);
    assert_eq!(oy, 0.0);
}

#[test]
fn test_lane_offset_negative() {
    // Lane -1 traveling East → north (-Y)
    let (ox, oy) = lane_offset(routing::types::Direction::East, -1, 20.0);
    assert_eq!(ox, 0.0);
    assert_eq!(oy, -20.0);

    // Lane -1 traveling West → also north (-Y)
    let (ox, oy) = lane_offset(routing::types::Direction::West, -1, 20.0);
    assert_eq!(ox, 0.0);
    assert_eq!(oy, -20.0);

    // Lane -1 traveling North → west (-X)
    let (ox, oy) = lane_offset(routing::types::Direction::North, -1, 20.0);
    assert_eq!(ox, -20.0);
    assert_eq!(oy, 0.0);
}

// ── Port claim tests ─────────────────────────────────────────────────────

#[test]
fn port_claims_alternate_sides_within_the_face() {
    let r = egui::Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(100.0, 80.0));
    let mut claims = PortClaims::new(10.0);
    let offsets: Vec<f32> = (0..6).map(|_| claims.claim("A", Face::Right, &r)).collect();
    // Centre, then +1, -1, +2, -2 spacings, clamped to 30% of the 80 px face
    assert_eq!(offsets, [0.0, 10.0, -10.0, 20.0, -20.0, 24.0]);
    // Other faces and nodes count separately
    assert_eq!(claims.claim("A", Face::Top, &r), 0.0);
    assert_eq!(claims.claim("B", Face::Right, &r), 0.0);
}

#[test]
fn direct_waypoints_ramp_out_of_facing_sides() {
    let (_, edges, _) = parse_diagram("- A -> B");
    let from = egui::Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(100.0, 80.0));
    let to = egui::Rect::from_min_max(Pos2::new(300.0, 0.0), Pos2::new(400.0, 80.0));
    let mut claims = PortClaims::new(22.0);
    let points = direct_waypoints(&edges[0], &from, &to, &mut claims, 10.0);
    assert_eq!(
        points,
        [
            Pos2::new(100.0, 40.0),
            Pos2::new(110.0, 40.0),
            Pos2::new(290.0, 40.0),
            Pos2::new(300.0, 40.0)
        ]
    );
    // A second edge between the same faces is spread off-centre
    let second = direct_waypoints(&edges[0], &from, &to, &mut claims, 10.0);
    assert_eq!(second[0], Pos2::new(100.0, 62.0));
}

#[test]
fn test_coord_direction() {
    use routing::types::{Direction, GridCoord};
    let a = GridCoord::from_int(2, 2);
    assert_eq!(
        coord_direction(a, GridCoord::from_int(3, 2)),
        Direction::East
    );
    assert_eq!(
        coord_direction(a, GridCoord::from_int(1, 2)),
        Direction::West
    );
    assert_eq!(
        coord_direction(a, GridCoord::from_int(2, 3)),
        Direction::South
    );
    assert_eq!(
        coord_direction(a, GridCoord::from_int(2, 1)),
        Direction::North
    );
    assert_eq!(direction_to_face(Direction::North), Face::Top);
    assert_eq!(direction_to_face(Direction::West), Face::Left);
}
