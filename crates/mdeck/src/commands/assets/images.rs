//! `mdeck ai images` and `mdeck ai icons`: fill the deck's `generate:`
//! placeholders with pictures in `talk.assets/images/` and
//! `talk.assets/icons/`, recorded in the manifest. The deck itself is never
//! touched.

use std::path::Path;

use anyhow::{Result, bail};
use colored::Colorize;
use futures::StreamExt;

use super::{check_slide, names, plural, read_deck, wanted};
use crate::assets::manifest::{self, Asset, Kind, Manifest};
use crate::assets::placeholders::{self, Placeholder};
use crate::assets::style::{self, Style};
use crate::cli::Select;
use crate::commands::ai;
use crate::config::Config;
use crate::parser::Presentation;
use crate::prompt;

/// Pictures generated at the same time.
const PARALLEL: usize = 4;

/// Images or icons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Which {
    Images,
    Icons,
}

impl Which {
    pub fn kind(self) -> Kind {
        match self {
            Which::Images => Kind::Image,
            Which::Icons => Kind::Icon,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Which::Images => "images",
            Which::Icons => "icons",
        }
    }
}

/// One placeholder to fill.
struct Job {
    placeholder: Placeholder,
    /// The prompt asked for (the placeholder's, or one the chat model wrote).
    prompt: String,
}

/// The full prompt sent to the image model.
fn full_prompt(which: Which, style: &Style, job: &Job) -> String {
    match which {
        Which::Images => {
            prompt::build_image_prompt(&style.prompt, &job.prompt, job.placeholder.orientation)
        }
        Which::Icons => prompt::build_icon_prompt(&style.prompt, &job.prompt),
    }
}

/// What to generate: the placeholders of `which` the selection asks for.
/// The states found are written back into the manifest.
fn plan(
    deck: &Path,
    pres: &Presentation,
    manifest: &mut Manifest,
    which: Which,
    style: &Style,
    select: &Select,
) -> Vec<Placeholder> {
    let id = style.id();
    let mut todo = Vec::new();
    for p in placeholders::scan(pres)
        .into_iter()
        .filter(|p| p.kind == which.kind())
    {
        let found = p.find(manifest, pres, &id);
        if let Some(f) = found {
            manifest.mark(f);
        }
        let exists =
            found.is_some_and(|f| Manifest::file(deck, &manifest.assets[f.index]).exists());
        if wanted(select, p.slide, found.map(|f| f.state), exists) {
            todo.push(p);
        }
    }
    todo
}

/// `mdeck ai images <deck>` / `mdeck ai icons <deck>`.
pub async fn run(
    file: &Path,
    which: Which,
    select: &Select,
    style_override: Option<&str>,
    quiet: bool,
) -> Result<()> {
    let pres = read_deck(file)?;
    check_slide(select, pres.slides.len())?;
    let config = Config::load_or_default().with_packs(file.parent());
    let styles = style::resolve(&config, &pres.meta, style_override);
    let style = match which {
        Which::Images => styles.image,
        Which::Icons => styles.icon,
    };
    let mut manifest = manifest::load(file)?.unwrap_or_else(Manifest::new);
    let todo = plan(file, &pres, &mut manifest, which, &style, select);
    if todo.is_empty() {
        if !quiet {
            eprintln!("No {} to generate in {}.", which.name(), file.display());
        }
        return Ok(());
    }
    if select.dry_run {
        print_dry_run(&pres, which, &style, &todo);
        return Ok(());
    }
    if !ai::has_capability("image") {
        bail!(
            "Image generation not configured. Run `mdeck ai config` to set up an image provider."
        );
    }
    let jobs = prompts(&pres, todo).await?;
    let client = ailloy::Client::for_capability("image")?;
    let references = Style::reference_files(&style.references)?;
    let has_chat = ai::has_capability("chat");
    let dir = manifest::folder_for(file).join(which.kind().folder());
    std::fs::create_dir_all(&dir)?;
    if !quiet {
        eprintln!(
            "Generating {} {} for {} (style {}).",
            jobs.len(),
            if jobs.len() == 1 {
                &which.name()[..which.name().len() - 1]
            } else {
                which.name()
            },
            file.display(),
            style.id()
        );
    }

    let total = jobs.len();
    let mut stream = futures::stream::iter(jobs.into_iter().map(|job| {
        let prompt = full_prompt(which, &style, &job);
        let client = &client;
        let references = &references;
        async move {
            let result = generate(client, &prompt, references).await;
            (job, result)
        }
    }))
    .buffer_unordered(PARALLEL);
    let (mut done, mut failures) = (0, 0);
    while let Some((job, result)) = stream.next().await {
        let preview = crate::commands::util::truncate_chars(&job.prompt, 50);
        match result {
            Ok((bytes, ext)) => {
                let name = names::generate_filename(has_chat, &job.prompt, ext, &dir).await;
                std::fs::write(dir.join(&name), &bytes)?;
                let slide = &pres.slides[job.placeholder.slide];
                manifest.upsert(Asset {
                    placeholder: Some(job.placeholder.prompt.clone()),
                    slide: Some(job.placeholder.slide + 1),
                    title: slide.title(),
                    hash: job
                        .placeholder
                        .prompt
                        .is_empty()
                        .then(|| manifest::slide_hash(slide, None)),
                    prompt: Some(job.prompt.clone()),
                    generated: Some(crate::commands::util::timestamp()),
                    ..Asset::new(
                        which.kind(),
                        format!("{}/{name}", which.kind().folder()),
                        &style.id(),
                    )
                });
                // save after every picture so an interruption keeps the work so far
                manifest::save(file, &manifest)?;
                done += 1;
                if !quiet {
                    eprintln!("  [{done}/{total}] {preview} {}", "✓".green().bold());
                }
            }
            Err(e) => {
                failures += 1;
                eprintln!("  {preview} {} {e:#}", "✗".red().bold());
            }
        }
    }
    if failures > 0 {
        bail!(
            "{failures} picture{} failed; {done} saved. Run the command again to retry the rest.",
            plural(failures)
        );
    }
    if !quiet {
        eprintln!(
            "Done: {done} saved in {}, recorded in {}.",
            dir.display(),
            manifest::path_for(file).display()
        );
    }
    Ok(())
}

/// Prompts for the jobs: the placeholder's own, or the chat model's for
/// an image with an empty one.
async fn prompts(pres: &Presentation, todo: Vec<Placeholder>) -> Result<Vec<Job>> {
    let mut jobs = Vec::new();
    for p in todo {
        let prompt = if p.prompt.trim().is_empty() {
            if !ai::has_capability("chat") {
                bail!(
                    "slide {} has an image with no prompt; add one (`![a prompt](generate:)`) or configure a chat provider",
                    p.slide + 1
                );
            }
            names::auto_prompt(pres, p.slide + 1).await?
        } else {
            p.prompt.clone()
        };
        jobs.push(Job {
            placeholder: p,
            prompt,
        });
    }
    Ok(jobs)
}

/// One picture: bytes and file extension. Reference images go along when
/// the style has them.
async fn generate(
    client: &ailloy::Client,
    prompt: &str,
    references: &[std::path::PathBuf],
) -> Result<(Vec<u8>, &'static str)> {
    if references.is_empty() {
        let r = client.generate_image(prompt).await?;
        return Ok((r.data, ai::image_ext(&r.format)));
    }
    let options = ailloy::ImageOptions::builder()
        .reference_images(references.to_vec())
        .build();
    let images = client.generate_images_with(prompt, &options).await?;
    let image = images
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("the model returned no image"))?;
    Ok((image.data, ai::image_ext(&image.format)))
}

fn print_dry_run(pres: &Presentation, which: Which, style: &Style, todo: &[Placeholder]) {
    for p in todo {
        let prompt = if p.prompt.is_empty() {
            "prompt written from the slide".dimmed().to_string()
        } else {
            format!("\"{}\"", p.prompt)
        };
        eprintln!(
            "  {:>3}  {}  {}",
            p.slide + 1,
            pres.slides[p.slide].title().unwrap_or_default().bold(),
            prompt
        );
    }
    eprintln!(
        "Dry run: {} {} in style {}. Nothing was generated.",
        todo.len(),
        which.name(),
        style.id()
    );
}

/// `mdeck ai images --prompt` / `mdeck ai icons --prompt`: one picture,
/// written to `--output` or a temporary file.
pub async fn one_off(
    which: Which,
    prompt_text: &str,
    style_name: Option<&str>,
    output: Option<std::path::PathBuf>,
) -> Result<()> {
    let config = Config::load_or_default().with_packs(Some(Path::new(".")));
    let meta = crate::parser::PresentationMeta::default();
    let styles = style::resolve(&config, &meta, style_name);
    let (style, job_prompt) = match which {
        Which::Images => (
            &styles.image,
            prompt::build_image_prompt(
                &styles.image.prompt,
                prompt_text,
                prompt::Orientation::Horizontal,
            ),
        ),
        Which::Icons => (
            &styles.icon,
            prompt::build_icon_prompt(&styles.icon.prompt, prompt_text),
        ),
    };
    ai::generate_one(
        &job_prompt,
        &Style::reference_files(&style.references)?,
        output,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    #[test]
    fn plan_takes_missing_and_stale_and_marks_states() {
        let dir = std::env::temp_dir().join(format!("mdeck-images-plan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("talk.assets/images")).unwrap();
        let deck = dir.join("talk.md");
        let pres = parser::parse(
            "# A\n\n![a rocket](generate:)\n\n# B\n\n![a boat](generate:)\n\n# C\n\n![a kite](generate:)\n\n```@architecture\n- Db (icon: generate:, pos: 1,1)\n```\n",
        );
        let style = Style::new("default", "img", Vec::new());
        let mut m = Manifest::new();
        for (prompt, st) in [("a boat", style.id()), ("a kite", "old".to_string())] {
            let file = format!("images/{}.png", prompt.replace(' ', "-"));
            std::fs::write(dir.join("talk.assets").join(&file), b"x").unwrap();
            m.upsert(Asset {
                placeholder: Some(prompt.into()),
                ..Asset::new(Kind::Image, file, &st)
            });
        }
        let todo = plan(
            &deck,
            &pres,
            &mut m,
            Which::Images,
            &style,
            &Select::default(),
        );
        let prompts: Vec<&str> = todo.iter().map(|p| p.prompt.as_str()).collect();
        assert_eq!(prompts, ["a rocket", "a kite"]);
        let kite = m
            .assets
            .iter()
            .find(|a| a.placeholder.as_deref() == Some("a kite"))
            .unwrap();
        assert_eq!(kite.state, manifest::State::Stale, "the state is recorded");
        let dry = Select {
            stale: true,
            ..Select::default()
        };
        let todo = plan(&deck, &pres, &mut m, Which::Images, &style, &dry);
        assert_eq!(todo.len(), 1);
        let icons = plan(
            &deck,
            &pres,
            &mut m,
            Which::Icons,
            &style,
            &Select::default(),
        );
        assert_eq!(icons.len(), 1);
        assert_eq!(icons[0].prompt, "Db");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn prompts_carry_the_style_and_the_shape() {
        let style = Style::new("photo", "STYLE", Vec::new());
        let job = Job {
            placeholder: Placeholder {
                kind: Kind::Image,
                slide: 0,
                prompt: "a cat".into(),
                orientation: prompt::Orientation::Vertical,
            },
            prompt: "a cat".into(),
        };
        let p = full_prompt(Which::Images, &style, &job);
        assert!(p.contains("STYLE") && p.contains("a cat") && p.contains("portrait"));
        let p = full_prompt(Which::Icons, &style, &job);
        assert!(p.contains("transparent background"));
    }
}
