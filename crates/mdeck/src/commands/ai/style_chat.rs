//! Interactive style creation: `mdeck ai style add -i`.

use std::io::{self, Write};

use anyhow::Result;
use colored::Colorize;
use futures::StreamExt;

use super::{APP_NAME, has_capability};
use crate::config::Config;

const STYLE_SYSTEM_PROMPT: &str = "\
You are a style design assistant for mdeck, a markdown-based presentation tool. \
Your job is to help the user craft a concise image generation style description \
that will be used as a prefix for all AI-generated images in their presentations.

A style description should be 1-3 sentences that define the visual aesthetic: \
color palette, mood, artistic technique, level of detail, and composition preferences. \
It must NOT describe specific subjects \u{2014} only the visual style.

Here are examples of good style descriptions:
- \"Modern, clean, and visually striking. Professional color palette with subtle gradients. \
Polished and contemporary, suitable for business or technical presentations.\"
- \"Warm watercolor illustrations with soft edges and muted earth tones. \
Hand-drawn feel with visible brushstrokes and gentle lighting.\"
- \"Retro 80s synthwave aesthetic with neon pinks, purples, and electric blues. \
Grid-based perspective with glowing edges and chrome reflections.\"

Ask focused questions (one or two at a time) about their preferred aesthetic. \
When you and the user agree on a style, output it wrapped exactly like this:

[STYLE: <the complete style description>]

If the user wants changes, refine and propose again. Keep the conversation friendly and concise.";

/// Extract a style description from the `[STYLE: <description>]` marker.
fn extract_style_description(text: &str) -> Option<String> {
    let marker = "[STYLE:";
    let start = text.find(marker)?;
    let after = &text[start + marker.len()..];
    let end = after.find(']')?;
    let desc = after[..end].trim();
    if desc.is_empty() {
        None
    } else {
        Some(desc.to_string())
    }
}

/// Stream a chat response from the AI, printing tokens in real-time.
/// Returns the full assembled response text.
async fn stream_chat_response(
    client: &ailloy::Client,
    history: &[ailloy::Message],
) -> Result<String> {
    let mut stream = client.chat_stream(history).await?;
    let mut assembled = String::new();

    while let Some(event) = stream.next().await {
        match event? {
            ailloy::StreamEvent::Delta(text) => {
                assembled.push_str(&text);
                print!("{text}");
                io::stdout().flush()?;
            }
            ailloy::StreamEvent::Done(_) => {
                println!();
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
        Ok(0) => Ok(None), // EOF
        Ok(_) => Ok(Some(input.trim().to_string())),
        Err(e) => Err(e.into()),
    }
}

/// The opening user message that makes the model greet and ask its first question.
fn greeting(name: Option<&str>, icon: bool) -> String {
    let article = if icon { "an icon" } else { "an image" };
    if let Some(n) = name {
        format!(
            "Greet me briefly and tell me you'll help me create {} style called \"{}\". \
             Ask what kind of visual aesthetic I'm going for.",
            article, n
        )
    } else {
        format!(
            "Greet me briefly and tell me you'll help me create {} style for my presentations. \
             Ask what kind of visual aesthetic I'm going for.",
            article,
        )
    }
}

/// What the REPL does after handling a slash command.
enum Slash {
    /// Not a command: send the line to the model.
    Send,
    /// Handled: read the next line.
    Next,
    Quit,
}

fn handle_slash(input: &str, history: &mut Vec<ailloy::Message>) -> Slash {
    match input {
        "/quit" | "/exit" | "/q" => Slash::Quit,
        "/clear" => {
            *history = vec![ailloy::Message::system(STYLE_SYSTEM_PROMPT)];
            eprintln!("{}", "History cleared.".dimmed());
            Slash::Next
        }
        "/help" => {
            eprintln!("{}", "Commands:".bold());
            eprintln!("  {}: Exit the session", "/quit".bold());
            eprintln!("  {}: Clear conversation history", "/clear".bold());
            eprintln!("  {}: Show this help", "/help".bold());
            Slash::Next
        }
        s if s.starts_with('/') => {
            eprintln!(
                "{} Unknown command: {}. Type {} for help.",
                "!".yellow().bold(),
                input,
                "/help".bold()
            );
            Slash::Next
        }
        _ => Slash::Send,
    }
}

/// Read one trimmed line from stdin after an inline `? ` question.
fn ask_line(question: &str) -> Result<String> {
    eprint!("{} {question}", "?".green().bold());
    io::stderr().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    Ok(line.trim().to_string())
}

/// The style's name: the one given on the command line, or asked for now.
/// `None` when the user gives no name.
fn resolve_style_name(name: Option<&str>) -> Result<Option<String>> {
    if let Some(n) = name {
        return Ok(Some(n.to_string()));
    }
    let name_input = ask_line("Name for this style: ")?;
    if name_input.is_empty() {
        eprintln!("{} No name provided, style not saved.", "!".yellow().bold());
        return Ok(None);
    }
    Ok(Some(name_input))
}

fn confirm_save(kind: &str, style_name: &str) -> Result<bool> {
    let confirm = ask_line(&format!(
        "Save {} style {}? [Y/n] ",
        kind,
        style_name.cyan().bold()
    ))?
    .to_lowercase();
    Ok(confirm.is_empty() || confirm == "y" || confirm == "yes")
}

fn save_style(style_name: &str, description: &str, icon: bool) -> Result<()> {
    let mut config = Config::load_or_default();
    if icon {
        config.add_icon_style(style_name, description);
    } else {
        config.add_style(style_name, description);
    }
    config.save()?;
    println!(
        "{} {} style {} saved.",
        "✓".green().bold(),
        if icon { "Icon" } else { "Image" },
        style_name.cyan().bold()
    );
    println!("  {}", description.dimmed());
    Ok(())
}

pub(super) async fn run_interactive_style(name: Option<String>, icon: bool) -> Result<()> {
    if !has_capability("chat") {
        anyhow::bail!(
            "Chat AI not configured. Run `{APP_NAME} ai config` to set up a chat provider."
        );
    }

    let kind = if icon { "icon" } else { "image" };

    eprintln!("{} Interactive {} style creator", "mdeck".bold(), kind);
    eprintln!(
        "Type {} to exit, {} for help.",
        "/quit".bold(),
        "/help".bold()
    );

    let client = ailloy::Client::for_capability("chat")?;

    let mut history: Vec<ailloy::Message> = vec![ailloy::Message::system(STYLE_SYSTEM_PROMPT)];
    history.push(ailloy::Message::user(greeting(name.as_deref(), icon)));

    eprintln!();
    let response = stream_chat_response(&client, &history).await?;
    history.push(ailloy::Message::assistant(&response));
    println!();

    // REPL loop
    loop {
        let input = match read_user_input()? {
            // Ctrl-D / piped EOF: no more input will ever come, so quit
            None => {
                eprintln!();
                break;
            }
            Some(s) if s.is_empty() => continue,
            Some(s) => s,
        };

        match handle_slash(&input, &mut history) {
            Slash::Quit => break,
            Slash::Next => continue,
            Slash::Send => {}
        }

        history.push(ailloy::Message::user(&input));

        let response = stream_chat_response(&client, &history).await?;
        history.push(ailloy::Message::assistant(&response));

        // Check for [STYLE: ...] marker
        if let Some(description) = extract_style_description(&response) {
            println!();

            let Some(style_name) = resolve_style_name(name.as_deref())? else {
                continue;
            };

            if confirm_save(kind, &style_name)? {
                save_style(&style_name, &description, icon)?;
                break;
            }
            // Tell the model the user wants to refine
            history.push(ailloy::Message::user(
                "I'm not happy with that style yet. Ask me what I'd like to change.",
            ));
            let followup = stream_chat_response(&client, &history).await?;
            history.push(ailloy::Message::assistant(&followup));
        }

        println!();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_style_description() {
        let text =
            "Here is the style: [STYLE: Modern, clean with subtle gradients and warm tones.]";
        assert_eq!(
            extract_style_description(text),
            Some("Modern, clean with subtle gradients and warm tones.".to_string())
        );
    }

    #[test]
    fn test_extract_style_description_empty() {
        assert_eq!(extract_style_description("[STYLE: ]"), None);
    }

    #[test]
    fn test_extract_style_description_missing() {
        assert_eq!(extract_style_description("No marker here"), None);
    }

    #[test]
    fn test_extract_style_description_multiline() {
        let text = "I suggest:\n[STYLE: Warm watercolor illustrations with soft edges and muted earth tones. Hand-drawn feel with visible brushstrokes.]\nWhat do you think?";
        assert_eq!(
            extract_style_description(text),
            Some(
                "Warm watercolor illustrations with soft edges and muted earth tones. Hand-drawn feel with visible brushstrokes.".to_string()
            )
        );
    }

    #[test]
    fn greeting_names_the_style_when_given() {
        assert_eq!(
            greeting(Some("noir"), true),
            "Greet me briefly and tell me you'll help me create an icon style called \"noir\". \
             Ask what kind of visual aesthetic I'm going for."
        );
        assert_eq!(
            greeting(None, false),
            "Greet me briefly and tell me you'll help me create an image style for my presentations. \
             Ask what kind of visual aesthetic I'm going for."
        );
    }
}
