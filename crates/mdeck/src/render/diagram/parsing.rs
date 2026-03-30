use std::collections::HashMap;

use super::types::*;

// ─── Diagram parser ──────────────────────────────────────────────────────────

/// Parse parenthetical metadata like `(icon: database, pos: 1,2, prompt: "...")`.
/// Returns the line content without the metadata and extracted fields.
pub(super) fn parse_node_metadata(s: &str) -> NodeMetadata<'_> {
    let trimmed = s.trim_end();
    if !trimmed.ends_with(')') {
        return NodeMetadata {
            before: trimmed,
            icon: String::new(),
            grid_pos: None,
            prompt: None,
        };
    }
    let Some(paren_start) = trimmed.rfind('(') else {
        return NodeMetadata {
            before: trimmed,
            icon: String::new(),
            grid_pos: None,
            prompt: None,
        };
    };
    // Only parse if there's whitespace before the paren
    if paren_start == 0 || trimmed.as_bytes()[paren_start - 1] != b' ' {
        return NodeMetadata {
            before: trimmed,
            icon: String::new(),
            grid_pos: None,
            prompt: None,
        };
    }

    let before = trimmed[..paren_start].trim_end();
    let meta_str = &trimmed[paren_start + 1..trimmed.len() - 1]; // contents between parens

    let mut icon = String::new();
    let mut grid_pos = None;
    let mut prompt = None;

    // Extract quoted prompt first (it may contain commas)
    let meta_str = extract_prompt(meta_str, &mut prompt);

    for part in meta_str.split(',') {
        let part = part.trim();
        if let Some(val) = part
            .strip_prefix("icon:")
            .or_else(|| part.strip_prefix("icon :"))
        {
            icon = val.trim().to_string();
        } else if let Some(val) = part
            .strip_prefix("pos:")
            .or_else(|| part.strip_prefix("pos :"))
        {
            let val = val.trim();
            // pos can be "x,y" but we already split on comma, so handle both forms
            if let Some((x_str, y_str)) = val.split_once(',') {
                if let (Ok(x), Ok(y)) = (x_str.trim().parse(), y_str.trim().parse()) {
                    grid_pos = Some((x, y));
                }
            } else if grid_pos.is_none() {
                // Might be split across commas: "pos: 1" then next part is "2"
                // Store x and look for y in next iteration
                if let Ok(x) = val.parse::<u32>() {
                    grid_pos = Some((x, 0)); // placeholder, y filled below
                }
            }
        } else if let Some((x, 0)) = grid_pos {
            // Continuation of pos value split by comma
            if let Ok(y) = part.trim().parse::<u32>() {
                grid_pos = Some((x, y));
            }
        }
    }

    NodeMetadata {
        before,
        icon,
        grid_pos,
        prompt,
    }
}

/// Extract a `prompt: "..."` or `prompt: '...'` value from the metadata string,
/// returning the remainder with the prompt portion removed.
pub(super) fn extract_prompt(meta_str: &str, prompt: &mut Option<String>) -> String {
    // Look for prompt: followed by a quoted string
    let prefix = if let Some(idx) = meta_str.find("prompt:") {
        idx
    } else if let Some(idx) = meta_str.find("prompt :") {
        idx
    } else {
        return meta_str.to_string();
    };

    let after_key = &meta_str[prefix..];
    let after_colon = after_key
        .strip_prefix("prompt:")
        .or_else(|| after_key.strip_prefix("prompt :"))
        .unwrap_or(after_key);
    let after_colon = after_colon.trim_start();

    let (quote_char, rest) = if let Some(stripped) = after_colon.strip_prefix('"') {
        ('"', stripped)
    } else if let Some(stripped) = after_colon.strip_prefix('\'') {
        ('\'', stripped)
    } else {
        return meta_str.to_string();
    };

    if let Some(end) = rest.find(quote_char) {
        *prompt = Some(rest[..end].to_string());
        // Remove the prompt portion from the metadata string
        let prompt_end = prefix + "prompt:".len() + (after_colon.len() - rest.len()) + end + 1;
        let mut result = meta_str[..prefix].to_string();
        if prompt_end < meta_str.len() {
            result.push_str(&meta_str[prompt_end..]);
        }
        result
    } else {
        meta_str.to_string()
    }
}

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
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut seen_nodes: HashMap<String, usize> = HashMap::new();
    let mut diagram_scale = DiagramScale::Fit;
    let mut parse_order_counter = 0usize;

    for line in content.lines() {
        let trimmed = line.trim();

        // Parse directives from comment lines (e.g. `# scale: fit`)
        if trimmed.starts_with('#') {
            if let Some(rest) = trimmed
                .strip_prefix("# scale:")
                .or_else(|| trimmed.strip_prefix("#scale:"))
            {
                let val = rest.trim();
                if val.eq_ignore_ascii_case("fit") {
                    diagram_scale = DiagramScale::Fit;
                } else if val.eq_ignore_ascii_case("scroll") {
                    diagram_scale = DiagramScale::Scroll;
                } else if let Ok(f) = val.parse::<f32>() {
                    diagram_scale = DiagramScale::Factor(f.clamp(0.1, 2.0));
                }
            }
            continue;
        }

        // Strip list-style prefixes and record reveal marker
        let (trimmed, reveal) = if let Some(rest) = trimmed.strip_prefix("+ ") {
            (rest, DiagramReveal::NextStep)
        } else if let Some(rest) = trimmed.strip_prefix("* ") {
            (rest, DiagramReveal::WithPrev)
        } else if let Some(rest) = trimmed.strip_prefix("- ") {
            (rest, DiagramReveal::Static)
        } else {
            (trimmed, DiagramReveal::Static)
        };

        if trimmed.is_empty() {
            continue;
        }

        // Parse and strip trailing metadata (icon, pos, prompt)
        let meta = parse_node_metadata(trimmed);
        let trimmed = meta.before;
        let meta_icon = meta.icon;
        let meta_pos = meta.grid_pos;
        let meta_prompt = meta.prompt;

        if let Some((arrow_pos, arrow_len, arrow_kind)) = detect_arrow(trimmed) {
            let from = trimmed[..arrow_pos].trim().to_string();
            let rest = &trimmed[arrow_pos + arrow_len..];
            let (to, label) = if let Some(colon_pos) = rest.find(": ") {
                (
                    rest[..colon_pos].trim().to_string(),
                    rest[colon_pos + 2..].trim().to_string(),
                )
            } else {
                (rest.trim().to_string(), String::new())
            };

            // Auto-create nodes for edges if not already declared
            for node_name in [&from, &to] {
                if !seen_nodes.contains_key(node_name) {
                    seen_nodes.insert(node_name.clone(), nodes.len());
                    nodes.push(DiagramNode {
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

            edges.push(DiagramEdge {
                from,
                to,
                label,
                arrow: arrow_kind,
                reveal,
                parse_order: parse_order_counter,
            });
            parse_order_counter += 1;
        } else if let Some(colon_pos) = trimmed.find(": ") {
            // Node declaration with label: "Name: Label"
            let name = trimmed[..colon_pos].trim().to_string();
            let label = trimmed[colon_pos + 2..].trim().to_string();

            if let Some(&idx) = seen_nodes.get(&name) {
                nodes[idx].label = label;
                if !meta_icon.is_empty() {
                    nodes[idx].icon = meta_icon.clone();
                }
                if meta_pos.is_some() {
                    nodes[idx].grid_pos = meta_pos;
                }
                if meta_prompt.is_some() {
                    nodes[idx].prompt = meta_prompt.clone();
                }
                nodes[idx].parse_order = parse_order_counter;
            } else {
                seen_nodes.insert(name.clone(), nodes.len());
                nodes.push(DiagramNode {
                    name,
                    label,
                    icon: meta_icon.clone(),
                    grid_pos: meta_pos,
                    prompt: meta_prompt.clone(),
                    reveal,
                    parse_order: parse_order_counter,
                });
            }
            parse_order_counter += 1;
        } else {
            // Plain node name (e.g. "Server" or "Server (icon: server, pos: 1,1)")
            let name = trimmed.trim().to_string();
            if !name.is_empty() {
                if let Some(&idx) = seen_nodes.get(&name) {
                    if !meta_icon.is_empty() {
                        nodes[idx].icon = meta_icon.clone();
                    }
                    if meta_pos.is_some() {
                        nodes[idx].grid_pos = meta_pos;
                    }
                    if meta_prompt.is_some() {
                        nodes[idx].prompt = meta_prompt.clone();
                    }
                    nodes[idx].parse_order = parse_order_counter;
                } else {
                    seen_nodes.insert(name.clone(), nodes.len());
                    nodes.push(DiagramNode {
                        name: name.clone(),
                        label: name,
                        icon: meta_icon.clone(),
                        grid_pos: meta_pos,
                        prompt: meta_prompt.clone(),
                        reveal,
                        parse_order: parse_order_counter,
                    });
                }
                parse_order_counter += 1;
            }
        }
    }

    (nodes, edges, diagram_scale)
}
