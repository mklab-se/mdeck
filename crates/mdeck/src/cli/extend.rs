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
        /// An extension: a crate folder, a crate name with an optional
        /// version (`acme-engines@1.2`), or a git repository
        /// (`git+https://github.com/acme/aurora#v0.2.0`, `git+ssh://...`,
        /// `git@host:org/repo`; `#` a tag, branch or commit). Repeat for more
        #[arg(
            long = "with",
            value_name = "PATH|CRATE[@VERSION]|GIT-URL[#REF]",
            required = true
        )]
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
    ///
    /// A pack is a folder (or a .zip of one, or a git repository) with an
    /// mdeck-pack.yaml manifest (name, version, description, min-mdeck) and
    /// any of these folders:
    ///
    ///   themes/        themes, chosen by name (`theme: acme`)
    ///   designs/       design sets a theme names with `designs:`
    ///   point-clouds/  .mdpc point clouds, used by name
    ///   styles/        named AI styles (<name>.yaml: prompt, kind, references)
    ///   fonts/         font files the pack's own themes name
    ///
    /// Packs come after the deck's and your own folders and before the
    /// built-ins; the deck's packs (./packs) come before yours.
    #[command(verbatim_doc_comment)]
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

    /// Export a built-in preview deck (every design, a chart, images, a
    /// picture) and the countdown and end moments with an engine and theme
    Preview {
        /// The engine to preview, by name (an extension's engine in a build
        /// with it) [default: the theme's]
        #[arg(long)]
        engine: Option<String>,

        /// The theme to preview with [default: the configured default]
        #[arg(long)]
        theme: Option<String>,

        /// Where the PNGs go
        #[arg(short, long, default_value = "sdk-preview")]
        output_dir: PathBuf,

        /// Export width in pixels
        #[arg(long, default_value_t = 1920)]
        width: u32,

        /// Export height in pixels
        #[arg(long, default_value_t = 1080)]
        height: u32,
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
            ExtendCommands::Sdk {
                command:
                    SdkCommands::Preview {
                        engine,
                        theme,
                        output_dir,
                        width,
                        height,
                    },
            } => crate::commands::sdk::preview::run(crate::commands::sdk::preview::PreviewArgs {
                engine,
                theme,
                output_dir,
                width,
                height,
                quiet,
            }),
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
