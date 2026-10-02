//! Visualization opportunity extraction and reporting.

use std::path::Path;

use anyhow::{Context, Result};

/// A structured visualization opportunity extracted from the AI outline.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct VisualizationOpportunity {
    pub visualization_name: String,
    pub description: String,
    pub data_description: String,
    pub rendering_description: String,
    pub suggested_syntax: String,
    pub ascii_mockup: String,
}

impl VisualizationOpportunity {
    /// Named and described well enough to report; an unnamed one is "Unknown".
    fn reportable(mut self) -> Option<Self> {
        if self.visualization_name.is_empty() && self.description.is_empty() {
            return None;
        }
        if self.visualization_name.is_empty() {
            self.visualization_name = "Unknown".to_string();
        }
        Some(self)
    }
}

/// Extract visualization opportunities from the AI outline: the JSON array
/// under `"opportunities"`, wherever it sits in the reply. Anything that does
/// not parse yields no opportunities rather than an error.
pub fn extract_opportunities(outline: &str) -> Vec<VisualizationOpportunity> {
    let Some(key) = outline.find("\"opportunities\"") else {
        return Vec::new();
    };
    let Some(open) = outline[key..].find('[') else {
        return Vec::new();
    };
    // Read exactly one array; whatever follows it is ignored.
    serde_json::Deserializer::from_str(&outline[key + open..])
        .into_iter::<Vec<VisualizationOpportunity>>()
        .next()
        .and_then(Result::ok)
        .unwrap_or_default()
        .into_iter()
        .filter_map(VisualizationOpportunity::reportable)
        .collect()
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
             (e.g., `@bar`, `@timeline`, `@architecture`). Each visualization type \
             is implemented as a Rust rendering function in `crates/mdeck/src/render/`. \
             The parser detects the `@` tag in `crates/mdeck/src/language/mod.rs` (FENCES) and `crates/mdeck/src/parser/model.rs` (`Chart::TAGS`) and \
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_opportunities_empty() {
        let outline = r#"{"slides": [], "opportunities": []}"#;
        assert!(extract_opportunities(outline).is_empty());
    }

    #[test]
    fn test_extract_opportunities_found() {
        let outline = r#"{
            "opportunities": [
                {
                    "visualization_name": "Swimlane Diagram",
                    "description": "Shows cross-team workflow with parallel lanes",
                    "data_description": "Teams as horizontal lanes with tasks flowing between them",
                    "rendering_description": "Horizontal lanes with arrows between them",
                    "suggested_syntax": "- Marketing -> Engineering: handoff",
                    "ascii_mockup": "| Marketing | --> | Engineering | --> | QA |"
                }
            ]
        }"#;
        let opps = extract_opportunities(outline);
        assert_eq!(opps.len(), 1);
        assert_eq!(opps[0].visualization_name, "Swimlane Diagram");
        assert!(opps[0].description.contains("cross-team"));
        assert!(!opps[0].ascii_mockup.is_empty());
    }

    #[test]
    fn test_extract_opportunities_multiple() {
        let outline = r#"{
            "opportunities": [
                {
                    "visualization_name": "Swimlane",
                    "description": "Cross-team flow"
                },
                {
                    "visualization_name": "Sankey",
                    "description": "Data flow volumes"
                }
            ]
        }"#;
        let opps = extract_opportunities(outline);
        assert_eq!(opps.len(), 2);
        assert_eq!(opps[0].visualization_name, "Swimlane");
        assert_eq!(opps[1].visualization_name, "Sankey");
    }

    #[test]
    fn test_extract_opportunities_json_escapes() {
        // The old hand-written scanner turned `\u00e5` into `u00e5` and cut a
        // value at an escaped quote followed by more text.
        let outline = r#"{"opportunities": [{"visualization_name": "R\u00e5 data",
            "description": "Say \"hi\" then go", "extra": 3}], "slides": []}"#;
        let opps = extract_opportunities(outline);
        assert_eq!(opps.len(), 1);
        assert_eq!(opps[0].visualization_name, "Rå data");
        assert_eq!(opps[0].description, "Say \"hi\" then go");
    }

    #[test]
    fn test_extract_opportunities_no_opportunities_key() {
        let outline = r#"{"slides": [{"title": "Intro"}]}"#;
        assert!(extract_opportunities(outline).is_empty());
    }

    #[test]
    fn test_parse_opportunity_full() {
        let json = r#"{
            "visualization_name": "Swimlane Diagram",
            "description": "Shows parallel workflows",
            "data_description": "Teams and tasks",
            "rendering_description": "Horizontal lanes with arrows",
            "suggested_syntax": "- Marketing -> Engineering: handoff",
            "ascii_mockup": "| Marketing | --> | Engineering |"
        }"#;
        let opp = serde_json::from_str::<VisualizationOpportunity>(json)
            .ok()
            .and_then(VisualizationOpportunity::reportable)
            .unwrap();
        assert_eq!(opp.visualization_name, "Swimlane Diagram");
        assert!(!opp.rendering_description.is_empty());
        assert!(!opp.ascii_mockup.is_empty());
    }
}
