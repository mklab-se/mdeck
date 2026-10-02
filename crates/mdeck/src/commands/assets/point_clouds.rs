//! `mdeck ai point-cloud <deck>`: a point cloud for every `picture`
//! name the deck uses that resolves nowhere (not in the deck's or the
//! user's library, not built in), written to `talk.assets/point-clouds/`
//! and recorded in the manifest. Presenting finds them there first.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use colored::Colorize;

use super::{check_slide, plural, read_deck, wanted};
use crate::assets::manifest::{self, Asset, Kind, Manifest, State};
use crate::assets::style::Style;
use crate::cli::Select;
use crate::commands::ai;
use crate::parser::Presentation;
use crate::render::illustration::{self, EXTENSION, Library, convert};

/// The style point clouds are made in: the particle prompt.
pub fn style() -> Style {
    Style::new("point-cloud", &illustration_prompt_template(), Vec::new())
}

fn illustration_prompt_template() -> String {
    crate::commands::point_cloud::image_prompt("{subject}")
}

/// A name the deck asks for, with the first slide that uses it.
#[derive(Debug, PartialEq)]
struct Wanted {
    name: String,
    slide: usize,
}

/// `picture` names to generate: the ones that do not resolve in the
/// deck's libraries, and the ones the manifest already holds (so `--force`
/// and `--stale` reach them). Names are listed once, at their first slide.
fn plan(deck: &Path, pres: &Presentation, manifest: &mut Manifest, select: &Select) -> Vec<Wanted> {
    let base = deck.parent().unwrap_or(Path::new("."));
    let mut lib = Library::for_deck(Some(base));
    let id = style().id();
    let mut first: BTreeMap<String, usize> = BTreeMap::new();
    for (i, slide) in pres.slides.iter().enumerate() {
        if let Some(name) = slide.illustration.as_deref().map(str::trim)
            && illustration::validate_name(name).is_ok()
        {
            first.entry(name.to_string()).or_insert(i);
        }
    }
    let mut out = Vec::new();
    for (name, slide) in first {
        let found = manifest.placeholder(Kind::PointCloud, &name, slide + 1, "", &id);
        if found.is_none() && lib.has(&name) {
            continue;
        }
        if let Some(f) = found {
            manifest.mark(f);
        }
        let exists =
            found.is_some_and(|f| Manifest::file(deck, &manifest.assets[f.index]).exists());
        if wanted(select, slide, found.map(|f| f.state), exists) {
            out.push(Wanted { name, slide });
        }
    }
    out.sort_by_key(|w| w.slide);
    out
}

/// `mdeck ai point-cloud <deck>`.
pub async fn run_deck(
    file: &Path,
    select: &Select,
    description: Option<&str>,
    quiet: bool,
) -> Result<()> {
    let pres = read_deck(file)?;
    check_slide(select, pres.slides.len())?;
    let mut manifest = manifest::load(file)?.unwrap_or_else(Manifest::new);
    let todo = plan(file, &pres, &mut manifest, select);
    if todo.is_empty() {
        if !quiet {
            eprintln!("No point clouds to generate in {}.", file.display());
        }
        return Ok(());
    }
    if select.dry_run {
        for w in &todo {
            eprintln!("  {:>3}  {}", w.slide + 1, w.name.bold());
        }
        eprintln!(
            "Dry run: {} point cloud{}. Nothing was generated.",
            todo.len(),
            plural(todo.len())
        );
        return Ok(());
    }
    if !ai::has_capability("image") {
        bail!(
            "Image generation not configured. Run `mdeck ai config` to set up an image provider."
        );
    }
    let client = ailloy::Client::for_capability("image")?;
    let dir = crate::assets::point_cloud_dir(file);
    std::fs::create_dir_all(&dir)?;
    let mut failures = 0;
    for w in &todo {
        let subject = description.unwrap_or(&w.name).to_string();
        let prompt = crate::commands::point_cloud::image_prompt(&subject);
        let made = async {
            let response = client.generate_image(&prompt).await?;
            let img =
                image::load_from_memory(&response.data).context("decoding the generated image")?;
            let mut cloud = convert::convert(&img, &w.name, &subject)?;
            cloud.prompt = Some(prompt.clone());
            cloud.generated = Some(crate::commands::util::timestamp());
            anyhow::Ok(cloud)
        }
        .await;
        match made {
            Ok(cloud) => {
                let rel = format!("{}/{}.{EXTENSION}", Kind::PointCloud.folder(), w.name);
                std::fs::write(manifest::folder_for(file).join(&rel), cloud.to_json())?;
                manifest.upsert(Asset {
                    placeholder: Some(w.name.clone()),
                    slide: Some(w.slide + 1),
                    title: pres.slides[w.slide].title(),
                    prompt: Some(subject),
                    generated: cloud.generated.clone(),
                    state: State::Current,
                    ..Asset::new(Kind::PointCloud, rel, &style().id())
                });
                manifest::save(file, &manifest)?;
                if !quiet {
                    eprintln!("{} {} ({} points)", "✓".green(), w.name, cloud.points.len());
                }
            }
            Err(e) => {
                failures += 1;
                eprintln!("{} {} {e:#}", "✗".red(), w.name);
            }
        }
    }
    if failures > 0 {
        bail!("{failures} point cloud{} failed", plural(failures));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    #[test]
    fn only_names_that_resolve_nowhere_or_were_generated_are_planned() {
        let dir = std::env::temp_dir().join(format!("mdeck-pc-plan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let deck = dir.join("talk.md");
        // a built-in name is never planned (built-ins come with the particles engine)
        let b = match illustration::builtin_names().first() {
            Some(builtin) => format!("## B\n<!-- picture: {builtin} -->\n\n- b\n\n"),
            None => String::new(),
        };
        let md = format!(
            "## A\n<!-- picture: zz-unknown-thing -->\n\n- a\n\n{b}## C\n<!-- picture: zz-unknown-thing -->\n\n- c\n"
        );
        let pres = parser::parse(&md);
        let mut m = Manifest::new();
        let todo = plan(&deck, &pres, &mut m, &Select::default());
        assert_eq!(
            todo,
            [Wanted {
                name: "zz-unknown-thing".into(),
                slide: 0
            }]
        );
        // generated and current: nothing to do unless forced
        std::fs::create_dir_all(dir.join("talk.assets/point-clouds")).unwrap();
        std::fs::write(
            dir.join("talk.assets/point-clouds/zz-unknown-thing.mdpc"),
            "{}",
        )
        .unwrap();
        m.upsert(Asset {
            placeholder: Some("zz-unknown-thing".into()),
            ..Asset::new(
                Kind::PointCloud,
                "point-clouds/zz-unknown-thing.mdpc".into(),
                &style().id(),
            )
        });
        assert!(plan(&deck, &pres, &mut m, &Select::default()).is_empty());
        let force = Select {
            force: true,
            ..Select::default()
        };
        assert_eq!(plan(&deck, &pres, &mut m, &force).len(), 1);
        std::fs::remove_dir_all(&dir).ok();
    }
}
