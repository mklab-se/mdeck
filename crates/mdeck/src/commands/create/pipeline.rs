//! The AI calls behind `ai create`: a file name, the outline, then the deck.

use anyhow::Result;
use colored::Colorize;
use futures::StreamExt;

use super::opportunities::{VisualizationOpportunity, extract_opportunities};
use super::prompts::{ANALYSIS_SYSTEM_PROMPT, generation_system_prompt};
use super::spinner::Spinner;

/// Suggest a filename based on the presentation context.
pub(super) async fn suggest_filename(client: &ailloy::Client, context: &str) -> Result<String> {
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

/// Run the full generation pipeline: analyze, then generate.
/// Returns (presentation_markdown, visualization_opportunities).
pub(super) async fn run_pipeline(
    client: &ailloy::Client,
    content: &str,
    context: &str,
    style: &Option<String>,
    quiet: bool,
) -> Result<(String, Vec<VisualizationOpportunity>)> {
    // Step A: Analyze content and create outline
    let spinner = Spinner::unless_quiet(quiet, "Analyzing content...".to_string());
    let outline = run_analysis(client, content, context).await?;
    if let Some(s) = spinner {
        s.stop_with(&format!("{} Content analyzed.", "✓".green().bold()));
    }

    let opportunities = extract_opportunities(&outline);

    // Step B: Generate slides
    let slide_count = outline_slide_count(&outline);
    let spinner = Spinner::unless_quiet(quiet, format!("Generating ~{slide_count} slides..."));
    let presentation_md = run_generation(client, &outline, context, style).await?;
    if let Some(s) = spinner {
        s.stop_with(&format!("{} Presentation generated.", "✓".green().bold()));
    }

    Ok((presentation_md, opportunities))
}

/// Roughly how many slides the outline has, for progress reporting.
fn outline_slide_count(outline: &str) -> usize {
    outline
        .matches("\"title\"")
        .count()
        .saturating_sub(1)
        .max(1)
}

/// The analysis request: the context, then the source content (cut at
/// 100 kB on a char boundary).
fn analysis_message(content: &str, context: &str) -> String {
    let mut user_message = format!("PRESENTATION CONTEXT:\n{context}\n\nSOURCE CONTENT:\n");

    const MAX_CONTENT_BYTES: usize = 100_000;
    if content.len() > MAX_CONTENT_BYTES {
        // Cut on a char boundary: a raw byte slice panics on multi-byte text
        user_message.push_str(crate::commands::util::truncate_bytes(
            content,
            MAX_CONTENT_BYTES,
        ));
        user_message.push_str("\n\n[Content truncated.]");
    } else {
        user_message.push_str(content);
    }
    user_message
}

/// Run the content analysis step (silent: output captured, not printed).
async fn run_analysis(client: &ailloy::Client, content: &str, context: &str) -> Result<String> {
    let user_message = analysis_message(content, context);
    let messages = vec![
        ailloy::Message::system(ANALYSIS_SYSTEM_PROMPT),
        ailloy::Message::user(&user_message),
    ];
    // Silent: don't print the JSON to the user
    collect_stream(client, &messages).await
}

/// Run the slide generation step (silent: output captured, not printed).
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

    // Silent generation: don't print raw markdown
    let assembled = collect_stream(client, &messages).await?;
    Ok(strip_markdown_fences(&assembled))
}

/// Stream a chat reply without printing it and return the whole text.
async fn collect_stream(client: &ailloy::Client, messages: &[ailloy::Message]) -> Result<String> {
    let mut stream = client.chat_stream(messages).await?;
    let mut assembled = String::new();
    while let Some(event) = stream.next().await {
        match event? {
            ailloy::StreamEvent::Delta(text) => assembled.push_str(&text),
            ailloy::StreamEvent::Done(_) => {}
        }
    }
    Ok(assembled)
}

/// The deck in a reply, without a code fence the model may wrap it in.
fn strip_markdown_fences(text: &str) -> String {
    crate::commands::ai_reply::strip_fence(text, &["markdown", "md"]).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analysis_message_truncates_long_content() {
        let short = analysis_message("body", "ctx");
        assert_eq!(short, "PRESENTATION CONTEXT:\nctx\n\nSOURCE CONTENT:\nbody");
        let long = analysis_message(&"å".repeat(60_000), "ctx");
        assert!(long.ends_with("\n\n[Content truncated.]"));
    }

    #[test]
    fn outline_slide_count_never_zero() {
        assert_eq!(outline_slide_count("{}"), 1);
        assert_eq!(
            outline_slide_count(r#"{"title": "D", "slides": [{"title": "a"}, {"title": "b"}]}"#),
            2
        );
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
}
