//! `mdeck ai art`: draw a picture for every slide that takes one, in the
//! style of the deck's art engine, with the configured image model. Scenes
//! come from a slide's own `@art:`, or the chat model writes one from the
//! slide's copy, its notes and the deck's `@art` world. Pictures go into
//! `art/` next to the deck and are recorded in `<deck>.art.yaml`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use colored::Colorize;
use futures::StreamExt;

use crate::parser::{self, Presentation};
use crate::render::art::sidecar::{self, Sidecar, Source};
use crate::render::art::style::Style;
use crate::render::art::{self, ArtKind};
use crate::theme::Theme;
use generate::{Fit, draw_one, file_name, image_prompt, reference_files, scenes};

mod generate;

/// Pictures generated at once; the image services throttle beyond this.
const PARALLEL: usize = 4;

/// The deck's theme on the engine it runs on (`--engine`, `@engine`, the theme's).
fn deck_theme(pres: &Presentation, base: &Path, engine: Option<&str>) -> Result<Theme> {
    let defaults = crate::config::Config::load_or_default()
        .defaults
        .unwrap_or_default();
    let (theme, problems) =
        crate::commands::check::deck_theme(pres, defaults.theme.as_deref(), base, engine)?;
    for p in problems {
        eprintln!("warning: {p}");
    }
    Ok(theme)
}

/// The engines that draw art, for messages.
fn art_engines() -> String {
    crate::engines::EngineKind::ALL
        .iter()
        .filter(|k| k.medium().is_some())
        .map(|k| k.name())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Which slides to draw: the ones that take art (or `--slide`), not pinned,
/// and unless `force`, not already current; with `stale`, only stale ones.
fn targets(
    pres: &Presentation,
    resolved: &[Option<sidecar::Resolved>],
    slide: Option<usize>,
    stale: bool,
    force: bool,
) -> Result<Vec<usize>> {
    let count = pres.slides.len();
    let mut out: Vec<usize> = match slide {
        Some(n) if n == 0 || n > count => bail!("slide {n} is outside 1-{count}"),
        Some(n) => {
            if !art::wants_art(&pres.slides[n - 1]) {
                bail!(
                    "slide {n} takes no art (it says `@art: none`, or its layout has no room for a picture)"
                );
            }
            vec![n - 1]
        }
        None => (0..count)
            .filter(|&i| art::wants_art(&pres.slides[i]))
            .collect(),
    };
    out.retain(|&i| {
        let r = resolved.get(i).and_then(|r| r.as_ref());
        if r.is_some_and(|r| r.source == Source::Pinned) {
            return false;
        }
        if stale {
            return r.is_some_and(|r| r.source == Source::Stale);
        }
        force
            || slide.is_some()
            || !r.is_some_and(|r| r.source == Source::Current && r.file.exists())
    });
    Ok(out)
}

/// What `mdeck ai art` should draw, and how.
pub struct Options {
    /// Draw only this slide (1-based).
    pub slide: Option<usize>,
    /// Only redraw stale pictures.
    pub stale: bool,
    /// Redraw current pictures too.
    pub force: bool,
    /// List what would be drawn and stop.
    pub dry_run: bool,
    /// Draw for this engine instead of the deck's.
    pub engine: Option<String>,
    /// Use this ailloy node instead of the default image one.
    pub node: Option<String>,
    pub quiet: bool,
}

/// The `mdeck ai art` command.
pub async fn run(file: PathBuf, opts: Options) -> Result<()> {
    let quiet = opts.quiet;
    let content = std::fs::read_to_string(&file)?;
    let base = file.parent().unwrap_or(Path::new(".")).to_path_buf();
    let pres = parser::parse(&content);
    if pres.slides.is_empty() {
        bail!("No slides found in {}", file.display());
    }
    let theme = deck_theme(&pres, &base, opts.engine.as_deref())?;
    let Some(medium) = theme.engine.medium() else {
        bail!(
            "the deck runs on the {} engine, which draws no art. Choose an engine that does ({}) with `@engine:` in the frontmatter, a theme on one, or --engine",
            theme.engine.name(),
            art_engines()
        );
    };
    let style = Style::for_medium(medium, &theme);
    let sc = sidecar::load(&file)?;
    let resolved = sidecar::resolve(&file, &pres, sc.as_ref(), &style.id());
    let todo = targets(&pres, &resolved, opts.slide, opts.stale, opts.force)?;
    if todo.is_empty() {
        if !quiet {
            eprintln!(
                "Nothing to do: every slide that takes art has a current picture for the {} engine.",
                medium.name
            );
        }
        return Ok(());
    }
    if !quiet {
        let kind = match style.kind {
            ArtKind::Line => "line art",
            ArtKind::Tonal => "pictures",
        };
        eprintln!(
            "Drawing {} for {} slide{} of {} ({} engine, style {}).",
            kind,
            todo.len(),
            plural(todo.len()),
            file.display(),
            medium.name,
            style.id()
        );
    }
    if opts.dry_run {
        print_dry_run(&pres, &todo);
        return Ok(());
    }

    let scenes = scenes(&pres, &todo, quiet).await?;
    let target = Target {
        file: &file,
        pres: &pres,
        style: &style,
    };
    let mut sc = sc.unwrap_or_else(empty_sidecar);
    let tally = draw_all(&target, &mut sc, scenes, opts.node.as_deref(), quiet).await?;
    if !tally.took_references && !quiet {
        eprintln!(
            "note: the image model does not take reference images; the style came from the prompt alone"
        );
    }
    let Tally { done, failures, .. } = tally;
    if failures > 0 {
        bail!(
            "{failures} picture{} failed; {done} saved. Run the command again to retry the rest.",
            plural(failures)
        );
    }
    if !quiet {
        eprintln!(
            "Done: {done} picture{} in {}. Present with `mdeck {}`.",
            plural(done),
            sidecar::folder_for(&file).display(),
            file.display()
        );
    }
    Ok(())
}

/// The deck and style pictures are drawn for.
struct Target<'a> {
    file: &'a Path,
    pres: &'a Presentation,
    style: &'a Style,
}

impl Target<'_> {
    /// Write slide `i`'s picture into `art/`, record it and save the sidecar.
    fn save(&self, sc: &mut Sidecar, i: usize, scene: String, bytes: &[u8]) -> Result<()> {
        let folder = sidecar::folder_for(self.file);
        let name = file_name(self.file, i, self.style);
        std::fs::write(folder.join(&name), bytes)?;
        sidecar::upsert(
            sc,
            self.pres,
            i,
            &self.style.id(),
            format!("art/{name}"),
            Some(scene),
            crate::commands::util::timestamp(),
        );
        sidecar::save(self.file, sc)?;
        Ok(())
    }
}

/// How a batch of pictures went.
struct Tally {
    done: usize,
    failures: usize,
    /// Whether the image model accepted the style's reference images.
    took_references: bool,
}

/// Draw every scene, `PARALLEL` at a time, saving each picture as it arrives.
async fn draw_all(
    target: &Target<'_>,
    sc: &mut Sidecar,
    scenes: Vec<(usize, String)>,
    node: Option<&str>,
    quiet: bool,
) -> Result<Tally> {
    let client = Arc::new(match node {
        Some(n) => ailloy::Client::with_node(n)?,
        None => ailloy::Client::for_capability("image")?,
    });
    let references = reference_files(target.style)?;
    let fit = Arc::new(Fit::new());
    std::fs::create_dir_all(sidecar::folder_for(target.file))?;

    let jobs = scenes.into_iter().map(|(i, scene)| {
        let client = client.clone();
        let references = references.clone();
        let fit = fit.clone();
        let with = image_prompt(&scene, target.style, true);
        let without = image_prompt(&scene, target.style, false);
        async move {
            let result = draw_one(&client, &with, &without, &references, &fit).await;
            (i, scene, result)
        }
    });
    let mut stream = futures::stream::iter(jobs).buffer_unordered(PARALLEL);
    let mut failures = 0;
    let mut done = 0;
    while let Some((i, scene, result)) = stream.next().await {
        match result {
            Ok(bytes) => {
                // save after every picture so an interruption keeps the work so far
                target.save(sc, i, scene, &bytes)?;
                done += 1;
                if !quiet {
                    eprintln!(
                        "{} slide {:>2}  {}",
                        "✓".green(),
                        i + 1,
                        target.pres.slides[i].title().unwrap_or_default()
                    );
                }
            }
            Err(e) => {
                failures += 1;
                if !quiet {
                    eprintln!("{} slide {:>2}  {e}", "✗".red(), i + 1);
                }
            }
        }
    }
    Ok(Tally {
        done,
        failures,
        took_references: fit.took_references(),
    })
}

/// "" for one, "s" for more.
fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

fn empty_sidecar() -> Sidecar {
    Sidecar {
        version: sidecar::VERSION,
        slides: vec![],
    }
}

/// `--dry-run`: each slide that would be drawn and where its scene comes from.
fn print_dry_run(pres: &Presentation, todo: &[usize]) {
    for &i in todo {
        let from = match art::slide_scene(&pres.slides[i]) {
            Some(s) => format!("@art: {s}"),
            None => "scene written from the slide".dimmed().to_string(),
        };
        eprintln!(
            "  {:>3}  {}  {}",
            i + 1,
            pres.slides[i].title().unwrap_or_default().bold(),
            from
        );
    }
    eprintln!(
        "Dry run: {} picture{} (about 20 seconds each, {} at a time). Nothing was generated.",
        todo.len(),
        plural(todo.len()),
        PARALLEL
    );
}

/// Draw the picture for one slide from a running app (the `S` key), for the
/// theme it is showing. Blocks; call it from a worker thread.
pub fn generate_one_blocking(deck: &Path, index: usize, theme: &Theme) -> Result<()> {
    crate::commands::util::block_on(async {
        let content = std::fs::read_to_string(deck)?;
        let pres = parser::parse(&content);
        if index >= pres.slides.len() {
            bail!("slide {} does not exist", index + 1);
        }
        let medium = theme
            .engine
            .medium()
            .context("this theme's engine draws no art")?;
        let style = Style::for_medium(medium, theme);
        let sc = sidecar::load(deck)?;
        let resolved = sidecar::resolve(deck, &pres, sc.as_ref(), &style.id());
        if resolved[index]
            .as_ref()
            .is_some_and(|r| r.source == Source::Pinned)
        {
            bail!("slide {}'s picture is pinned", index + 1);
        }
        let targets = targets(&pres, &resolved, Some(index + 1), false, true)?;
        let scenes = scenes(&pres, &targets, true).await?;
        let (i, scene) = scenes.into_iter().next().context("no scene")?;
        let client = ailloy::Client::for_capability("image")?;
        let references = reference_files(&style)?;
        let fit = Fit::new();
        let bytes = draw_one(
            &client,
            &image_prompt(&scene, &style, true),
            &image_prompt(&scene, &style, false),
            &references,
            &fit,
        )
        .await?;
        std::fs::create_dir_all(sidecar::folder_for(deck))?;
        let mut sc = sc.unwrap_or_else(empty_sidecar);
        let target = Target {
            file: deck,
            pres: &pres,
            style: &style,
        };
        target.save(&mut sc, i, scene, &bytes)
    })?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deck() -> Presentation {
        parser::parse(
            "---\ntitle: Harbour\n@art: a Victorian harbour town that builds software\n---\n# Launch\n\nWe ship today\n\n# Why\n@art: a lighthouse keeper with a laptop\n\n- one\n\n???\nTell the story of the storm.\n\n# Code\n\n```rust\nfn main() {}\n```\n\n# None\n@art: none\n\n- two\n",
        )
    }

    #[test]
    fn targets_skip_slides_without_art_and_current_pictures() {
        let pres = deck();
        let none = vec![None; pres.slides.len()];
        assert_eq!(
            targets(&pres, &none, None, false, false).unwrap(),
            vec![0, 1]
        );
        assert!(targets(&pres, &none, Some(3), false, false).is_err());
        assert!(targets(&pres, &none, Some(9), false, false).is_err());
        // --stale only touches stale pictures
        assert!(targets(&pres, &none, None, true, false).unwrap().is_empty());
    }
}
