//! Ad-hoc image generation (`mdeck ai generate-image`) and the helpers that
//! show a generated image in the terminal.

use anyhow::Result;
use colored::Colorize;

use super::{APP_NAME, has_capability};
use crate::config::Config;
use crate::prompt;

pub(super) async fn generate_image_cmd(args: crate::cli::GenerateImageArgs) -> Result<()> {
    if !has_capability("image") {
        anyhow::bail!(
            "Image generation not configured. Run `{APP_NAME} ai config` to set up an image provider."
        );
    }

    let config = Config::load_or_default();
    let style = resolve_style(&config, args.style.as_deref(), args.icon);
    let combined_prompt = if args.icon {
        prompt::build_icon_prompt(&style, &args.prompt)
    } else {
        prompt::build_image_prompt(&style, &args.prompt, prompt::Orientation::Horizontal)
    };

    println!("Generating image...");

    let client = ailloy::Client::for_capability("image")?;
    let response = client.generate_image(&combined_prompt).await?;

    let ext = image_ext(&response.format);
    let path = if let Some(ref output) = args.output {
        output.clone()
    } else {
        std::env::temp_dir().join(format!("mdeck-generated.{ext}"))
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, &response.data)?;

    println!(
        "{} Generated {}x{} {} image",
        "✓".green().bold(),
        response.width,
        response.height,
        ext.to_uppercase()
    );
    if let Some(ref revised) = response.revised_prompt {
        println!("  Revised prompt: {}", revised.dimmed());
    }
    println!();
    display_image_result(&path);

    if args.output.is_none() {
        offer_cleanup(&path);
    }

    Ok(())
}

/// Explicit `--style` (name lookup or literal) > config default > hardcoded.
fn resolve_style(config: &Config, style: Option<&str>, icon: bool) -> String {
    match (style, icon) {
        (Some(s), true) => config.get_icon_style(s).unwrap_or(s).to_string(),
        (Some(s), false) => config.get_style(s).unwrap_or(s).to_string(),
        (None, true) => config.resolve_icon_style().to_string(),
        (None, false) => config.resolve_image_style().to_string(),
    }
}

pub fn image_ext(format: &ailloy::ImageFormat) -> &'static str {
    match format {
        ailloy::ImageFormat::Png => "png",
        ailloy::ImageFormat::Jpeg => "jpg",
        ailloy::ImageFormat::Webp => "webp",
    }
}

/// Display the generated test image: inline if the terminal supports it, plus a hyperlink.
pub fn display_image_result(path: &std::path::Path) {
    use std::io::Write;

    let display_path = path.display();
    let file_url = format!("file://{display_path}");

    // Try inline image display (terminal-specific protocols)
    if try_display_inline(path) {
        // Flush to ensure the image escape sequence is sent before the link
        let _ = std::io::stdout().flush();
        println!();
    }

    // OSC 8 clickable hyperlink: ESC ] 8 ; ; url BEL text ESC ] 8 ; ; BEL
    println!("  Image saved: \x1b]8;;{file_url}\x07{display_path}\x1b]8;;\x07");
}

/// Attempt to display an image inline using terminal-specific image protocols.
/// Returns true if we attempted display (we can't easily detect if it actually rendered).
fn try_display_inline(path: &std::path::Path) -> bool {
    use std::io::Write;

    let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();
    let term = std::env::var("TERM").unwrap_or_default();

    let Ok(data) = std::fs::read(path) else {
        return false;
    };

    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);

    // Kitty graphics protocol: Kitty, Ghostty
    if term_program.contains("kitty")
        || term_program.contains("ghostty")
        || term.contains("xterm-kitty")
        || term.contains("xterm-ghostty")
    {
        display_kitty(&b64);
        let _ = std::io::stdout().flush();
        return true;
    }

    // iTerm2 inline image protocol: iTerm2, WezTerm
    if term_program.contains("iTerm") || term_program.contains("WezTerm") {
        display_iterm2(&b64);
        let _ = std::io::stdout().flush();
        return true;
    }

    false
}

/// Display image using the iTerm2 inline image protocol (iTerm2, WezTerm).
fn display_iterm2(b64: &str) {
    // ESC ] 1337 ; File=[args] : base64data BEL
    print!("\x1b]1337;File=inline=1;width=20;preserveAspectRatio=1:{b64}\x07");
}

/// Display image using the Kitty graphics protocol.
/// Sends base64 PNG data in chunks of up to 4096 bytes.
fn display_kitty(b64: &str) {
    // Kitty protocol: ESC_APC G <key>=<value>,... ; <base64 data> ESC \
    // First chunk: a=T (transmit+display), f=100 (PNG), m=1 (more chunks follow)
    // Last chunk: m=0 (final)
    let chunk_size = 4096;
    let chunks: Vec<&str> = b64
        .as_bytes()
        .chunks(chunk_size)
        .map(|c| std::str::from_utf8(c).unwrap_or(""))
        .collect();

    for (i, chunk) in chunks.iter().enumerate() {
        let is_first = i == 0;
        let is_last = i == chunks.len() - 1;
        let more = if is_last { 0 } else { 1 };

        if is_first {
            print!("\x1b_Ga=T,f=100,m={more};{chunk}\x1b\\");
        } else {
            print!("\x1b_Gm={more};{chunk}\x1b\\");
        }
    }
}

/// Ask the user whether to keep or delete the test image.
pub(super) fn offer_cleanup(path: &std::path::Path) {
    println!();
    let keep = inquire::Confirm::new("Keep the generated image?")
        .with_default(false)
        .prompt()
        .unwrap_or(false);

    if keep {
        println!("  Image kept at {}", path.display());
    } else if std::fs::remove_file(path).is_ok() {
        println!("  Image deleted.");
    }
}
