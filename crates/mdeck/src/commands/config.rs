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
    show_defaults(&config);
    println!();
    show_routing(&config);
    println!();
    show_styles(&config);
    println!();
    show_ai();
    Ok(())
}

/// One indented `key: value` line.
fn field(key: &str, value: impl std::fmt::Display) {
    println!("  {} {}", key.bold(), value);
}

fn show_defaults(config: &Config) {
    let Some(defaults) = &config.defaults else {
        println!("{} (not set)", "defaults:".bold());
        return;
    };
    let or_unset = |v: &Option<String>| v.as_deref().unwrap_or("(not set)").to_string();
    println!("{}", "defaults:".bold());
    field("theme:", or_unset(&defaults.theme));
    field("transition:", or_unset(&defaults.transition));
    field("aspect:", or_unset(&defaults.aspect));
    field("start_mode:", or_unset(&defaults.start_mode));
    field("image_style:", or_unset(&defaults.image_style));
    field("icon_style:", or_unset(&defaults.icon_style));
    field(
        "monitor_position:",
        format_monitor_position(defaults.monitor_position),
    );
}

fn show_routing(config: &Config) {
    match &config.routing {
        Some(r) => {
            println!("{}", "routing:".bold());
            field("length:", r.length);
            field("turn:", r.turn);
            field("lane_change:", r.lane_change);
            field("crossing:", r.crossing);
        }
        None => println!("{} (defaults)", "routing:".bold()),
    }
}

fn show_styles(config: &Config) {
    println!(
        "{} {} image, {} icon (see {})",
        "styles:".bold(),
        config.list_styles().len(),
        config.list_icon_styles().len(),
        "mdeck ai style list".cyan()
    );
}

/// The default chat node from ailloy's configuration.
fn show_ai() {
    let node = ailloy::config::Config::load().ok().and_then(|c| {
        c.default_chat_node().ok().map(|(id, node)| {
            (
                id.to_string(),
                format!("{:?}", node.provider),
                node.model.clone(),
            )
        })
    });
    match node {
        Some((id, provider, model)) => {
            println!("{}", "ai (via ailloy):".bold());
            field("node:", id.cyan());
            field("provider:", provider);
            if let Some(model) = model {
                field("model:", model);
            }
        }
        None => {
            println!("{} (not set: run {})", "ai:".bold(), "ailloy config".cyan());
        }
    }
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
