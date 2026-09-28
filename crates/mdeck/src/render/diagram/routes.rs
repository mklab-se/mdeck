use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex, mpsc};

use eframe::egui::{self, Pos2};

use super::layout::layout_nodes;
use super::parsing::parse_diagram;
use super::routing;
use super::routing::types::{CostWeights, RouteResult, RoutingConfig, RoutingOutput};
use super::scene::node_rects;
use super::types::*;
use crate::check::{CheckCategory, CheckReport, CheckWarning};

// ─── Routing weights ─────────────────────────────────────────────────────────

/// Routing weights loaded once from config at startup.
static ROUTING_WEIGHTS: LazyLock<CostWeights> = LazyLock::new(|| {
    crate::config::Config::load_or_default()
        .routing
        .unwrap_or_default()
        .to_cost_weights()
});

/// The routing weights of `~/.config/mdeck/config.yaml` (`routing:`), read
/// on first use and kept for the life of the process.
///
/// This is the one place the diagram renderer reads configuration. The public
/// entry points (`draw_diagram_sized`, `check_diagram_routes`) are called from
/// the renderer and the checker without a config in hand, so they fetch the
/// weights here and pass them down; everything below them takes the weights
/// as a value (`RoutingInput::new`, `RoutingConfig`), so routing stays pure
/// and testable.
pub(super) fn configured_weights() -> CostWeights {
    *ROUTING_WEIGHTS
}

// ─── Route cache ─────────────────────────────────────────────────────────────

// Global cache for routing results. Routing is expensive (A* search with rayon
// parallelism per edge) and the inputs rarely change between frames. Using a
// global Mutex instead of thread_local allows background threads to pre-populate
// the cache that the render thread later reads.
static ROUTE_CACHE: LazyLock<Mutex<HashMap<u64, RoutingOutput>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Every distinct (diagram, size, step) triple gets an entry, so resizes and
/// reveals would grow the cache without bound. Past this it is simply reset.
const ROUTE_CACHE_CAP: usize = 256;

fn route_cache() -> std::sync::MutexGuard<'static, HashMap<u64, RoutingOutput>> {
    // A panic while holding the lock must not poison every later frame
    ROUTE_CACHE.lock().unwrap_or_else(|e| e.into_inner())
}

/// Clear all cached routes (call on file reload).
pub fn clear_route_cache() {
    route_cache().clear();
}

/// Look up or compute the routing for `cache_key`, keeping the cache bounded.
fn cached_routes(cache_key: u64, compute: impl FnOnce() -> RoutingOutput) -> RoutingOutput {
    let mut cache = route_cache();
    if let Some(output) = cache.get(&cache_key) {
        return output.clone();
    }
    let output = compute();
    if cache.len() >= ROUTE_CACHE_CAP {
        cache.clear();
    }
    cache.insert(cache_key, output.clone());
    output
}

// ─── Routing input ───────────────────────────────────────────────────────────

/// Whether an edge can be routed: not a self-loop, both ends on the diagram.
pub(super) fn is_routable(edge: &DiagramEdge, rects: &HashMap<String, egui::Rect>) -> bool {
    edge.from != edge.to && rects.contains_key(&edge.from) && rects.contains_key(&edge.to)
}

/// What the routing engine sees of a laid-out diagram.
pub(super) struct RoutingInput {
    nodes: Vec<routing::types::DiagramNode>,
    edges: Vec<routing::types::DiagramEdge>,
    config: RoutingConfig,
}

impl RoutingInput {
    /// Place every node in its grid cell (1-based, as the router counts) and
    /// size the corridors from the gaps between nodes. `edges` should already
    /// be filtered to the routable, visible ones.
    pub(super) fn new<'e>(
        nodes: &[DiagramNode],
        edges: impl IntoIterator<Item = &'e DiagramEdge>,
        grid: &GridInfo,
        rects: &HashMap<String, egui::Rect>,
        lane_spacing: f32,
        weights: CostWeights,
    ) -> Self {
        let nodes = nodes
            .iter()
            .filter_map(|n| {
                let rect = rects.get(&n.name)?;
                let (col, row) = grid.cell_at(rect.center())?;
                Some(routing::types::DiagramNode {
                    name: n.name.clone(),
                    col: (col + 1) as i32, // convert 0-indexed to 1-based
                    row: (row + 1) as i32,
                })
            })
            .collect();
        let edges = edges.into_iter().map(routing_edge).collect();
        let config = RoutingConfig {
            h_lane_capacity: compute_h_capacity(grid, rects, lane_spacing),
            v_lane_capacity: compute_v_capacity(grid, rects, lane_spacing),
            weights,
        };
        Self {
            nodes,
            edges,
            config,
        }
    }

    /// Route every edge, from the cache when these inputs were routed before.
    /// The nodes as the router places them (1-based cells).
    pub(super) fn nodes(&self) -> &[routing::types::DiagramNode] {
        &self.nodes
    }

    pub(super) fn config(&self) -> &RoutingConfig {
        &self.config
    }

    pub(super) fn route(&self) -> RoutingOutput {
        cached_routes(self.cache_key(), || {
            routing::route_all_edges(&self.nodes, &self.edges, &self.config)
        })
    }

    /// Compute a hash key for the routing inputs.
    fn cache_key(&self) -> u64 {
        let mut hasher = std::hash::DefaultHasher::new();
        for n in &self.nodes {
            n.name.hash(&mut hasher);
            n.col.hash(&mut hasher);
            n.row.hash(&mut hasher);
        }
        for e in &self.edges {
            e.source.hash(&mut hasher);
            e.target.hash(&mut hasher);
            e.label.hash(&mut hasher);
        }
        let config = &self.config;
        config.h_lane_capacity.hash(&mut hasher);
        config.v_lane_capacity.hash(&mut hasher);
        config.weights.length.to_bits().hash(&mut hasher);
        config.weights.turn.to_bits().hash(&mut hasher);
        config.weights.lane_change.to_bits().hash(&mut hasher);
        config.weights.crossing.to_bits().hash(&mut hasher);
        hasher.finish()
    }
}

/// The router's view of an edge: its ends and optional label.
pub(super) fn routing_edge(edge: &DiagramEdge) -> routing::types::DiagramEdge {
    routing::types::DiagramEdge {
        source: edge.from.clone(),
        target: edge.to.clone(),
        label: if edge.label.is_empty() {
            None
        } else {
            Some(edge.label.clone())
        },
    }
}

/// Compute lane capacity for horizontal corridors (edges travel left/right).
/// The gap available is the vertical space between node edges.
pub(super) fn compute_h_capacity(
    grid: &GridInfo,
    node_rects: &HashMap<String, egui::Rect>,
    lane_spacing: f32,
) -> i32 {
    lane_capacity(grid, node_rects, lane_spacing, |rect| {
        grid.cell_h - rect.height()
    })
}

/// Compute lane capacity for vertical corridors (edges travel up/down).
/// The gap available is the horizontal space between node edges.
pub(super) fn compute_v_capacity(
    grid: &GridInfo,
    node_rects: &HashMap<String, egui::Rect>,
    lane_spacing: f32,
) -> i32 {
    lane_capacity(grid, node_rects, lane_spacing, |rect| {
        grid.cell_w - rect.width()
    })
}

/// How many lanes fit in the narrowest `gap` beside any node, at least one;
/// three when there is nothing to measure.
fn lane_capacity(
    grid: &GridInfo,
    node_rects: &HashMap<String, egui::Rect>,
    lane_spacing: f32,
    gap: impl Fn(&egui::Rect) -> f32,
) -> i32 {
    let min_gap = (0..grid.rows)
        .flat_map(|row| (0..grid.cols).map(move |col| (col, row)))
        .filter_map(|(col, row)| find_rect_at(grid, node_rects, col, row))
        .map(|rect| gap(&rect))
        .reduce(f32::min);
    let Some(min_gap) = min_gap.filter(|g| g.is_finite() && *g > 0.0) else {
        return 3; // nothing measured: a sensible default
    };
    if lane_spacing <= 0.0 {
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

// ─── Route checks ────────────────────────────────────────────────────────────

/// Check a single diagram's routes and return any failure warning strings.
/// Also populates the route cache as a side effect.
pub fn check_diagram_routes(content: &str) -> Vec<String> {
    let (nodes, edges, _scale_directive) = parse_diagram(content);
    if nodes.is_empty() || edges.is_empty() {
        return Vec::new();
    }
    reference_input(&nodes, &edges)
        .route()
        .results
        .iter()
        .filter_map(|(_edge, result)| match result {
            RouteResult::Failure { warning } => Some(warning.clone()),
            RouteResult::Success(_) => None,
        })
        .collect()
}

/// The routing input of a diagram on a full-width slide at the default
/// diagram height, unscaled, with the configured weights: what `--check`
/// routes and the debug overlay reports.
pub(super) fn reference_input(nodes: &[DiagramNode], edges: &[DiagramEdge]) -> RoutingInput {
    let scale = 1.0_f32;
    let max_width = 1920.0_f32;
    let diagram_height = 500.0 * scale;
    let padding = 30.0 * scale;
    let area_width = max_width - padding * 2.0;
    let area_height = diagram_height - padding * 2.0;
    let lane_spacing = EdgeMetrics::new(scale).lane_spacing;

    let (layouts, grid) = layout_nodes(nodes, area_width, area_height, 0.0, 0.0, scale);
    let rects = node_rects(nodes, &layouts, Pos2::ZERO);
    let routable = edges.iter().filter(|e| is_routable(e, &rects));
    RoutingInput::new(
        nodes,
        routable,
        &grid,
        &rects,
        lane_spacing,
        configured_weights(),
    )
}

/// Pre-compute routes for all diagrams and collect a `CheckReport` with any warnings.
/// The `cancel` flag is checked before each computation; set it to `true` to abort
/// early (e.g. on file reload). Routing already uses rayon internally, so a single
/// background thread is sufficient to saturate cores.
///
/// `diagrams` is a list of `(1-indexed slide number, diagram content)`.
pub fn precache_all_diagrams_with_report(
    diagrams: Vec<(usize, String)>,
    cancel: Arc<AtomicBool>,
) -> mpsc::Receiver<CheckReport> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut report = CheckReport::new();
        for (slide_num, content) in &diagrams {
            if cancel.load(Ordering::Relaxed) {
                let _ = tx.send(report);
                return;
            }
            for warning_msg in check_diagram_routes(content) {
                report.add(CheckWarning {
                    slide: *slide_num,
                    category: CheckCategory::DiagramRouting,
                    message: warning_msg,
                });
            }
        }
        let _ = tx.send(report);
    });
    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(center: (f32, f32)) -> egui::Rect {
        egui::Rect::from_center_size(Pos2::new(center.0, center.1), egui::vec2(130.0, 90.0))
    }

    fn two_by_two(occupied: &[(usize, usize)]) -> GridInfo {
        GridInfo {
            cols: 2,
            rows: 2,
            cell_w: 200.0,
            cell_h: 150.0,
            origin_x: 0.0,
            origin_y: 0.0,
            occupied: occupied.iter().copied().collect(),
        }
    }

    #[test]
    fn test_compute_h_capacity() {
        let grid = two_by_two(&[(0, 0), (1, 0), (0, 1), (1, 1)]);
        let rects = HashMap::from([
            ("A".into(), rect((100.0, 75.0))),
            ("B".into(), rect((300.0, 75.0))),
            ("C".into(), rect((100.0, 225.0))),
            ("D".into(), rect((300.0, 225.0))),
        ]);
        let cap = compute_h_capacity(&grid, &rects, 20.0);
        // cell_h=150, node_h=90, gap=60, 60/20 = 3
        assert_eq!(cap, 3);
    }

    #[test]
    fn test_compute_v_capacity() {
        let grid = two_by_two(&[(0, 0), (1, 0)]);
        let rects = HashMap::from([
            ("A".into(), rect((100.0, 75.0))),
            ("B".into(), rect((300.0, 75.0))),
        ]);
        let cap = compute_v_capacity(&grid, &rects, 20.0);
        // cell_w=200, node_w=130, gap=70, 70/20 = 3
        assert_eq!(cap, 3);
    }

    #[test]
    fn capacity_defaults_without_lane_spacing() {
        let grid = two_by_two(&[(0, 0)]);
        let rects = HashMap::from([("A".into(), rect((100.0, 75.0)))]);
        assert_eq!(compute_v_capacity(&grid, &rects, 0.0), 3);
        // A gap narrower than one lane still leaves one
        assert_eq!(compute_v_capacity(&grid, &rects, 100.0), 1);
    }

    #[test]
    fn capacity_defaults_when_no_node_is_measured() {
        // Occupied cells whose nodes have no rect: nothing to measure a gap
        // from, so the default applies (it used to be i32::MAX lanes).
        let grid = two_by_two(&[(0, 0), (1, 1)]);
        let rects = HashMap::new();
        assert_eq!(compute_h_capacity(&grid, &rects, 20.0), 3);
        assert_eq!(compute_v_capacity(&grid, &rects, 20.0), 3);
    }

    #[test]
    fn routable_edges_skip_self_loops_and_unknown_nodes() {
        let (_, edges, _) = parse_diagram("- A -> B\n- A -> A\n- A -> C");
        let rects = HashMap::from([
            ("A".into(), rect((100.0, 75.0))),
            ("B".into(), rect((300.0, 75.0))),
        ]);
        let routable: Vec<bool> = edges.iter().map(|e| is_routable(e, &rects)).collect();
        assert_eq!(routable, [true, false, false]);
    }

    #[test]
    fn routing_edge_drops_empty_labels() {
        let (_, edges, _) = parse_diagram("- A -> B\n- B -> C: calls");
        assert_eq!(routing_edge(&edges[0]).label, None);
        assert_eq!(routing_edge(&edges[1]).label.as_deref(), Some("calls"));
        assert_eq!(routing_edge(&edges[1]).source, "B");
    }

    #[test]
    fn cache_key_follows_weights() {
        let (nodes, edges, _) = parse_diagram("A (pos: 1,1)\nB (pos: 2,1)\nA -> B");
        let (layouts, grid) = layout_nodes(&nodes, 800.0, 400.0, 0.0, 0.0, 1.0);
        let rects = node_rects(&nodes, &layouts, Pos2::ZERO);
        let input = |weights| RoutingInput::new(&nodes, &edges, &grid, &rects, 20.0, weights);
        let default = CostWeights::default();
        let heavier = CostWeights {
            turn: default.turn + 1.0,
            ..default
        };
        assert_eq!(input(default).cache_key(), input(default).cache_key());
        assert_ne!(input(default).cache_key(), input(heavier).cache_key());
        assert_eq!(input(default).nodes[1].col, 2);
    }

    #[test]
    fn check_reports_no_warnings_for_a_simple_chain() {
        assert!(check_diagram_routes("- A -> B\n- B -> C").is_empty());
        assert!(check_diagram_routes("").is_empty());
    }
}
