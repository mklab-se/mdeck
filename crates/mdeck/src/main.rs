mod app;
mod banner;
mod check;
mod cli;
mod commands;
mod config;
mod deck;
mod engines;
mod incident_log;
mod parser;
mod prompt;
mod render;
mod theme;

use clap::{CommandFactory, Parser};
use colored::Colorize;

fn main() {
    clap_complete::CompleteEnv::with_factory(cli::Cli::command).complete();

    let cli = cli::Cli::parse();

    // The renderer never reads configuration; what it needs is handed over here.
    let config = config::Config::load_or_default();
    render::diagram::set_routing_weights(config.routing.unwrap_or_default().to_cost_weights());

    if cli.no_color {
        colored::control::set_override(false);
    }

    if let Err(e) = cli.run() {
        eprintln!("{} {e:#}", "Error:".red().bold());
        std::process::exit(1);
    }
}
