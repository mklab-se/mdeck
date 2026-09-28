//! `mdeck ai story`: write story scripts for the particles engine with AI.
//!
//! For each target slide the model receives the story vocabulary, the deck's
//! outline and cast so far, the slide's copy and notes, and the author's
//! ```@story hint when there is one. It answers with JSON (every JSON
//! document is valid YAML, and models are more reliable at strict JSON than at
//! indentation), which is validated and written to the YAML sidecar.

mod prompt;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use colored::Colorize;

pub use prompt::generate_script;

use crate::parser::{self, Presentation};
use crate::render::illustration::Library;
use crate::render::story::sidecar::{self, Entry, Sidecar};
use crate::render::story::{self, Script};

/// Parse `--range 3-7` (1-based, inclusive).
fn parse_range(range: &str, count: usize) -> Result<Vec<usize>> {
    let (a, b) = range.split_once('-').context("range must look like 3-7")?;
    let a: usize = a.trim().parse().context("range start")?;
    let b: usize = b.trim().parse().context("range end")?;
    if a == 0 || b < a || b > count {
        bail!("range {range} is outside 1-{count}");
    }
    Ok((a - 1..b).collect())
}

/// Generate a script for one slide from a running app: loads the deck and
/// sidecar afresh, writes the entry, and returns the script. Blocks; call it
/// from a worker thread.
pub fn generate_one_blocking(deck: &Path, index: usize) -> Result<Script> {
    crate::commands::util::block_on(async {
        let content = std::fs::read_to_string(deck)?;
        let base = deck.parent().unwrap_or(Path::new("."));
        let pres = parser::parse(&content);
        if index >= pres.slides.len() {
            bail!("slide {} does not exist", index + 1);
        }
        let client = ailloy::Client::for_capability("chat")?;
        let mut sc = load_or_empty(deck)?;
        if sc.slides.iter().any(|e| e.pinned && e.slide == index + 1) {
            bail!("slide {} has a pinned (hand-written) story", index + 1);
        }
        let cast = known_cast(&sc);
        let mut lib = Library::for_deck(Some(base));
        let script = generate_script(&client, &pres, index, &cast, &mut lib).await?;
        upsert(&mut sc, &pres, index, script.clone());
        sidecar::save(deck, &sc)?;
        Ok(script)
    })?
}

fn known_cast(sc: &Sidecar) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for e in &sc.slides {
        for m in &e.scene.cast {
            if m.kind == "person"
                && let Some(l) = &m.label
                && !names.contains(l)
            {
                names.push(l.clone());
            }
        }
    }
    names
}

fn upsert(sc: &mut Sidecar, pres: &Presentation, index: usize, script: Script) {
    let slide = &pres.slides[index];
    let entry = Entry {
        slide: index + 1,
        title: slide.title(),
        hash: sidecar::slide_hash(slide, pres.meta.story.as_deref()),
        generated: Some(timestamp()),
        pinned: false,
        scene: script,
    };
    sc.slides.retain(|e| e.slide != index + 1);
    sc.slides.push(entry);
    sc.slides.sort_by_key(|e| e.slide);
}

/// UTC timestamp as `YYYY-MM-DD HH:MM`, without a date crate.
pub(crate) fn timestamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    // civil-from-days (Howard Hinnant)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}",
        rem / 3600,
        (rem % 3600) / 60
    )
}

/// The `mdeck ai story` command.
pub async fn run(
    file: PathBuf,
    slide: Option<usize>,
    range: Option<String>,
    stale_only: bool,
    force: bool,
    dry_run: bool,
    quiet: bool,
) -> Result<()> {
    let content = std::fs::read_to_string(&file)?;
    let base = file.parent().unwrap_or(Path::new("."));
    let pres = parser::parse(&content);
    if pres.slides.is_empty() {
        bail!("No slides found in {}", file.display());
    }

    let (path, both) = sidecar::resolve_path(&file);
    if both && !quiet {
        eprintln!(
            "{} both .yaml and .yml sidecars exist; using {}",
            "Warning:".yellow().bold(),
            path.display()
        );
    }
    let mut sc = load_or_empty(&file)?;

    let mut targets = requested_slides(slide, range.as_deref(), pres.slides.len())?;
    keep_stageable(&mut targets, &pres, &sc, quiet);
    prune_unstageable(&file, &mut sc, &pres, dry_run, quiet)?;

    // Unless forced, skip slides whose sidecar entry is still current.
    if !force {
        targets.retain(|&i| {
            let hash = sidecar::slide_hash(&pres.slides[i], pres.meta.story.as_deref());
            !sc.slides.iter().any(|e| e.hash == hash)
        });
    }
    if stale_only {
        targets.retain(|&i| sc.slides.iter().any(|e| e.slide == i + 1));
    }

    if targets.is_empty() {
        if !quiet {
            eprintln!("Nothing to do: every targeted slide already has a current story.");
        }
        return Ok(());
    }

    let client = ailloy::Client::for_capability("chat")?;
    if !quiet {
        eprintln!(
            "Writing stories for {} slide{} of {} → {}",
            targets.len(),
            if targets.len() == 1 { "" } else { "s" },
            file.display(),
            path.display()
        );
    }

    let mut lib = Library::for_deck(Some(base));
    let writer = Writer {
        client: &client,
        pres: &pres,
        file: &file,
        dry_run,
        quiet,
    };
    let failures = writer.write_all(&targets, &mut sc, &mut lib).await?;

    if failures > 0 {
        bail!(
            "{failures} slide(s) failed; the others were saved to {}",
            path.display()
        );
    }
    if !quiet {
        if dry_run {
            eprintln!("Dry run: nothing was written.");
        } else {
            eprintln!("Done. Present with `mdeck {}`.", file.display());
        }
    }
    Ok(())
}

/// The deck's story sidecar, or an empty one.
fn load_or_empty(deck: &Path) -> Result<Sidecar> {
    Ok(sidecar::load(deck)?.unwrap_or(Sidecar {
        version: sidecar::VERSION,
        slides: vec![],
    }))
}

/// 0-based indices of the slides `--slide` or `--range` name, or all of them.
fn requested_slides(slide: Option<usize>, range: Option<&str>, count: usize) -> Result<Vec<usize>> {
    if let Some(n) = slide {
        if n == 0 || n > count {
            bail!("slide {n} is outside 1-{count}");
        }
        Ok(vec![n - 1])
    } else if let Some(r) = range {
        parse_range(r, count)
    } else {
        Ok((0..count).collect())
    }
}

/// Only slides with a stage get stories; pinned (hand-written) entries are
/// left alone.
fn keep_stageable(targets: &mut Vec<usize>, pres: &Presentation, sc: &Sidecar, quiet: bool) {
    let requested = targets.len();
    targets.retain(|&i| story::allowed(&pres.slides[i], i));
    let no_stage = requested - targets.len();
    if no_stage > 0 && !quiet {
        eprintln!(
            "Skipping {no_stage} slide{} without a stage (code, charts, diagrams, tables, images, titles).",
            if no_stage == 1 { "" } else { "s" }
        );
    }
    targets.retain(|&i| !sc.slides.iter().any(|e| e.pinned && e.slide == i + 1));
}

/// Entries for slides that can no longer play a story are dropped (and the
/// sidecar saved, unless this is a dry run).
fn prune_unstageable(
    file: &Path,
    sc: &mut Sidecar,
    pres: &Presentation,
    dry_run: bool,
    quiet: bool,
) -> Result<()> {
    let before = sc.slides.len();
    sc.slides.retain(|e| {
        pres.slides
            .get(e.slide.wrapping_sub(1))
            .is_some_and(|s| story::allowed(s, e.slide - 1))
    });
    let pruned = before - sc.slides.len();
    if pruned > 0 && !dry_run {
        sidecar::save(file, sc)?;
        if !quiet {
            eprintln!(
                "Removed {pruned} stored stor{} for slides without a stage.",
                if pruned == 1 { "y" } else { "ies" }
            );
        }
    }
    Ok(())
}

/// Writes the stories for a run of slides, one at a time.
struct Writer<'a> {
    client: &'a ailloy::Client,
    pres: &'a Presentation,
    file: &'a Path,
    dry_run: bool,
    quiet: bool,
}

impl Writer<'_> {
    /// Generate each target's script and store it. Returns how many failed.
    async fn write_all(
        &self,
        targets: &[usize],
        sc: &mut Sidecar,
        lib: &mut Library,
    ) -> Result<usize> {
        let mut failures = 0usize;
        for &i in targets {
            let title = self.pres.slides[i].title().unwrap_or_default();
            if !self.quiet {
                eprint!("  slide {:>2}  {:<40} ", i + 1, truncate(&title, 40));
            }
            let cast = known_cast(sc);
            match generate_script(self.client, self.pres, i, &cast, lib).await {
                Ok(script) => {
                    if !self.quiet {
                        print_script(&script, self.dry_run);
                    }
                    if self.dry_run {
                        continue;
                    }
                    upsert(sc, self.pres, i, script);
                    // Save after every slide so an interruption keeps the work so far.
                    sidecar::save(self.file, sc)?;
                }
                Err(e) => {
                    failures += 1;
                    if !self.quiet {
                        eprintln!("{} {e}", "✗".red());
                    }
                }
            }
        }
        Ok(failures)
    }
}

/// The finished line for a slide, its spoken beats and, in a dry run, the YAML.
fn print_script(script: &Script, dry_run: bool) {
    eprintln!(
        "{} {} cast, {} beats",
        "✓".green(),
        script.cast.len(),
        script.beats.len()
    );
    for (b, beat) in script.beats.iter().enumerate() {
        if let Some(say) = &beat.say {
            eprintln!("            {}  {say}", format!("{}.", b + 1).dimmed());
        }
    }
    if dry_run {
        eprintln!(
            "{}",
            serde_norway::to_string(script).unwrap_or_default().dimmed()
        );
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let cut: String = s.chars().take(n.saturating_sub(1)).collect();
        format!("{cut}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_are_one_based_and_inclusive() {
        assert_eq!(parse_range("3-5", 10).unwrap(), vec![2, 3, 4]);
        assert!(parse_range("0-2", 10).is_err());
        assert!(parse_range("5-3", 10).is_err());
        assert!(parse_range("9-12", 10).is_err());
    }

    #[test]
    fn requested_slides_prefers_slide_then_range_then_all() {
        assert_eq!(requested_slides(Some(2), Some("1-3"), 4).unwrap(), vec![1]);
        assert!(requested_slides(Some(5), None, 4).is_err());
        assert_eq!(requested_slides(None, Some("2-3"), 4).unwrap(), vec![1, 2]);
        assert_eq!(requested_slides(None, None, 3).unwrap(), vec![0, 1, 2]);
    }
}
