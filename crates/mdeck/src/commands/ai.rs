use crate::cli::AiCommands;
use crate::config::{AiConfig, AiProvider, Config};
use anyhow::{Context, Result};
use colored::Colorize;

pub fn run(cmd: AiCommands) -> Result<()> {
    match cmd {
        AiCommands::Init => init(),
        AiCommands::Status => status(),
        AiCommands::Remove => remove(),
    }
}

fn init() -> Result<()> {
    let available: Vec<&AiProvider> = AiProvider::all()
        .iter()
        .filter(|p| p.is_available())
        .collect();

    if available.is_empty() {
        println!(
            "{} No supported AI providers found on your system.\n",
            "!".yellow().bold()
        );
        println!("Install one of the following:");
        for provider in AiProvider::all() {
            println!(
                "  {} — {}",
                provider.display_name().bold(),
                provider.description()
            );
        }
        anyhow::bail!("No AI providers available");
    }

    let items: Vec<String> = available
        .iter()
        .map(|p| format!("{:<14} — {}", p.display_name(), p.description()))
        .collect();

    let selection = inquire::Select::new("Select an AI provider:", items.clone())
        .prompt()
        .context("Selection cancelled")?;

    let idx = items.iter().position(|i| i == &selection).unwrap();
    let provider = available[idx].clone();

    let model = provider.default_model().map(String::from);

    let ai_config = AiConfig {
        provider: provider.clone(),
        model,
    };

    let mut config = Config::load_or_default();
    config.ai = Some(ai_config);
    let path = config.save()?;

    println!();
    println!(
        "{} Configured {} as AI provider.",
        "Done!".green().bold(),
        provider.display_name().cyan()
    );
    println!("  Saved to {}", path.display().to_string().dimmed());

    Ok(())
}

fn status() -> Result<()> {
    let config = Config::load_or_default();

    match config.ai {
        Some(ai) => {
            println!(
                "{} {}",
                "Provider:".bold(),
                ai.provider.display_name().cyan()
            );
            if let Some(model) = &ai.model {
                println!("{} {}", "Model:".bold(), model);
            }
        }
        None => {
            println!(
                "{} AI is not configured. Run {} to set up a provider.",
                "!".yellow().bold(),
                "mdeck ai init".cyan()
            );
        }
    }

    Ok(())
}

fn remove() -> Result<()> {
    let mut config = Config::load_or_default();

    if config.ai.is_none() {
        println!("AI is not configured. Nothing to remove.");
        return Ok(());
    }

    config.ai = None;
    let path = config.save()?;

    println!("{} AI configuration removed.", "Done!".green().bold());
    println!("  Saved to {}", path.display().to_string().dimmed());

    Ok(())
}
