//! `mdeck ai deck`: create a presentation from content using AI.
//!
//! Accepts text, markdown, PDF, or DOCX input and generates a complete
//! mdeck-format presentation with speaker notes, visualizations, and
//! image generation markers.

mod extractors;
mod input;
mod interactive;
mod opportunities;
mod pipeline;
mod prompts;
mod report;
mod spinner;

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use colored::Colorize;

use crate::cli::DeckArgs;
use crate::commands::ai;

use self::interactive::run_interactive_chat;

const APP_NAME: &str = "mdeck";
/// Output file used when no `--output` is given and no name can be suggested.
const DEFAULT_OUTPUT: &str = "presentation.md";

// ── Entry point ─────────────────────────────────────────────────────────────

pub async fn run(args: DeckArgs, quiet: bool) -> Result<()> {
    if !ai::has_capability("chat") {
        anyhow::bail!(
            "Chat AI not configured. Run `{APP_NAME} ai config` to set up a chat provider."
        );
    }

    if !quiet {
        eprintln!("{}", "MDeck AI Presentation Creator".bold());
        eprintln!();
    }

    // Step 1: Resolve input content
    let Some((source_label, content)) = input::resolve_input(&args, quiet)? else {
        // No input provided at all: show the subcommand help and exit
        input::print_create_help()?;
        return Ok(());
    };
    if content.trim().is_empty() {
        anyhow::bail!("No content found in input. Please provide non-empty content.");
    }

    // Show input info for file/stdin sources, but not for text the user just typed
    if !quiet && source_label != input::TEXT_INPUT {
        eprintln!(
            "  {} {} ({} words)",
            "Input:".bold(),
            source_label,
            content.split_whitespace().count()
        );
    }

    let client = ailloy::Client::for_capability("chat")?;

    // Step 2: What the presentation should do, from a chat or from --prompt
    let context = gather_context(&client, &args, &content, quiet).await?;

    // Step 3: Determine output filename
    let output_file = choose_output_file(&client, args.output.as_deref(), &context, quiet).await?;
    let output_dir = output_file
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .to_path_buf();

    // Step 4: Confirmation: show what will be created and ask for approval
    if !confirm_generation(&output_file, args.interactive, quiet)? {
        return Ok(());
    }

    // Step 5: Generate the presentation
    let (presentation_md, opportunities) =
        pipeline::run_pipeline(&client, &content, &context, &args.style, quiet).await?;

    // Step 6: Write output
    write_presentation(&output_dir, &output_file, &presentation_md, quiet)?;

    // Step 7: Auto-generate images if image capability is available
    report::generate_images(&output_file, &presentation_md, &args.style, quiet).await?;

    if !quiet {
        eprintln!();
        eprintln!(
            "  Launch: {}",
            format!("mdeck {}", output_file.display()).cyan()
        );
    }

    // Step 8: Visualization opportunities, shown last as a warning
    if !opportunities.is_empty() && !quiet {
        report::report_opportunities(&output_dir, &opportunities)?;
    }

    Ok(())
}

/// The brief the deck is written to: shaped in an interactive chat, or
/// `--prompt` (or a sensible default) otherwise.
async fn gather_context(
    client: &ailloy::Client,
    args: &DeckArgs,
    content: &str,
    quiet: bool,
) -> Result<String> {
    if args.interactive {
        run_interactive_chat(client, content, args.prompt.as_deref(), quiet).await
    } else {
        Ok(args
            .prompt
            .clone()
            .unwrap_or_else(|| "General audience. Focus on key takeaways.".to_string()))
    }
}

/// An explicit `--output` is always respected (even `presentation.md`);
/// otherwise ask the AI for a name.
async fn choose_output_file(
    client: &ailloy::Client,
    output: Option<&Path>,
    context: &str,
    quiet: bool,
) -> Result<PathBuf> {
    let file = match output {
        Some(explicit) => resolve_output(explicit)?.0,
        None if !quiet => {
            let suggested = pipeline::suggest_filename(client, context).await?;
            resolve_output(Path::new(&suggested))?.0
        }
        None => resolve_output(Path::new(DEFAULT_OUTPUT))?.0,
    };
    Ok(file)
}

/// Show the file about to be written; in interactive mode, ask to go ahead.
/// Returns false when the user cancels.
fn confirm_generation(output_file: &Path, interactive: bool, quiet: bool) -> Result<bool> {
    if !quiet {
        eprintln!();
        eprintln!("{}", "  Ready to generate:".bold());
        eprintln!("    {} {}", "File:".bold(), output_file.display());
        eprintln!();
    }

    if interactive {
        eprint!("{} Proceed with generation? [Y/n] ", "?".green().bold());
        io::stderr().flush()?;
        let mut confirm = String::new();
        io::stdin().read_line(&mut confirm)?;
        let confirm = confirm.trim().to_lowercase();
        if confirm == "n" || confirm == "no" {
            eprintln!("{} Cancelled.", "!".yellow().bold());
            return Ok(false);
        }
    }
    Ok(true)
}

fn write_presentation(
    output_dir: &Path,
    output_file: &Path,
    presentation_md: &str,
    quiet: bool,
) -> Result<()> {
    std::fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "Failed to create output directory: {}",
            output_dir.display()
        )
    })?;

    std::fs::write(output_file, presentation_md)
        .with_context(|| format!("Failed to write output: {}", output_file.display()))?;

    if !quiet {
        eprintln!(
            "{} Presentation created: {}",
            "✓".green().bold(),
            output_file.display()
        );
    }
    Ok(())
}

// ── Output resolution ───────────────────────────────────────────────────────

/// Resolve the output path into (markdown_file, output_directory).
fn resolve_output(output: &Path) -> Result<(PathBuf, PathBuf)> {
    let output_str = output.to_string_lossy();

    if output_str.ends_with('/') || output_str.ends_with('\\') || output.is_dir() {
        let dir = output.to_path_buf();
        let file = dir.join("presentation.md");
        Ok((file, dir))
    } else if output
        .extension()
        .is_some_and(|ext| ext == "md" || ext == "markdown")
    {
        let dir = output
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."))
            .to_path_buf();
        Ok((output.to_path_buf(), dir))
    } else {
        let dir = output.to_path_buf();
        let file = dir.join("presentation.md");
        Ok((file, dir))
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_output_md_file() {
        let (file, dir) = resolve_output(Path::new("slides.md")).unwrap();
        assert_eq!(file, PathBuf::from("slides.md"));
        assert_eq!(dir, PathBuf::from("."));
    }

    #[test]
    fn test_resolve_output_directory_slash() {
        let (file, dir) = resolve_output(Path::new("output/")).unwrap();
        assert_eq!(file, PathBuf::from("output/presentation.md"));
        assert_eq!(dir, PathBuf::from("output"));
    }

    #[test]
    fn test_resolve_output_no_extension() {
        let (file, dir) = resolve_output(Path::new("my-presentation")).unwrap();
        assert_eq!(file, PathBuf::from("my-presentation/presentation.md"));
        assert_eq!(dir, PathBuf::from("my-presentation"));
    }

    #[test]
    fn test_resolve_output_nested_path() {
        let (file, dir) = resolve_output(Path::new("dir/subdir/pres.md")).unwrap();
        assert_eq!(file, PathBuf::from("dir/subdir/pres.md"));
        assert_eq!(dir, PathBuf::from("dir/subdir"));
    }

    #[test]
    fn test_resolve_output_markdown_extension() {
        let (file, _dir) = resolve_output(Path::new("talk.markdown")).unwrap();
        assert_eq!(file, PathBuf::from("talk.markdown"));
    }
}
