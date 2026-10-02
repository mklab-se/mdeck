use std::collections::HashMap;

use super::types::*;
use crate::render::visualizations::VizReveal;
use crate::render::visualizations::grammar::{Arrow, Item, Problem, Source};

// ─── Diagram parser ──────────────────────────────────────────────────────────

/// The attributes a component takes.
const NODE_ATTRS: &[&str] = &["icon", "pos", "prompt"];

pub(super) fn parse_diagram(content: &str) -> (Vec<DiagramNode>, Vec<DiagramEdge>, DiagramScale) {
    let src = Source::parse(content);
    let parser = read(&src);
    (parser.nodes, parser.edges, parser.scale)
}

/// The problems in an `@architecture` block.
pub fn check(content: &str) -> Vec<Problem> {
    let src = Source::parse(content);
    read(&src);
    src.into_problems()
}

fn read(src: &Source) -> Parser {
    src.check_settings(&["scale"]);
    let mut parser = Parser::default();
    if let Some(s) = src.setting_line("scale") {
        match scale_setting(s.value) {
            Some(scale) => parser.scale = scale,
            None => src.problem(
                s.offset,
                format!(
                    "scale: '{}' is not fit, scroll or a factor like 0.7",
                    s.value
                ),
            ),
        }
    }
    for item in &src.items {
        parser.item(src, item);
    }
    parser
}

/// `fit`, `scroll` or a factor (clamped to 0.1 to 2).
fn scale_setting(val: &str) -> Option<DiagramScale> {
    if val.eq_ignore_ascii_case("fit") {
        Some(DiagramScale::Fit)
    } else if val.eq_ignore_ascii_case("scroll") {
        Some(DiagramScale::Scroll)
    } else {
        let f = val.parse::<f32>().ok().filter(|f| f.is_finite())?;
        Some(DiagramScale::Factor(f.clamp(0.1, 2.0)))
    }
}

fn reveal_of(reveal: VizReveal) -> DiagramReveal {
    match reveal {
        VizReveal::Static => DiagramReveal::Static,
        VizReveal::NextStep => DiagramReveal::NextStep,
        VizReveal::WithPrev => DiagramReveal::WithPrev,
    }
}

fn arrow_kind(arrow: Arrow) -> ArrowKind {
    match arrow {
        Arrow::Forward => ArrowKind::Forward,
        Arrow::Reverse => ArrowKind::Reverse,
        Arrow::Both => ArrowKind::Bidirectional,
        Arrow::Line => ArrowKind::DashedLine,
        Arrow::Dashed => ArrowKind::DashedArrow,
    }
}

/// A component's attributes as read.
struct NodeMetadata {
    icon: String,
    grid_pos: Option<(u32, u32)>,
    prompt: Option<String>,
}

impl NodeMetadata {
    fn read(src: &Source, item: &Item) -> Self {
        item.check_attrs(src, NODE_ATTRS);
        let grid_pos = item.attr("pos").and_then(|p| {
            let pos = p
                .split_once(',')
                .and_then(|(x, y)| Some((x.trim().parse().ok()?, y.trim().parse().ok()?)));
            if pos.is_none() {
                src.problem(
                    item.offset,
                    format!("pos: '{p}' is not a grid position like 2,1"),
                );
            }
            pos
        });
        NodeMetadata {
            icon: item.attr("icon").unwrap_or_default().to_string(),
            grid_pos,
            prompt: item.attr("prompt").map(str::to_string),
        }
    }
}

/// Diagram state built up item by item.
struct Parser {
    nodes: Vec<DiagramNode>,
    edges: Vec<DiagramEdge>,
    seen_nodes: HashMap<String, usize>,
    scale: DiagramScale,
    parse_order: usize,
}

impl Default for Parser {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            seen_nodes: HashMap::new(),
            scale: DiagramScale::Fit,
            parse_order: 0,
        }
    }
}

impl Parser {
    fn item(&mut self, src: &Source, item: &Item) {
        let reveal = reveal_of(item.reveal);
        if item.text.is_empty() {
            src.problem(item.offset, "an empty item");
            return;
        }
        if let Some(rel) = item.relation(&Arrow::ALL) {
            item.check_attrs(src, &[]);
            self.edge(
                rel.from.to_string(),
                rel.to.to_string(),
                rel.label.unwrap_or_default().to_string(),
                arrow_kind(rel.arrow),
                reveal,
            );
            return;
        }
        let meta = NodeMetadata::read(src, item);
        // "Name: Label" or "Name"
        let (name, label) = match item.text.split_once(':') {
            Some((n, l)) => (n.trim(), Some(l.trim()).filter(|l| !l.is_empty())),
            None => (item.text, None),
        };
        self.node(name.to_string(), label.map(str::to_string), &meta, reveal);
    }

    fn edge(
        &mut self,
        from: String,
        to: String,
        label: String,
        arrow: ArrowKind,
        reveal: DiagramReveal,
    ) {
        // Auto-create nodes for edges if not already declared
        for node_name in [&from, &to] {
            if !self.seen_nodes.contains_key(node_name) {
                self.seen_nodes.insert(node_name.clone(), self.nodes.len());
                self.nodes.push(DiagramNode {
                    name: node_name.clone(),
                    label: node_name.clone(),
                    icon: String::new(),
                    grid_pos: None,
                    prompt: None,
                    reveal: DiagramReveal::Static,
                    parse_order: 0,
                });
            }
        }

        self.edges.push(DiagramEdge {
            from,
            to,
            label,
            arrow,
            reveal,
            parse_order: self.parse_order,
        });
        self.parse_order += 1;
    }

    /// Declare a node, or update one already declared (or created by an
    /// edge): a given label and any metadata replace what it had, and it
    /// moves to this line in file order. Its reveal marker stays the first.
    fn node(
        &mut self,
        name: String,
        label: Option<String>,
        meta: &NodeMetadata,
        reveal: DiagramReveal,
    ) {
        if let Some(&idx) = self.seen_nodes.get(&name) {
            let node = &mut self.nodes[idx];
            if let Some(label) = label {
                node.label = label;
            }
            if !meta.icon.is_empty() {
                node.icon = meta.icon.clone();
            }
            if meta.grid_pos.is_some() {
                node.grid_pos = meta.grid_pos;
            }
            if meta.prompt.is_some() {
                node.prompt = meta.prompt.clone();
            }
            node.parse_order = self.parse_order;
        } else {
            self.seen_nodes.insert(name.clone(), self.nodes.len());
            self.nodes.push(DiagramNode {
                label: label.unwrap_or_else(|| name.clone()),
                name,
                icon: meta.icon.clone(),
                grid_pos: meta.grid_pos,
                prompt: meta.prompt.clone(),
                reveal,
                parse_order: self.parse_order,
            });
        }
        self.parse_order += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(check(content).is_empty());
    }

    #[test]
    fn test_arrow_types() {
        let content = "- A -> B\n- C <- D\n- E <-> F\n- G -- H\n- I --> J";
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
    fn test_node_with_quoted_prompt() {
        let content =
            "- Gateway (icon: generate-image, prompt: \"A router, with (parens)\", pos: 1,2)";
        let (nodes, _, _) = parse_diagram(content);
        assert_eq!(nodes[0].prompt.as_deref(), Some("A router, with (parens)"));
        assert_eq!(nodes[0].grid_pos, Some((1, 2)));
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
        let (_, edges, _) = parse_diagram("- A <- B");
        assert!(matches!(edges[0].arrow, ArrowKind::Reverse));
        assert_eq!(edges[0].from, "A");
        assert_eq!(edges[0].to, "B");
    }

    #[test]
    fn lines_that_are_not_items_are_reported_not_drawn() {
        let content = "A -> B\n- C (colour: red, pos: x)\n- D";
        let (nodes, edges, _) = parse_diagram(content);
        assert!(edges.is_empty(), "a bare line is not an item");
        assert_eq!(nodes.len(), 2);
        let lines: Vec<usize> = check(content).iter().map(|p| p.offset).collect();
        assert_eq!(lines, [0, 1, 1]);
    }

    // ── Scale setting tests ──────────────────────────────────────────────────

    #[test]
    fn test_scale_setting_default() {
        let (_, _, scale) = parse_diagram("- A -> B");
        assert_eq!(scale, DiagramScale::Fit);
    }

    #[test]
    fn test_scale_settings() {
        let (_, _, scale) = parse_diagram("scale: fit\n- A -> B");
        assert_eq!(scale, DiagramScale::Fit);
        let (_, _, scale) = parse_diagram("scale: scroll\n- A -> B");
        assert_eq!(scale, DiagramScale::Scroll);
        let (_, _, scale) = parse_diagram("scale: 0.7\n- A -> B");
        assert!(matches!(scale, DiagramScale::Factor(f) if (f - 0.7).abs() < 0.001));
        let (_, _, scale) = parse_diagram("scale: 5.0\n- A -> B");
        assert!(matches!(scale, DiagramScale::Factor(f) if (f - 2.0).abs() < 0.001));
        assert_eq!(check("scale: huge\n- A")[0].offset, 0);
    }

    #[test]
    fn test_v1_commented_scale_is_a_comment_and_reported() {
        let (_, _, scale) = parse_diagram("# scale: scroll\n- A -> B");
        assert_eq!(scale, DiagramScale::Fit);
        assert!(
            check("# scale: scroll\n- A -> B")[0]
                .message
                .contains("scale: scroll")
        );
    }
}
