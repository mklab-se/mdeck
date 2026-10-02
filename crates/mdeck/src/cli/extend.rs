//! Subcommands for extending mdeck: `sdk`, `build`, `pack` and
//! `extensions`, flattened into the top-level command list.

use clap::Subcommand;
use std::path::PathBuf;

use crate::commands::sdk::TemplateKind;

#[derive(Subcommand)]
pub enum ExtendCommands {
    /// Write extensions in Rust: `mdeck sdk new engine glow` scaffolds a crate
    Sdk {
        #[command(subcommand)]
        command: SdkCommands,
    },

    /// Build an mdeck with extension crates in it (needs a Rust toolchain)
    Build {
        /// An extension: a crate folder, or a crate name with an optional
        /// version (`acme-engines@1.2`). Repeat for more
        #[arg(long = "with", value_name = "PATH|CRATE[@VERSION]", required = true)]
        with: Vec<String>,

        /// Where the binary goes: a file, or a folder to put it in
        /// [default: ./target/release/mdeck]
        #[arg(long)]
        out: Option<PathBuf>,

        /// The binary's name
        #[arg(long, default_value = "mdeck")]
        name: String,

        /// Build against this mdeck checkout instead of the published mdeck
        /// (also `MDECK_SOURCE`)
        #[arg(long, value_name = "DIR")]
        mdeck_path: Option<PathBuf>,
    },

    /// Packs of themes, designs, point clouds, styles and fonts: install, list, remove
    Pack {
        #[command(subcommand)]
        command: PackCommands,
    },

    /// What extends this mdeck: packs, engines, visuals, transitions, themes
    Extensions {
        #[command(subcommand)]
        command: ExtensionsCommands,
    },
}

#[derive(Subcommand)]
pub enum SdkCommands {
    /// Create an extension crate that builds and runs as it is
    New {
        /// What to make
        #[arg(value_enum)]
        kind: TemplateKind,

        /// Its name (lowercase letters, digits, hyphens): the crate and the
        /// name decks select it by
        name: String,

        /// The folder to create [default: ./<name>]
        #[arg(long)]
        dir: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
pub enum PackCommands {
    /// Install a pack from a folder, a .zip or a git URL (replaces an
    /// installed pack of the same name)
    Install {
        /// A pack folder, a .zip of one, or a git URL
        source: String,

        /// Install into this deck folder's `packs/` instead of the user folder
        #[arg(long)]
        deck: bool,

        /// Install into the user folder (the default)
        #[arg(long, conflicts_with = "deck")]
        user: bool,
    },

    /// List the packs installed for the user and in this folder's `packs/`
    List,

    /// Remove an installed pack
    Remove {
        /// The pack's name
        name: String,

        /// Remove it from this deck folder's `packs/` instead of the user folder
        #[arg(long)]
        deck: bool,
    },
}

#[derive(Subcommand)]
pub enum ExtensionsCommands {
    /// List installed packs and what this mdeck provides, with origins
    List,
}

impl ExtendCommands {
    pub fn run(self, quiet: bool) -> anyhow::Result<()> {
        match self {
            ExtendCommands::Sdk {
                command: SdkCommands::New { kind, name, dir },
            } => crate::commands::sdk::run_new(kind, &name, dir, quiet),
            ExtendCommands::Build {
                with,
                out,
                name,
                mdeck_path,
            } => crate::commands::build::run(crate::commands::build::BuildArgs {
                with,
                out,
                name,
                mdeck_path,
                quiet,
            }),
            ExtendCommands::Pack { command } => match command {
                PackCommands::Install { source, deck, .. } => {
                    crate::commands::pack::install(&source, deck, quiet)
                }
                PackCommands::List => crate::commands::pack::list(),
                PackCommands::Remove { name, deck } => {
                    crate::commands::pack::remove(&name, deck, quiet)
                }
            },
            ExtendCommands::Extensions {
                command: ExtensionsCommands::List,
            } => crate::commands::extensions::list(),
        }
    }
}
