//! The AI calls behind `mdeck ai deck`: a file name, the outline, then the deck.

use std::path::Path;

use anyhow::Result;
use colored::Colorize;
use futures::StreamExt;

use crate::check::CheckCategory;

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

/// Run the full generation pipeline: analyze, then generate a deck for
/// `deck` (the file it will be written to), checked as it would be there.
/// Returns (presentation_markdown, visualization_opportunities).
pub(super) async fn run_pipeline(
    client: &ailloy::Client,
    content: &str,
    context: &str,
    style: &Option<String>,
    deck: &Path,
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
    let (presentation_md, problems) =
        run_generation(client, &outline, context, style, deck).await?;
    if let Some(s) = spinner {
        s.stop_with(&format!("{} Presentation generated.", "✓".green().bold()));
    }
    // asked once more and still not clean: write it, and say what is left
    if !problems.is_empty() && !quiet {
        eprintln!(
            "  {} the deck still has problems; `mdeck {} --check` lists them:",
            "warning:".yellow().bold(),
            deck.display()
        );
        for p in &problems {
            eprintln!("    {p}");
        }
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
/// The deck is parsed and checked (GEN-07); when the check finds problems
/// the model is asked once more with them. Returns the deck and the
/// problems the second answer still has.
async fn run_generation(
    client: &ailloy::Client,
    outline: &str,
    context: &str,
    style: &Option<String>,
    deck: &Path,
) -> Result<(String, Vec<String>)> {
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
    let mut attempts = 0;
    crate::commands::ai_reply::chat_validated(
        async |h: &[ailloy::Message]| collect_stream(client, h).await,
        messages,
        |reply| {
            attempts += 1;
            accept_deck(reply, deck, attempts > 1)
        },
        fix_deck_message,
        "the generated deck",
    )
    .await
}

/// The deck in `reply` and what `--check` finds wrong with it at `deck`.
/// Problems send it back (`Err`, the list for the model) unless `last`,
/// when it is accepted with them.
fn accept_deck(reply: &str, deck: &Path, last: bool) -> Result<(String, Vec<String>), String> {
    let md = strip_markdown_fences(reply);
    let problems = deck_problems(&md, deck);
    if problems.is_empty() || last {
        Ok((md, problems))
    } else {
        Err(problems.join("\n"))
    }
}

/// What `mdeck --check` reports for the markdown `md` written to `deck`,
/// one line each. Generated assets are left out: the images the deck asks
/// for are generated after it is written, and fonts depend on the machine.
pub(super) fn deck_problems(md: &str, deck: &Path) -> Vec<String> {
    let presentation = crate::parser::parse(md);
    let base = deck
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    match crate::commands::check::collect(deck, md, &presentation, base, None) {
        Ok(report) => report
            .warnings()
            .filter(|w| !matches!(w.category, CheckCategory::Assets | CheckCategory::Fonts))
            .map(|w| w.to_string().trim().to_string())
            .collect(),
        Err(e) => vec![e.to_string()],
    }
}

/// The follow-up request when the deck has problems.
fn fix_deck_message(problems: &str) -> String {
    format!(
        "`mdeck --check` reports these problems in that deck:\n{problems}\n\n\
         Fix them and reply with the whole corrected deck, markdown only."
    )
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

    fn run<T>(f: impl Future<Output = T>) -> T {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(f)
    }

    fn deck() -> &'static Path {
        Path::new("/nonexistent/talk.md")
    }

    #[test]
    fn a_clean_deck_has_no_problems() {
        let md = "# Hello\n\n---\n\n## Points\n\n- one\n- two\n";
        assert!(deck_problems(md, deck()).is_empty());
    }

    #[test]
    fn a_deck_with_v1_syntax_is_sent_back_with_its_problems() {
        let bad = "# Hello\n\n```@barchart\n- A: 1\n```\n";
        let problems = deck_problems(bad, deck());
        assert!(
            problems.iter().any(|p| p.contains("@barchart")),
            "{problems:?}"
        );
        let err = accept_deck(bad, deck(), false).unwrap_err();
        assert!(err.contains("@barchart"), "{err}");
        assert!(fix_deck_message(&err).contains("@barchart"));
        // the second answer is accepted, with what is left
        let (md, left) = accept_deck(bad, deck(), true).unwrap();
        assert_eq!(md, bad.trim());
        assert!(!left.is_empty());
    }

    #[test]
    fn the_deck_is_asked_for_again_with_the_check_problems() {
        let bad = "# Hello\n\n```@barchart\n- A: 1\n```\n".to_string();
        let good = "# Hello\n\n```@bar\n- A: 1\n```\n".to_string();
        let mut replies = vec![good.clone(), bad];
        let mut seen: Vec<Vec<ailloy::Message>> = Vec::new();
        let mut attempts = 0;
        let (md, left) = run(crate::commands::ai_reply::chat_validated(
            async |h: &[ailloy::Message]| {
                seen.push(h.to_vec());
                Ok(replies.pop().unwrap())
            },
            vec![ailloy::Message::user("make a deck")],
            |r| {
                attempts += 1;
                accept_deck(r, deck(), attempts > 1)
            },
            fix_deck_message,
            "the generated deck",
        ))
        .unwrap();
        assert_eq!(md, good.trim());
        assert!(left.is_empty(), "{left:?}");
        assert_eq!(seen.len(), 2);
        let follow_up = format!("{:?}", seen[1].last().unwrap());
        assert!(follow_up.contains("@barchart"), "{follow_up}");
    }

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
