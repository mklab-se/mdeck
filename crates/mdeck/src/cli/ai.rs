//! `mdeck ai ...` subcommands and their arguments.

use clap::{Args, Subcommand};
use std::path::PathBuf;

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
    /// Draw a picture for every slide on an art engine, like line (saved in art/ next to the deck, recorded in <deck>.art.yaml)
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
        /// Draw for this engine instead of the deck's (e.g. line)
        #[arg(long)]
        engine: Option<String>,
        /// Use this image node instead of the default (see `ailloy ai config list-nodes`)
        #[arg(long)]
        node: Option<String>,
    },
    /// Show AI status (same as running `mdeck ai` without a subcommand)
    Status,
    /// AI agent skill information: helps set up Claude Code skills for mdeck
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
        /// Interactive mode: AI helps you craft the style
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
        /// Interactive mode: AI helps you craft the style
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
