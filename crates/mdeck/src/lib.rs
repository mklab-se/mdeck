//! mdeck as a library: the parser, designs, themes, the presentation window,
//! export and commands, and the built-in engines, visuals, themes and point
//! clouds. The `mdeck` binary is [`builtins`] plus [`run`]; a custom build
//! registers its own extensions next to the built-ins and calls [`run`] the
//! same way (D11, D15, EXT-06).
//!
//! ```no_run
//! let mut registry = mdeck_sdk::registry::Registry::new();
//! mdeck::builtins(&mut registry).unwrap();
//! // registry.set_origin("acme-glow"); acme_glow::register(&mut registry).unwrap();
//! std::process::exit(match mdeck::run(registry) {
//!     code if code == std::process::ExitCode::SUCCESS => 0,
//!     _ => 1,
//! });
//! ```

mod app;
mod assets;
mod banner;
mod check;
mod cli;
mod commands;
mod config;
mod deck;
mod engines;
mod extensions;
#[cfg(test)]
mod guards;
mod incident_log;
mod language;
mod parser;
mod prompt;
mod registry;
mod render;
mod theme;

use std::ffi::OsString;
use std::process::ExitCode;

use clap::{CommandFactory, Parser};
use colored::Colorize;
use mdeck_sdk::registry::{Registry, RegistryError};

/// Register everything mdeck ships with: its engines, visuals, themes and
/// point clouds, exactly as an extension registers its own (EXT-06).
pub fn builtins(registry: &mut Registry) -> Result<(), RegistryError> {
    engines::register(registry)?;
    render::visualizations::register(registry)?;
    for (name, yaml) in theme::lookup::BUILTIN {
        registry.theme(name, yaml)?;
    }
    for (name, text) in render::point_cloud::BUILTIN {
        registry.point_cloud(name, text.as_bytes())?;
    }
    Ok(())
}

/// Run mdeck with `registry` on the process's command line: what the `mdeck`
/// binary does with the built-ins, and a custom build with its extensions.
pub fn run(registry: Registry) -> ExitCode {
    run_with_args(registry, std::env::args_os())
}

/// [`run`] on `args` (the program name first) instead of the process's
/// command line.
pub fn run_with_args<I, T>(registry: Registry, args: I) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    if registry::install(registry).is_err() {
        eprintln!("{} mdeck::run was called twice", "Error:".red().bold());
        return ExitCode::FAILURE;
    }
    clap_complete::CompleteEnv::with_factory(cli::Cli::command).complete();

    let cli = match cli::Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(e) => e.exit(),
    };

    // The renderer never reads configuration; what it needs is handed over here.
    let config = config::Config::load_or_default();
    render::diagram::set_routing_weights(
        config.routing.clone().unwrap_or_default().to_cost_weights(),
    );
    extensions::external::configure(&config);

    if cli.no_color {
        colored::control::set_override(false);
    }

    match cli.run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{} {e:#}", "Error:".red().bold());
            ExitCode::FAILURE
        }
    }
}
