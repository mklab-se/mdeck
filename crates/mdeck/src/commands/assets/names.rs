//! Chat helpers for `mdeck ai images`: prompts for empty alt texts and file names
//! for the generated images.

use std::path::Path;

use anyhow::Result;

use crate::parser;

pub(super) async fn auto_prompt(
    presentation: &parser::Presentation,
    slide_number: usize,
) -> Result<String> {
    let slide = &presentation.slides[slide_number - 1];
    let raw = &slide.raw_source;

    let client = ailloy::Client::for_capability("chat")?;
    let response = client
        .chat(&[
            ailloy::Message::system(
                "You are a concise image prompt generator. Given slide content, \
                 generate a short, descriptive image prompt suitable for AI image generation. \
                 Respond with ONLY the prompt, no explanation.",
            ),
            ailloy::Message::user(format!(
                "Generate a concise image prompt for a presentation slide about:\n\n{raw}"
            )),
        ])
        .await?;

    Ok(response.content.trim().to_string())
}

pub(super) async fn generate_filename(
    has_chat: bool,
    prompt_text: &str,
    ext: &str,
    dir: &Path,
) -> String {
    let base = if has_chat {
        chat_filename(prompt_text)
            .await
            .unwrap_or_else(|_| hex_filename())
    } else {
        hex_filename()
    };

    // Check for collisions
    let mut candidate = format!("{base}.{ext}");
    let mut counter = 2;
    while dir.join(&candidate).exists() {
        candidate = format!("{base}-{counter}.{ext}");
        counter += 1;
    }
    candidate
}

async fn chat_filename(prompt_text: &str) -> Result<String> {
    let client = ailloy::Client::for_capability("chat")?;
    let response = client
        .chat(&[
            ailloy::Message::system(
                "Generate a short kebab-case filename (no extension) for an image. \
                 Respond with ONLY the filename, 2-4 words, lowercase, hyphens between words. \
                 Example: golden-sunset",
            ),
            ailloy::Message::user(prompt_text),
        ])
        .await?;

    let name = response
        .content
        .trim()
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect::<String>();

    if name.is_empty() {
        Ok(hex_filename())
    } else {
        // Truncate to reasonable length
        Ok(name.chars().take(40).collect())
    }
}

fn hex_filename() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("image-{:08x}", (n & 0xFFFF_FFFF) as u32)
}
