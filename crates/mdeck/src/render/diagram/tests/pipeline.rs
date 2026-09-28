use eframe::egui::{self, Pos2};

use super::super::ports::*;
use super::super::routing;
use super::super::types::*;

// ── Integration tests: full routing pipeline ─────────────────────────────

#[test]
fn test_integration_two_adjacent_nodes() {
    // Two nodes side by side: A(1,1) → B(2,1)
    let nodes = vec![
        routing::types::DiagramNode {
            name: "A".into(),
            col: 1,
            row: 1,
        },
        routing::types::DiagramNode {
            name: "B".into(),
            col: 2,
            row: 1,
        },
    ];
    let edges = vec![routing::types::DiagramEdge {
        source: "A".into(),
        target: "B".into(),
        label: None,
    }];
    let config = routing::types::RoutingConfig {
        h_lane_capacity: 3,
        v_lane_capacity: 3,
        weights: routing::types::CostWeights::default(),
    };
    let output = routing::route_all_edges(&nodes, &edges, &config);
    assert_eq!(output.results.len(), 1);
    match &output.results[0].1 {
        routing::types::RouteResult::Success(route) => {
            assert!(route.waypoints.len() >= 2);
            // Source at (1,1), target at (2,1): should go east
            let first = route.waypoints[0].coord;
            let last = route.waypoints.last().unwrap().coord;
            assert_eq!(first, routing::types::GridCoord::from_int(1, 1));
            assert_eq!(last, routing::types::GridCoord::from_int(2, 1));
        }
        routing::types::RouteResult::Failure { warning } => {
            panic!("Expected success, got failure: {warning}");
        }
    }
}

#[test]
fn test_integration_l_shaped_route() {
    // A(1,1) → C(2,2): should produce an L-shape with 1 turn
    let nodes = vec![
        routing::types::DiagramNode {
            name: "A".into(),
            col: 1,
            row: 1,
        },
        routing::types::DiagramNode {
            name: "B".into(),
            col: 2,
            row: 1,
        },
        routing::types::DiagramNode {
            name: "C".into(),
            col: 2,
            row: 2,
        },
    ];
    let edges = vec![routing::types::DiagramEdge {
        source: "A".into(),
        target: "C".into(),
        label: None,
    }];
    let config = routing::types::RoutingConfig {
        h_lane_capacity: 3,
        v_lane_capacity: 3,
        weights: routing::types::CostWeights::default(),
    };
    let output = routing::route_all_edges(&nodes, &edges, &config);
    match &output.results[0].1 {
        routing::types::RouteResult::Success(route) => {
            assert!(route.complexity.turns >= 1);
            assert!(route.waypoints.len() >= 3);
        }
        routing::types::RouteResult::Failure { warning } => {
            panic!("Expected success, got failure: {warning}");
        }
    }
}

#[test]
fn test_integration_waypoints_to_pixels() {
    // Test the full pipeline: route → pixels
    let nodes = vec![
        routing::types::DiagramNode {
            name: "A".into(),
            col: 1,
            row: 1,
        },
        routing::types::DiagramNode {
            name: "B".into(),
            col: 2,
            row: 1,
        },
    ];
    let edges = vec![routing::types::DiagramEdge {
        source: "A".into(),
        target: "B".into(),
        label: None,
    }];
    let config = routing::types::RoutingConfig {
        h_lane_capacity: 3,
        v_lane_capacity: 3,
        weights: routing::types::CostWeights::default(),
    };
    let output = routing::route_all_edges(&nodes, &edges, &config);

    let grid = GridInfo {
        cols: 2,
        rows: 1,
        cell_w: 200.0,
        cell_h: 150.0,
        origin_x: 0.0,
        origin_y: 0.0,
        occupied: [(0, 0), (1, 0)].iter().cloned().collect(),
    };
    let from_rect = egui::Rect::from_center_size(Pos2::new(100.0, 75.0), egui::vec2(130.0, 90.0));
    let to_rect = egui::Rect::from_center_size(Pos2::new(300.0, 75.0), egui::vec2(130.0, 90.0));

    match &output.results[0].1 {
        routing::types::RouteResult::Success(route) => {
            let pixels = waypoints_to_pixels(
                route,
                &grid,
                (&from_rect, &to_rect),
                (0.0, 0.0),
                &EdgeMetrics::new(1.0),
            );
            assert!(pixels.len() >= 2);
            // First pixel should be near the right face of from_rect
            assert!((pixels[0].x - from_rect.right()).abs() < 1.0);
            // Last pixel should be near the left face of to_rect
            assert!((pixels.last().unwrap().x - to_rect.left()).abs() < 1.0);
            // All segments should be orthogonal
            for w in pixels.windows(2) {
                let dx = (w[0].x - w[1].x).abs();
                let dy = (w[0].y - w[1].y).abs();
                assert!(
                    dx < 1.5 || dy < 1.5,
                    "Non-orthogonal: {:?} → {:?}",
                    w[0],
                    w[1]
                );
            }
        }
        routing::types::RouteResult::Failure { warning } => {
            panic!("Expected success, got failure: {warning}");
        }
    }
}

#[test]
fn test_integration_route_with_lane_offset() {
    // Two parallel edges between same nodes should get different lanes
    let nodes = vec![
        routing::types::DiagramNode {
            name: "A".into(),
            col: 1,
            row: 1,
        },
        routing::types::DiagramNode {
            name: "B".into(),
            col: 2,
            row: 1,
        },
    ];
    let edges = vec![
        routing::types::DiagramEdge {
            source: "A".into(),
            target: "B".into(),
            label: None,
        },
        routing::types::DiagramEdge {
            source: "A".into(),
            target: "B".into(),
            label: Some("second".into()),
        },
    ];
    let config = routing::types::RoutingConfig {
        h_lane_capacity: 5,
        v_lane_capacity: 5,
        weights: routing::types::CostWeights::default(),
    };
    let output = routing::route_all_edges(&nodes, &edges, &config);
    assert_eq!(output.results.len(), 2);
    // Both should succeed
    for (_, result) in &output.results {
        assert!(matches!(result, routing::types::RouteResult::Success(_)));
    }
}

/// Regression test: a turn with a lane change must produce a clean corner,
/// not an S-curve jog. When a Westbound L-1 segment turns Southbound L0,
/// the corner point should combine the incoming Y offset with the outgoing
/// X offset so both segments stay straight.
#[test]
fn test_turn_with_lane_change_no_scurve() {
    // Reproduce the Hub-and-Spoke API→Auth route:
    //   (2,2) L-1 → (1.5,2) L-1 → (1,2) L0 → (1,2.5) L0 → (1,3)
    // This goes West at lane -1, turns South at lane 0 at coord (1,2).
    let route = routing::types::Route {
        waypoints: vec![
            routing::types::Waypoint {
                coord: routing::types::GridCoord::from_int(2, 2),
                lane: -1,
            },
            routing::types::Waypoint {
                coord: routing::types::GridCoord { col2: 3, row2: 4 }, // (1.5, 2)
                lane: -1,
            },
            routing::types::Waypoint {
                coord: routing::types::GridCoord::from_int(1, 2),
                lane: 0,
            },
            routing::types::Waypoint {
                coord: routing::types::GridCoord { col2: 2, row2: 5 }, // (1, 2.5)
                lane: 0,
            },
            routing::types::Waypoint {
                coord: routing::types::GridCoord::from_int(1, 3),
                lane: 0,
            },
        ],
        complexity: routing::types::RouteComplexity {
            length: 2.0,
            turns: 1,
            lane_changes: 0,
            crossings: 0,
        },
    };

    let grid = GridInfo {
        cols: 3,
        rows: 3,
        cell_w: 300.0,
        cell_h: 200.0,
        origin_x: 0.0,
        origin_y: 0.0,
        occupied: [
            (0, 0),
            (1, 0),
            (2, 0),
            (0, 1),
            (1, 1),
            (2, 1),
            (0, 2),
            (1, 2),
            (2, 2),
        ]
        .iter()
        .cloned()
        .collect(),
    };
    let metrics = EdgeMetrics::new(1.0);
    let lane_spacing = metrics.lane_spacing;

    // API at (2,2) → center pixel (450, 300)
    let from_rect = egui::Rect::from_center_size(Pos2::new(450.0, 300.0), egui::vec2(160.0, 120.0));
    // Auth at (1,3) → center pixel (150, 500)
    let to_rect = egui::Rect::from_center_size(Pos2::new(150.0, 500.0), egui::vec2(160.0, 120.0));

    // Port offsets as the renderer derives them: leaving west on lane -1
    // shifts the start north; arriving south on lane 0 is centred
    let ports = route_ports(&route, lane_spacing);
    assert_eq!(ports, (-20.0, 0.0));

    let pixels = waypoints_to_pixels(&route, &grid, (&from_rect, &to_rect), ports, &metrics);

    // All segments must be orthogonal (no diagonal jogs)
    for w in pixels.windows(2) {
        let dx = (w[0].x - w[1].x).abs();
        let dy = (w[0].y - w[1].y).abs();
        assert!(
            dx < 1.5 || dy < 1.5,
            "Non-orthogonal segment: {:?} → {:?} (dx={dx:.1}, dy={dy:.1})",
            w[0],
            w[1]
        );
    }

    // The key check: no S-curve at the turn. Find the vertical segments
    // near the turn column (x ≈ 150, column 1 center). The Y values must
    // be monotonically decreasing (going up) or increasing (going down),
    // never reversing direction.
    let turn_col_x = 150.0; // center of column 1
    let vertical_near_turn: Vec<&Pos2> = pixels
        .iter()
        .filter(|p| (p.x - turn_col_x).abs() < lane_spacing * 2.0)
        .collect();

    if vertical_near_turn.len() >= 2 {
        // Check Y values don't reverse: once they start going down, they
        // must keep going down (no up-then-down S-curve).
        let mut prev_y = vertical_near_turn[0].y;
        let mut direction: Option<bool> = None; // true = going down
        for pt in &vertical_near_turn[1..] {
            let dy = pt.y - prev_y;
            if dy.abs() > 1.0 {
                let going_down = dy > 0.0;
                if let Some(was_down) = direction {
                    assert_eq!(
                        was_down, going_down,
                        "S-curve detected at turn: Y reversed direction at {:?} \
                         (vertical points: {:?})",
                        pt, vertical_near_turn
                    );
                }
                direction = Some(going_down);
            }
            prev_y = pt.y;
        }
    }
}
