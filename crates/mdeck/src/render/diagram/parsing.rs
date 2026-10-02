use std::collections::HashMap;

use super::metadata::parse_node_metadata;
use super::types::*;

// ─── Diagram parser ──────────────────────────────────────────────────────────

/// Detect arrow type and position in a line. Returns (arrow_pos, arrow_len, ArrowKind).
pub(super) fn detect_arrow(s: &str) -> Option<(usize, usize, ArrowKind)> {
    // Order matters: check longer patterns first to avoid partial matches
    if let Some(p) = s.find(" <-> ") {
        return Some((p, 5, ArrowKind::Bidirectional));
    }
    if let Some(p) = s.find(" --> ") {
        return Some((p, 5, ArrowKind::DashedArrow));
    }
    if let Some(p) = s.find(" -> ") {
        return Some((p, 4, ArrowKind::Forward));
    }
    if let Some(p) = s.find(" <- ") {
        return Some((p, 4, ArrowKind::Reverse));
    }
    if let Some(p) = s.find(" -- ") {
        return Some((p, 4, ArrowKind::DashedLine));
    }
    None
}

pub(super) fn parse_diagram(content: &str) -> (Vec<DiagramNode>, Vec<DiagramEdge>, DiagramScale) {
    let mut parser = Parser::default();
    for line in content.lines() {
        parser.line(line.trim());
    }
    (parser.nodes, parser.edges, parser.scale)
}

/// Parse a `# scale: fit | scroll | <factor>` directive line.
fn scale_directive(line: &str) -> Option<DiagramScale> {
    let val = line
        .strip_prefix("# scale:")
        .or_else(|| line.strip_prefix("#scale:"))?
        .trim();
    if val.eq_ignore_ascii_case("fit") {
        Some(DiagramScale::Fit)
    } else if val.eq_ignore_ascii_case("scroll") {
        Some(DiagramScale::Scroll)
    } else {
        let f = val.parse::<f32>().ok()?;
        Some(DiagramScale::Factor(f.clamp(0.1, 2.0)))
    }
}

/// Strip a list-style prefix and return the reveal marker it stands for.
fn split_reveal(line: &str) -> (&str, DiagramReveal) {
    if let Some(rest) = line.strip_prefix("+ ") {
        (rest, DiagramReveal::NextStep)
    } else if let Some(rest) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
        (rest, DiagramReveal::Static)
    } else {
        (line, DiagramReveal::Static)
    }
}

/// Diagram state built up line by line.
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
    fn line(&mut self, trimmed: &str) {
        // Parse directives from comment lines (e.g. `# scale: fit`)
        if trimmed.starts_with('#') {
            if let Some(scale) = scale_directive(trimmed) {
                self.scale = scale;
            }
            return;
        }

        let (trimmed, reveal) = split_reveal(trimmed);
        if trimmed.is_empty() {
            return;
        }

        // Parse and strip trailing metadata (icon, pos, prompt)
        let meta = parse_node_metadata(trimmed);
        let body = meta.before;

        if let Some((arrow_pos, arrow_len, arrow_kind)) = detect_arrow(body) {
            let from = body[..arrow_pos].trim().to_string();
            let (to, label) = split_label(&body[arrow_pos + arrow_len..]);
            self.edge(from, to, label, arrow_kind, reveal);
        } else if let Some(colon_pos) = body.find(": ") {
            // Node declaration with label: "Name: Label"
            let name = body[..colon_pos].trim().to_string();
            let label = body[colon_pos + 2..].trim().to_string();
            self.node(name, Some(label), &meta, reveal);
        } else {
            // Plain node name (e.g. "Server" or "Server (icon: server, pos: 1,1)")
            let name = body.trim().to_string();
            if !name.is_empty() {
                self.node(name, None, &meta, reveal);
            }
        }
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

/// Split `To: label` after an arrow into the target and its (maybe empty) label.
fn split_label(rest: &str) -> (String, String) {
    if let Some(colon_pos) = rest.find(": ") {
        (
            rest[..colon_pos].trim().to_string(),
            rest[colon_pos + 2..].trim().to_string(),
        )
    } else {
        (rest.trim().to_string(), String::new())
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
        assert_eq!(nodes[2].reveal, DiagramReveal::Static);
    }

    #[test]
    fn test_reveal_markers_on_edges() {
        let content = "- A -> B\n+ C -> D\n* E -> F";
        let (_, edges, _) = parse_diagram(content);
        assert_eq!(edges[0].reveal, DiagramReveal::Static);
        assert_eq!(edges[1].reveal, DiagramReveal::NextStep);
        assert_eq!(edges[2].reveal, DiagramReveal::Static);
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
}
