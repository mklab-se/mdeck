//! `mdeck ai create` — create a presentation from content using AI.
//!
//! Accepts text, markdown, PDF, or DOCX input and generates a complete
//! mdeck-format presentation with speaker notes, visualizations, and
//! image generation markers.

mod extractors;
mod interactive;
mod opportunities;
mod prompts;

use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use colored::Colorize;
use futures::StreamExt;

use crate::cli::CreateArgs;
use crate::commands::ai;

use self::extractors::extract_from_file;
use self::interactive::run_interactive_chat;
use self::opportunities::{extract_opportunities, write_opportunities};
use self::prompts::{ANALYSIS_SYSTEM_PROMPT, generation_system_prompt};

const APP_NAME: &str = "mdeck";
/// Output file used when no `--output` is given and no name can be suggested.
const DEFAULT_OUTPUT: &str = "presentation.md";

// ── Entry point ─────────────────────────────────────────────────────────────

pub async fn run(args: CreateArgs, quiet: bool) -> Result<()> {
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
    let (source_label, content) = resolve_input(&args, quiet)?;
    let word_count = content.split_whitespace().count();

    if content.trim().is_empty() {
        anyhow::bail!("No content found in input. Please provide non-empty content.");
    }

    // Show input info for file/stdin sources, but not for text the user just typed
    if !quiet && source_label != "(text input)" {
        eprintln!(
            "  {} {} ({} words)",
            "Input:".bold(),
            source_label,
            word_count
        );
    }

    let client = ailloy::Client::for_capability("chat")?;

    // Step 2: Interactive mode — AI-driven conversation to shape the presentation
    let context = if args.interactive {
        run_interactive_chat(&client, &content, args.prompt.as_deref(), quiet).await?
    } else {
        // Non-interactive: use --prompt directly or a sensible default
        args.prompt
            .clone()
            .unwrap_or_else(|| "General audience. Focus on key takeaways.".to_string())
    };

    // Step 3: Determine output filename. An explicit --output is always
    // respected (even `presentation.md`); otherwise ask the AI for a name.
    let output_file = match &args.output {
        Some(explicit) => resolve_output(explicit)?.0,
        None if !quiet => {
            let suggested = suggest_filename(&client, &context).await?;
            resolve_output(Path::new(&suggested))?.0
        }
        None => resolve_output(Path::new(DEFAULT_OUTPUT))?.0,
    };
    let output_dir = output_file
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .to_path_buf();

    // Step 4: Confirmation — show what will be created and ask for approval
    if !quiet {
        eprintln!();
        eprintln!("{}", "  Ready to generate:".bold());
        eprintln!("    {} {}", "File:".bold(), output_file.display());
        eprintln!();
    }

    if args.interactive {
        eprint!("{} Proceed with generation? [Y/n] ", "?".green().bold());
        io::stderr().flush()?;
        let mut confirm = String::new();
        io::stdin().read_line(&mut confirm)?;
        let confirm = confirm.trim().to_lowercase();
        if confirm == "n" || confirm == "no" {
            eprintln!("{} Cancelled.", "!".yellow().bold());
            return Ok(());
        }
    }

    // Step 5: Generate the presentation
    let (presentation_md, opportunities) =
        run_pipeline(&client, &content, &context, &args.style, quiet).await?;

    // Step 6: Write output
    std::fs::create_dir_all(&output_dir).with_context(|| {
        format!(
            "Failed to create output directory: {}",
            output_dir.display()
        )
    })?;

    std::fs::write(&output_file, &presentation_md)
        .with_context(|| format!("Failed to write output: {}", output_file.display()))?;

    if !quiet {
        eprintln!(
            "{} Presentation created: {}",
            "✓".green().bold(),
            output_file.display()
        );
    }

    // Step 7: Auto-generate images if image capability is available
    let image_count = presentation_md.matches("(image-generation)").count();
    if image_count > 0 {
        if ai::has_capability("image") {
            if !quiet {
                eprintln!();
                eprintln!(
                    "  {} Generating {} image{}...",
                    "ℹ".blue().bold(),
                    image_count,
                    if image_count == 1 { "" } else { "s" }
                );
            }
            // Run generate with quiet=true to suppress inline image display in terminal
            crate::commands::generate::run(output_file.clone(), true, args.style.clone(), true)
                .await?;
            if !quiet {
                eprintln!(
                    "  {} {} image{} generated.",
                    "✓".green().bold(),
                    image_count,
                    if image_count == 1 { "" } else { "s" }
                );
            }
        } else if !quiet {
            eprintln!(
                "  {} {} image{} marked but no image provider configured.",
                "ℹ".blue().bold(),
                image_count,
                if image_count == 1 { "" } else { "s" }
            );
            eprintln!("    Run `{APP_NAME} ai config` to add an image provider, then:");
            eprintln!(
                "    {}",
                format!("mdeck ai generate {}", output_file.display()).cyan()
            );
        }
    }

    if !quiet {
        eprintln!();
        eprintln!(
            "  Launch: {}",
            format!("mdeck {}", output_file.display()).cyan()
        );
    }

    // Step 8: Visualization opportunities — shown last as a warning
    if !opportunities.is_empty() && !quiet {
        let opp_file = output_dir.join("visualization-opportunities.md");
        write_opportunities(&opp_file, &opportunities)?;
        eprintln!();
        eprintln!(
            "  {} This presentation could be even better.",
            "!".yellow().bold(),
        );
        eprintln!(
            "    MDeck identified {} visualization{} that would enhance the slides",
            opportunities.len(),
            if opportunities.len() == 1 { "" } else { "s" },
        );
        eprintln!(
            "    but {} not yet supported.",
            if opportunities.len() == 1 {
                "is"
            } else {
                "are"
            }
        );
        eprintln!();
        eprintln!(
            "    The file {} contains detailed feature request{}",
            opp_file.display().to_string().cyan(),
            if opportunities.len() == 1 { "" } else { "s" },
        );
        eprintln!(
            "    ready to be copied into a GitHub issue. By sharing {} you help",
            if opportunities.len() == 1 {
                "it,"
            } else {
                "them,"
            }
        );
        eprintln!("    yourself and the MDeck community.");
        eprintln!();
        eprintln!(
            "    {}",
            "https://github.com/mklab-se/mdeck/issues/new".cyan()
        );
    }

    Ok(())
}

// ── Input resolution ────────────────────────────────────────────────────────

/// Resolve the input source and extract text content.
/// Returns (source_label, extracted_text).
fn resolve_input(args: &CreateArgs, quiet: bool) -> Result<(String, String)> {
    if let Some(ref input) = args.input {
        let path = Path::new(input);
        if path.exists() && path.is_file() {
            let label = format!("{}", path.display());
            let content = extract_from_file(path)?;
            return Ok((label, content));
        }
        return Ok(("(text input)".to_string(), input.clone()));
    }

    // Try stdin if it's piped
    if !io::stdin().is_terminal() {
        let mut content = String::new();
        io::stdin()
            .read_to_string(&mut content)
            .context("Failed to read from stdin")?;
        if content.trim().is_empty() {
            anyhow::bail!("No content received from stdin.");
        }
        return Ok(("(stdin)".to_string(), content));
    }

    // Interactive mode — ask for input
    if args.interactive {
        if !quiet {
            eprintln!(
                "{} What should the presentation be about?",
                "?".green().bold()
            );
            eprintln!("  Enter a file path, or describe the topic in your own words.");
            eprintln!();
        }
        eprint!("{} ", ">".bold());
        io::stderr().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_string();
        if input.is_empty() {
            anyhow::bail!("No input provided.");
        }
        let path = Path::new(&input);
        if path.exists() && path.is_file() {
            let label = format!("{}", path.display());
            let content = extract_from_file(path)?;
            return Ok((label, content));
        }
        return Ok(("(text input)".to_string(), input));
    }

    // No input provided — show help
    use clap::CommandFactory;
    let mut cmd = crate::cli::Cli::command();
    for sub in cmd.get_subcommands_mut() {
        if sub.get_name() == "ai" {
            for sub2 in sub.get_subcommands_mut() {
                if sub2.get_name() == "create" {
                    sub2.clone().name("mdeck ai create").print_help()?;
                    println!();
                    std::process::exit(0);
                }
            }
        }
    }
    anyhow::bail!("No input provided. Run `mdeck ai create --help` for usage.");
}

// ── Spinner ─────────────────────────────────────────────────────────────────

/// A terminal spinner that animates on a background thread.
struct Spinner {
    handle: Option<std::thread::JoinHandle<()>>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Spinner {
    /// Start a spinner with the given message. The spinner animates until `stop()` is called.
    fn start(message: String) -> Self {
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stop_clone = stop.clone();
        let handle = std::thread::spawn(move || {
            const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let mut i = 0;
            while !stop_clone.load(std::sync::atomic::Ordering::Relaxed) {
                eprint!("\r  {} {}", FRAMES[i % FRAMES.len()], message);
                let _ = io::stderr().flush();
                i += 1;
                std::thread::sleep(std::time::Duration::from_millis(80));
            }
        });
        Self {
            handle: Some(handle),
            stop,
        }
    }

    /// Stop the spinner and replace its line with a completion message.
    fn stop_with(mut self, message: &str) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        // Clear the spinner line and print the completion message
        eprint!("\r\x1b[2K  {message}\n");
        let _ = io::stderr().flush();
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

// ── AI pipeline ─────────────────────────────────────────────────────────────

/// Suggest a filename based on the presentation context.
async fn suggest_filename(client: &ailloy::Client, context: &str) -> Result<String> {
    let messages = vec![
        ailloy::Message::system(
            "Given a presentation description, suggest a short kebab-case filename (2-4 words, no extension). \
             Reply with ONLY the filename, nothing else. Example: git-flow-adoption",
        ),
        ailloy::Message::user(context),
    ];
    let response = client.chat(&messages).await?;
    let name = response
        .content
        .trim()
        .trim_matches('"')
        .trim_matches('`')
        .to_string();
    if name.is_empty() || name.len() > 60 {
        Ok("presentation.md".to_string())
    } else {
        Ok(format!("{name}.md"))
    }
}

/// Run the full generation pipeline: analyze → generate.
/// Returns (presentation_markdown, visualization_opportunities).
async fn run_pipeline(
    client: &ailloy::Client,
    content: &str,
    context: &str,
    style: &Option<String>,
    quiet: bool,
) -> Result<(String, Vec<opportunities::VisualizationOpportunity>)> {
    // Step A: Analyze content and create outline
    let spinner = if !quiet {
        Some(Spinner::start("Analyzing content...".to_string()))
    } else {
        None
    };

    let outline = run_analysis(client, content, context).await?;

    if let Some(s) = spinner {
        s.stop_with(&format!("{} Content analyzed.", "✓".green().bold()));
    }

    // Extract opportunities from the outline
    let opportunities = extract_opportunities(&outline);

    // Count slides in outline for progress reporting
    let slide_count = outline
        .matches("\"title\"")
        .count()
        .saturating_sub(1)
        .max(1);

    // Step B: Generate slides
    let spinner = if !quiet {
        Some(Spinner::start(format!(
            "Generating ~{slide_count} slides..."
        )))
    } else {
        None
    };

    let presentation_md = run_generation(client, &outline, context, style).await?;

    if let Some(s) = spinner {
        s.stop_with(&format!("{} Presentation generated.", "✓".green().bold()));
    }

    Ok((presentation_md, opportunities))
}

/// Run the content analysis step (silent — output captured, not printed).
async fn run_analysis(client: &ailloy::Client, content: &str, context: &str) -> Result<String> {
    let mut user_message = format!("PRESENTATION CONTEXT:\n{context}\n\nSOURCE CONTENT:\n");

    const MAX_CONTENT_BYTES: usize = 100_000;
    if content.len() > MAX_CONTENT_BYTES {
        // Cut on a char boundary — a raw byte slice panics on multi-byte text
        user_message.push_str(crate::commands::util::truncate_bytes(
            content,
            MAX_CONTENT_BYTES,
        ));
        user_message.push_str("\n\n[Content truncated.]");
    } else {
        user_message.push_str(content);
    }

    let messages = vec![
        ailloy::Message::system(ANALYSIS_SYSTEM_PROMPT),
        ailloy::Message::user(&user_message),
    ];

    // Silent — don't print the JSON to the user
    let mut stream = client.chat_stream(&messages).await?;
    let mut assembled = String::new();
    while let Some(event) = stream.next().await {
        match event? {
            ailloy::StreamEvent::Delta(text) => assembled.push_str(&text),
            ailloy::StreamEvent::Done(_) => {}
        }
    }

    Ok(assembled)
}

/// Run the slide generation step (silent — output captured, not printed).
async fn run_generation(
    client: &ailloy::Client,
    outline: &str,
    context: &str,
    style: &Option<String>,
) -> Result<String> {
    let system_prompt = generation_system_prompt(style);

    let user_message = format!(
        "Generate a complete mdeck presentation from this outline:\n\n{outline}\n\n\
         CONTEXT:\n{context}"
    );

    let messages = vec![
        ailloy::Message::system(&system_prompt),
        ailloy::Message::user(&user_message),
    ];

    // Silent generation — don't print raw markdown
    let mut stream = client.chat_stream(&messages).await?;
    let mut assembled = String::new();
    while let Some(event) = stream.next().await {
        match event? {
            ailloy::StreamEvent::Delta(text) => assembled.push_str(&text),
            ailloy::StreamEvent::Done(_) => {}
        }
    }

    let cleaned = strip_markdown_fences(&assembled);
    Ok(cleaned)
}

/// Strip markdown code fences if the AI wrapped the response.
fn strip_markdown_fences(text: &str) -> String {
    let trimmed = text.trim();
    if let Some(rest) = trimmed.strip_prefix("```markdown") {
        if let Some(content) = rest.strip_suffix("```") {
            return content.trim().to_string();
        }
    }
    if let Some(rest) = trimmed.strip_prefix("```md") {
        if let Some(content) = rest.strip_suffix("```") {
            return content.trim().to_string();
        }
    }
    if let Some(rest) = trimmed.strip_prefix("```") {
        if let Some(content) = rest.strip_suffix("```") {
            let first_line = content.lines().next().unwrap_or("");
            if first_line.trim().is_empty() || first_line.trim() == "---" {
                return content.trim().to_string();
            }
        }
    }
    trimmed.to_string()
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
    use self::extractors::extract_text_from_docx_xml;
    use self::interactive::{build_full_context, extract_ready_summary};
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
    fn test_strip_markdown_fences_wrapped() {
        let input = "```markdown\n---\ntitle: Test\n---\n# Slide\n```";
        let result = strip_markdown_fences(input);
        assert!(result.starts_with("---"));
        assert!(!result.contains("```"));
    }

    #[test]
    fn test_strip_markdown_fences_unwrapped() {
        let input = "---\ntitle: Test\n---\n# Slide";
        let result = strip_markdown_fences(input);
        assert_eq!(result, input);
    }

    #[test]
    fn test_strip_markdown_fences_md() {
        let input = "```md\n---\ntitle: Test\n---\n```";
        let result = strip_markdown_fences(input);
        assert!(result.starts_with("---"));
    }

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
    fn test_extract_ready_summary() {
        let text = "Great! Here's what we'll create:\n\nA presentation about Git Flow for developers.\n\n[READY]";
        let summary = extract_ready_summary(text).unwrap();
        assert!(summary.contains("Git Flow"));
        assert!(!summary.contains("[READY]"));
    }

    #[test]
    fn test_extract_ready_summary_none() {
        assert!(extract_ready_summary("Just chatting, no marker here.").is_none());
    }

    // ── DOCX XML parsing tests ──────────────────────────────────────────────

    #[test]
    fn test_docx_xml_basic_paragraph() {
        let xml = r#"<w:body><w:p><w:r><w:t>Hello world</w:t></w:r></w:p></w:body>"#;
        let text = extract_text_from_docx_xml(xml);
        assert_eq!(text.trim(), "Hello world");
    }

    #[test]
    fn test_docx_xml_multiple_paragraphs() {
        let xml = r#"<w:body><w:p><w:r><w:t>First</w:t></w:r></w:p><w:p><w:r><w:t>Second</w:t></w:r></w:p></w:body>"#;
        let text = extract_text_from_docx_xml(xml);
        let lines: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(lines, vec!["First", "Second"]);
    }

    #[test]
    fn test_docx_xml_multiple_runs() {
        let xml = r#"<w:p><w:r><w:t>Hello </w:t></w:r><w:r><w:t>world</w:t></w:r></w:p>"#;
        let text = extract_text_from_docx_xml(xml);
        assert_eq!(text.trim(), "Hello world");
    }

    #[test]
    fn test_docx_xml_text_with_attributes() {
        let xml = r#"<w:p><w:r><w:t xml:space="preserve">Preserved text</w:t></w:r></w:p>"#;
        let text = extract_text_from_docx_xml(xml);
        assert_eq!(text.trim(), "Preserved text");
    }

    #[test]
    fn test_docx_xml_ignores_non_text_tags() {
        let xml = r#"<w:p><w:pPr><w:jc w:val="center"/></w:pPr><w:r><w:rPr><w:b/></w:rPr><w:t>Bold text</w:t></w:r></w:p>"#;
        let text = extract_text_from_docx_xml(xml);
        assert_eq!(text.trim(), "Bold text");
    }

    #[test]
    fn test_docx_xml_table_not_confused_with_text() {
        let xml = r#"<w:body><w:tbl><w:tr><w:tc><w:p><w:r><w:t>Cell</w:t></w:r></w:p></w:tc></w:tr></w:tbl></w:body>"#;
        let text = extract_text_from_docx_xml(xml);
        assert!(text.contains("Cell"));
    }

    #[test]
    fn test_docx_xml_empty_document() {
        let xml = r#"<w:body></w:body>"#;
        let text = extract_text_from_docx_xml(xml);
        assert!(text.trim().is_empty());
    }

    #[test]
    fn test_docx_xml_self_closing_text_tag() {
        let xml = r#"<w:p><w:r><w:t/>Outside text</w:r></w:p>"#;
        let text = extract_text_from_docx_xml(xml);
        assert!(!text.contains("Outside"));
    }

    // ── Input resolution tests ──────────────────────────────────────────────

    #[test]
    fn test_resolve_input_literal_text() {
        let args = CreateArgs {
            input: Some("A presentation about Rust programming".to_string()),
            output: Some(PathBuf::from("out.md")),
            prompt: None,
            interactive: false,
            style: None,
        };
        let (label, content) = resolve_input(&args, true).unwrap();
        assert_eq!(label, "(text input)");
        assert_eq!(content, "A presentation about Rust programming");
    }

    #[test]
    fn test_resolve_input_existing_file() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let cargo_toml = format!("{manifest_dir}/Cargo.toml");
        let args = CreateArgs {
            input: Some(cargo_toml),
            output: Some(PathBuf::from("out.md")),
            prompt: None,
            interactive: false,
            style: None,
        };
        let (label, content) = resolve_input(&args, true).unwrap();
        assert!(label.contains("Cargo.toml"));
        assert!(content.contains("mdeck"));
    }

    #[test]
    fn test_resolve_input_no_input_no_stdin() {
        let args = CreateArgs {
            input: None,
            output: Some(PathBuf::from("out.md")),
            prompt: None,
            interactive: false,
            style: None,
        };
        let result = resolve_input(&args, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_input_interactive_with_input_provided() {
        let args = CreateArgs {
            input: Some("A talk about functional programming".to_string()),
            output: Some(PathBuf::from("out.md")),
            prompt: None,
            interactive: true,
            style: None,
        };
        let (label, content) = resolve_input(&args, true).unwrap();
        assert_eq!(label, "(text input)");
        assert_eq!(content, "A talk about functional programming");
    }

    #[test]
    fn test_resolve_output_markdown_extension() {
        let (file, _dir) = resolve_output(Path::new("talk.markdown")).unwrap();
        assert_eq!(file, PathBuf::from("talk.markdown"));
    }

    #[test]
    fn test_strip_fences_generic_wrapper() {
        let input = "```\nfunction foo() {}\n```";
        let result = strip_markdown_fences(input);
        assert_eq!(result, "function foo() {}");
    }

    #[test]
    fn test_strip_fences_code_with_language() {
        let input = "```rust\nfn main() {}\n```";
        let result = strip_markdown_fences(input);
        assert_eq!(result, input);
    }

    #[test]
    fn test_strip_fences_with_whitespace() {
        let input = "  ```markdown\n---\ntitle: Test\n---\n```  ";
        let result = strip_markdown_fences(input);
        assert!(result.starts_with("---"));
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
        let opp = opportunities::parse_opportunity_for_test(json).unwrap();
        assert_eq!(opp.visualization_name, "Swimlane Diagram");
        assert!(!opp.rendering_description.is_empty());
        assert!(!opp.ascii_mockup.is_empty());
    }

    #[test]
    fn test_build_full_context() {
        let content = "Some source text about Git.";
        let summary = "A presentation about Git Flow for developers.";
        let result = build_full_context(content, summary);
        assert!(result.contains("PRESENTATION BRIEF:"));
        assert!(result.contains("Git Flow"));
        assert!(result.contains("SOURCE MATERIAL"));
        assert!(result.contains("5 words"));
    }
}
