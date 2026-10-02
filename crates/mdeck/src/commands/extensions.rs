//! `mdeck extensions list` (EXT-04): everything that extends this mdeck and
//! where it came from.

use std::path::Path;

use anyhow::Result;
use colored::Colorize;

use crate::extensions::{self, Kind, external, packs};

pub fn list() -> Result<()> {
    println!("{}", "Packs".bold());
    let installed = packs::installed(Some(Path::new(".")));
    if installed.is_empty() {
        println!("  none (`mdeck pack install <path|git-url>`)");
    } else {
        crate::commands::pack::print_packs(&installed);
    }

    let provided = extensions::provided();
    for kind in [Kind::Engine, Kind::Visual, Kind::Transition, Kind::Theme] {
        println!();
        println!("{}", kind.plural().bold());
        let items: Vec<_> = provided.iter().filter(|p| p.kind == kind).collect();
        let width = items.iter().map(|p| p.name.len()).max().unwrap_or(4);
        for p in items {
            println!("  {:width$}  {}", p.name, p.origin.dimmed());
        }
    }

    let config = crate::config::Config::load_or_default();
    let programs = external::all(&config);
    println!();
    println!("{}", "External visual programs".bold());
    if programs.is_empty() {
        println!("  none (`visuals:` in the config)");
    } else {
        let width = programs.iter().map(|v| v.tag.len()).max().unwrap_or(4);
        for v in programs {
            println!("  {:width$}  {}", v.tag, v.command.dimmed());
        }
    }
    Ok(())
}
