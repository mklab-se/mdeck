mod edges;
mod icons;
mod layout;
mod parsing;
pub mod routing;
mod types;

use edges::*;
use icons::*;
use layout::*;
use parsing::*;
use types::*;

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex, mpsc};
use std::time::Instant;

use crate::check::{CheckCategory, CheckReport, CheckWarning};

use crate::render::image_cache::ImageCache;
use crate::theme::Theme;
use eframe::egui::{self, Color32, FontId, Pos2, Stroke};

// ─── Route cache ────────────────────────────────────────────────────────────

// Global cache for routing results. Routing is expensive (A* search with rayon
// parallelism per edge) and the inputs rarely change between frames. Using a
// global Mutex instead of thread_local allows background threads to pre-populate
// the cache that the render thread later reads.
static ROUTE_CACHE: LazyLock<Mutex<HashMap<u64, routing::types::RoutingOutput>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Every distinct (diagram, size, step) triple gets an entry, so resizes and
/// reveals would grow the cache without bound. Past this it is simply reset.
const ROUTE_CACHE_CAP: usize = 256;

fn route_cache() -> std::sync::MutexGuard<'static, HashMap<u64, routing::types::RoutingOutput>> {
    // A panic while holding the lock must not poison every later frame
    ROUTE_CACHE.lock().unwrap_or_else(|e| e.into_inner())
}

/// Clear all cached routes (call on file reload).
pub fn clear_route_cache() {
    route_cache().clear();
}

/// Look up or compute the routing for `cache_key`, keeping the cache bounded.
fn cached_routes(
    cache_key: u64,
    compute: impl FnOnce() -> routing::types::RoutingOutput,
) -> routing::types::RoutingOutput {
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

/// Check a single diagram's routes and return any failure warning strings.
/// Also populates the route cache as a side effect.
pub fn check_diagram_routes(content: &str) -> Vec<String> {
    let (nodes, edges, _scale_directive) = parse_diagram(content);
    if nodes.is_empty() || edges.is_empty() {
        return Vec::new();
    }

    // Reuse the same layout/routing setup as precache_diagram_routes
    let scale = 1.0_f32;
    let max_width = 1920.0_f32;
    let diagram_height = 500.0 * scale;
    let padding = 30.0 * scale;
    let area_width = max_width - padding * 2.0;
    let area_height = diagram_height - padding * 2.0;
    let lane_spacing = 20.0 * scale;

    let (layouts, grid) = layout_nodes(&nodes, area_width, area_height, 0.0, 0.0, scale);

    let mut node_rects: HashMap<String, egui::Rect> = HashMap::new();
    for (i, node) in nodes.iter().enumerate() {
        let layout = &layouts[i];
        let node_rect = egui::Rect::from_center_size(
            egui::pos2(layout.center_x, layout.center_y),
            egui::vec2(layout.width, layout.height),
        );
        node_rects.insert(node.name.clone(), node_rect);
    }

    let routing_nodes: Vec<routing::types::DiagramNode> = nodes
        .iter()
        .filter_map(|n| {
            let rect = node_rects.get(&n.name)?;
            let center = rect.center();
            let (col, row) = grid.cell_at(center)?;
            Some(routing::types::DiagramNode {
                name: n.name.clone(),
                col: (col + 1) as i32,
                row: (row + 1) as i32,
            })
        })
        .collect();

    let routing_edges: Vec<routing::types::DiagramEdge> = edges
        .iter()
        .filter(|e| {
            e.from != e.to && node_rects.contains_key(&e.from) && node_rects.contains_key(&e.to)
        })
        .map(|edge| routing::types::DiagramEdge {
            source: edge.from.clone(),
            target: edge.to.clone(),
            label: if edge.label.is_empty() {
                None
            } else {
                Some(edge.label.clone())
            },
        })
        .collect();

    let config = routing::types::RoutingConfig {
        h_lane_capacity: compute_h_capacity(&grid, &node_rects, lane_spacing),
        v_lane_capacity: compute_v_capacity(&grid, &node_rects, lane_spacing),
        weights: *ROUTING_WEIGHTS,
    };

    let cache_key = route_cache_key(&routing_nodes, &routing_edges, &config);
    let output = cached_routes(cache_key, || {
        routing::route_all_edges(&routing_nodes, &routing_edges, &config)
    });

    output
        .results
        .iter()
        .filter_map(|(_edge, result)| {
            if let routing::types::RouteResult::Failure { warning } = result {
                Some(warning.clone())
            } else {
                None
            }
        })
        .collect()
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

/// Routing weights loaded once from config at startup.
static ROUTING_WEIGHTS: LazyLock<routing::types::CostWeights> = LazyLock::new(|| {
    crate::config::Config::load_or_default()
        .routing
        .unwrap_or_default()
        .to_cost_weights()
});

/// Compute a hash key for the routing inputs.
fn route_cache_key(
    nodes: &[routing::types::DiagramNode],
    edges: &[routing::types::DiagramEdge],
    config: &routing::types::RoutingConfig,
) -> u64 {
    let mut hasher = std::hash::DefaultHasher::new();
    for n in nodes {
        n.name.hash(&mut hasher);
        n.col.hash(&mut hasher);
        n.row.hash(&mut hasher);
    }
    for e in edges {
        e.source.hash(&mut hasher);
        e.target.hash(&mut hasher);
        e.label.hash(&mut hasher);
    }
    config.h_lane_capacity.hash(&mut hasher);
    config.v_lane_capacity.hash(&mut hasher);
    config.weights.length.to_bits().hash(&mut hasher);
    config.weights.turn.to_bits().hash(&mut hasher);
    config.weights.lane_change.to_bits().hash(&mut hasher);
    config.weights.crossing.to_bits().hash(&mut hasher);
    hasher.finish()
}

// ─── Debug info ──────────────────────────────────────────────────────────────

/// Generate a structured text summary of diagram nodes, edges, and routing results.
/// Used by the debug overlay to show routing engine inputs/outputs.
pub fn diagram_debug_info(content: &str) -> String {
    use std::fmt::Write;

    let (nodes, edges, _scale_directive) = parse_diagram(content);
    if nodes.is_empty() {
        return "No nodes parsed.".to_string();
    }

    // Determine grid positions using the same logic as layout_nodes:
    // if any node has grid_pos, use explicit placement; otherwise auto-layout.
    let has_grid = nodes.iter().any(|n| n.grid_pos.is_some());

    // Assign (col, row) to each node (1-based, matching routing convention)
    let positions: Vec<(i32, i32)> = if has_grid {
        let mut max_col: u32 = 1;
        let mut max_row: u32 = 1;
        for node in &nodes {
            if let Some((c, r)) = node.grid_pos {
                max_col = max_col.max(c);
                max_row = max_row.max(r);
            }
        }
        let mut occupied: Vec<(u32, u32)> = nodes.iter().filter_map(|n| n.grid_pos).collect();
        let mut next_unplaced = 0u32;
        nodes
            .iter()
            .map(|node| {
                let (c, r) = node.grid_pos.unwrap_or_else(|| {
                    loop {
                        let c = next_unplaced % max_col + 1;
                        let r = next_unplaced / max_col + 1;
                        next_unplaced += 1;
                        if !occupied.contains(&(c, r)) {
                            occupied.push((c, r));
                            return (c, r);
                        }
                    }
                });
                (c as i32, r as i32)
            })
            .collect()
    } else {
        let n = nodes.len();
        if n <= 5 {
            // Single row
            (0..n).map(|i| (i as i32 + 1, 1)).collect()
        } else {
            // Grid
            let cols = ((n as f32).sqrt().ceil() as usize).max(2);
            (0..n)
                .map(|i| ((i % cols) as i32 + 1, (i / cols) as i32 + 1))
                .collect()
        }
    };

    let mut out = String::new();

    // NODES section
    writeln!(out, "NODES ({}):", nodes.len()).unwrap();
    for (i, node) in nodes.iter().enumerate() {
        let (c, r) = positions[i];
        writeln!(out, "  {} @ ({},{})", node.name, c, r).unwrap();
    }

    // EDGES section
    writeln!(out).unwrap();
    writeln!(out, "EDGES ({}):", edges.len()).unwrap();
    for edge in &edges {
        let arrow_str = match edge.arrow {
            ArrowKind::Forward => "->",
            ArrowKind::Reverse => "<-",
            ArrowKind::Bidirectional => "<->",
            ArrowKind::DashedLine => "--",
            ArrowKind::DashedArrow => "-->",
        };
        if edge.label.is_empty() {
            writeln!(out, "  {} {} {}", edge.from, arrow_str, edge.to).unwrap();
        } else {
            writeln!(
                out,
                "  {} {} {} \"{}\"",
                edge.from, arrow_str, edge.to, edge.label
            )
            .unwrap();
        }
    }

    // Build routing types and run the routing engine
    let routing_nodes: Vec<routing::types::DiagramNode> = nodes
        .iter()
        .enumerate()
        .map(|(i, node)| {
            let (c, r) = positions[i];
            routing::types::DiagramNode {
                name: node.name.clone(),
                col: c,
                row: r,
            }
        })
        .collect();

    let routing_edges: Vec<routing::types::DiagramEdge> = edges
        .iter()
        .map(|e| routing::types::DiagramEdge {
            source: e.from.clone(),
            target: e.to.clone(),
            label: if e.label.is_empty() {
                None
            } else {
                Some(e.label.clone())
            },
        })
        .collect();

    let config = routing::types::RoutingConfig::default();
    writeln!(out).unwrap();
    writeln!(
        out,
        "CONFIG: h_lanes={}, v_lanes={}",
        config.h_lane_capacity, config.v_lane_capacity
    )
    .unwrap();

    let routing_output = routing::route_all_edges(&routing_nodes, &routing_edges, &config);

    writeln!(out).unwrap();
    writeln!(out, "ROUTING RESULTS:").unwrap();
    for (edge, result) in &routing_output.results {
        match result {
            routing::types::RouteResult::Success(route) => {
                let label = edge
                    .label
                    .as_deref()
                    .map_or(String::new(), |l| format!(" \"{l}\""));
                writeln!(
                    out,
                    "  {} -> {}{}: OK len={:.1} turns={} lc={} cx={}",
                    edge.source,
                    edge.target,
                    label,
                    route.complexity.length,
                    route.complexity.turns,
                    route.complexity.lane_changes,
                    route.complexity.crossings,
                )
                .unwrap();
                let mut route_str = String::new();
                for (i, w) in route.waypoints.iter().enumerate() {
                    if i > 0 {
                        // Lane label goes between coordinates (on the segment)
                        let prev = &route.waypoints[i - 1];
                        route_str.push_str(&format!(" L{} → ", prev.lane));
                    }
                    route_str.push_str(&format!("{}", w.coord));
                }
                writeln!(out, "    {route_str}").unwrap();
            }
            routing::types::RouteResult::Failure { warning } => {
                let label = edge
                    .label
                    .as_deref()
                    .map_or(String::new(), |l| format!(" \"{l}\""));
                writeln!(
                    out,
                    "  {} -> {}{}: FAIL: {}",
                    edge.source, edge.target, label, warning
                )
                .unwrap();
            }
        }
    }

    out
}

// ─── Diagram renderer ────────────────────────────────────────────────────────

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

/// Draw a diagram parsed from `- Node: label` and `- A -> B: label` lines.
#[allow(clippy::too_many_arguments)]
/// Draw a diagram. `max_height` controls the vertical space; pass 0 for a default.
pub fn draw_diagram_sized(
    ui: &egui::Ui,
    content: &str,
    theme: &Theme,
    pos: Pos2,
    max_width: f32,
    max_height: f32,
    opacity: f32,
    image_cache: &ImageCache,
    reveal_step: usize,
    reveal_timestamp: Option<Instant>,
    scale: f32,
) -> f32 {
    let (nodes, edges, scale_directive) = parse_diagram(content);

    // Compute reveal step assignments for each element.
    // Static elements are always visible (step 0). Each `+` increments the step counter.
    // `*` elements share the same step as the previous `+`.
    // Process all elements in file order (by parse_order) so that interleaved nodes and
    // edges get correct step numbers matching their source ordering.
    #[derive(Clone, Copy)]
    enum ElementRef {
        Node(usize),
        Edge(usize),
    }
    let mut all_elements: Vec<ElementRef> = Vec::new();
    for (i, _) in nodes.iter().enumerate() {
        all_elements.push(ElementRef::Node(i));
    }
    for (i, _) in edges.iter().enumerate() {
        all_elements.push(ElementRef::Edge(i));
    }
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
            DiagramReveal::WithPrev => step_counter,
        };
    }

    if nodes.is_empty() {
        // Fallback for unparseable diagrams
        let color = Theme::with_opacity(theme.foreground, opacity * 0.6);
        let bg = Theme::with_opacity(theme.code_background, opacity);
        let height = 200.0 * scale;
        let rect = egui::Rect::from_min_size(pos, egui::vec2(max_width, height));
        ui.painter().rect_filled(rect, 8.0 * scale, bg);
        let galley = ui.painter().layout(
            "[Diagram]".to_string(),
            FontId::proportional(theme.body_size * 0.8 * scale),
            color,
            max_width,
        );
        let text_pos = Pos2::new(
            pos.x + (max_width - galley.rect.width()) / 2.0,
            pos.y + (height - galley.rect.height()) / 2.0,
        );
        ui.painter().galley(text_pos, galley, color);
        return height;
    }

    // Use the provided max_height, or default to 500px
    let diagram_height = if max_height > 0.0 {
        max_height
    } else {
        500.0 * scale
    };
    let padding = 30.0 * scale;
    let area_width = max_width - padding * 2.0;
    let area_height = diagram_height - padding * 2.0;

    let abs_origin_x = pos.x + padding;
    let abs_origin_y = pos.y + padding;
    let (mut layouts, mut grid) = layout_nodes(
        &nodes,
        area_width,
        area_height,
        abs_origin_x,
        abs_origin_y,
        scale,
    );

    // Scale-to-fit: if the layout overflows the available area, scale down uniformly.
    // This handles large diagrams (3+ rows) where minimum node sizes cause overflow.
    let fit_scale = match scale_directive {
        DiagramScale::Fit => {
            if area_height > 0.0 {
                let mut bbox_bottom = 0.0f32;
                for layout in &layouts {
                    let bottom = layout.center_y + layout.height / 2.0;
                    bbox_bottom = bbox_bottom.max(bottom);
                }
                if bbox_bottom > area_height {
                    (area_height / bbox_bottom).clamp(0.3, 1.0)
                } else {
                    1.0
                }
            } else {
                1.0
            }
        }
        DiagramScale::Factor(f) => f,
        DiagramScale::Scroll => 1.0,
    };

    if (fit_scale - 1.0).abs() > 0.001 {
        let center_x = area_width / 2.0;
        let center_y = area_height / 2.0;
        for layout in &mut layouts {
            layout.center_x = center_x + (layout.center_x - center_x) * fit_scale;
            layout.center_y = center_y + (layout.center_y - center_y) * fit_scale;
            layout.width *= fit_scale;
            layout.height *= fit_scale;
        }
        grid.cell_w *= fit_scale;
        grid.cell_h *= fit_scale;
    }

    // Build name -> layout index map and compute absolute positions
    let mut node_rects: HashMap<String, egui::Rect> = HashMap::new();
    let painter = ui.painter();

    let accent = theme.accent;
    let node_border_color = Theme::with_opacity(accent, opacity * 0.8);
    let node_fill = Theme::with_opacity(theme.code_background, opacity * 0.95);
    let shadow_color = Theme::with_opacity(Color32::from_rgb(0, 0, 0), opacity * 0.1);
    let label_color = Theme::with_opacity(theme.foreground, opacity);
    let icon_color = Theme::with_opacity(accent, opacity * 0.9);

    // Draw nodes (skip those not yet revealed)
    for (i, node) in nodes.iter().enumerate() {
        // Always compute rect for routing, but skip drawing if not revealed
        let layout = &layouts[i];
        let abs_x = pos.x + padding + layout.center_x;
        let abs_y = pos.y + padding + layout.center_y;

        let node_rect = egui::Rect::from_center_size(
            egui::pos2(abs_x, abs_y),
            egui::vec2(layout.width, layout.height),
        );
        // Always register node rect (needed for edge routing even if not visible)
        node_rects.insert(node.name.clone(), node_rect);

        // Skip drawing if this node hasn't been revealed yet
        let node_step = node_steps.get(i).copied().unwrap_or(0);
        if node_step > reveal_step {
            continue;
        }

        let corner_radius = 8.0 * scale;

        // Drop shadow
        let shadow_rect = node_rect.translate(egui::vec2(3.0 * scale, 3.0 * scale));
        painter.rect_filled(shadow_rect, corner_radius, shadow_color);

        // Node background
        painter.rect_filled(node_rect, corner_radius, node_fill);

        // Node border
        painter.rect_stroke(
            node_rect,
            corner_radius,
            Stroke::new(2.5 * scale, node_border_color),
            egui::StrokeKind::Outside,
        );

        // Icon area (top portion of node)
        let icon_size = layout.height * 0.5;
        let icon_center = Pos2::new(abs_x, abs_y - layout.height * 0.12);

        // Try loading icon image from media/diagram-icons/{icon}.png
        let icon_path = if !node.icon.is_empty() {
            format!("media/diagram-icons/{}.png", node.icon)
        } else {
            String::new()
        };

        let has_image = if !icon_path.is_empty() {
            if let Some(texture) = image_cache.get_or_load(ui, &icon_path) {
                // Draw the icon image, preserving aspect ratio
                let max_size = icon_size * 0.85;
                let tex_size = texture.size_vec2();
                let aspect = tex_size.x / tex_size.y.max(1.0);
                let (w, h) = if aspect >= 1.0 {
                    (max_size, max_size / aspect)
                } else {
                    (max_size * aspect, max_size)
                };
                let img_rect = egui::Rect::from_center_size(icon_center, egui::vec2(w, h));
                let tint = Theme::with_opacity(Color32::WHITE, opacity);
                painter.image(
                    texture.id(),
                    img_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    tint,
                );
                true
            } else {
                false
            }
        } else {
            false
        };

        if !has_image {
            // Draw geometric fallback icon
            let icon_name = if node.icon.is_empty() {
                "box"
            } else {
                &node.icon
            };
            draw_icon_fallback(
                painter,
                icon_name,
                icon_center,
                icon_size,
                icon_color,
                2.0 * scale,
                scale,
            );
        }

        // Label text below icon
        let label_font_size = theme.body_size * 0.8 * scale;
        let galley = painter.layout(
            node.label.clone(),
            FontId::proportional(label_font_size),
            label_color,
            layout.width - 8.0 * scale,
        );
        let text_y = abs_y + layout.height * 0.25;
        let text_pos = egui::pos2(abs_x - galley.rect.width() / 2.0, text_y);
        painter.galley(text_pos, galley, label_color);
    }

    // ── Draw edges (orthogonal routing) ────────────────────────────────────

    let edge_palette = theme.edge_palette();
    let label_text_color = Theme::with_opacity(theme.foreground, opacity * 0.8);
    let line_width = 4.0 * scale;
    let arrow_size = 20.0 * scale;
    let node_margin = 10.0 * scale;
    let corner_radius = 10.0 * scale;
    let lane_spacing = 20.0 * scale;
    let port_spacing = 22.0 * scale;

    let animation_duration = 0.4; // seconds
    let mut needs_repaint = false;

    // Build routing input from diagram data
    // Filter to only visible edges and collect their grid positions
    let visible_edges: Vec<(usize, &DiagramEdge)> = edges
        .iter()
        .enumerate()
        .filter(|(edge_idx, edge)| {
            let edge_step = edge_steps.get(*edge_idx).copied().unwrap_or(0);
            if edge_step > reveal_step {
                return false;
            }
            // Skip self-loops and edges with missing nodes
            if edge.from == edge.to {
                return false;
            }
            node_rects.contains_key(&edge.from) && node_rects.contains_key(&edge.to)
        })
        .collect();

    // Build routing nodes from the parsed diagram nodes that have grid positions
    let routing_nodes: Vec<routing::types::DiagramNode> = nodes
        .iter()
        .filter_map(|n| {
            let rect = node_rects.get(&n.name)?;
            let center = rect.center();
            let (col, row) = grid.cell_at(center)?;
            Some(routing::types::DiagramNode {
                name: n.name.clone(),
                col: (col + 1) as i32, // convert 0-indexed to 1-based
                row: (row + 1) as i32,
            })
        })
        .collect();

    let routing_edges: Vec<routing::types::DiagramEdge> = visible_edges
        .iter()
        .map(|(_, edge)| routing::types::DiagramEdge {
            source: edge.from.clone(),
            target: edge.to.clone(),
            label: if edge.label.is_empty() {
                None
            } else {
                Some(edge.label.clone())
            },
        })
        .collect();

    let config = routing::types::RoutingConfig {
        h_lane_capacity: compute_h_capacity(&grid, &node_rects, lane_spacing),
        v_lane_capacity: compute_v_capacity(&grid, &node_rects, lane_spacing),
        weights: *ROUTING_WEIGHTS,
    };

    // Use cached routing output — only recompute when inputs change.
    let cache_key = route_cache_key(&routing_nodes, &routing_edges, &config);
    let routing_output = cached_routes(cache_key, || {
        routing::route_all_edges(&routing_nodes, &routing_edges, &config)
    });

    // Track port usage per (node, face) to spread connections
    let mut port_counts: HashMap<(String, Face), usize> = HashMap::new();
    let claim_port = |counts: &mut HashMap<(String, Face), usize>,
                      node_name: &str,
                      face: Face,
                      rect: &egui::Rect|
     -> f32 {
        let key = (node_name.to_string(), face);
        let idx = counts.entry(key).or_insert(0);
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
        let offset = sign * level as f32 * port_spacing;
        offset.clamp(-max_offset, max_offset)
    };

    // Draw each routed edge
    for (result_idx, (_, route_result)) in routing_output.results.iter().enumerate() {
        let (edge_idx, edge) = visible_edges[result_idx];

        let from_rect = &node_rects[&edge.from];
        let to_rect = &node_rects[&edge.to];

        // Each edge gets a distinct color from the palette
        let base_color = edge_palette[edge_idx % edge_palette.len()];
        let is_dashed = matches!(edge.arrow, ArrowKind::DashedLine | ArrowKind::DashedArrow);
        let current_edge_color = if is_dashed {
            Theme::with_opacity(base_color, opacity * 0.55)
        } else {
            Theme::with_opacity(base_color, opacity * 0.85)
        };

        // Compute animation progress for edges appearing on the current step
        let edge_step = edge_steps.get(edge_idx).copied().unwrap_or(0);
        let anim_progress = if edge_step == reveal_step && edge_step > 0 {
            if let Some(ts) = reveal_timestamp {
                let elapsed = ts.elapsed().as_secs_f32();
                let t = (elapsed / animation_duration).min(1.0);
                let eased = if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0_f32 * t + 2.0).powi(2) / 2.0
                };
                if t < 1.0 {
                    needs_repaint = true;
                }
                eased
            } else {
                1.0
            }
        } else {
            1.0
        };

        let pixel_waypoints = match route_result {
            routing::types::RouteResult::Success(route) => {
                // Determine faces from route direction
                let first = &route.waypoints[0];
                let second = &route.waypoints[1];
                let exit_dir = coord_direction(first.coord, second.coord);
                let exit_face = direction_to_face(exit_dir);

                let n = route.waypoints.len();
                let penult = &route.waypoints[n - 2];
                let last = &route.waypoints[n - 1];
                let entry_dir = coord_direction(penult.coord, last.coord);
                let entry_face = direction_to_face(entry_dir.opposite());

                // Derive port offsets from lane so face points align with corridor
                let exit_lane = first.lane;
                let (elx, ely) = lane_offset(exit_dir, exit_lane, lane_spacing);
                let port_start = match exit_face {
                    Face::Left | Face::Right => ely,
                    Face::Top | Face::Bottom => elx,
                };

                let entry_lane = penult.lane;
                let (nlx, nly) = lane_offset(entry_dir, entry_lane, lane_spacing);
                let port_end = match entry_face {
                    Face::Left | Face::Right => nly,
                    Face::Top | Face::Bottom => nlx,
                };

                waypoints_to_pixels(
                    route,
                    &grid,
                    from_rect,
                    to_rect,
                    node_margin,
                    lane_spacing,
                    port_start,
                    port_end,
                )
            }
            routing::types::RouteResult::Failure { .. } => {
                // Fallback: direct connection
                let exit_face = choose_exit_face(from_rect, to_rect.center());
                let entry_face = choose_entry_face(to_rect, from_rect.center());
                let port_start = claim_port(&mut port_counts, &edge.from, exit_face, from_rect);
                let port_end = claim_port(&mut port_counts, &edge.to, entry_face, to_rect);
                let fp_start = face_point_with_port(from_rect, exit_face, port_start);
                let fp_end = face_point_with_port(to_rect, entry_face, port_end);
                let ramp_start = ramp_from_face(from_rect, exit_face, port_start, node_margin);
                let ramp_end = ramp_from_face(to_rect, entry_face, port_end, node_margin);
                vec![fp_start, ramp_start, ramp_end, fp_end]
            }
        };

        // Use edge color as label background so labels visually match their edge
        let edge_label_bg = Theme::with_opacity(current_edge_color, opacity * 0.80);
        draw_routed_edge(
            painter,
            &pixel_waypoints,
            edge.arrow,
            &edge.label,
            current_edge_color,
            edge_label_bg,
            label_text_color,
            line_width,
            arrow_size,
            corner_radius,
            theme,
            scale,
            opacity,
            anim_progress,
        );
    }

    // Request repaint while edges are still animating
    if needs_repaint {
        ui.ctx().request_repaint();
    }

    diagram_height
}

// ─── Diagram tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod diagram_tests {
    use super::*;
    use std::collections::HashSet;

    // ── Parsing tests ────────────────────────────────────────────────────────

    #[test]
    fn test_parse_simple_chain() {
        let content = "- A -> B: sends\n- B -> C: forwards";
        let (nodes, edges, _) = parse_diagram(content);
        assert_eq!(nodes.len(), 3);
        assert_eq!(edges.len(), 2);
        assert_eq!(edges[0].from, "A");
        assert_eq!(edges[0].to, "B");
        assert_eq!(edges[0].label, "sends");
        assert!(matches!(edges[0].arrow, ArrowKind::Forward));
    }

    #[test]
    fn test_skip_comments() {
        let content = "# Components\n- A -> B\n# Relationships\n- B -> C";
        let (nodes, edges, _) = parse_diagram(content);
        assert_eq!(nodes.len(), 3);
        assert_eq!(edges.len(), 2);
        assert!(!nodes.iter().any(|n| n.name.starts_with('#')));
    }

    #[test]
    fn test_parse_metadata() {
        let meta = parse_node_metadata("Server (icon: server, pos: 2, 3)");
        assert_eq!(meta.before, "Server");
        assert_eq!(meta.icon, "server");
        assert_eq!(meta.grid_pos, Some((2, 3)));
        assert!(meta.prompt.is_none());
    }

    #[test]
    fn test_parse_metadata_with_prompt() {
        let meta = parse_node_metadata(
            "Gateway (icon: generate-image, prompt: \"An API gateway\", pos: 1, 1)",
        );
        assert_eq!(meta.before, "Gateway");
        assert_eq!(meta.icon, "generate-image");
        assert_eq!(meta.grid_pos, Some((1, 1)));
        assert_eq!(meta.prompt.as_deref(), Some("An API gateway"));
    }

    #[test]
    fn test_arrow_types() {
        let content = "A -> B\nC <- D\nE <-> F\nG -- H\nI --> J";
        let (_, edges, _) = parse_diagram(content);
        assert!(matches!(edges[0].arrow, ArrowKind::Forward));
        assert!(matches!(edges[1].arrow, ArrowKind::Reverse));
        assert!(matches!(edges[2].arrow, ArrowKind::Bidirectional));
        assert!(matches!(edges[3].arrow, ArrowKind::DashedLine));
        assert!(matches!(edges[4].arrow, ArrowKind::DashedArrow));
    }

    #[test]
    fn test_node_with_label_and_metadata() {
        let content = "- DB: Database (icon: database, pos: 1, 2)";
        let (nodes, _, _) = parse_diagram(content);
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].name, "DB");
        assert_eq!(nodes[0].label, "Database");
        assert_eq!(nodes[0].icon, "database");
        assert_eq!(nodes[0].grid_pos, Some((1, 2)));
    }

    #[test]
    fn test_detect_arrow_ordering() {
        // <-> must be detected before -> and <-
        assert!(matches!(
            detect_arrow("A <-> B"),
            Some((_, _, ArrowKind::Bidirectional))
        ));
        assert!(matches!(
            detect_arrow("A --> B"),
            Some((_, _, ArrowKind::DashedArrow))
        ));
        assert!(matches!(
            detect_arrow("A -> B"),
            Some((_, _, ArrowKind::Forward))
        ));
        assert!(matches!(
            detect_arrow("A <- B"),
            Some((_, _, ArrowKind::Reverse))
        ));
        assert!(matches!(
            detect_arrow("A -- B"),
            Some((_, _, ArrowKind::DashedLine))
        ));
        assert!(detect_arrow("A B").is_none());
    }

    #[test]
    fn test_empty_diagram() {
        let (nodes, edges, _) = parse_diagram("");
        assert_eq!(nodes.len(), 0);
        assert_eq!(edges.len(), 0);
    }

    #[test]
    fn test_comments_only() {
        let (nodes, edges, _) = parse_diagram("# comment\n# another");
        assert_eq!(nodes.len(), 0);
        assert_eq!(edges.len(), 0);
    }

    #[test]
    fn test_reveal_markers_parsed() {
        let content = "- A (pos: 1, 1)\n+ B (pos: 2, 1)\n* C (pos: 3, 1)";
        let (nodes, _, _) = parse_diagram(content);
        assert_eq!(nodes[0].reveal, DiagramReveal::Static);
        assert_eq!(nodes[1].reveal, DiagramReveal::NextStep);
        assert_eq!(nodes[2].reveal, DiagramReveal::WithPrev);
    }

    #[test]
    fn test_reveal_markers_on_edges() {
        let content = "- A -> B\n+ C -> D\n* E -> F";
        let (_, edges, _) = parse_diagram(content);
        assert_eq!(edges[0].reveal, DiagramReveal::Static);
        assert_eq!(edges[1].reveal, DiagramReveal::NextStep);
        assert_eq!(edges[2].reveal, DiagramReveal::WithPrev);
    }

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
    fn test_parse_diagram_whitespace() {
        let content = "  A -> B  ";
        let (nodes, edges, _) = parse_diagram(content);
        assert_eq!(nodes.len(), 2);
        assert_eq!(edges.len(), 1);
    }

    #[test]
    fn test_parse_diagram_mixed_definitions() {
        let content = "- Server: Web Server\n- Server -> DB: queries\n- DB: Database";
        let (nodes, edges, _) = parse_diagram(content);
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].label, "Web Server");
        assert_eq!(nodes[1].label, "Database");
        assert_eq!(edges.len(), 1);
    }

    #[test]
    fn test_parse_diagram_reverse_arrow() {
        let content = "A <- B";
        let (_, edges, _) = parse_diagram(content);
        assert!(matches!(edges[0].arrow, ArrowKind::Reverse));
        assert_eq!(edges[0].from, "A");
        assert_eq!(edges[0].to, "B");
    }

    #[test]
    fn test_parse_diagram_bidirectional() {
        let content = "A <-> B";
        let (_, edges, _) = parse_diagram(content);
        assert!(matches!(edges[0].arrow, ArrowKind::Bidirectional));
    }

    #[test]
    fn test_detect_arrow_none() {
        assert!(detect_arrow("no arrow here").is_none());
        assert!(detect_arrow("A B C").is_none());
    }

    #[test]
    fn test_detect_arrow_with_labels() {
        let result = detect_arrow("Client -> Server: HTTP");
        assert!(result.is_some());
        let (pos, len, kind) = result.unwrap();
        assert!(matches!(kind, ArrowKind::Forward));
        assert_eq!(&"Client -> Server: HTTP"[pos + 1..pos + len - 1], "->");
    }

    // ── Scale directive tests ─────────────────────────────────────────────────

    #[test]
    fn test_scale_directive_default() {
        let (_, _, scale) = parse_diagram("A -> B");
        assert_eq!(scale, DiagramScale::Fit);
    }

    #[test]
    fn test_scale_directive_fit() {
        let (_, _, scale) = parse_diagram("# scale: fit\nA -> B");
        assert_eq!(scale, DiagramScale::Fit);
    }

    #[test]
    fn test_scale_directive_scroll() {
        let (_, _, scale) = parse_diagram("# scale: scroll\nA -> B");
        assert_eq!(scale, DiagramScale::Scroll);
    }

    #[test]
    fn test_scale_directive_factor() {
        let (_, _, scale) = parse_diagram("# scale: 0.7\nA -> B");
        assert!(matches!(scale, DiagramScale::Factor(f) if (f - 0.7).abs() < 0.001));
    }

    #[test]
    fn test_scale_directive_factor_clamped() {
        let (_, _, scale) = parse_diagram("# scale: 5.0\nA -> B");
        assert!(matches!(scale, DiagramScale::Factor(f) if (f - 2.0).abs() < 0.001));
    }

    // ── Rendering geometry tests ─────────────────────────────────────────────

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

    // ── Face/ramp helper tests ───────────────────────────────────────────────

    #[test]
    fn test_face_selection_horizontal() {
        let r = egui::Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(100.0, 80.0));
        assert!(matches!(
            choose_exit_face(&r, Pos2::new(200.0, 40.0)),
            Face::Right
        ));
        assert!(matches!(
            choose_exit_face(&r, Pos2::new(-100.0, 40.0)),
            Face::Left
        ));
    }

    #[test]
    fn test_face_selection_vertical() {
        let r = egui::Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(100.0, 80.0));
        assert!(matches!(
            choose_exit_face(&r, Pos2::new(50.0, 200.0)),
            Face::Bottom
        ));
        assert!(matches!(
            choose_exit_face(&r, Pos2::new(50.0, -100.0)),
            Face::Top
        ));
    }

    #[test]
    fn test_face_selection_diagonal_right_down() {
        let r = egui::Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(100.0, 80.0));
        // When dx and dy are equal, dx.abs() >= dy.abs() is true, so Right
        let face = choose_exit_face(&r, Pos2::new(200.0, 200.0));
        assert!(matches!(face, Face::Right | Face::Bottom));
    }

    #[test]
    fn test_entry_face_matches_direction() {
        let r = egui::Rect::from_min_max(Pos2::new(100.0, 100.0), Pos2::new(200.0, 180.0));
        assert!(matches!(
            choose_entry_face(&r, Pos2::new(0.0, 140.0)),
            Face::Left
        ));
        assert!(matches!(
            choose_entry_face(&r, Pos2::new(300.0, 140.0)),
            Face::Right
        ));
        assert!(matches!(
            choose_entry_face(&r, Pos2::new(150.0, 0.0)),
            Face::Top
        ));
        assert!(matches!(
            choose_entry_face(&r, Pos2::new(150.0, 300.0)),
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

    // ── GridInfo tests ───────────────────────────────────────────────────────

    #[test]
    fn test_cell_at_origin() {
        let grid = GridInfo {
            cols: 3,
            rows: 2,
            cell_w: 100.0,
            cell_h: 80.0,
            origin_x: 0.0,
            origin_y: 0.0,
            occupied: HashSet::new(),
        };
        assert_eq!(grid.cell_at(Pos2::new(10.0, 10.0)), Some((0, 0)));
    }

    #[test]
    fn test_cell_at_center_cell() {
        let grid = GridInfo {
            cols: 3,
            rows: 2,
            cell_w: 100.0,
            cell_h: 80.0,
            origin_x: 0.0,
            origin_y: 0.0,
            occupied: HashSet::new(),
        };
        assert_eq!(grid.cell_at(Pos2::new(150.0, 40.0)), Some((1, 0)));
    }

    #[test]
    fn test_cell_at_out_of_bounds() {
        let grid = GridInfo {
            cols: 3,
            rows: 2,
            cell_w: 100.0,
            cell_h: 80.0,
            origin_x: 0.0,
            origin_y: 0.0,
            occupied: HashSet::new(),
        };
        assert_eq!(grid.cell_at(Pos2::new(-10.0, 10.0)), None);
        assert_eq!(grid.cell_at(Pos2::new(10.0, -10.0)), None);
        assert_eq!(grid.cell_at(Pos2::new(310.0, 10.0)), None);
        assert_eq!(grid.cell_at(Pos2::new(10.0, 170.0)), None);
    }

    #[test]
    fn test_is_cell_empty() {
        let mut occupied = HashSet::new();
        occupied.insert((1, 0));
        let grid = GridInfo {
            cols: 3,
            rows: 2,
            cell_w: 100.0,
            cell_h: 80.0,
            origin_x: 0.0,
            origin_y: 0.0,
            occupied,
        };
        assert!(grid.is_cell_empty(0, 0));
        assert!(!grid.is_cell_empty(1, 0));
        assert!(grid.is_cell_empty(2, 0));
    }

    #[test]
    fn test_corridor_positions() {
        let grid = GridInfo {
            cols: 3,
            rows: 2,
            cell_w: 100.0,
            cell_h: 80.0,
            origin_x: 10.0,
            origin_y: 20.0,
            occupied: HashSet::new(),
        };
        assert_eq!(grid.h_corridor_y(0), 20.0);
        assert_eq!(grid.h_corridor_y(1), 100.0);
        assert_eq!(grid.h_corridor_y(2), 180.0);
        assert_eq!(grid.v_corridor_x(0), 10.0);
        assert_eq!(grid.v_corridor_x(1), 110.0);
        assert_eq!(grid.v_corridor_x(3), 310.0);
    }

    // ── Edge palette tests ───────────────────────────────────────────────────

    #[test]
    fn test_edge_palette_dark_has_entries() {
        let theme = Theme::dark();
        let palette = theme.edge_palette();
        assert!(!palette.is_empty());
        assert!(palette.len() >= 6);
    }

    #[test]
    fn test_edge_palette_light_has_entries() {
        let theme = Theme::light();
        let palette = theme.edge_palette();
        assert!(!palette.is_empty());
        assert!(palette.len() >= 6);
    }

    #[test]
    fn test_edge_palette_colors_are_distinct() {
        let theme = Theme::dark();
        let palette = theme.edge_palette();
        for i in 0..palette.len() {
            for j in i + 1..palette.len() {
                assert_ne!(
                    palette[i], palette[j],
                    "Colors at {i} and {j} should differ"
                );
            }
        }
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
        // Junction at (1.5, 1.0) — between columns 1 and 2
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

        // Lane +1 traveling West → also south (+Y) — absolute, not direction-relative
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
                // Source at (1,1), target at (2,1) — should go east
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
        let from_rect =
            egui::Rect::from_center_size(Pos2::new(100.0, 75.0), egui::vec2(130.0, 90.0));
        let to_rect = egui::Rect::from_center_size(Pos2::new(300.0, 75.0), egui::vec2(130.0, 90.0));

        match &output.results[0].1 {
            routing::types::RouteResult::Success(route) => {
                let pixels =
                    waypoints_to_pixels(route, &grid, &from_rect, &to_rect, 10.0, 20.0, 0.0, 0.0);
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
        let lane_spacing = 20.0;

        // API at (2,2) → center pixel (450, 300)
        let from_rect =
            egui::Rect::from_center_size(Pos2::new(450.0, 300.0), egui::vec2(160.0, 120.0));
        // Auth at (1,3) → center pixel (150, 500)
        let to_rect =
            egui::Rect::from_center_size(Pos2::new(150.0, 500.0), egui::vec2(160.0, 120.0));

        // Derive port offsets the same way the renderer does
        let exit_dir = coord_direction(route.waypoints[0].coord, route.waypoints[1].coord);
        let exit_lane = route.waypoints[0].lane;
        let (elx, ely) = lane_offset(exit_dir, exit_lane, lane_spacing);
        let exit_face = direction_to_face(exit_dir);
        let port_start = match exit_face {
            Face::Left | Face::Right => ely,
            Face::Top | Face::Bottom => elx,
        };

        let n = route.waypoints.len();
        let entry_dir = coord_direction(route.waypoints[n - 2].coord, route.waypoints[n - 1].coord);
        let entry_lane = route.waypoints[n - 2].lane;
        let (nlx, nly) = lane_offset(entry_dir, entry_lane, lane_spacing);
        let entry_face = direction_to_face(entry_dir.opposite());
        let port_end = match entry_face {
            Face::Left | Face::Right => nly,
            Face::Top | Face::Bottom => nlx,
        };

        let pixels = waypoints_to_pixels(
            &route,
            &grid,
            &from_rect,
            &to_rect,
            10.0,
            lane_spacing,
            port_start,
            port_end,
        );

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
        // be monotonically decreasing (going up) or increasing (going down) —
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

    // ── Capacity computation tests ───────────────────────────────────────────

    #[test]
    fn test_compute_h_capacity() {
        let grid = GridInfo {
            cols: 2,
            rows: 2,
            cell_w: 200.0,
            cell_h: 150.0,
            origin_x: 0.0,
            origin_y: 0.0,
            occupied: [(0, 0), (1, 0), (0, 1), (1, 1)].iter().cloned().collect(),
        };
        let mut rects = HashMap::new();
        rects.insert(
            "A".into(),
            egui::Rect::from_center_size(Pos2::new(100.0, 75.0), egui::vec2(130.0, 90.0)),
        );
        rects.insert(
            "B".into(),
            egui::Rect::from_center_size(Pos2::new(300.0, 75.0), egui::vec2(130.0, 90.0)),
        );
        rects.insert(
            "C".into(),
            egui::Rect::from_center_size(Pos2::new(100.0, 225.0), egui::vec2(130.0, 90.0)),
        );
        rects.insert(
            "D".into(),
            egui::Rect::from_center_size(Pos2::new(300.0, 225.0), egui::vec2(130.0, 90.0)),
        );
        let cap = compute_h_capacity(&grid, &rects, 20.0);
        // cell_h=150, node_h=90, gap=60, 60/20 = 3
        assert_eq!(cap, 3);
    }

    #[test]
    fn test_compute_v_capacity() {
        let grid = GridInfo {
            cols: 2,
            rows: 2,
            cell_w: 200.0,
            cell_h: 150.0,
            origin_x: 0.0,
            origin_y: 0.0,
            occupied: [(0, 0), (1, 0)].iter().cloned().collect(),
        };
        let mut rects = HashMap::new();
        rects.insert(
            "A".into(),
            egui::Rect::from_center_size(Pos2::new(100.0, 75.0), egui::vec2(130.0, 90.0)),
        );
        rects.insert(
            "B".into(),
            egui::Rect::from_center_size(Pos2::new(300.0, 75.0), egui::vec2(130.0, 90.0)),
        );
        let cap = compute_v_capacity(&grid, &rects, 20.0);
        // cell_w=200, node_w=130, gap=70, 70/20 = 3
        assert_eq!(cap, 3);
    }

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

    /// Helper: compute reveal step assignments in file order (mirrors draw_diagram_sized logic).
    fn compute_steps(nodes: &[DiagramNode], edges: &[DiagramEdge]) -> (Vec<usize>, Vec<usize>) {
        #[derive(Clone, Copy)]
        enum ElementRef {
            Node(usize),
            Edge(usize),
        }
        let mut all_elements: Vec<ElementRef> = Vec::new();
        for (i, _) in nodes.iter().enumerate() {
            all_elements.push(ElementRef::Node(i));
        }
        for (i, _) in edges.iter().enumerate() {
            all_elements.push(ElementRef::Edge(i));
        }
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
                DiagramReveal::WithPrev => step_counter,
            };
        }
        (node_steps, edge_steps)
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
* Build -> Test: on success
+ Deploy (icon: cloud, pos: 4,1)
* Test -> Deploy: all green";

        let (nodes, edges, _) = parse_diagram(content);
        assert_eq!(nodes.len(), 4); // Source, Build, Test, Deploy
        assert_eq!(edges.len(), 3); // Source->Build, Build->Test, Test->Deploy

        let (node_steps, edge_steps) = compute_steps(&nodes, &edges);

        // Source is static (step 0)
        assert_eq!(node_steps[0], 0, "Source should be step 0");
        // + Build → step 1
        assert_eq!(node_steps[1], 1, "Build should be step 1");
        // + Source -> Build → step 2
        assert_eq!(edge_steps[0], 2, "Source->Build should be step 2");
        // + Test → step 3
        assert_eq!(node_steps[2], 3, "Test should be step 3");
        // * Build -> Test → step 3 (with prev)
        assert_eq!(edge_steps[1], 3, "Build->Test should be step 3 (with prev)");
        // + Deploy → step 4
        assert_eq!(node_steps[3], 4, "Deploy should be step 4");
        // * Test -> Deploy → step 4 (with prev)
        assert_eq!(
            edge_steps[2], 4,
            "Test->Deploy should be step 4 (with prev)"
        );
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
* Monitor -- Server: observes
* Monitor -- DB: observes";

        let (nodes, edges, _) = parse_diagram(content);
        let (node_steps, edge_steps) = compute_steps(&nodes, &edges);

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
        // * Monitor -- Server → step 4
        assert_eq!(edge_steps[3], 4, "Monitor--Server = 4");
        // * Monitor -- DB → step 4
        assert_eq!(edge_steps[4], 4, "Monitor--DB = 4");
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
}
