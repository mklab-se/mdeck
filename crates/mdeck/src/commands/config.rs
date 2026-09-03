use crate::cli::ConfigCommands;
use crate::config::Config;
use anyhow::Result;
use colored::Colorize;

pub fn run(cmd: ConfigCommands) -> Result<()> {
    match cmd {
        ConfigCommands::Show => show(),
        ConfigCommands::Set { key, value } => set(&key, &value),
    }
}

fn show() -> Result<()> {
    let config = Config::load_or_default();
    let path = Config::path()?;

    println!(
        "{} {}\n",
        "Config:".bold(),
        path.display().to_string().dimmed()
    );

    match &config.defaults {
        Some(defaults) => {
            println!("{}", "defaults:".bold());
            println!(
                "  {} {}",
                "theme:".bold(),
                defaults.theme.as_deref().unwrap_or("(not set)")
            );
            println!(
                "  {} {}",
                "transition:".bold(),
                defaults.transition.as_deref().unwrap_or("(not set)")
            );
            println!(
                "  {} {}",
                "aspect:".bold(),
                defaults.aspect.as_deref().unwrap_or("(not set)")
            );
            println!(
                "  {} {}",
                "start_mode:".bold(),
                defaults.start_mode.as_deref().unwrap_or("(not set)")
            );
            println!(
                "  {} {}",
                "image_style:".bold(),
                defaults.image_style.as_deref().unwrap_or("(not set)")
            );
            println!(
                "  {} {}",
                "icon_style:".bold(),
                defaults.icon_style.as_deref().unwrap_or("(not set)")
            );
            println!(
                "  {} {}",
                "monitor_position:".bold(),
                format_monitor_position(defaults.monitor_position)
            );
        }
        None => {
            println!("{} (not set)", "defaults:".bold());
        }
    }

    println!();
    match &config.routing {
        Some(r) => {
            println!("{}", "routing:".bold());
            println!("  {} {}", "length:".bold(), r.length);
            println!("  {} {}", "turn:".bold(), r.turn);
            println!("  {} {}", "lane_change:".bold(), r.lane_change);
            println!("  {} {}", "crossing:".bold(), r.crossing);
        }
        None => println!("{} (defaults)", "routing:".bold()),
    }

    println!();
    let styles = config.list_styles();
    let icon_styles = config.list_icon_styles();
    println!(
        "{} {} image, {} icon (see {})",
        "styles:".bold(),
        styles.len(),
        icon_styles.len(),
        "mdeck ai style list".cyan()
    );

    println!();

    match ailloy::config::Config::load().ok().and_then(|c| {
        c.default_chat_node().ok().map(|(id, node)| {
            (
                id.to_string(),
                format!("{:?}", node.provider),
                node.model.clone(),
            )
        })
    }) {
        Some((id, provider, model)) => {
            println!("{}", "ai (via ailloy):".bold());
            println!("  {} {}", "node:".bold(), id.cyan());
            println!("  {} {}", "provider:".bold(), provider);
            if let Some(model) = model {
                println!("  {} {}", "model:".bold(), model);
            }
        }
        None => {
            println!(
                "{} (not set — run {})",
                "ai:".bold(),
                "ailloy config".cyan()
            );
        }
    }

    Ok(())
}

fn format_monitor_position(pos: Option<[f32; 2]>) -> String {
    match pos {
        Some([x, y]) => format!("{x:.0}, {y:.0}"),
        None => "(not set)".to_string(),
    }
}

fn set(key: &str, value: &str) -> Result<()> {
    let mut config = Config::load_or_default();
    config.set(key, value)?;
    let path = config.save()?;

    println!(
        "{} Set {} = {}",
        "Done!".green().bold(),
        key.bold(),
        value.cyan()
    );
    println!("  Saved to {}", path.display().to_string().dimmed());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monitor_position_formatting() {
        assert_eq!(format_monitor_position(None), "(not set)");
        assert_eq!(format_monitor_position(Some([1920.0, 0.0])), "1920, 0");
        assert_eq!(format_monitor_position(Some([-1440.5, 12.0])), "-1440, 12");
    }
}
