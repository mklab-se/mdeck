//! `mdeck ai ...`: everything AI makes, one subcommand per kind, plus the
//! management commands. Bare `mdeck ai <deck.md>` makes everything the deck
//! is missing.

use clap::{Args, Subcommand};
use std::path::PathBuf;

/// `mdeck ai [<deck.md>] [options]` or `mdeck ai <subcommand>`.
#[derive(Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct AiArgs {
    #[command(subcommand)]
    pub command: Option<AiCommands>,
    /// Deck to generate every missing asset for (artworks, images, icons, point clouds)
    pub file: Option<PathBuf>,
    #[command(flatten)]
    pub select: Select,
}

/// Which of a deck's assets to (re)generate. By default only the missing ones.
#[derive(Args, Clone, Debug, Default, PartialEq)]
pub struct Select {
    /// Only this slide (1-based); regenerates its assets even when they are current
    #[arg(long)]
    pub slide: Option<usize>,
    /// Only regenerate assets that have gone stale
    #[arg(long)]
    pub stale: bool,
    /// Regenerate assets that are current too (pinned ones are always kept)
    #[arg(long)]
    pub force: bool,
    /// List what would be generated, without generating anything
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Subcommand)]
pub enum AiCommands {
    /// Images for the deck's `![prompt](generate:)` placeholders (or one image from --prompt)
    Images(ImagesArgs),
    /// Diagram icons for `(icon: generate:, prompt: "...")` (or one icon from --prompt)
    Icons(ImagesArgs),
    /// A picture for every slide on an art engine, like blueprint or watercolour
    Pictures {
        /// The deck
        file: PathBuf,
        #[command(flatten)]
        select: Select,
        /// Draw for this engine instead of the deck's (e.g. blueprint)
        #[arg(long)]
        engine: Option<String>,
        /// Use this image node instead of the default (see `ailloy ai config list-nodes`)
        #[arg(long)]
        node: Option<String>,
    },
    /// Point clouds: the deck's missing `picture` names, or one from --name/--description
    PointCloud(PointCloudArgs),
    /// A theme from a design system folder (written to ./themes)
    Theme {
        /// Theme name (lowercase letters, digits, '-' and '_')
        name: String,
        /// Design system folder to convert (SKILL.md, readme.md, CSS tokens, *.tokens.json); its fonts and logos are copied into the theme
        #[arg(long)]
        from: PathBuf,
        /// Write to the user theme folder instead of ./themes
        #[arg(long)]
        user: bool,
        /// Overwrite an existing theme of the same name
        #[arg(long)]
        force: bool,
    },
    /// A whole deck from content (a file, text or stdin)
    Deck(DeckArgs),
    /// The AI agent skill for writing mdeck decks
    Skill {
        /// Output the skill markdown content (ready to save as a skill file)
        #[arg(long)]
        emit: bool,

        /// Output detailed reference documentation for AI agents
        #[arg(long)]
        reference: bool,
    },
    /// Show AI status (same as running `mdeck ai` without arguments)
    Status,
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
    /// Manage named styles (a prompt plus optional reference images)
    Style {
        #[command(subcommand)]
        command: StyleCommands,
    },
}

#[derive(Args)]
pub struct ImagesArgs {
    /// The deck (omit with --prompt for a one-off picture)
    #[arg(required_unless_present = "prompt")]
    pub file: Option<PathBuf>,
    #[command(flatten)]
    pub select: Select,
    /// Named style or literal description, instead of the deck's
    #[arg(long)]
    pub style: Option<String>,
    /// One-off: generate one picture from this prompt instead of a deck's placeholders
    #[arg(long, conflicts_with = "file")]
    pub prompt: Option<String>,
    /// One-off: where to write the picture
    #[arg(long, requires = "prompt")]
    pub output: Option<PathBuf>,
}

#[derive(Args)]
pub struct PointCloudArgs {
    /// The deck: generate every `picture` name it uses that resolves nowhere
    #[arg(required_unless_present = "name")]
    pub file: Option<PathBuf>,
    #[command(flatten)]
    pub select: Select,
    /// One-off: the name to save a library point cloud under
    #[arg(long, conflicts_with = "file", requires = "description")]
    pub name: Option<String>,
    /// What to draw, e.g. "A server, in a rack, in a datacenter" (for a deck: the
    /// description for every missing name; the name itself otherwise)
    #[arg(long)]
    pub description: Option<String>,
    /// One-off: save to the user library (~/.config/mdeck/illustrations) instead of ./illustrations
    #[arg(long, requires = "name")]
    pub user: bool,
}

#[derive(Args)]
pub struct DeckArgs {
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
        /// Reference image sent with every generation in this style (repeatable)
        #[arg(long = "reference")]
        references: Vec<PathBuf>,
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
        /// Reference image sent with every generation in this style (repeatable)
        #[arg(long = "reference")]
        references: Vec<PathBuf>,
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

#[cfg(test)]
mod tests {
    use crate::cli::{Cli, Commands};
    use clap::Parser;

    use super::*;

    fn ai(args: &[&str]) -> AiArgs {
        let mut v = vec!["mdeck", "ai"];
        v.extend_from_slice(args);
        match Cli::try_parse_from(v).unwrap().command {
            Some(Commands::Ai(a)) => a,
            _ => panic!("not an ai command"),
        }
    }

    #[test]
    fn bare_ai_with_a_deck_generates_everything_missing() {
        let a = ai(&["talk.md", "--dry-run", "--slide", "3"]);
        assert!(a.command.is_none());
        assert_eq!(a.file.as_deref(), Some(std::path::Path::new("talk.md")));
        assert_eq!(
            a.select,
            Select {
                slide: Some(3),
                dry_run: true,
                ..Select::default()
            }
        );
        let a = ai(&[]);
        assert!(a.command.is_none() && a.file.is_none(), "status");
    }

    #[test]
    fn each_kind_has_its_subcommand() {
        let a = ai(&["images", "talk.md", "--stale", "--style", "photo"]);
        let Some(AiCommands::Images(i)) = a.command else {
            panic!()
        };
        assert!(i.select.stale && i.style.as_deref() == Some("photo"));
        let a = ai(&["icons", "--prompt", "a database", "--output", "db.png"]);
        let Some(AiCommands::Icons(i)) = a.command else {
            panic!()
        };
        assert!(i.file.is_none() && i.prompt.is_some());
        let a = ai(&["pictures", "talk.md", "--force", "--engine", "sketch"]);
        let Some(AiCommands::Pictures { select, engine, .. }) = a.command else {
            panic!()
        };
        assert!(select.force && engine.as_deref() == Some("sketch"));
        let a = ai(&[
            "point-cloud",
            "--name",
            "kettle",
            "--description",
            "a kettle",
        ]);
        assert!(
            matches!(a.command, Some(AiCommands::PointCloud(p)) if p.name.as_deref() == Some("kettle"))
        );
        let a = ai(&["point-cloud", "talk.md", "--dry-run"]);
        assert!(matches!(a.command, Some(AiCommands::PointCloud(p)) if p.select.dry_run));
        let a = ai(&["theme", "acme", "--from", "ds/"]);
        assert!(matches!(a.command, Some(AiCommands::Theme { .. })));
        let a = ai(&["deck", "--input", "notes.txt"]);
        assert!(matches!(a.command, Some(AiCommands::Deck(_))));
        assert!(matches!(
            ai(&["skill", "--emit"]).command,
            Some(AiCommands::Skill { emit: true, .. })
        ));
    }

    #[test]
    fn deck_and_one_off_forms_do_not_mix() {
        let bad = |args: &[&str]| {
            let mut v = vec!["mdeck", "ai"];
            v.extend_from_slice(args);
            Cli::try_parse_from(v).is_err()
        };
        assert!(bad(&["images"]), "a deck or --prompt is needed");
        assert!(bad(&["images", "talk.md", "--prompt", "x"]));
        assert!(bad(&["point-cloud"]));
        assert!(
            bad(&["point-cloud", "--name", "kettle"]),
            "--name needs --description"
        );
        assert!(bad(&["theme", "acme"]), "--from is required");
        assert!(bad(&["generate", "talk.md"]), "the v1 command is gone");
        assert!(bad(&["art", "talk.md"]), "the v1 command is gone");
    }
}
