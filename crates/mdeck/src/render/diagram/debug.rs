use std::fmt::{self, Write};

use super::parsing::parse_diagram;
use super::routes::reference_input;
use super::routing::types::{Route, RouteResult};
use super::types::*;

// ─── Debug info ──────────────────────────────────────────────────────────────

/// Generate a structured text summary of diagram nodes, edges, and routing results.
/// Used by the debug overlay to show routing engine inputs/outputs. The routes
/// are the ones `--check` computes: the reference layout with measured lane
/// capacities and the configured weights.
pub fn diagram_debug_info(content: &str) -> String {
    let (nodes, edges, _scale_directive) = parse_diagram(content);
    if nodes.is_empty() {
        return "No nodes parsed.".to_string();
    }
    let mut out = String::new();
    // Writing to a String cannot fail
    let _ = write_report(&mut out, &nodes, &edges);
    out
}

fn write_report(out: &mut String, nodes: &[DiagramNode], edges: &[DiagramEdge]) -> fmt::Result {
    let input = reference_input(nodes, edges);

    writeln!(out, "NODES ({}):", input.nodes().len())?;
    for node in input.nodes() {
        writeln!(out, "  {} @ ({},{})", node.name, node.col, node.row)?;
    }

    writeln!(out)?;
    writeln!(out, "EDGES ({}):", edges.len())?;
    for edge in edges {
        write!(out, "  {} {} {}", edge.from, edge.arrow.symbol(), edge.to)?;
        if !edge.label.is_empty() {
            write!(out, " \"{}\"", edge.label)?;
        }
        writeln!(out)?;
    }

    let config = input.config();
    writeln!(out)?;
    writeln!(
        out,
        "CONFIG: h_lanes={}, v_lanes={}",
        config.h_lane_capacity, config.v_lane_capacity
    )?;

    let routing_output = input.route();
    writeln!(out)?;
    writeln!(out, "ROUTING RESULTS:")?;
    for (edge, result) in &routing_output.results {
        let label = edge
            .label
            .as_deref()
            .map_or(String::new(), |l| format!(" \"{l}\""));
        write!(out, "  {} -> {}{}: ", edge.source, edge.target, label)?;
        match result {
            RouteResult::Success(route) => {
                writeln!(
                    out,
                    "OK len={:.1} turns={} lc={} cx={}",
                    route.complexity.length,
                    route.complexity.turns,
                    route.complexity.lane_changes,
                    route.complexity.crossings,
                )?;
                writeln!(out, "    {}", route_path(route))?;
            }
            RouteResult::Failure { warning } => writeln!(out, "FAIL: {warning}")?,
        }
    }
    Ok(())
}

/// A route's coordinates with each segment's lane between them:
/// `(1,1) L0 → (2,1)`.
fn route_path(route: &Route) -> String {
    let mut path = String::new();
    for (i, w) in route.waypoints.iter().enumerate() {
        if i > 0 {
            // Lane label goes between coordinates (on the segment)
            let prev = &route.waypoints[i - 1];
            let _ = write!(path, " L{} → ", prev.lane);
        }
        let _ = write!(path, "{}", w.coord);
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagram_debug_info_basic() {
        let content = "A (pos: 1,1)\nB (pos: 2,1)\nA -> B: link";
        let info = diagram_debug_info(content);
        assert!(info.contains("NODES (2):"));
        assert!(info.contains("A @ (1,1)"));
        assert!(info.contains("B @ (2,1)"));
        assert!(info.contains("EDGES (1):"));
        assert!(info.contains("A -> B \"link\""));
        assert!(info.contains("ROUTING RESULTS:"));
        assert!(info.contains("OK"));
    }

    #[test]
    fn test_diagram_debug_info_empty() {
        let info = diagram_debug_info("");
        assert_eq!(info, "No nodes parsed.");
    }

    #[test]
    fn test_diagram_debug_info_auto_layout() {
        let content = "A\nB\nC\nA -> B\nB -> C";
        let info = diagram_debug_info(content);
        // Auto-layout: 3 nodes → single row at (1,1), (2,1), (3,1)
        assert!(info.contains("NODES (3):"));
        assert!(info.contains("A @ (1,1)"));
        assert!(info.contains("B @ (2,1)"));
        assert!(info.contains("C @ (3,1)"));
        assert!(info.contains("EDGES (2):"));
        assert!(info.contains("ROUTING RESULTS:"));
    }

    #[test]
    fn debug_info_report_layout() {
        let info = diagram_debug_info("A (pos: 1,1)\nB (pos: 2,1)\nA --> B");
        let lines: Vec<&str> = info.lines().collect();
        assert_eq!(lines[..4], ["NODES (2):", "  A @ (1,1)", "  B @ (2,1)", ""]);
        assert_eq!(lines[4..6], ["EDGES (1):", "  A --> B"]);
        assert!(lines[7].starts_with("CONFIG: h_lanes="));
        assert_eq!(lines[9], "ROUTING RESULTS:");
        assert!(lines[10].starts_with("  A -> B: OK len="));
        assert!(lines[11].starts_with("    (1,1) L"), "{}", lines[11]);
    }

    #[test]
    fn auto_layout_wraps_past_five() {
        let info = diagram_debug_info("A\nB\nC\nD\nE\nF\nG\nA -> B");
        // 7 nodes: 3 columns
        assert!(info.contains("C @ (3,1)"), "{info}");
        assert!(info.contains("D @ (1,2)"), "{info}");
        assert!(info.contains("G @ (1,3)"), "{info}");
    }

    #[test]
    fn the_overlay_routes_what_check_routes() {
        // The overlay used the default three lanes per corridor while the
        // slide and `--check` measured them from the layout.
        let content = "A (pos: 1,1)\nB (pos: 3,1)\nC (pos: 2,2)\nA -> B\nA -> C";
        let (nodes, edges, _) = parse_diagram(content);
        let input = reference_input(&nodes, &edges);
        let config = input.config();
        let expected = format!(
            "CONFIG: h_lanes={}, v_lanes={}",
            config.h_lane_capacity, config.v_lane_capacity
        );
        assert!(diagram_debug_info(content).contains(&expected));
        assert_ne!(
            (config.h_lane_capacity, config.v_lane_capacity),
            (3, 3),
            "the example should measure something other than the default"
        );
    }
}
