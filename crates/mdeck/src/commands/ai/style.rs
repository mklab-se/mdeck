//! `mdeck ai style ...`: manage image and icon styles.

use anyhow::Result;
use colored::Colorize;

use super::APP_NAME;
use crate::cli::StyleCommands;
use crate::config::{Config, DefaultsConfig};
use crate::prompt;

pub(super) async fn run_style(cmd: StyleCommands) -> Result<()> {
    match cmd {
        StyleCommands::Add {
            name,
            description,
            icon,
            references,
            interactive,
        }
        | StyleCommands::Set {
            name,
            description,
            icon,
            references,
            interactive,
        } => {
            if interactive {
                let saved = name.clone();
                super::style_chat::run_interactive_style(name, icon).await?;
                return match saved {
                    Some(n) if !references.is_empty() => save_references(&n, icon, references),
                    _ => Ok(()),
                };
            }
            let saved = name.clone();
            save_style(name, description, icon)?;
            match saved {
                Some(n) if !references.is_empty() => save_references(&n, icon, references),
                _ => Ok(()),
            }
        }
        StyleCommands::Remove { name, icon } => remove_style(&name, icon),
        StyleCommands::List => {
            list_styles();
            Ok(())
        }
        StyleCommands::Clear => {
            let mut config = Config::load_or_default();
            config.clear_styles();
            config.save()?;
            println!(
                "{} All styles cleared and defaults reset.",
                "✓".green().bold()
            );
            Ok(())
        }
        StyleCommands::SetDefault { name } => set_default(&name, false),
        StyleCommands::SetIconDefault { name } => set_default(&name, true),
        StyleCommands::ShowDefaults => {
            show_defaults();
            Ok(())
        }
    }
}

/// `style add` / `style set` with a name and description on the command line.
fn save_style(name: Option<String>, description: Option<String>, icon: bool) -> Result<()> {
    let name = name
        .ok_or_else(|| anyhow::anyhow!("Style name is required. Use -i for interactive mode."))?;
    let description = description.ok_or_else(|| {
        anyhow::anyhow!("Style description is required. Use -i for interactive mode.")
    })?;
    let mut config = Config::load_or_default();
    if icon {
        config.add_icon_style(&name, &description);
        config.save()?;
        println!(
            "{} Icon style {} saved.",
            "✓".green().bold(),
            name.cyan().bold()
        );
    } else {
        config.add_style(&name, &description);
        config.save()?;
        println!(
            "{} Image style {} saved.",
            "✓".green().bold(),
            name.cyan().bold()
        );
    }
    Ok(())
}

/// Give a saved style its reference images (absolute, so the style works
/// from any deck).
fn save_references(name: &str, icon: bool, references: Vec<std::path::PathBuf>) -> Result<()> {
    let mut refs = Vec::new();
    for r in references {
        if !r.is_file() {
            anyhow::bail!("reference image {} not found", r.display());
        }
        refs.push(std::fs::canonicalize(&r).unwrap_or(r));
    }
    let mut config = Config::load_or_default();
    if !config.set_style_references(name, icon, refs) {
        anyhow::bail!("no style named {name}");
    }
    config.save()?;
    println!("  with its reference images");
    Ok(())
}

fn remove_style(name: &str, icon: bool) -> Result<()> {
    let mut config = Config::load_or_default();
    let removed = if icon {
        config.remove_icon_style(name)
    } else {
        config.remove_style(name)
    };
    if removed {
        config.save()?;
        let kind = if icon { "Icon style" } else { "Image style" };
        println!("{} {kind} {} removed.", "✓".green().bold(), name.cyan());
    } else {
        let kind = if icon { "icon style" } else { "image style" };
        println!(
            "{} No {kind} named {} found.",
            "!".yellow().bold(),
            name.cyan()
        );
    }
    Ok(())
}

fn list_styles() {
    let config = Config::load_or_default();
    let styles = config.list_styles();
    let icon_styles = config.list_icon_styles();

    if styles.is_empty() && icon_styles.is_empty() {
        println!("No styles defined.");
        println!(
            "  Use {} to add one.",
            format!("{APP_NAME} ai style add <name> <description>").cyan()
        );
        return;
    }

    let defaults = config.defaults.as_ref();
    if !styles.is_empty() {
        let default_name = defaults.and_then(|d| d.image_style.as_deref());
        print_style_group("Image Styles", &styles, default_name);
    }

    if !icon_styles.is_empty() {
        if !styles.is_empty() {
            println!();
        }
        let default_name = defaults.and_then(|d| d.icon_style.as_deref());
        print_style_group("Icon Styles", &icon_styles, default_name);
    }
}

/// One titled list of styles, marking the default one.
fn print_style_group(title: &str, styles: &[(&str, &str)], default_name: Option<&str>) {
    println!("{}", title.bold().underline());
    for (name, desc) in styles {
        let marker = if default_name == Some(*name) {
            " (default)".green().to_string()
        } else {
            String::new()
        };
        println!("  {}{marker}", name.cyan().bold());
        println!("    {desc}");
    }
}

/// `style set-default` (image) or `style set-icon-default` (icon).
fn set_default(name: &str, icon: bool) -> Result<()> {
    let mut config = Config::load_or_default();
    let known = if icon {
        config.get_icon_style(name).is_some()
    } else {
        config.get_style(name).is_some()
    };
    if !known {
        if icon {
            anyhow::bail!(
                "No icon style named '{}'. Use `{APP_NAME} ai style add --icon` first.",
                name
            );
        }
        anyhow::bail!(
            "No image style named '{}'. Use `{APP_NAME} ai style add` first.",
            name
        );
    }
    let defaults = config.defaults.get_or_insert_with(DefaultsConfig::default);
    if icon {
        defaults.icon_style = Some(name.to_string());
    } else {
        defaults.image_style = Some(name.to_string());
    }
    config.save()?;
    println!(
        "{} Default {} style set to {}.",
        "✓".green().bold(),
        if icon { "icon" } else { "image" },
        name.cyan().bold()
    );
    Ok(())
}

fn show_defaults() {
    let config = Config::load_or_default();
    let defaults = config.defaults.as_ref();

    println!("{}", "Default Image Style".bold().underline());
    let image = defaults.and_then(|d| d.image_style.clone());
    print_default(
        image.as_deref(),
        |n| config.get_style(n),
        prompt::DEFAULT_IMAGE_STYLE,
    );

    println!("\n{}", "Default Icon Style".bold().underline());
    let icon = defaults.and_then(|d| d.icon_style.clone());
    print_default(
        icon.as_deref(),
        |n| config.get_icon_style(n),
        prompt::DEFAULT_ICON_STYLE,
    );
}

/// The configured default style's name and text, or the hardcoded fallback.
fn print_default<'a>(
    name: Option<&str>,
    lookup: impl Fn(&str) -> Option<&'a str>,
    hardcoded: &str,
) {
    match name {
        Some(name) => {
            if let Some(desc) = lookup(name) {
                println!("  {} {}", "Name:".bold(), name.cyan());
                println!("  {desc}");
            } else {
                println!(
                    "  {} (configured as '{}' but style not found, using hardcoded)",
                    "!".yellow().bold(),
                    name
                );
                println!("  {}", hardcoded.dimmed());
            }
        }
        None => {
            println!("  {} (hardcoded)", "(none set)".dimmed());
            println!("  {}", hardcoded.dimmed());
        }
    }
}
