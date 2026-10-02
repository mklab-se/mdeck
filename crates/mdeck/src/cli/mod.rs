//! The command line: arguments, subcommands and dispatch.

mod ai;

use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

pub use ai::{AiCommands, CreateArgs, GenerateImageArgs, StyleCommands};

#[derive(Parser)]
#[command(name = "mdeck")]
#[command(author, version, about)]
#[command(long_about = "A markdown-based presentation tool.\n\n\
    Write your slides in standard markdown and present them beautifully.\n\n\
    Examples:\n  \
    mdeck slides.md              Launch presentation (fullscreen)\n  \
    mdeck slides.md --windowed   Launch in a window\n  \
    mdeck spec                   Print format specification\n  \
    mdeck spec --short           Print quick reference card\n  \
    mdeck theme list             Themes you can use (built-in and your own)\n  \
    mdeck theme new acme         Start a custom theme in ./themes")]
#[command(propagate_version = true)]
#[command(args_conflicts_with_subcommands = true)]
pub struct Cli {
    /// Markdown file to present
    pub file: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Launch in a window instead of fullscreen
    #[arg(long, global = false)]
    pub windowed: bool,

    /// Start on a specific slide (1-indexed)
    #[arg(long, global = false)]
    pub slide: Option<usize>,

    /// Start in grid overview mode
    #[arg(long, global = false)]
    pub overview: bool,

    /// Present on this engine instead of the theme's (plain, particles, ...)
    #[arg(long, global = false)]
    pub engine: Option<String>,

    /// Present in this theme instead of @theme and the config default
    #[arg(long, global = false)]
    pub theme: Option<String>,

    /// Open the presenter view (current and next slide, notes, timer) in a
    /// second window; `V` toggles it while presenting
    #[arg(long, global = false)]
    pub presenter: bool,

    /// Show every slide and reveal step settled: no transitions, entry
    /// animations or engine motion (also `defaults.reduced_motion`)
    #[arg(long, global = false)]
    pub reduced_motion: bool,

    /// Validate presentation and report problems without launching GUI
    #[arg(long, global = false)]
    pub check: bool,

    /// Increase output verbosity (with --check: print per-slide details)
    #[arg(short, long, action = ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Suppress non-essential output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Disable colored output
    #[arg(long, global = true)]
    pub no_color: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage AI features (shows status when run without a subcommand)
    Ai {
        #[command(subcommand)]
        command: Option<AiCommands>,
    },

    /// View and modify configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },

    /// Generate shell completions
    Completion {
        /// Target shell
        #[arg(value_enum)]
        shell: Shell,
    },

    /// Export slides as PNG images or a PDF
    Export {
        /// Markdown file to export
        file: PathBuf,

        /// Output directory (PNG files, or the PDF)
        #[arg(short, long, default_value = "export")]
        output_dir: PathBuf,

        /// Output format: png (one file per slide) or pdf (one document)
        #[arg(long, value_enum, default_value_t = crate::commands::export::Format::Png)]
        format: crate::commands::export::Format,

        /// PDF only: add speaker notes pages (slide on top, notes below)
        #[arg(long)]
        notes: bool,

        /// Export width in pixels
        #[arg(long, default_value = "1920")]
        width: u32,

        /// Export height in pixels
        #[arg(long, default_value = "1080")]
        height: u32,

        /// Debug mode: export every reveal step of every slide
        #[arg(long)]
        debug: bool,

        /// Export only this slide (1-based); file names keep the deck's numbering
        #[arg(long, conflicts_with = "range")]
        slide: Option<usize>,

        /// Export only these slides, e.g. 3-7 (1-based, inclusive)
        #[arg(long)]
        range: Option<String>,

        /// Theme to export with, overriding @theme and the config default
        #[arg(long)]
        theme: Option<String>,

        /// Engine to export with, overriding @engine and the theme's (plain, particles, ...)
        #[arg(long)]
        engine: Option<String>,

        /// A still of the motion: run the engine this many seconds from a cold start
        #[arg(long, value_name = "SECONDS")]
        at: Option<f32>,

        /// Export a moment instead of the slides: the opening countdown (or one
        /// of its digits, or the burst) or the end
        #[arg(long, value_enum)]
        moment: Option<crate::commands::export::Moment>,

        /// Draw the presenter view (current and next slide, notes, timer) for
        /// each slide, to look at it without a second display
        #[arg(long, hide = true)]
        presenter_view: bool,
    },

    /// Point cloud illustrations for the particle field (generate, import, list, show)
    Illustration {
        #[command(subcommand)]
        command: IllustrationCommands,
    },

    /// Custom themes: list, check, create (optionally from a design system) and preview
    Theme {
        #[command(subcommand)]
        command: ThemeCommands,
    },

    /// Print the mdeck markdown format specification
    Spec {
        /// Print a concise quick-reference card instead of the full spec
        #[arg(long)]
        short: bool,
    },

    /// Show version information
    Version,
}

#[derive(Subcommand)]
pub enum IllustrationCommands {
    /// Generate an illustration from a description with the AI image provider
    Generate {
        /// Name to save it under (lowercase letters, digits and hyphens)
        #[arg(long)]
        name: String,
        /// What to draw, e.g. "A server, in a rack, in a datacenter"
        #[arg(long)]
        description: String,
        /// Save to the user library (~/.config/mdeck/illustrations) instead of ./illustrations
        #[arg(long)]
        user: bool,
        /// Overwrite an existing illustration of the same name
        #[arg(long)]
        force: bool,
    },
    /// Convert an image (light strokes on dark) into an illustration
    Import {
        /// Image file (PNG, JPEG or WebP)
        image: PathBuf,
        /// Name to save it under (lowercase letters, digits and hyphens)
        #[arg(long)]
        name: String,
        /// Save to the user library instead of ./illustrations
        #[arg(long)]
        user: bool,
        /// Overwrite an existing illustration of the same name
        #[arg(long)]
        force: bool,
    },
    /// List every illustration visible from the current directory
    List,
    /// Preview an illustration as an image
    Show {
        /// Illustration name
        name: String,
        /// Save the preview PNG here instead of a temporary file
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Offer one of your illustrations to MDeck's built-in set (opens a prefilled GitHub issue)
    Contribute {
        /// Illustration name (a deck or user illustration, not a built-in)
        name: String,
        /// Print the issue link instead of opening it in the browser
        #[arg(long)]
        no_open: bool,
    },
}

#[derive(Subcommand)]
pub enum ThemeCommands {
    /// List every theme visible from the current directory
    List,
    /// Check a theme for errors, fallbacks and hard-to-read colours
    Check {
        /// Theme name, or a path to a theme file
        name: String,
    },
    /// Write a new theme to ./themes (a commented starter, or from a design system with AI)
    New {
        /// Theme name (lowercase letters, digits, '-' and '_')
        name: String,
        /// Design system folder to convert with AI (SKILL.md, readme.md, CSS tokens, *.tokens.json); its fonts and logos are copied into the theme
        #[arg(long)]
        from: Option<PathBuf>,
        /// Write to the user theme folder instead of ./themes
        #[arg(long)]
        user: bool,
        /// Overwrite an existing theme of the same name
        #[arg(long)]
        force: bool,
    },
    /// Export a sampler deck in a theme, to look at
    Preview {
        /// Theme name, or a path to a theme file
        name: String,
        /// Output directory for the PNGs
        #[arg(long, short = 'o')]
        output_dir: PathBuf,
        /// Width in pixels
        #[arg(long, default_value = "1920")]
        width: u32,
        /// Height in pixels
        #[arg(long, default_value = "1080")]
        height: u32,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Display current configuration
    Show,

    /// Set a configuration value
    Set {
        /// Configuration key: defaults.theme (a built-in or user theme), defaults.transition
        /// (slide|fade|spatial|none), defaults.aspect (16:9|4:3|16:10),
        /// defaults.start_mode (first|overview|<slide number>)
        key: String,

        /// Value to set
        value: String,
    },
}

#[derive(Clone, ValueEnum)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    Powershell,
}

impl Cli {
    pub fn run(self) -> anyhow::Result<()> {
        match self.command {
            Some(Commands::Ai { command }) => {
                crate::commands::util::block_on(crate::commands::ai::run(command, self.quiet))?
            }
            Some(Commands::Config { command }) => crate::commands::config::run(command),
            Some(Commands::Illustration { command }) => crate::commands::util::block_on(
                crate::commands::illustration::run(command, self.quiet),
            )?,
            Some(Commands::Completion { shell }) => {
                crate::commands::completion::run(shell);
                Ok(())
            }
            Some(Commands::Export {
                file,
                output_dir,
                width,
                height,
                debug,
                slide,
                range,
                format,
                notes,
                theme,
                engine,
                at,
                moment,
                presenter_view,
            }) => crate::commands::export::run(crate::commands::export::ExportArgs {
                file,
                output_dir,
                width,
                height,
                debug,
                slide,
                range,
                format,
                notes,
                theme: theme.map_or(crate::commands::export::ThemeChoice::Deck, |name| {
                    crate::commands::export::ThemeChoice::Named(name)
                }),
                engine,
                at,
                moment,
                presenter_view,
            }),
            Some(Commands::Theme { command }) => {
                crate::commands::util::block_on(crate::commands::theme::run(command, self.quiet))?
            }
            Some(Commands::Spec { short }) => {
                crate::commands::spec::run(short);
                Ok(())
            }
            Some(Commands::Version) => {
                crate::banner::print_banner_with_version();
                Ok(())
            }
            None => self.run_file(),
        }
    }

    /// No subcommand: present (or `--check`) the file, or print the help.
    fn run_file(self) -> anyhow::Result<()> {
        let Some(file) = self.file else {
            use clap::CommandFactory;
            let mut cmd = Self::command();
            cmd.print_help()?;
            println!();
            return Ok(());
        };
        if !file.exists() {
            anyhow::bail!("File not found: {}", file.display());
        }
        if self.check {
            return crate::commands::check::run(file, self.verbose, self.quiet, self.engine);
        }
        crate::app::run(crate::app::RunOptions {
            file,
            windowed: self.windowed,
            start_slide: self.slide,
            start_overview: self.overview,
            quiet: self.quiet,
            engine: self.engine,
            theme: self.theme,
            reduced_motion: self.reduced_motion,
            presenter: self.presenter,
        })
    }
}
