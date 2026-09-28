// A build that leaves engines out (`--no-default-features`) also leaves the
// render code only they draw with unused; the full build checks dead code.
#![cfg_attr(
    not(all(
        feature = "particles",
        feature = "led",
        feature = "splitflap",
        feature = "laser",
        feature = "blocks",
        feature = "blueprint",
        feature = "sketch",
        feature = "chalkboard",
        feature = "watercolour",
        feature = "darkroom"
    )),
    allow(dead_code)
)]

mod app;
mod banner;
mod check;
mod cli;
mod commands;
mod config;
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

    if cli.no_color {
        colored::control::set_override(false);
    }

    if let Err(e) = cli.run() {
        eprintln!("{} {e:#}", "Error:".red().bold());
        std::process::exit(1);
    }
}
