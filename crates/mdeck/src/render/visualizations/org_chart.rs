use std::collections::HashMap;

use eframe::egui::{self, Color32, FontId, Pos2, Stroke};

use crate::theme::Theme;

use super::{
    VIZ_CORNER_NODE, VIZ_FONT_PRIMARY_LABEL, VIZ_STROKE_BORDER, VIZ_STROKE_SEPARATOR, VizCtx,
    VizReveal, assign_steps,
    grammar::{Arrow, Problem, Source},
};

// ─── Parsing ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct OrgEdge {
    parent: String,
    child: String,
    reveal: VizReveal,
}

/// `- Manager -> Report` links, and `- Name` for a root (or a lone node).
/// Without any `- Name` line the roots are the names that are never a
/// report.
fn read(src: &Source) -> (Vec<String>, Vec<OrgEdge>) {
    src.check_settings(&[]);
    let mut roots = Vec::new();
    let mut edges = Vec::new();
    let mut seen_nodes: Vec<String> = Vec::new();
    let mut see = |name: &str| {
        if !seen_nodes.iter().any(|n| n == name) {
            seen_nodes.push(name.to_string());
        }
    };
    for item in &src.items {
        item.check_attrs(src, &[]);
        if let Some(rel) = item.relation(&Arrow::FORWARD) {
            if rel.label.is_some() {
                src.problem(item.offset, "org chart links take no label");
            }
            see(rel.from);
            see(rel.to);
            edges.push(OrgEdge {
                parent: rel.from.to_string(),
                child: rel.to.to_string(),
                reveal: item.reveal,
            });
        } else if item.text.contains("->") || item.text.contains(':') || item.text.is_empty() {
            src.problem(
                item.offset,
                format!(
                    "'{}' is not a person or a link; write '- CEO' or '- CEO -> CTO'",
                    item.text
                ),
            );
        } else {
            see(item.text);
            if !roots.iter().any(|r| r == item.text) {
                roots.push(item.text.to_string());
            }
        }
    }

    // If no explicit roots, find nodes that are never children
    if roots.is_empty() {
        let children: Vec<&str> = edges.iter().map(|e| e.child.as_str()).collect();
        for node in &seen_nodes {
            if !children.contains(&node.as_str()) {
                roots.push(node.clone());
            }
        }
    }

    (roots, edges)
}

fn parse_org_chart(content: &str) -> (Vec<String>, Vec<OrgEdge>) {
    read(&Source::parse(content))
}

/// The problems in an `@orgchart` block.
pub fn check(content: &str) -> Vec<Problem> {
    let src = Source::parse(content);
    read(&src);
    src.into_problems()
}
// ─── Tree layout ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct LayoutNode {
    label: String,
    x: f32,
    y: f32,
    depth: usize,
}

fn build_layout(
    roots: &[String],
    edges: &[OrgEdge],
    area_width: f32,
    area_height: f32,
) -> Vec<LayoutNode> {
    // Build children map
    let mut children_map: HashMap<String, Vec<String>> = HashMap::new();
    for edge in edges {
        children_map
            .entry(edge.parent.clone())
            .or_default()
            .push(edge.child.clone());
    }

    // BFS to compute depth and collect nodes per level
    let mut levels: Vec<Vec<String>> = Vec::new();
    let mut visited: HashMap<String, usize> = HashMap::new();

    let mut queue: Vec<(String, usize)> = Vec::new();
    for root in roots {
        queue.push((root.clone(), 0));
    }

    while let Some((node, depth)) = queue.first().cloned() {
        queue.remove(0);
        if visited.contains_key(&node) {
            continue;
        }
        visited.insert(node.clone(), depth);
        while levels.len() <= depth {
            levels.push(Vec::new());
        }
        levels[depth].push(node.clone());
        if let Some(children) = children_map.get(&node) {
            for child in children {
                if !visited.contains_key(child) {
                    queue.push((child.clone(), depth + 1));
                }
            }
        }
    }

    let num_levels = levels.len().max(1);
    let level_height = area_height / num_levels as f32;

    let mut layout_nodes = Vec::new();
    for (depth, level) in levels.iter().enumerate() {
        let n = level.len();
        let spacing = area_width / (n + 1) as f32;
        for (i, node) in level.iter().enumerate() {
            layout_nodes.push(LayoutNode {
                label: node.clone(),
                x: spacing * (i + 1) as f32,
                y: level_height * depth as f32 + level_height * 0.5,
                depth,
            });
        }
    }

    layout_nodes
}

// ─── Reveal ─────────────────────────────────────────────────────────────────

/// Reveal steps: root declarations are static and each edge carries its own
/// marker. Returns each edge's step and each node's, which is the earliest
/// step of anything that introduces it.
fn reveal_steps(roots: &[String], edges: &[OrgEdge]) -> (Vec<usize>, HashMap<String, usize>) {
    let reveals: Vec<VizReveal> = roots
        .iter()
        .map(|_| VizReveal::Static)
        .chain(edges.iter().map(|e| e.reveal))
        .collect();
    let steps = assign_steps(&reveals);
    let edge_steps = steps[roots.len()..].to_vec();

    let mut node_step: HashMap<String, usize> = HashMap::new();
    let mut introduce = |name: &String, step: usize| {
        node_step
            .entry(name.clone())
            .and_modify(|s| *s = (*s).min(step))
            .or_insert(step);
    };
    for (root, &step) in roots.iter().zip(&steps) {
        introduce(root, step);
    }
    for (edge, &step) in edges.iter().zip(&edge_steps) {
        introduce(&edge.parent, step);
        introduce(&edge.child, step);
    }
    (edge_steps, node_step)
}

// ─── Renderer ───────────────────────────────────────────────────────────────

/// Where each node sits on the slide and how big its box is.
struct Placed {
    centers: HashMap<String, (f32, f32)>,
    sizes: HashMap<String, (f32, f32)>,
}

pub fn draw_org_chart(
    cx: &VizCtx,
    content: &str,
    pos: Pos2,
    max_width: f32,
    max_height: f32,
) -> f32 {
    let scale = cx.scale;
    let (roots, edges) = parse_org_chart(content);
    if roots.is_empty() && edges.is_empty() {
        return 0.0;
    }

    let height = if max_height > 0.0 {
        max_height
    } else {
        500.0 * scale
    };

    let (edge_steps, node_step) = reveal_steps(&roots, &edges);
    let label_font = cx.font(VIZ_FONT_PRIMARY_LABEL);

    let padding = 40.0 * scale;
    let layout = build_layout(
        &roots,
        &edges,
        max_width - padding * 2.0,
        height - padding * 2.0,
    );

    // Boxes sized to their labels
    let painter = cx.ui.painter();
    let min_node_w = 120.0 * scale;
    let node_h_padding = 16.0 * scale;
    let node_w_padding = 24.0 * scale;
    let mut placed = Placed {
        centers: HashMap::new(),
        sizes: HashMap::new(),
    };
    for node in &layout {
        placed.centers.insert(
            node.label.clone(),
            (pos.x + padding + node.x, pos.y + padding + node.y),
        );
        let galley = painter.layout_no_wrap(node.label.clone(), label_font.clone(), cx.fg(1.0));
        let w = (galley.rect.width() + node_w_padding * 2.0).max(min_node_w);
        let h = galley.rect.height() + node_h_padding * 2.0;
        placed.sizes.insert(node.label.clone(), (w, h));
    }

    // Edges first (behind nodes)
    for (edge, &step) in edges.iter().zip(&edge_steps) {
        if step <= cx.reveal_step {
            draw_edge(cx, edge, &placed, cx.anim(step));
        }
    }

    let palette = cx.theme.edge_palette();
    let default_size = (min_node_w, 40.0 * scale);
    for node in &layout {
        let step = node_step.get(&node.label).copied().unwrap_or(0);
        if step > cx.reveal_step {
            continue;
        }
        let anim = cx.anim(step);
        let (nx, ny) = placed
            .centers
            .get(&node.label)
            .copied()
            .unwrap_or((0.0, 0.0));
        let (node_w, node_h) = placed
            .sizes
            .get(&node.label)
            .copied()
            .unwrap_or(default_size);
        let size = egui::vec2(node_w, node_h);
        let color = palette[node.depth % palette.len()];
        draw_node(
            cx,
            &node.label,
            Pos2::new(nx, ny),
            size,
            color,
            &label_font,
            anim,
        );
    }

    height
}

/// A right-angle connector from the parent's bottom to the child's top.
fn draw_edge(cx: &VizCtx, edge: &OrgEdge, placed: &Placed, anim: f32) {
    let (Some(&(px, py)), Some(&(ccx, cy))) = (
        placed.centers.get(&edge.parent),
        placed.centers.get(&edge.child),
    ) else {
        return;
    };
    let scale = cx.scale;
    let painter = cx.ui.painter();
    let edge_color = Theme::with_opacity(cx.theme.foreground, cx.opacity * 0.3 * anim);
    let stroke = Stroke::new(VIZ_STROKE_SEPARATOR * scale, edge_color);
    let box_h = |name: &String| placed.sizes.get(name).map(|s| s.1).unwrap_or(40.0 * scale);

    let p_bottom = py + box_h(&edge.parent) / 2.0;
    let c_top = cy - box_h(&edge.child) / 2.0;
    let mid_y = (p_bottom + c_top) / 2.0;

    painter.line_segment([Pos2::new(px, p_bottom), Pos2::new(px, mid_y)], stroke);
    painter.line_segment([Pos2::new(px, mid_y), Pos2::new(ccx, mid_y)], stroke);
    painter.line_segment([Pos2::new(ccx, mid_y), Pos2::new(ccx, c_top)], stroke);
}

/// A node: a tinted box with a border and its label centred on `center`.
fn draw_node(
    cx: &VizCtx,
    label: &str,
    center: Pos2,
    size: egui::Vec2,
    color: Color32,
    font: &FontId,
    anim: f32,
) {
    let VizCtx { opacity, scale, .. } = *cx;
    let painter = cx.ui.painter();
    let corner_radius = VIZ_CORNER_NODE * scale;
    let bg_color = Theme::with_opacity(color, opacity * 0.15 * anim);
    let border_color = Theme::with_opacity(color, opacity * 0.6 * anim);
    let rect = egui::Rect::from_center_size(center, size);

    painter.rect_filled(rect, corner_radius, bg_color);
    crate::render::hints::push(cx.ui.ctx(), crate::render::hints::Hint::Frame(rect));
    painter.rect_stroke(
        rect,
        corner_radius,
        Stroke::new(VIZ_STROKE_BORDER * scale, border_color),
        egui::StrokeKind::Outside,
    );

    let text_color = Theme::with_opacity(cx.theme.foreground, opacity * anim);
    let galley = painter.layout_no_wrap(label.to_string(), font.clone(), text_color);
    let tx = center.x - galley.rect.width() / 2.0;
    let ty = center.y - galley.rect.height() / 2.0;
    painter.galley(Pos2::new(tx, ty), galley, text_color);
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_org_chart_basic() {
        let content = "- CEO\n- CEO -> CTO\n- CEO -> CFO";
        let (roots, edges) = parse_org_chart(content);
        assert_eq!(roots, vec!["CEO"]);
        assert_eq!(edges.len(), 2);
        assert_eq!(edges[0].parent, "CEO");
        assert_eq!(edges[0].child, "CTO");
        assert_eq!(edges[1].parent, "CEO");
        assert_eq!(edges[1].child, "CFO");
    }

    #[test]
    fn test_parse_org_chart_implicit_root() {
        let content = "- CEO -> CTO\n- CEO -> CFO";
        let (roots, edges) = parse_org_chart(content);
        // CEO is never a child, so it becomes root
        assert_eq!(roots, vec!["CEO"]);
        assert_eq!(edges.len(), 2);
    }

    #[test]
    fn test_parse_org_chart_reveal_markers() {
        let content = "- CEO\n- CEO -> CTO\n+ CTO -> VP Engineering";
        let (_, edges) = parse_org_chart(content);
        assert_eq!(edges[0].reveal, VizReveal::Static);
        assert_eq!(edges[1].reveal, VizReveal::NextStep);
    }

    #[test]
    fn test_parse_org_chart_skips_comments() {
        let content = "# header\n- CEO\n# note\n- CEO -> CTO";
        let (roots, edges) = parse_org_chart(content);
        assert_eq!(roots, vec!["CEO"]);
        assert_eq!(edges.len(), 1);
    }

    #[test]
    fn test_build_layout_depths() {
        let roots = vec!["CEO".to_string()];
        let edges = vec![
            OrgEdge {
                parent: "CEO".to_string(),
                child: "CTO".to_string(),
                reveal: VizReveal::Static,
            },
            OrgEdge {
                parent: "CTO".to_string(),
                child: "VP".to_string(),
                reveal: VizReveal::Static,
            },
        ];
        let layout = build_layout(&roots, &edges, 800.0, 600.0);
        assert_eq!(layout.len(), 3);
        let ceo = layout.iter().find(|n| n.label == "CEO").unwrap();
        let cto = layout.iter().find(|n| n.label == "CTO").unwrap();
        let vp = layout.iter().find(|n| n.label == "VP").unwrap();
        assert_eq!(ceo.depth, 0);
        assert_eq!(cto.depth, 1);
        assert_eq!(vp.depth, 2);
    }

    #[test]
    fn test_reveal_steps_introduce_nodes_at_their_first_edge() {
        let (roots, edges) = parse_org_chart("- CEO\n+ CEO -> CTO\n+ CTO -> Dev\n+ CEO -> CFO");
        let (edge_steps, node_step) = reveal_steps(&roots, &edges);
        assert_eq!(edge_steps, vec![1, 2, 3]);
        assert_eq!(node_step["CEO"], 0);
        assert_eq!(node_step["CTO"], 1);
        assert_eq!(node_step["Dev"], 2);
        assert_eq!(node_step["CFO"], 3);
    }
}
