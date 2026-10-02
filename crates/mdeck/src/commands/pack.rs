//! `mdeck pack install|list|remove` (EXT-09, EXT-10).

use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use colored::Colorize;

use crate::extensions::packs::{self, Origin, Scope};

fn root(deck: bool) -> Result<(PathBuf, Scope)> {
    if deck {
        Ok((packs::deck_root(Path::new(".")), Scope::Deck))
    } else {
        packs::user_root()
            .map(|r| (r, Scope::User))
            .ok_or_else(|| anyhow!("could not find the user config folder"))
    }
}

pub fn install(source: &str, deck: bool, quiet: bool) -> Result<()> {
    let (root, scope) = root(deck)?;
    let pack = packs::install(&Origin::parse(source), &root, scope)?;
    if !quiet {
        println!(
            "Installed {} {} into {}",
            pack.manifest.name.bold(),
            pack.manifest.version,
            pack.dir.display()
        );
        let contents = describe(&pack);
        if !contents.is_empty() {
            println!("  {contents}");
        }
    }
    Ok(())
}

pub fn remove(name: &str, deck: bool, quiet: bool) -> Result<()> {
    let (root, _) = root(deck)?;
    let dir = packs::remove(&root, name)?;
    if !quiet {
        println!("Removed {} ({})", name.bold(), dir.display());
    }
    Ok(())
}

pub fn list() -> Result<()> {
    let installed = packs::installed(Some(Path::new(".")));
    if installed.is_empty() {
        println!("No packs installed. `mdeck pack install <path|git-url>` adds one.");
        return Ok(());
    }
    print_packs(&installed);
    Ok(())
}

/// One line per pack, then what it carries.
pub fn print_packs(installed: &[packs::Installed]) {
    let width = installed
        .iter()
        .map(|p| p.manifest.name.len())
        .max()
        .unwrap_or(4);
    for p in installed {
        println!(
            "  {:width$}  {:8}  {:4}  {}",
            p.manifest.name.bold(),
            p.manifest.version,
            p.scope.label(),
            p.manifest.description
        );
        let contents = describe(p);
        println!(
            "  {:width$}  {}",
            "",
            if contents.is_empty() {
                p.dir.display().to_string()
            } else {
                format!("{contents}  {}", p.dir.display())
            }
            .dimmed()
        );
    }
}

/// `themes 2, point-clouds 5`.
fn describe(p: &packs::Installed) -> String {
    p.contents()
        .iter()
        .map(|(f, n)| format!("{} {n}", f.dir()))
        .collect::<Vec<_>>()
        .join(", ")
}
