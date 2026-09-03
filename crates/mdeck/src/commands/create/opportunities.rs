//! Visualization opportunity extraction and reporting.

use std::path::Path;

use anyhow::{Context, Result};

/// A structured visualization opportunity extracted from the AI outline.
#[derive(Debug, Clone)]
pub struct VisualizationOpportunity {
    pub visualization_name: String,
    pub description: String,
    pub data_description: String,
    pub rendering_description: String,
    pub suggested_syntax: String,
    pub ascii_mockup: String,
}

/// Extract visualization opportunities from the AI outline JSON.
pub fn extract_opportunities(outline: &str) -> Vec<VisualizationOpportunity> {
    let mut opportunities = Vec::new();

    // Find the "opportunities" array in the JSON
    let Some(start) = outline.find("\"opportunities\"") else {
        return opportunities;
    };
    let Some(arr_start) = outline[start..].find('[') else {
        return opportunities;
    };
    let arr_content = &outline[start + arr_start..];

    // Find matching closing bracket
    let mut depth = 0;
    let mut end = 0;
    for (i, c) in arr_content.char_indices() {
        match c {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    end = i + c.len_utf8();
                    break;
                }
            }
            _ => {}
        }
    }
    if end == 0 {
        return opportunities;
    }

    let arr_str = &arr_content[..end];

    // Parse individual opportunity objects
    let mut obj_depth = 0;
    let mut obj_start = None;
    for (i, c) in arr_str.char_indices() {
        match c {
            '{' => {
                if obj_depth == 0 {
                    obj_start = Some(i);
                }
                obj_depth += 1;
            }
            '}' => {
                obj_depth -= 1;
                if obj_depth == 0
                    && let Some(start) = obj_start
                {
                    let obj = &arr_str[start..i + c.len_utf8()];
                    if let Some(opp) = parse_opportunity(obj) {
                        opportunities.push(opp);
                    }
                }
            }
            _ => {}
        }
    }

    opportunities
}

/// Parse a single opportunity JSON object.
fn parse_opportunity(json: &str) -> Option<VisualizationOpportunity> {
    fn extract_field(json: &str, field: &str) -> String {
        let pattern = format!("\"{field}\"");
        let Some(pos) = json.find(&pattern) else {
            return String::new();
        };
        let after = &json[pos + pattern.len()..];
        // Skip `: "`
        let Some(quote_start) = after.find('"') else {
            return String::new();
        };
        let value_start = &after[quote_start + 1..];
        let mut result = String::new();
        let mut escaped = false;
        for c in value_start.chars() {
            if escaped {
                match c {
                    'n' => result.push('\n'),
                    't' => result.push('\t'),
                    'r' => result.push('\r'),
                    _ => result.push(c), // \", \\, etc.
                }
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                break;
            } else {
                result.push(c);
            }
        }
        result
    }

    let viz_name = extract_field(json, "visualization_name");
    let description = extract_field(json, "description");

    if description.is_empty() && viz_name.is_empty() {
        return None;
    }

    Some(VisualizationOpportunity {
        visualization_name: if viz_name.is_empty() {
            "Unknown".to_string()
        } else {
            viz_name
        },
        description,
        data_description: extract_field(json, "data_description"),
        rendering_description: extract_field(json, "rendering_description"),
        suggested_syntax: extract_field(json, "suggested_syntax"),
        ascii_mockup: extract_field(json, "ascii_mockup"),
    })
}

/// Expose `parse_opportunity` for tests in the parent module.
#[cfg(test)]
pub fn parse_opportunity_for_test(json: &str) -> Option<VisualizationOpportunity> {
    parse_opportunity(json)
}

/// Write visualization opportunities to a file in GitHub-issue-ready format.
/// If the file already exists, appends only new opportunities (by name) to avoid duplicates.
pub fn write_opportunities(path: &Path, opportunities: &[VisualizationOpportunity]) -> Result<()> {
    let header = "# Visualization Opportunities for MDeck\n\n\
         Each section below is a self-contained feature request ready to be submitted \
         as a GitHub issue. Copy the section you're interested in and paste it at:\n\
         https://github.com/mklab-se/mdeck/issues/new\n\n";

    // Read existing file to find already-listed opportunities and the next number
    let (mut content, mut next_number) = if path.exists() {
        let existing = std::fs::read_to_string(path).unwrap_or_default();
        // Count existing entries to continue numbering
        let count = existing.matches("## ").count();
        (existing, count + 1)
    } else {
        (header.to_string(), 1)
    };

    // Collect existing visualization names (lowercase, no spaces) to deduplicate
    let existing_lower = content.to_lowercase();

    let mut added = 0;
    for opp in opportunities {
        let tag = opp.visualization_name.to_lowercase().replace(' ', "");
        // Skip if this visualization type is already in the file
        if existing_lower.contains(&format!("`@{tag}`")) {
            continue;
        }

        content.push_str(&format!(
            "---\n\n## {next_number}. Feature Request: `@{tag}` Visualization\n\n"
        ));

        content.push_str("### Summary\n\n");
        content.push_str(&format!("{}\n\n", opp.description));

        if !opp.data_description.is_empty() {
            content.push_str("### Data Model\n\n");
            content.push_str(&format!("{}\n\n", opp.data_description));
        }

        if !opp.rendering_description.is_empty() {
            content.push_str("### Rendering Specification\n\n");
            content.push_str(&format!("{}\n\n", opp.rendering_description));
        }

        if !opp.ascii_mockup.is_empty() {
            content.push_str("### Visual Mockup\n\n```\n");
            content.push_str(&opp.ascii_mockup);
            content.push_str("\n```\n\n");
        }

        if !opp.suggested_syntax.is_empty() {
            content.push_str("### Proposed Syntax\n\n````markdown\n");
            content.push_str(&format!("```@{tag}\n"));
            content.push_str(&opp.suggested_syntax);
            content.push_str("\n```\n````\n\n");
        }

        content.push_str("### Implementation Notes\n\n");
        content.push_str(
            "MDeck renders visualizations from fenced code blocks with `@` language tags \
             (e.g., `@barchart`, `@timeline`, `@architecture`). Each visualization type \
             is implemented as a Rust rendering function in `crates/mdeck/src/render/`. \
             The parser detects the `@` tag in `crates/mdeck/src/parser/blocks.rs` and \
             creates a corresponding `Block` variant. Progressive reveal is supported \
             via `+` and `*` list markers.\n\n",
        );

        next_number += 1;
        added += 1;
    }

    if added > 0 {
        std::fs::write(path, content)
            .with_context(|| format!("Failed to write opportunities: {}", path.display()))?;
    }

    Ok(())
}
