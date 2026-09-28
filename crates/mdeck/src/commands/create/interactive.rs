//! Interactive AI chat for gathering presentation context.

use std::io::{self, Write};

use anyhow::Result;
use colored::Colorize;
use futures::StreamExt;

use super::prompts::INTERACTIVE_SYSTEM_PROMPT;

/// Run an interactive AI chat to gather presentation context.
pub async fn run_interactive_chat(
    client: &ailloy::Client,
    content: &str,
    initial_prompt: Option<&str>,
    _quiet: bool,
) -> Result<String> {
    eprintln!(
        "  {} Type {} to start generation, {} to exit.\n",
        "ℹ".blue().bold(),
        "/start".bold(),
        "/quit".bold()
    );

    let mut history: Vec<ailloy::Message> =
        vec![ailloy::Message::system(INTERACTIVE_SYSTEM_PROMPT)];
    history.push(ailloy::Message::user(&opening_message(
        content,
        initial_prompt,
    )));

    // Get initial AI response
    let response = stream_chat(client, &history).await?;
    history.push(ailloy::Message::assistant(&response));

    if let Some(summary) = extract_ready_summary(&response) {
        return Ok(build_full_context(content, &summary));
    }

    eprintln!();

    // Chat loop
    loop {
        let input = match read_user_input()? {
            // Ctrl-D / piped EOF: there will never be more input, so stop
            // instead of re-prompting forever.
            None => {
                eprintln!();
                anyhow::bail!("Cancelled (end of input).");
            }
            Some(s) if s.is_empty() => continue,
            Some(s) => s,
        };

        let summary = match input.as_str() {
            "/quit" | "/exit" | "/q" => {
                anyhow::bail!("Cancelled.");
            }
            "/start" => Some(force_summary(client, &mut history).await?),
            other if other.starts_with('/') => {
                print_command_help(other);
                continue;
            }
            _ => {
                history.push(ailloy::Message::user(&input));
                let response = stream_chat(client, &history).await?;
                history.push(ailloy::Message::assistant(&response));
                eprintln!();
                extract_ready_summary(&response)
            }
        };

        if let Some(summary) = summary {
            return Ok(build_full_context(content, &summary));
        }
    }
}

/// The first user message: the user's own words, with or without a
/// `--prompt`, or a pointer to long source material.
fn opening_message(content: &str, initial_prompt: Option<&str>) -> String {
    let word_count = content.split_whitespace().count();
    let is_short_text = word_count < 200;

    match (initial_prompt, is_short_text) {
        (Some(prompt), true) => {
            // Short text input + explicit prompt: send both directly
            format!(
                "I want to create a presentation. Here's what I told you:\n\n\
                 \"{content}\"\n\n\
                 Additional context: {prompt}\n\n\
                 Acknowledge what I've already told you \u{2014} don't ask me things I already \
                 answered. Then ask a focused follow-up question about something I \
                 haven't covered yet."
            )
        }
        (Some(prompt), false) => {
            // Long content + prompt: summarize content, include prompt
            format!(
                "I want to create a presentation. I have {word_count} words of source \
                 material to work from.\n\n\
                 My instructions: {prompt}\n\n\
                 Acknowledge my instructions and ask a focused follow-up question \
                 about something I haven't covered yet."
            )
        }
        (None, true) => {
            // Short text input, no prompt: the text IS the user's intent
            format!(
                "I want to create a presentation. Here's what I told you:\n\n\
                 \"{content}\"\n\n\
                 Acknowledge what I've already told you \u{2014} don't ask me things I already \
                 answered. If I mentioned the audience, don't ask who the audience is. \
                 If I mentioned the goal, don't ask what the goal is. Instead, ask a \
                 focused follow-up question about something I haven't covered yet."
            )
        }
        (None, false) => {
            // Long content, no prompt: reference the content
            format!(
                "I want to create a presentation from {word_count} words of source \
                 material I've provided. Ask me a focused question about who the \
                 audience is and what I want to achieve with this presentation."
            )
        }
    }
}

/// `/start`: make the model summarize and produce `[READY]`; its reply is
/// the brief even when the marker is missing.
async fn force_summary(
    client: &ailloy::Client,
    history: &mut Vec<ailloy::Message>,
) -> Result<String> {
    history.push(ailloy::Message::user(
        "I'm ready to generate. Please summarize what we've discussed and proceed.",
    ));
    let response = stream_chat(client, history).await?;
    history.push(ailloy::Message::assistant(&response));
    eprintln!();
    Ok(extract_ready_summary(&response).unwrap_or(response))
}

/// `/help`, or a hint for an unknown command.
fn print_command_help(command: &str) {
    if command == "/help" {
        eprintln!("{}", "Commands:".bold());
        eprintln!("  {}: Start generating the presentation", "/start".bold());
        eprintln!("  {}: Exit without generating", "/quit".bold());
        eprintln!("  {}: Show this help", "/help".bold());
    } else {
        eprintln!(
            "{} Unknown command. Type {} for help.",
            "!".yellow().bold(),
            "/help".bold()
        );
    }
}

/// Extract the summary text before the [READY] marker.
pub fn extract_ready_summary(text: &str) -> Option<String> {
    let marker = "[READY]";
    let idx = text.find(marker)?;
    let summary = text[..idx].trim().to_string();
    if summary.is_empty() {
        None
    } else {
        Some(summary)
    }
}

/// Combine source content and chat summary into the full context for generation.
pub fn build_full_context(content: &str, summary: &str) -> String {
    format!(
        "PRESENTATION BRIEF:\n{summary}\n\n\
         SOURCE MATERIAL ({} words):\n{content}",
        content.split_whitespace().count()
    )
}

/// Stream a chat response, printing tokens to stderr. Returns assembled text.
/// The `[READY]` marker is suppressed from output but preserved in the returned string.
async fn stream_chat(client: &ailloy::Client, history: &[ailloy::Message]) -> Result<String> {
    let mut stream = client.chat_stream(history).await?;
    let mut assembled = String::new();
    // Buffer to detect and suppress [READY] marker from display
    let mut display_buf = String::new();
    const MARKER: &str = "[READY]";

    while let Some(event) = stream.next().await {
        match event? {
            ailloy::StreamEvent::Delta(text) => {
                assembled.push_str(&text);
                display_buf.push_str(&text);

                // Check if we might be in the middle of [READY]
                if MARKER.starts_with(&display_buf) {
                    // Could still be building toward [READY]: hold the buffer
                    continue;
                }

                if display_buf.contains(MARKER) {
                    // Found [READY]: print everything before it, discard the marker
                    let before = display_buf.split(MARKER).next().unwrap_or("");
                    if !before.is_empty() {
                        eprint!("{before}");
                    }
                    // Print anything after the marker (unlikely but handle it)
                    let after_idx = display_buf.find(MARKER).unwrap() + MARKER.len();
                    let after = &display_buf[after_idx..];
                    if !after.is_empty() {
                        eprint!("{after}");
                    }
                    display_buf.clear();
                } else {
                    // No marker possible: flush the buffer
                    eprint!("{display_buf}");
                    display_buf.clear();
                }
                io::stderr().flush()?;
            }
            ailloy::StreamEvent::Done(_) => {
                // Flush any remaining buffer (excluding [READY])
                if !display_buf.is_empty() && !display_buf.contains(MARKER) {
                    eprint!("{display_buf}");
                } else if display_buf.contains(MARKER) {
                    let before = display_buf.split(MARKER).next().unwrap_or("");
                    if !before.is_empty() {
                        eprint!("{before}");
                    }
                }
                eprintln!();
            }
        }
    }

    Ok(assembled)
}

/// Read a line of user input with a `> ` prompt.
///
/// Returns `Ok(None)` only at end of input (Ctrl-D / piped EOF); an empty
/// line is returned as `Some("")` so callers can tell the two apart.
fn read_user_input() -> Result<Option<String>> {
    eprint!("{} ", ">".bold());
    io::stderr().flush()?;

    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(0) => Ok(None),
        Ok(_) => Ok(Some(input.trim().to_string())),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn opening_message_quotes_short_text_and_counts_long() {
        let short = opening_message("Git for beginners", Some("devs"));
        assert!(short.contains("\"Git for beginners\"\n\nAdditional context: devs\n\n"));
        assert!(short.contains("told you \u{2014} don't ask"));
        let long = opening_message(&"word ".repeat(250), None);
        assert!(long.starts_with("I want to create a presentation from 250 words"));
    }
}
