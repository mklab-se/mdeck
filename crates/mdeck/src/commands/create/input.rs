//! Where `mdeck ai deck` gets its source content: an argument, piped stdin or a
//! question asked in interactive mode.

use std::io::{self, IsTerminal, Read, Write};
use std::path::Path;

use anyhow::{Context, Result};
use colored::Colorize;

use crate::cli::DeckArgs;

use super::extractors::extract_from_file;

/// The label an input gets when it is typed text rather than a file or stdin.
pub(super) const TEXT_INPUT: &str = "(text input)";

/// Print the help text of `mdeck ai deck`.
pub(super) fn print_create_help() -> Result<()> {
    use clap::CommandFactory;
    let mut cmd = crate::cli::Cli::command();
    for sub in cmd.get_subcommands_mut() {
        if sub.get_name() == "ai" {
            for sub2 in sub.get_subcommands_mut() {
                if sub2.get_name() == "deck" {
                    sub2.clone().name("mdeck ai deck").print_help()?;
                    println!();
                    return Ok(());
                }
            }
        }
    }
    anyhow::bail!("No input provided. Run `mdeck ai deck --help` for usage.");
}

/// Resolve the input source and extract text content.
/// Returns `Some((source_label, extracted_text))`, or `None` when no input
/// was given at all (the caller shows help in that case).
pub(super) fn resolve_input(args: &DeckArgs, quiet: bool) -> Result<Option<(String, String)>> {
    let stdin = io::stdin();
    let piped: Option<Box<dyn Read>> = if stdin.is_terminal() {
        None
    } else {
        Some(Box::new(stdin.lock()))
    };
    resolve_input_with(args, quiet, piped)
}

/// Like [`resolve_input`], but with the piped stdin (if any) passed in so the
/// logic can be tested without touching the process's real stdin.
fn resolve_input_with(
    args: &DeckArgs,
    quiet: bool,
    piped_stdin: Option<Box<dyn Read>>,
) -> Result<Option<(String, String)>> {
    if let Some(ref input) = args.input {
        return file_or_text(input).map(Some);
    }

    // Try stdin if it's piped
    if let Some(mut stdin) = piped_stdin {
        let mut content = String::new();
        stdin
            .read_to_string(&mut content)
            .context("Failed to read from stdin")?;
        if content.trim().is_empty() {
            anyhow::bail!("No content received from stdin.");
        }
        return Ok(Some(("(stdin)".to_string(), content)));
    }

    if args.interactive {
        let input = ask_for_input(quiet)?;
        return file_or_text(&input).map(Some);
    }

    // No input provided at all
    Ok(None)
}

/// A path to an existing file is read (and extracted); anything else is the
/// content itself.
fn file_or_text(input: &str) -> Result<(String, String)> {
    let path = Path::new(input);
    if path.exists() && path.is_file() {
        let label = format!("{}", path.display());
        let content = extract_from_file(path)?;
        return Ok((label, content));
    }
    Ok((TEXT_INPUT.to_string(), input.to_string()))
}

/// Interactive mode: ask what the presentation should be about.
fn ask_for_input(quiet: bool) -> Result<String> {
    if !quiet {
        eprintln!(
            "{} What should the presentation be about?",
            "?".green().bold()
        );
        eprintln!("  Enter a file path, or describe the topic in your own words.");
        eprintln!();
    }
    eprint!("{} ", ">".bold());
    io::stderr().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_string();
    if input.is_empty() {
        anyhow::bail!("No input provided.");
    }
    Ok(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_resolve_input_literal_text() {
        let args = DeckArgs {
            input: Some("A presentation about Rust programming".to_string()),
            output: Some(PathBuf::from("out.md")),
            prompt: None,
            interactive: false,
            style: None,
        };
        let (label, content) = resolve_input_with(&args, true, None).unwrap().unwrap();
        assert_eq!(label, "(text input)");
        assert_eq!(content, "A presentation about Rust programming");
    }

    #[test]
    fn test_resolve_input_existing_file() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let cargo_toml = format!("{manifest_dir}/Cargo.toml");
        let args = DeckArgs {
            input: Some(cargo_toml),
            output: Some(PathBuf::from("out.md")),
            prompt: None,
            interactive: false,
            style: None,
        };
        let (label, content) = resolve_input_with(&args, true, None).unwrap().unwrap();
        assert!(label.contains("Cargo.toml"));
        assert!(content.contains("mdeck"));
    }

    #[test]
    fn test_resolve_input_reads_piped_stdin() {
        let args = DeckArgs {
            input: None,
            output: Some(PathBuf::from("out.md")),
            prompt: None,
            interactive: false,
            style: None,
        };
        let piped: Box<dyn Read> = Box::new(std::io::Cursor::new("piped notes"));
        let (label, content) = resolve_input_with(&args, true, Some(piped))
            .unwrap()
            .unwrap();
        assert_eq!(label, "(stdin)");
        assert_eq!(content, "piped notes");

        let empty: Box<dyn Read> = Box::new(std::io::Cursor::new("  \n"));
        assert!(resolve_input_with(&args, true, Some(empty)).is_err());
    }

    #[test]
    fn test_resolve_input_no_input_no_stdin() {
        let args = DeckArgs {
            input: None,
            output: Some(PathBuf::from("out.md")),
            prompt: None,
            interactive: false,
            style: None,
        };
        let result = resolve_input_with(&args, true, None).unwrap();
        assert!(
            result.is_none(),
            "no input and no stdin should ask for help"
        );
    }

    #[test]
    fn test_resolve_input_interactive_with_input_provided() {
        let args = DeckArgs {
            input: Some("A talk about functional programming".to_string()),
            output: Some(PathBuf::from("out.md")),
            prompt: None,
            interactive: true,
            style: None,
        };
        let (label, content) = resolve_input_with(&args, true, None).unwrap().unwrap();
        assert_eq!(label, "(text input)");
        assert_eq!(content, "A talk about functional programming");
    }
}
