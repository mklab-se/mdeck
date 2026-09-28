use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use rayon::prelude::*;

use super::graph::RoutingGraph;
use super::lanes::LaneOccupancy;
use super::types::{
    CostWeights, Direction, GridCoord, Lane, Route, RouteComplexity, SegmentId, Waypoint,
};

/// State key for the visited set: identifies a unique search state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct StateKey {
    coord: GridCoord,
    lane: Lane,
    last_direction: Direction,
}

/// A* search state.
#[derive(Debug, Clone)]
struct SearchState {
    coord: GridCoord,
    lane: Lane,
    last_direction: Direction,
    /// Weighted cost so far.
    g_cost: f64,
    /// Heuristic estimate to target.
    h_cost: f64,
    /// Breakdown of cost components for final route complexity.
    length_so_far: f64,
    turns_so_far: u32,
    lane_changes_so_far: u32,
    crossings_so_far: u32,
}

impl SearchState {
    fn f_cost(&self) -> f64 {
        self.g_cost + self.h_cost
    }

    fn key(&self) -> StateKey {
        StateKey {
            coord: self.coord,
            lane: self.lane,
            last_direction: self.last_direction,
        }
    }
}

/// Wrapper for the priority queue with deterministic ordering.
/// BinaryHeap is a max-heap, so we reverse the ordering (lowest cost = highest priority).
#[derive(Debug)]
struct PqEntry {
    f_cost: f64,
    g_cost: f64,
    coord: GridCoord,
    lane: Lane,
    direction: Direction,
    state: SearchState,
}

impl PartialEq for PqEntry {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for PqEntry {}

impl Ord for PqEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap behavior in BinaryHeap.
        // Lower f_cost is better → compare other.f_cost to self.f_cost.
        other
            .f_cost
            .partial_cmp(&self.f_cost)
            .unwrap_or(Ordering::Equal)
            .then_with(|| {
                // For same f_cost, prefer lower g_cost (explored less).
                other
                    .g_cost
                    .partial_cmp(&self.g_cost)
                    .unwrap_or(Ordering::Equal)
            })
            .then(other.coord.cmp(&self.coord))
            // Prefer lanes closer to center (smaller |lane|).
            .then_with(|| self.lane.abs().cmp(&other.lane.abs()).reverse())
            // Deterministic tie-break for same |lane|: prefer positive lane.
            .then(self.lane.cmp(&other.lane))
            .then(other.direction.cmp(&self.direction))
    }
}

impl PartialOrd for PqEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Heuristic: Manhattan distance in actual grid units (halved doubled-coords).
fn heuristic(from: GridCoord, to: GridCoord) -> f64 {
    from.manhattan_to(to) as f64 / 2.0
}

/// Length of one step in doubled coords, in grid units.
const STEP_LENGTH: f64 = 0.5;

/// What one A* run searches: the graph, the lanes already taken, the
/// endpoints and the cost weights.
struct SearchCx<'a> {
    graph: &'a RoutingGraph,
    occupancy: &'a LaneOccupancy,
    source: GridCoord,
    target: GridCoord,
    weights: &'a CostWeights,
}

/// The A* bookkeeping: the open set, the best cost found per state, and
/// each state's parent (with the lane it was reached on).
#[derive(Default)]
struct Frontier {
    open: BinaryHeap<PqEntry>,
    best_g: HashMap<StateKey, f64>,
    came_from: HashMap<StateKey, (StateKey, Lane)>,
}

impl Frontier {
    /// Put a start state in the open set, unconditionally.
    fn seed(&mut self, state: SearchState) {
        self.best_g.insert(state.key(), state.g_cost);
        self.push(state);
    }

    /// Record `state`, reached from `parent`, unless its state is already
    /// known at a cost no higher.
    fn relax(&mut self, state: SearchState, parent: (StateKey, Lane)) {
        let key = state.key();
        if let Some(&best) = self.best_g.get(&key)
            && state.g_cost >= best
        {
            return;
        }
        self.best_g.insert(key, state.g_cost);
        self.came_from.insert(key, parent);
        self.push(state);
    }

    /// Whether a cheaper path to `state` has been found since it was queued.
    fn is_stale(&self, state: &SearchState) -> bool {
        matches!(self.best_g.get(&state.key()), Some(&best) if state.g_cost > best)
    }

    fn push(&mut self, state: SearchState) {
        self.open.push(PqEntry {
            f_cost: state.f_cost(),
            g_cost: state.g_cost,
            coord: state.coord,
            lane: state.lane,
            direction: state.last_direction,
            state,
        });
    }
}

/// Whether `coord` is the centre of an occupied cell that is neither
/// endpoint, so a route may not pass through it.
fn blocks_route(
    graph: &RoutingGraph,
    coord: GridCoord,
    source: GridCoord,
    target: GridCoord,
) -> bool {
    coord.is_cell_center() && graph.is_occupied(&coord) && coord != source && coord != target
}

/// The cost so far `g` plus one step: its length, a turn, a lane change
/// (only when not turning) and each crossing, weighted.
fn step_g(g: f64, weights: &CostWeights, is_turn: bool, lane_changed: bool, crossings: u32) -> f64 {
    let turn_raw = if is_turn { 1.0 } else { 0.0 };
    let lane_change_raw = if lane_changed && !is_turn { 1.0 } else { 0.0 };
    g + weights.length * STEP_LENGTH
        + weights.turn * turn_raw
        + weights.lane_change * lane_change_raw
        + weights.crossing * crossings as f64
}

/// The state one step from `current` to `coord` in `dir` on `lane`, having
/// crossed `crossings` other routes.
fn advance(
    current: &SearchState,
    coord: GridCoord,
    lane: Lane,
    dir: Direction,
    crossings: u32,
    cx: &SearchCx,
) -> SearchState {
    let is_turn = current.last_direction.is_turn(dir);
    let lane_changed = lane != current.lane;
    SearchState {
        coord,
        lane,
        last_direction: dir,
        g_cost: step_g(current.g_cost, cx.weights, is_turn, lane_changed, crossings),
        h_cost: heuristic(coord, cx.target),
        length_so_far: current.length_so_far + STEP_LENGTH,
        turns_so_far: current.turns_so_far + if is_turn { 1 } else { 0 },
        lane_changes_so_far: current.lane_changes_so_far
            + if lane_changed && !is_turn { 1 } else { 0 },
        crossings_so_far: current.crossings_so_far + crossings,
    }
}

/// Queue every state one step from `current`: forward or sideways, never
/// back, never into an occupied cell, on each free lane of the segment.
fn expand(cx: &SearchCx, frontier: &mut Frontier, current: &SearchState) {
    // Leaving an occupied cell's centre is only OK from the source or target.
    if blocks_route(cx.graph, current.coord, cx.source, cx.target) {
        return;
    }
    let parent = (current.key(), current.lane);
    for &(neighbor, seg, dir) in cx.graph.neighbors(&current.coord) {
        if dir == current.last_direction.opposite()
            || blocks_route(cx.graph, neighbor, cx.source, cx.target)
        {
            continue;
        }
        let available = cx.occupancy.available_lanes(&seg, cx.graph.capacity(&seg));
        for &next_lane in &available {
            // Per-lane crossing detection: includes pass-through crossings
            // and turn conflicts (lane-dependent).
            let crossings = cx
                .occupancy
                .count_crossings(&seg, next_lane, &[cx.source, cx.target]);
            let next = advance(current, neighbor, next_lane, dir, crossings, cx);
            frontier.relax(next, parent);
        }
    }
}

/// Run A* search from `source` to `target` starting in direction `initial_dir`.
///
/// The search starts at the source cell center, steps one unit in `initial_dir` to the
/// adjacent junction, then explores the routing graph. The search terminates when
/// the target cell center is reached.
///
/// Returns `None` if no route is found.
fn astar_single_direction(
    graph: &RoutingGraph,
    occupancy: &LaneOccupancy,
    source: GridCoord,
    target: GridCoord,
    initial_dir: Direction,
    weights: &CostWeights,
) -> Option<Route> {
    // The first step: source center → adjacent junction in initial_dir.
    let first_junction = source.step(initial_dir);
    if !graph.contains(&first_junction) {
        return None;
    }
    let first_seg = SegmentId::new(source, first_junction);
    let first_lanes = occupancy.available_lanes(&first_seg, graph.capacity(&first_seg));
    if first_lanes.is_empty() {
        return None;
    }

    let cx = SearchCx {
        graph,
        occupancy,
        source,
        target,
        weights,
    };
    let mut frontier = Frontier::default();
    for &lane in &first_lanes {
        frontier.seed(SearchState {
            coord: first_junction,
            lane,
            last_direction: initial_dir,
            g_cost: weights.length * STEP_LENGTH,
            h_cost: heuristic(first_junction, target),
            length_so_far: STEP_LENGTH,
            turns_so_far: 0,
            lane_changes_so_far: 0,
            crossings_so_far: 0,
        });
    }

    while let Some(entry) = frontier.open.pop() {
        let current = entry.state;
        if frontier.is_stale(&current) {
            continue;
        }
        if current.coord == target {
            return Some(reconstruct_route(
                &frontier.came_from,
                current.key(),
                source,
                initial_dir,
                &first_lanes,
                &current,
            ));
        }
        expand(&cx, &mut frontier, &current);
    }

    None
}

/// Reconstruct the route from the A* search results.
fn reconstruct_route(
    came_from: &HashMap<StateKey, (StateKey, Lane)>,
    final_key: StateKey,
    source: GridCoord,
    _initial_dir: Direction,
    first_lanes: &[Lane],
    final_state: &SearchState,
) -> Route {
    // Trace back through came_from to build the path.
    let mut path_keys = vec![final_key];
    let mut current_key = final_key;
    while let Some(&(parent_key, _parent_lane)) = came_from.get(&current_key) {
        path_keys.push(parent_key);
        current_key = parent_key;
    }
    path_keys.reverse();

    // Build waypoints: source center, then each state's coord+lane.
    let mut waypoints = Vec::with_capacity(path_keys.len() + 1);

    // Source center waypoint: use the lane of the first segment.
    let first_lane = if let Some(first) = path_keys.first() {
        // The lane used on the first segment is the lane of the first state.
        first.lane
    } else if let Some(&l) = first_lanes.first() {
        l
    } else {
        0
    };

    waypoints.push(Waypoint {
        coord: source,
        lane: first_lane,
    });

    // Add each waypoint from the path.
    for (i, key) in path_keys.iter().enumerate() {
        let lane = if i + 1 < path_keys.len() {
            // Lane for the next segment: it's the lane of the next state.
            path_keys[i + 1].lane
        } else {
            // Last waypoint (target): lane is unused, use 0.
            0
        };
        waypoints.push(Waypoint {
            coord: key.coord,
            lane,
        });
    }

    let complexity = RouteComplexity {
        length: final_state.length_so_far,
        turns: final_state.turns_so_far,
        lane_changes: final_state.lane_changes_so_far,
        crossings: final_state.crossings_so_far,
    };

    Route {
        waypoints,
        complexity,
    }
}

/// Search for the best route from source to target, trying all 4 initial directions in parallel.
///
/// Returns the route with the lowest complexity, with deterministic tie-breaking.
pub fn find_best_route(
    graph: &RoutingGraph,
    occupancy: &LaneOccupancy,
    source: GridCoord,
    target: GridCoord,
    weights: &CostWeights,
) -> Option<Route> {
    // If source == target, return a trivial route.
    if source == target {
        return Some(Route {
            waypoints: vec![Waypoint {
                coord: source,
                lane: 0,
            }],
            complexity: RouteComplexity {
                length: 0.0,
                turns: 0,
                lane_changes: 0,
                crossings: 0,
            },
        });
    }

    // Launch 4 parallel A* searches, one per initial direction.
    let results: Vec<Option<Route>> = Direction::ALL
        .par_iter()
        .map(|&dir| astar_single_direction(graph, occupancy, source, target, dir, weights))
        .collect();

    // Collect successful results and pick the best using weighted comparison.
    let mut best: Option<Route> = None;
    for route in results.into_iter().flatten() {
        best = Some(match best {
            None => route,
            Some(current_best) => {
                let route_total = route.complexity.total(weights);
                let best_total = current_best.complexity.total(weights);
                if route_total < best_total {
                    route
                } else if (route_total - best_total).abs() < f64::EPSILON {
                    // Deterministic tie-break: compare waypoint sequences.
                    if route_tiebreak(&route) < route_tiebreak(&current_best) {
                        route
                    } else {
                        current_best
                    }
                } else {
                    current_best
                }
            }
        });
    }

    best
}

/// Generate a deterministic tie-breaking key for a route.
/// Returns a vector of (coord, lane) tuples that can be compared lexicographically.
fn route_tiebreak(route: &Route) -> Vec<(i32, i32, Lane)> {
    route
        .waypoints
        .iter()
        .map(|w| (w.coord.col2, w.coord.row2, w.lane))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn weights() -> CostWeights {
        CostWeights {
            length: 1.0,
            turn: 3.0,
            lane_change: 5.0,
            crossing: 7.0,
        }
    }

    fn state(coord: GridCoord, lane: Lane, dir: Direction, g: f64) -> SearchState {
        SearchState {
            coord,
            lane,
            last_direction: dir,
            g_cost: g,
            h_cost: 0.0,
            length_so_far: 1.0,
            turns_so_far: 1,
            lane_changes_so_far: 1,
            crossings_so_far: 1,
        }
    }

    #[test]
    fn a_step_costs_its_length_turns_lane_changes_and_crossings() {
        let w = weights();
        assert_eq!(step_g(2.0, &w, false, false, 0), 2.5);
        assert_eq!(step_g(2.0, &w, true, false, 0), 5.5);
        assert_eq!(step_g(2.0, &w, false, true, 0), 7.5);
        // a lane change while turning is part of the turn
        assert_eq!(step_g(2.0, &w, true, true, 0), 5.5);
        assert_eq!(step_g(2.0, &w, false, false, 2), 16.5);
    }

    #[test]
    fn advancing_counts_what_the_step_did() {
        let graph = RoutingGraph::build(&[(0, 0), (2, 0)], 3, 3);
        let occupancy = LaneOccupancy::new();
        let w = weights();
        let cx = SearchCx {
            graph: &graph,
            occupancy: &occupancy,
            source: GridCoord::from_int(0, 0),
            target: GridCoord::from_int(2, 0),
            weights: &w,
        };
        let from = state(GridCoord::from_grid(0.5, 0.0), 0, Direction::East, 1.0);
        let to = GridCoord::from_grid(0.5, 0.5);
        let next = advance(&from, to, 1, Direction::South, 2, &cx);
        assert_eq!(next.coord, to);
        assert_eq!(next.last_direction, Direction::South);
        assert_eq!(next.g_cost, step_g(1.0, &w, true, true, 2));
        assert_eq!(next.h_cost, heuristic(to, cx.target));
        assert_eq!(next.length_so_far, 1.5);
        assert_eq!(next.turns_so_far, 2);
        assert_eq!(next.lane_changes_so_far, 1);
        assert_eq!(next.crossings_so_far, 3);

        let straight = advance(&from, GridCoord::from_int(1, 0), 1, Direction::East, 0, &cx);
        assert_eq!(straight.turns_so_far, 1);
        assert_eq!(straight.lane_changes_so_far, 2);
    }

    #[test]
    fn only_occupied_cells_other_than_the_endpoints_block() {
        let graph = RoutingGraph::build(&[(0, 0), (1, 0), (2, 0)], 3, 3);
        let (a, b) = (GridCoord::from_int(0, 0), GridCoord::from_int(2, 0));
        assert!(!blocks_route(&graph, a, a, b));
        assert!(!blocks_route(&graph, b, a, b));
        assert!(blocks_route(&graph, GridCoord::from_int(1, 0), a, b));
        // junctions and empty cells never block
        assert!(!blocks_route(&graph, GridCoord::from_grid(0.5, 0.5), a, b));
        assert!(!blocks_route(&graph, GridCoord::from_int(1, 1), a, b));
    }

    #[test]
    fn the_frontier_keeps_only_cheaper_paths() {
        let mut f = Frontier::default();
        let at = GridCoord::from_grid(0.5, 0.0);
        let parent = (state(at, 0, Direction::North, 0.0).key(), 0);
        f.seed(state(at, 0, Direction::East, 2.0));
        // an equal or dearer path to the same state is dropped
        f.relax(state(at, 0, Direction::East, 2.0), parent);
        f.relax(state(at, 0, Direction::East, 3.0), parent);
        assert_eq!(f.open.len(), 1);
        assert!(f.came_from.is_empty());
        // a cheaper one replaces it and makes the queued one stale
        f.relax(state(at, 0, Direction::East, 1.0), parent);
        assert_eq!(f.open.len(), 2);
        assert_eq!(f.came_from.len(), 1);
        assert!(f.is_stale(&state(at, 0, Direction::East, 2.0)));
        assert!(!f.is_stale(&state(at, 0, Direction::East, 1.0)));
        // the cheapest pops first
        assert_eq!(f.open.pop().unwrap().g_cost, 1.0);
    }
}
