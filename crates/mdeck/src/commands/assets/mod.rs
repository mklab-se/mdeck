//! Generating a deck's assets into `talk.assets/` (see [`crate::assets`]):
//! `mdeck ai images`, `mdeck ai icons`, `mdeck ai point-cloud <deck>`, and
//! bare `mdeck ai <deck>`, which runs every kind the deck needs (artworks
//! through `commands::art`). Each takes the same [`Select`]: by default what
//! is missing or stale, `--stale` only stale ones, `--force` current ones
//! too, `--slide N` that slide's (even when current), `--dry-run` lists and
//! stops. Pinned assets are never regenerated.

pub mod images;
mod names;
pub mod point_clouds;

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use crate::assets::manifest::State;
use crate::cli::Select;

/// Whether a target on slide `slide` (0-based) whose asset is in `state`
/// (`None`: there is none) with its file present or not is (re)generated.
pub fn wanted(select: &Select, slide: usize, state: Option<State>, file_exists: bool) -> bool {
    if select.slide.is_some_and(|n| n != slide + 1) {
        return false;
    }
    if state == Some(State::Pinned) {
        return false;
    }
    let missing = state.is_none() || !file_exists;
    if select.stale {
        return !missing && state == Some(State::Stale);
    }
    select.force || select.slide.is_some() || missing || state == Some(State::Stale)
}

/// Fail on a `--slide` outside the deck.
pub fn check_slide(select: &Select, count: usize) -> Result<()> {
    match select.slide {
        Some(n) if n == 0 || n > count => bail!("slide {n} is outside 1-{count}"),
        _ => Ok(()),
    }
}

/// Read and parse a deck.
pub fn read_deck(file: &Path) -> Result<crate::parser::Presentation> {
    let content =
        std::fs::read_to_string(file).map_err(|e| anyhow::anyhow!("{}: {e}", file.display()))?;
    let pres = crate::parser::parse(&content);
    if pres.slides.is_empty() {
        bail!("No slides found in {}", file.display());
    }
    Ok(pres)
}

/// "" for one, "s" for more.
pub fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

/// Bare `mdeck ai <deck>`: everything the deck is missing (or, with the
/// selection flags, what they ask for): artworks when it runs on an art
/// engine, then images, icons and point clouds.
pub async fn run_all(file: PathBuf, select: Select, quiet: bool) -> Result<()> {
    let pres = read_deck(&file)?;
    check_slide(&select, pres.slides.len())?;
    let base = file.parent().unwrap_or(Path::new(".")).to_path_buf();
    let mut failed = Vec::new();
    let theme = crate::commands::art::deck_theme(&pres, &base, None)?;
    if theme.engine.medium().is_some() {
        if !quiet {
            eprintln!("Artworks:");
        }
        let opts = crate::commands::art::Options {
            select: select.clone(),
            engine: None,
            node: None,
            quiet,
        };
        if let Err(e) = crate::commands::art::run(file.clone(), opts).await {
            eprintln!("artworks: {e:#}");
            failed.push("artworks");
        }
    }
    for kind in [images::Which::Images, images::Which::Icons] {
        if let Err(e) = images::run(&file, kind, &select, None, quiet).await {
            eprintln!("{}: {e:#}", kind.name());
            failed.push(kind.name());
        }
    }
    if let Err(e) = point_clouds::run_deck(&file, &select, None, quiet).await {
        eprintln!("point clouds: {e:#}");
        failed.push("point clouds");
    }
    if !failed.is_empty() {
        bail!("some assets were not generated ({})", failed.join(", "));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sel(slide: Option<usize>, stale: bool, force: bool) -> Select {
        Select {
            slide,
            stale,
            force,
            dry_run: false,
        }
    }

    #[test]
    fn by_default_missing_and_stale_assets_are_made() {
        let s = sel(None, false, false);
        assert!(wanted(&s, 0, None, false), "missing");
        assert!(wanted(&s, 0, Some(State::Current), false), "file gone");
        assert!(wanted(&s, 0, Some(State::Stale), true));
        assert!(!wanted(&s, 0, Some(State::Current), true));
        assert!(!wanted(&s, 0, Some(State::Pinned), false), "pins are kept");
    }

    #[test]
    fn stale_force_and_slide_narrow_or_widen_the_choice() {
        let stale = sel(None, true, false);
        assert!(wanted(&stale, 0, Some(State::Stale), true));
        assert!(!wanted(&stale, 0, None, false));
        assert!(!wanted(&stale, 0, Some(State::Current), true));
        let force = sel(None, false, true);
        assert!(wanted(&force, 0, Some(State::Current), true));
        assert!(!wanted(&force, 0, Some(State::Pinned), true));
        let slide = sel(Some(2), false, false);
        assert!(wanted(&slide, 1, Some(State::Current), true));
        assert!(!wanted(&slide, 0, None, false));
        assert!(check_slide(&slide, 1).is_err());
        assert!(check_slide(&slide, 2).is_ok());
        assert!(check_slide(&sel(Some(0), false, false), 2).is_err());
    }
}
