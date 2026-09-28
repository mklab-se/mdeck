use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

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
pub enum AiCommands {
    /// Test AI integration by sending a message
    Test {
        /// Message to send (default: "Say hello in one sentence.")
        message: Option<String>,
    },
    /// Enable AI features for mdeck
    Enable,
    /// Disable AI features for mdeck
    Disable,
    /// Interactively configure AI provider and model settings
    Config,
    /// Manage image styles
    Style {
        #[command(subcommand)]
        command: StyleCommands,
    },
    /// Generate a single image from a prompt
    GenerateImage(GenerateImageArgs),
    /// Generate AI images for a presentation
    Generate {
        /// Markdown file to process
        file: PathBuf,
        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
        /// Override the image style
        #[arg(long)]
        style: Option<String>,
    },
    /// Create a presentation from content using AI
    Create(CreateArgs),
    /// Write particle stories for themes on the particles engine, like Ember (saved next to the deck as <deck>.scenes.yaml)
    Story {
        /// Markdown file to process
        file: PathBuf,
        /// Only this slide (1-based)
        #[arg(long, conflicts_with = "range")]
        slide: Option<usize>,
        /// Only these slides, e.g. 3-7 (1-based, inclusive)
        #[arg(long)]
        range: Option<String>,
        /// Only refresh slides whose story has gone stale
        #[arg(long)]
        stale: bool,
        /// Regenerate even when the current story is up to date
        #[arg(long)]
        force: bool,
        /// Print the scripts and their spoken lines without writing the sidecar
        #[arg(long)]
        dry_run: bool,
    },
    /// Draw a picture for every slide on an art engine, like blueprint (saved in art/ next to the deck, recorded in <deck>.art.yaml)
    Art {
        /// Markdown file to process
        file: PathBuf,
        /// Only this slide (1-based); redraws it even when its picture is current
        #[arg(long)]
        slide: Option<usize>,
        /// Only redraw slides whose picture has gone stale
        #[arg(long)]
        stale: bool,
        /// Redraw even when a slide's picture is current
        #[arg(long)]
        force: bool,
        /// List the slides that would be drawn, without generating anything
        #[arg(long)]
        dry_run: bool,
        /// Draw for this engine instead of the deck's (e.g. blueprint)
        #[arg(long)]
        engine: Option<String>,
        /// Use this image node instead of the default (see `ailloy ai config list-nodes`)
        #[arg(long)]
        node: Option<String>,
    },
    /// Show AI status (same as running `mdeck ai` without a subcommand)
    Status,
    /// AI agent skill information — helps set up Claude Code skills for mdeck
    Skill {
        /// Output the skill markdown content (ready to save as a skill file)
        #[arg(long)]
        emit: bool,

        /// Output detailed reference documentation for AI agents
        #[arg(long)]
        reference: bool,
    },
}

#[derive(Args)]
pub struct GenerateImageArgs {
    /// Image prompt
    #[arg(long)]
    pub prompt: String,
    /// Named style or literal description to apply
    #[arg(long)]
    pub style: Option<String>,
    /// Output file path
    #[arg(long)]
    pub output: Option<PathBuf>,
    /// Generate as icon (square, transparent bg)
    #[arg(long)]
    pub icon: bool,
}

#[derive(Args)]
pub struct CreateArgs {
    /// Input: file path or quoted text (auto-detected). Reads stdin if omitted and stdin is piped.
    #[arg(long)]
    pub input: Option<String>,

    /// Output path: .md file or directory (creates presentation.md inside).
    /// Defaults to an AI-suggested name (presentation.md with --quiet).
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Custom prompt: audience, purpose, tone guidance for the presentation
    #[arg(long)]
    pub prompt: Option<String>,

    /// Interactive mode: guided conversation to shape the presentation before generating
    #[arg(short, long)]
    pub interactive: bool,

    /// Image style for AI-generated images
    #[arg(long)]
    pub style: Option<String>,
}

#[derive(Subcommand)]
pub enum StyleCommands {
    /// Add or update a named image style
    Add {
        /// Style name (prompted interactively if omitted with -i)
        name: Option<String>,
        /// Style description (prompted interactively if omitted with -i)
        description: Option<String>,
        /// Add as icon style instead of image style
        #[arg(long)]
        icon: bool,
        /// Interactive mode — AI helps you craft the style
        #[arg(short, long)]
        interactive: bool,
    },
    /// Set (add or update) a named image style (alias for `add`)
    Set {
        /// Style name (prompted interactively if omitted with -i)
        name: Option<String>,
        /// Style description (prompted interactively if omitted with -i)
        description: Option<String>,
        /// Set as icon style instead of image style
        #[arg(long)]
        icon: bool,
        /// Interactive mode — AI helps you craft the style
        #[arg(short, long)]
        interactive: bool,
    },
    /// Remove a named style
    Remove {
        /// Style name
        name: String,
        /// Remove from icon styles instead of image styles
        #[arg(long)]
        icon: bool,
    },
    /// List all defined styles
    List,
    /// Remove all styles and reset defaults
    Clear,
    /// Set the default image style (used when no style is specified)
    SetDefault {
        /// Name of an existing style
        name: String,
    },
    /// Set the default icon style (used for diagram icon generation)
    SetIconDefault {
        /// Name of an existing icon style
        name: String,
    },
    /// Show the current default styles (including hardcoded fallbacks)
    ShowDefaults,
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
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?;
                rt.block_on(crate::commands::ai::run(command, self.quiet))
            }
            Some(Commands::Config { command }) => crate::commands::config::run(command),
            Some(Commands::Illustration { command }) => {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?;
                rt.block_on(crate::commands::illustration::run(command, self.quiet))
            }
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
            }) => crate::commands::export::run(
                file,
                output_dir,
                width,
                height,
                debug,
                slide,
                range,
                format,
                notes,
                theme.map_or(crate::commands::export::ThemeChoice::Deck, |name| {
                    crate::commands::export::ThemeChoice::Named(name)
                }),
                engine,
            ),
            Some(Commands::Theme { command }) => {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?;
                rt.block_on(crate::commands::theme::run(command, self.quiet))
            }
            Some(Commands::Spec { short }) => {
                crate::commands::spec::run(short);
                Ok(())
            }
            Some(Commands::Version) => {
                crate::banner::print_banner_with_version();
                Ok(())
            }
            None => {
                if let Some(file) = self.file {
                    if !file.exists() {
                        anyhow::bail!("File not found: {}", file.display());
                    }
                    if self.check {
                        return crate::commands::check::run(
                            file,
                            self.verbose,
                            self.quiet,
                            self.engine,
                        );
                    }
                    crate::app::run(
                        file,
                        self.windowed,
                        self.slide,
                        self.overview,
                        self.quiet,
                        self.engine,
                    )
                } else {
                    use clap::CommandFactory;
                    let mut cmd = Self::command();
                    cmd.print_help()?;
                    println!();
                    Ok(())
                }
            }
        }
    }
}
