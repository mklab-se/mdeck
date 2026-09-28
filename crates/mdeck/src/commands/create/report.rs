//! What `ai create` does after writing the deck: generate its images and
//! point at visualizations mdeck does not have yet.

use std::path::Path;

use anyhow::Result;
use colored::Colorize;

use super::APP_NAME;
use super::opportunities::{VisualizationOpportunity, write_opportunities};
use crate::commands::ai;

/// "" for one, "s" for more.
fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

/// Generate the deck's `(image-generation)` images when an image provider is
/// configured; otherwise say how to do it later.
pub(super) async fn generate_images(
    output_file: &Path,
    presentation_md: &str,
    style: &Option<String>,
    quiet: bool,
) -> Result<()> {
    let image_count = presentation_md.matches("(image-generation)").count();
    if image_count == 0 {
        return Ok(());
    }
    if ai::has_capability("image") {
        if !quiet {
            eprintln!();
            eprintln!(
                "  {} Generating {} image{}...",
                "ℹ".blue().bold(),
                image_count,
                plural(image_count)
            );
        }
        // Run generate with quiet=true to suppress inline image display in terminal
        crate::commands::generate::run(output_file.to_path_buf(), true, style.clone(), true)
            .await?;
        if !quiet {
            eprintln!(
                "  {} {} image{} generated.",
                "✓".green().bold(),
                image_count,
                plural(image_count)
            );
        }
    } else if !quiet {
        eprintln!(
            "  {} {} image{} marked but no image provider configured.",
            "ℹ".blue().bold(),
            image_count,
            plural(image_count)
        );
        eprintln!("    Run `{APP_NAME} ai config` to add an image provider, then:");
        eprintln!(
            "    {}",
            format!("mdeck ai generate {}", output_file.display()).cyan()
        );
    }
    Ok(())
}

/// Write the opportunities next to the deck and invite the user to share them.
pub(super) fn report_opportunities(
    output_dir: &Path,
    opportunities: &[VisualizationOpportunity],
) -> Result<()> {
    let opp_file = output_dir.join("visualization-opportunities.md");
    write_opportunities(&opp_file, opportunities)?;
    let one = opportunities.len() == 1;
    eprintln!();
    eprintln!(
        "  {} This presentation could be even better.",
        "!".yellow().bold(),
    );
    eprintln!(
        "    MDeck identified {} visualization{} that would enhance the slides",
        opportunities.len(),
        plural(opportunities.len()),
    );
    eprintln!(
        "    but {} not yet supported.",
        if one { "is" } else { "are" }
    );
    eprintln!();
    eprintln!(
        "    The file {} contains detailed feature request{}",
        opp_file.display().to_string().cyan(),
        plural(opportunities.len()),
    );
    eprintln!(
        "    ready to be copied into a GitHub issue. By sharing {} you help",
        if one { "it," } else { "them," }
    );
    eprintln!("    yourself and the MDeck community.");
    eprintln!();
    eprintln!(
        "    {}",
        "https://github.com/mklab-se/mdeck/issues/new".cyan()
    );
    Ok(())
}
