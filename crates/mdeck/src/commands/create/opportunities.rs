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

/// Parse a single opportunity JSON object (tests in the parent module).
#[cfg(test)]
pub fn parse_opportunity_for_test(json: &str) -> Option<VisualizationOpportunity> {
    serde_json::from_str::<VisualizationOpportunity>(json)
        .ok()
        .and_then(VisualizationOpportunity::reportable)
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
