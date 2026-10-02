//! `mdeck theme`: list, check, scaffold (optionally from a design system
//! with AI) and preview custom themes.

mod from;
mod preview;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use colored::Colorize;

use crate::cli::ThemeCommands;
use crate::theme::lookup::{self, Lookup, Origin};
use crate::theme::validate;

const SPEC: &str = include_str!("../../../doc/mdeck-spec.md");

/// Section 9.4 of the spec: the theme format, the design-system mapping and
/// the conversion recipe. It is what the model reads for `--from`, so the
/// spec stays the one description of the format.
pub fn theme_section() -> &'static str {
    let start = SPEC.find("### 9.4 Custom themes").unwrap_or(0);
    let end = SPEC[start..]
        .find("\n## 10.")
        .map(|i| start + i)
        .unwrap_or(SPEC.len());
    &SPEC[start..end]
}

/// Themes are looked up from the working directory, as for a deck there.
fn here() -> Lookup {
    Lookup::for_deck(Some(Path::new(".")))
}

pub async fn run(cmd: ThemeCommands, quiet: bool) -> Result<()> {
    match cmd {
        ThemeCommands::List => list(),
        ThemeCommands::Check { name } => check(&name),
        ThemeCommands::New {
            name,
            from,
            user,
            force,
        } => new(&name, from.as_deref(), user, force, quiet).await,
        ThemeCommands::Preview {
            name,
            output_dir,
            width,
            height,
        } => preview::preview(&name, output_dir, width, height),
    }
}

fn list() -> Result<()> {
    let l = here();
    for found in l.available() {
        let (label, path) = match &found.origin {
            Origin::Builtin => ("built-in", String::new()),
            o => (
                o.label(),
                o.path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
            ),
        };
        let desc = match l.load_found(&found) {
            Ok(b) => format!("{} engine", b.theme.engine.name()),
            Err(_) => "invalid (run `mdeck theme check`)".red().to_string(),
        };
        println!(
            "  {:<16} {:<9} {:<14} {}",
            found.name,
            label,
            desc,
            path.dimmed()
        );
    }
    Ok(())
}

/// Print a theme's problems; returns whether it loaded.
fn report(name: &str, l: &Lookup) -> bool {
    match l.load(name) {
        Err(e) => {
            println!("{} {e}", "error:".red().bold());
            false
        }
        Ok(built) => {
            let t = &built.theme;
            let from = t
                .source
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "built-in".into());
            println!(
                "{} ({from}): {} engine, countdown {}",
                t.name.bold(),
                t.engine.name(),
                t.countdown.name()
            );
            let advice = validate::review(t);
            for w in &built.warnings {
                println!("  {} {w}", "warning:".yellow().bold());
            }
            for a in &advice {
                println!("  {} {a}", "contrast:".yellow().bold());
            }
            if built.warnings.is_empty() && advice.is_empty() {
                println!("  {}", "No issues found.".green());
            }
            true
        }
    }
}

fn check(name: &str) -> Result<()> {
    if !report(name, &here()) {
        std::process::exit(1);
    }
    Ok(())
}

/// Where `new` writes: `./themes` or the user folder.
fn target_dir(user: bool) -> Result<PathBuf> {
    if user {
        lookup::user_dir().context("no user configuration folder on this system")
    } else {
        Ok(PathBuf::from("themes"))
    }
}

/// The starter theme `mdeck theme new` writes: every key, commented, with
/// the values `dark` gives.
fn starter(name: &str) -> String {
    format!(
        r##"# {name}: a custom MDeck theme. Every key is optional; unset keys come
# from the theme named by `extends`. See `mdeck spec`, section 9.4.
# Check it with `mdeck theme check {name}` and look at it with
# `mdeck theme preview {name} --output-dir /tmp/{name}`.
name: {name}
extends: dark              # dark | light | nord | ember | another theme
# engine: plain            # plain | particles | led | splitflap | blocks | thermal | ...
# countdown: none          # none | plain | burst (burst: the engine's own countdown)
colors:
  background: "#1e1e1e"    # slide background
  text: "#c8c8c8"          # body text
  heading: "#ffffff"       # headings
  accent: "#5294e2"        # links, quote bars, highlights
  # muted: "#8a8a8a"       # captions, eyebrows, slide numbers
  # strong: "#ffffff"      # **bold** text
  # rule: "#3a3a3a"        # hairlines
  # accent-soft: "#8fb8ee" # lighter accent
  # secondary: "#e8a838"   # a second, rarer highlight
  # code-background: "#2d2d2d"
  # code-text: "#d4d4d4"
  # positive: "#5cdb95"
  # negative: "#ff6b6b"
  # series: ["#5cb8ff", "#ff7e67", "#5cdb95", "#e8a838"]
# fonts:                   # a bundled face or a .ttf/.otf file in this folder
#   display: sans          # sans, mono, spectral-light, hanken-light,
#   body: sans             # hanken-regular, hanken-medium, jetbrains-mono
#   strong: sans
#   mono: mono
# sizes: {{ h1: 96, h2: 72, h3: 52, body: 44, code: 30 }}   # px at 1920x1080
# text: {{ line-height: 1.4 }}
# charts: {{ fill-opacity: 0.85 }}
# code: {{ syntax: base16-ocean.dark }}
# logo:                    # a PNG or SVG in this folder, in a corner of every slide
#   file: logo.svg
#   position: top-right    # top-left | top-right | bottom-left | bottom-right
#   height: 56             # px at 1920x1080
#   opacity: 0.6
"##
    )
}

async fn new(name: &str, from: Option<&Path>, user: bool, force: bool, quiet: bool) -> Result<()> {
    if !lookup::valid_name(name) {
        bail!("theme names are lowercase letters, digits, '-' and '_' (got '{name}')");
    }
    let dir = target_dir(user)?;
    let single = dir.join(format!("{name}.yaml"));
    let folder = dir.join(name);
    if !force && (single.exists() || folder.join("theme.yaml").exists()) {
        bail!(
            "theme '{name}' already exists in {} (use --force to overwrite)",
            dir.display()
        );
    }
    let Some(from) = from else {
        std::fs::create_dir_all(&dir)?;
        std::fs::write(&single, starter(name))?;
        if !quiet {
            println!("Wrote {}", single.display());
            println!("Use it with `@theme: {name}` in a deck's frontmatter.");
        }
        return Ok(());
    };
    let target = from::Target {
        name,
        dir: &dir,
        single: &single,
        folder: &folder,
    };
    from::new_from(&target, from, quiet).await
}

/// A lookup that finds themes written to `dir` (a deck's `themes/` folder or
/// the user folder) before anything else.
fn written_in(dir: &Path) -> Lookup {
    Lookup {
        deck: None,
        user: Some(dir.to_path_buf()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    use crate::theme::file::ThemeFile;

    #[test]
    fn the_theme_section_is_the_whole_of_9_4() {
        let s = theme_section();
        assert!(s.starts_with("### 9.4 Custom themes"));
        assert!(s.contains("From a design system to a theme"));
        assert!(s.contains("mdeck theme preview"));
        assert!(!s.contains("## 10."));
    }

    #[test]
    fn the_starter_theme_loads() {
        let f = ThemeFile::parse(&starter("acme"))
            .unwrap()
            .over(&lookup::builtin_file("dark").unwrap());
        let t = Theme::build("acme", &f).unwrap();
        assert_eq!(t.theme.name, "acme");
        assert!(t.warnings.is_empty());
    }
}
