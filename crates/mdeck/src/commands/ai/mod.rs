//! AI feature management
//!
//! `mdeck ai`                : show status (chat + image generation)
//! `mdeck ai test`           : test AI connection (chat and/or image generation)
//! `mdeck ai enable`         : enable AI for mdeck
//! `mdeck ai disable`        : disable AI for mdeck
//! `mdeck ai config`         : interactive config wizard
//! `mdeck ai style ...`      : manage image styles
//! `mdeck ai generate-image` : generate a single image from a prompt
//! `mdeck ai generate`       : generate all AI images for a presentation

mod image;
mod style;
mod style_chat;

use anyhow::Result;
use colored::Colorize;

use ailloy::config_tui;

pub use image::{display_image_result, image_ext};

use crate::cli::AiCommands;
use crate::config::Config;
use crate::prompt;

const APP_NAME: &str = "mdeck";

pub async fn run(cmd: Option<AiCommands>, quiet: bool) -> Result<()> {
    match cmd {
        None => config_tui::print_ai_status(APP_NAME, &["chat", "image"]),
        Some(AiCommands::Test { message }) => test(message).await,
        Some(AiCommands::Enable) => config_tui::enable_ai(APP_NAME),
        Some(AiCommands::Disable) => config_tui::disable_ai(APP_NAME),
        Some(AiCommands::Config) => {
            let mut config = ailloy::config::Config::load_global()?;
            config_tui::run_interactive_config(&mut config, &["chat", "image"]).await?;
            Ok(())
        }
        Some(AiCommands::Style { command }) => style::run_style(command).await,
        Some(AiCommands::GenerateImage(args)) => image::generate_image_cmd(args).await,
        Some(AiCommands::Generate { file, force, style }) => {
            crate::commands::generate::run(file, force, style, quiet).await
        }
        Some(AiCommands::Create(args)) => crate::commands::create::run(args, quiet).await,
        Some(AiCommands::Story {
            file,
            slide,
            range,
            stale,
            force,
            dry_run,
        }) => crate::commands::story::run(file, slide, range, stale, force, dry_run, quiet).await,
        Some(AiCommands::Art {
            file,
            slide,
            stale,
            force,
            dry_run,
            engine,
            node,
        }) => {
            let opts = crate::commands::art::Options {
                slide,
                stale,
                force,
                dry_run,
                engine,
                node,
                quiet,
            };
            crate::commands::art::run(file, opts).await
        }
        Some(AiCommands::Status) => config_tui::print_ai_status(APP_NAME, &["chat", "image"]),
        Some(AiCommands::Skill { emit, reference }) => {
            crate::commands::skill::run(emit, reference);
            Ok(())
        }
    }
}

/// Check if ailloy has a default node for a capability.
pub fn has_capability(cap: &str) -> bool {
    if config_tui::is_ai_disabled(APP_NAME) {
        return false;
    }
    ailloy::config::Config::load()
        .ok()
        .and_then(|c| c.default_node_for(cap).ok().map(|_| true))
        .unwrap_or(false)
}

async fn test(message: Option<String>) -> Result<()> {
    let has_chat = has_capability("chat");
    let has_image = has_capability("image");

    if !has_chat && !has_image {
        println!("{} No AI features configured.\n", "✗".red().bold());
        println!(
            "  Run {} to set up AI.",
            format!("{APP_NAME} ai config").cyan()
        );
        anyhow::bail!("No AI features configured");
    }

    // Always prompt so the user sees what's available
    let choice = inquire::Select::new(
        "What would you like to test?",
        test_options(has_chat, has_image),
    )
    .prompt()?;

    let mut all_passed = true;
    if choice.contains("Chat") || choice.contains("Both") {
        all_passed &= test_chat(message).await;
    }
    if choice.contains("Image") || choice.contains("Both") {
        all_passed &= test_image(has_image).await?;
    }

    print_unconfigured(has_chat, has_image);

    if all_passed {
        Ok(())
    } else {
        println!(
            "\n  Run {} to check your configuration.",
            format!("{APP_NAME} ai config").cyan()
        );
        anyhow::bail!("One or more AI tests failed");
    }
}

/// The choices `ai test` offers, given which capabilities are configured.
fn test_options(has_chat: bool, has_image: bool) -> Vec<&'static str> {
    if has_chat && has_image {
        vec![
            "Both chat completion and image generation",
            "Chat completion only",
            "Image generation only",
        ]
    } else if has_chat {
        vec!["Chat completion"]
    } else {
        vec!["Image generation"]
    }
}

fn print_fail(e: &anyhow::Error) {
    println!("  {}\n", "✗ FAIL".red().bold());
    println!("  Error: {e}");
}

/// Send one chat message and print the reply. Returns whether it passed.
async fn test_chat(message: Option<String>) -> bool {
    println!("\n{}", "Testing chat completion...".bold());
    let msg = message.unwrap_or_else(|| "Say hello in one sentence.".to_string());

    let result: Result<ailloy::ChatResponse> = async {
        let client = ailloy::Client::for_capability("chat")?;
        client.chat(&[ailloy::Message::user(&msg)]).await
    }
    .await;

    match result {
        Ok(response) => {
            println!("  {}\n", "✓ PASS".green().bold());
            println!("  {}", response.content);
            true
        }
        Err(e) => {
            print_fail(&e);
            false
        }
    }
}

/// The image prompt `ai test` sends for a normal image or an icon.
fn test_image_prompt(config: &Config, icon: bool) -> String {
    if icon {
        let style = config.resolve_icon_style();
        prompt::build_icon_prompt(style, "A database")
    } else {
        let style = config.resolve_image_style();
        prompt::build_image_prompt(
            style,
            "A bunch of papers, presentation slides, and notes scattered on a messy wooden \
             desk \u{2014} a tribute to the old way of making presentations before mdeck.",
            prompt::Orientation::Horizontal,
        )
    }
}

/// Generate one image, show it and offer to delete it. Returns whether it passed.
async fn test_image(has_image: bool) -> Result<bool> {
    println!("\n{}", "Testing image generation...".bold());

    // Ask what kind of image to test
    let image_type = if has_image {
        let choices = vec!["Normal image", "Icon"];
        inquire::Select::new("What type of image?", choices)
            .prompt()
            .unwrap_or("Normal image")
    } else {
        "Normal image"
    };

    let config = Config::load_or_default();
    let test_prompt = test_image_prompt(&config, image_type == "Icon");

    let result: Result<ailloy::ImageResponse> = async {
        let client = ailloy::Client::for_capability("image")?;
        client.generate_image(&test_prompt).await
    }
    .await;

    match result {
        Ok(response) => {
            let ext = image_ext(&response.format);
            let path = std::env::temp_dir().join(format!("mdeck-ai-test.{ext}"));
            std::fs::write(&path, &response.data)?;

            println!("  {}", "✓ PASS".green().bold());
            println!(
                "  Generated {}x{} {} image",
                response.width,
                response.height,
                ext.to_uppercase()
            );
            if let Some(ref revised) = response.revised_prompt {
                println!("  Revised prompt: {}", revised.dimmed());
            }
            println!();
            display_image_result(&path);
            image::offer_cleanup(&path);
            Ok(true)
        }
        Err(e) => {
            print_fail(&e);
            Ok(false)
        }
    }
}

/// Say which capability was not tested, and why.
fn print_unconfigured(has_chat: bool, has_image: bool) {
    if !has_image {
        println!(
            "\n  {} Image generation not configured: run {} to add an image provider",
            "ℹ".blue().bold(),
            format!("{APP_NAME} ai config").cyan()
        );
    }
    if !has_chat {
        println!(
            "\n  {} Chat completion not configured: run {} to add a chat provider",
            "ℹ".blue().bold(),
            format!("{APP_NAME} ai config").cyan()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_options_follow_capabilities() {
        assert_eq!(test_options(true, true).len(), 3);
        assert_eq!(test_options(true, false), vec!["Chat completion"]);
        assert_eq!(test_options(false, true), vec!["Image generation"]);
    }

    #[test]
    fn test_image_prompt_keeps_the_desk_scene() {
        let config = Config::default();
        let p = test_image_prompt(&config, false);
        assert!(p.contains("messy wooden desk \u{2014} a tribute"));
    }
}
