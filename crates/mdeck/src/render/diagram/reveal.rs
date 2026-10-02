use std::time::Instant;

use super::types::*;

// ─── Reveal steps ────────────────────────────────────────────────────────────

/// Seconds an edge takes to grow in on the step that reveals it.
const EDGE_GROW_SECONDS: f32 = 0.4;

/// Count the number of reveal steps (`+` markers) in a diagram content string.
pub fn count_diagram_steps(content: &str) -> usize {
    let mut count = 0;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with("+ ") {
            count += 1;
        }
    }
    count
}

/// The reveal step of every node and edge: static elements are always visible
/// (step 0), each `+` increments the step counter and `*` shares the step of
/// the previous `+`. Elements are walked in file order (by `parse_order`) so
/// that interleaved nodes and edges get steps matching their source ordering.
pub(super) fn reveal_steps(
    nodes: &[DiagramNode],
    edges: &[DiagramEdge],
) -> (Vec<usize>, Vec<usize>) {
    #[derive(Clone, Copy)]
    enum ElementRef {
        Node(usize),
        Edge(usize),
    }
    let mut all_elements: Vec<ElementRef> = (0..nodes.len()).map(ElementRef::Node).collect();
    all_elements.extend((0..edges.len()).map(ElementRef::Edge));
    all_elements.sort_by_key(|e| match e {
        ElementRef::Node(i) => nodes[*i].parse_order,
        ElementRef::Edge(i) => edges[*i].parse_order,
    });

    let mut step_counter = 0usize;
    let mut node_steps = vec![0usize; nodes.len()];
    let mut edge_steps = vec![0usize; edges.len()];
    for elem in &all_elements {
        let (reveal, target, idx) = match elem {
            ElementRef::Node(i) => (nodes[*i].reveal, &mut node_steps, *i),
            ElementRef::Edge(i) => (edges[*i].reveal, &mut edge_steps, *i),
        };
        target[idx] = match reveal {
            DiagramReveal::Static => 0,
            DiagramReveal::NextStep => {
                step_counter += 1;
                step_counter
            }
        };
    }
    (node_steps, edge_steps)
}

/// How far an edge revealed on `edge_step` has grown, 0.0 to 1.0, and whether
/// it is still growing (so the caller keeps repainting). Only edges revealed on
/// the current step animate; earlier ones and static ones are complete.
pub(super) fn edge_progress(
    edge_step: usize,
    reveal_step: usize,
    reveal_timestamp: Option<Instant>,
) -> (f32, bool) {
    if edge_step != reveal_step || edge_step == 0 {
        return (1.0, false);
    }
    match reveal_timestamp {
        Some(ts) => grow_progress(ts.elapsed().as_secs_f32()),
        None => (1.0, false),
    }
}

/// Eased progress `elapsed` seconds into an edge's growth, and whether it is
/// still under way.
fn grow_progress(elapsed: f32) -> (f32, bool) {
    let t = (elapsed / EDGE_GROW_SECONDS).min(1.0);
    let eased = if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0_f32 * t + 2.0).powi(2) / 2.0
    };
    (eased, t < 1.0)
}

#[cfg(test)]
mod tests {
    use super::super::parsing::parse_diagram;
    use super::*;

    #[test]
    fn test_count_diagram_steps() {
        let content = "- A\n+ B\n+ C\n* D";
        assert_eq!(count_diagram_steps(content), 2);
    }

    #[test]
    fn test_count_diagram_steps_none() {
        let content = "- A\n- B\n- C";
        assert_eq!(count_diagram_steps(content), 0);
    }

    #[test]
    fn test_count_diagram_steps_skips_comments() {
        assert_eq!(count_diagram_steps("+ A\n# + not a step"), 1);
    }

    #[test]
    fn test_reveal_steps_interleaved_file_order() {
        // Reproduces the Pipeline Growth diagram where nodes and edges
        // are interleaved. Steps must follow file order, not nodes-then-edges.
        let content = "\
- Source (icon: storage, pos: 1,1)
+ Build  (icon: container, pos: 2,1)
+ Source -> Build: triggers
+ Test   (icon: function, pos: 3,1)
+ Build -> Test: on success
+ Deploy (icon: cloud, pos: 4,1)
+ Test -> Deploy: all green";

        let (nodes, edges, _) = parse_diagram(content);
        assert_eq!(nodes.len(), 4); // Source, Build, Test, Deploy
        assert_eq!(edges.len(), 3); // Source->Build, Build->Test, Test->Deploy

        let (node_steps, edge_steps) = reveal_steps(&nodes, &edges);

        // Source is static (step 0)
        assert_eq!(node_steps[0], 0, "Source should be step 0");
        // + Build → step 1
        assert_eq!(node_steps[1], 1, "Build should be step 1");
        // + Source -> Build → step 2
        assert_eq!(edge_steps[0], 2, "Source->Build should be step 2");
        // + Test → step 3
        assert_eq!(node_steps[2], 3, "Test should be step 3");
        // + Build -> Test → step 4
        assert_eq!(edge_steps[1], 4, "Build->Test should be step 4");
        // + Deploy → step 5
        assert_eq!(node_steps[3], 5, "Deploy should be step 5");
        // + Test -> Deploy → step 6
        assert_eq!(edge_steps[2], 6, "Test->Deploy should be step 6");
    }

    #[test]
    fn test_reveal_steps_incremental_build() {
        // The "Incremental Build" diagram from test-diagram.md
        let content = "\
- Server (icon: server, pos: 1,1)
- DB     (icon: database, pos: 2,1)
- Server -> DB: queries

+ Cache (icon: cache, pos: 1,2)
+ Server -> Cache: reads
+ Cache -> DB: fills

+ Monitor (icon: monitor, pos: 2,2)
  - Monitor -- Server: observes
  - Monitor -- DB: observes";

        let (nodes, edges, _) = parse_diagram(content);
        let (node_steps, edge_steps) = reveal_steps(&nodes, &edges);

        // Static elements: step 0
        assert_eq!(node_steps[0], 0, "Server = 0");
        assert_eq!(node_steps[1], 0, "DB = 0");
        assert_eq!(edge_steps[0], 0, "Server->DB = 0");

        // + Cache → step 1
        assert_eq!(node_steps[2], 1, "Cache = 1");
        // + Server -> Cache → step 2
        assert_eq!(edge_steps[1], 2, "Server->Cache = 2");
        // + Cache -> DB → step 3
        assert_eq!(edge_steps[2], 3, "Cache->DB = 3");

        // + Monitor → step 4
        assert_eq!(node_steps[3], 4, "Monitor = 4");
        // `*` (and `-`) items are static: they show from the start
        assert_eq!(edge_steps[3], 0, "Monitor--Server = 0");
        assert_eq!(edge_steps[4], 0, "Monitor--DB = 0");
    }

    #[test]
    fn edge_progress_only_animates_the_current_step() {
        let now = Some(Instant::now());
        // Static and earlier edges are complete
        assert_eq!(edge_progress(0, 0, now), (1.0, false));
        assert_eq!(edge_progress(1, 2, now), (1.0, false));
        // Without a timestamp there is nothing to animate against
        assert_eq!(edge_progress(2, 2, None), (1.0, false));
        // An edge revealed just now is still growing
        let (progress, growing) = edge_progress(2, 2, now);
        assert!(growing);
        assert!(progress < 0.5);
    }

    #[test]
    fn grow_progress_eases_in_and_out() {
        assert_eq!(grow_progress(0.0), (0.0, true));
        let (mid, growing) = grow_progress(EDGE_GROW_SECONDS / 2.0);
        assert!((mid - 0.5).abs() < 1e-6);
        assert!(growing);
        let (quarter, _) = grow_progress(EDGE_GROW_SECONDS / 4.0);
        assert!((quarter - 0.125).abs() < 1e-6);
        assert_eq!(grow_progress(EDGE_GROW_SECONDS), (1.0, false));
        assert_eq!(grow_progress(10.0), (1.0, false));
    }
}
