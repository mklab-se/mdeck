//! `mdeck illustration`: make, import, list and preview point cloud
//! illustrations for the particle field.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use colored::Colorize;

use crate::cli::IllustrationCommands;
use crate::commands::ai;
use crate::render::illustration::{self, Cloud, EXTENSION, Source, convert};

pub async fn run(command: IllustrationCommands, quiet: bool) -> Result<()> {
    match command {
        IllustrationCommands::Generate {
            name,
            description,
            user,
            force,
        } => generate(&name, &description, user, force, quiet).await,
        IllustrationCommands::Import {
            image,
            name,
            user,
            force,
        } => import(&image, &name, user, force, quiet),
        IllustrationCommands::List => {
            list();
            Ok(())
        }
        IllustrationCommands::Show { name, output } => show(&name, output, quiet),
    }
}

/// The prompt sent to the image model for `description`.
pub fn image_prompt(description: &str) -> String {
    format!(
        "{description}. \
         A sparse constellation of small glowing warm-orange particles on a pure black \
         background, forming only the essential outline and a few key details of this single \
         subject, the way a night sky hints at a figure. The particles are separate points of \
         light, not solid strokes, with black space between them. The subject is centred and \
         fills the frame. No text, no ground, no shadows, no other objects, no frame."
    )
}

/// Where a new cloud is written: the deck-local folder in the working
/// directory, or the user library.
fn target_dir(user: bool) -> Result<PathBuf> {
    if user {
        illustration::user_dir().context("no config directory for the user library")
    } else {
        Ok(illustration::deck_dir(Path::new(".")))
    }
}

fn write_cloud(cloud: &Cloud, user: bool, force: bool) -> Result<PathBuf> {
    let dir = target_dir(user)?;
    let path = dir.join(format!("{}.{EXTENSION}", cloud.name));
    if path.exists() && !force {
        bail!(
            "{} already exists; pass --force to overwrite it",
            path.display()
        );
    }
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    std::fs::write(&path, cloud.to_json())
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(path)
}

async fn generate(
    name: &str,
    description: &str,
    user: bool,
    force: bool,
    quiet: bool,
) -> Result<()> {
    illustration::validate_name(name).map_err(|e| anyhow::anyhow!(e))?;
    if !ai::has_capability("image") {
        bail!(
            "Image generation not configured. Run `mdeck ai config` to set up an image provider."
        );
    }
    // refuse early so no credits are spent on a file we will not write
    let dir = target_dir(user)?;
    let path = dir.join(format!("{name}.{EXTENSION}"));
    if path.exists() && !force {
        bail!(
            "{} already exists; pass --force to overwrite it",
            path.display()
        );
    }

    let prompt = image_prompt(description);
    if !quiet {
        println!("Generating an image for {}...", name.cyan());
    }
    let client = ailloy::Client::for_capability("image")?;
    let response = client.generate_image(&prompt).await?;
    let img = image::load_from_memory(&response.data).context("decoding the generated image")?;

    let mut cloud = convert::convert(&img, name, description).map_err(|e| anyhow::anyhow!(e))?;
    cloud.prompt = Some(prompt);
    cloud.generated = Some(crate::commands::story::timestamp());
    let path = write_cloud(&cloud, user, force)?;

    // keep the source image next to the cloud for inspection
    let ext = ai::image_ext(&response.format);
    let source = path.with_extension(ext);
    std::fs::write(&source, &response.data).ok();

    if !quiet {
        println!(
            "{} {} ({} points, aspect {:.2})",
            "✓".green().bold(),
            path.display(),
            cloud.points.len(),
            cloud.aspect
        );
        println!("  source image: {}", source.display().to_string().dimmed());
        preview(&cloud, None, quiet)?;
    }
    Ok(())
}

fn import(image_path: &Path, name: &str, user: bool, force: bool, quiet: bool) -> Result<()> {
    illustration::validate_name(name).map_err(|e| anyhow::anyhow!(e))?;
    let img =
        image::open(image_path).with_context(|| format!("opening {}", image_path.display()))?;
    let description = image_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let cloud = convert::convert(&img, name, &description).map_err(|e| anyhow::anyhow!(e))?;
    let path = write_cloud(&cloud, user, force)?;
    if !quiet {
        println!(
            "{} {} ({} points, aspect {:.2})",
            "✓".green().bold(),
            path.display(),
            cloud.points.len(),
            cloud.aspect
        );
        preview(&cloud, None, quiet)?;
    }
    Ok(())
}

fn list() {
    let entries = illustration::catalogue(Some(Path::new(".")));
    if entries.is_empty() {
        println!("No illustrations found.");
        return;
    }
    let width = entries.iter().map(|(n, _, _)| n.len()).max().unwrap_or(4);
    for (name, src, shadowed) in entries {
        let mut line = format!("{:width$}  {}", name.bold(), src.label());
        if let Source::Deck(p) | Source::User(p) = &src {
            line.push_str(&format!("  {}", p.display().to_string().dimmed()));
        }
        if !shadowed.is_empty() {
            let names: Vec<&str> = shadowed.iter().map(|s| s.label()).collect();
            line.push_str(&format!(
                "  {}",
                format!("(shadows {})", names.join(", ")).dimmed()
            ));
        }
        println!("{line}");
    }
}

fn show(name: &str, output: Option<PathBuf>, quiet: bool) -> Result<()> {
    let Some((src, cloud)) =
        illustration::resolve(name, Some(Path::new("."))).map_err(|e| anyhow::anyhow!(e))?
    else {
        bail!("no illustration named `{name}` (run `mdeck illustration list`)");
    };
    if !quiet {
        println!(
            "{} — {} ({} points, aspect {:.2}, {})",
            name.bold(),
            cloud.description,
            cloud.points.len(),
            cloud.aspect,
            src.label()
        );
    }
    preview(&cloud, output, quiet)
}

/// Plot the cloud as glowing dots on black and show it (or save it).
fn preview(cloud: &Cloud, output: Option<PathBuf>, quiet: bool) -> Result<()> {
    let path =
        output.unwrap_or_else(|| std::env::temp_dir().join(format!("mdeck-{}.png", cloud.name)));
    let img = render_preview(cloud, 720);
    img.save(&path)
        .with_context(|| format!("writing {}", path.display()))?;
    if !quiet {
        ai::display_image_result(&path);
    }
    Ok(())
}

/// A `size`-tall (or wide, for wide clouds) preview: every point as a soft
/// dot, the earliest points brightest so the importance order is visible.
pub fn render_preview(cloud: &Cloud, size: u32) -> image::RgbaImage {
    let margin = 0.08;
    let (w, h) = if cloud.aspect >= 1.0 {
        ((size as f32 / cloud.aspect).round() as u32, size)
    } else {
        (size, (size as f32 * cloud.aspect).round() as u32)
    };
    let mut img = image::RgbaImage::from_pixel(w.max(1), h.max(1), image::Rgba([8, 8, 10, 255]));
    let n = cloud.points.len().max(1) as f32;
    let r = 2.2f32;
    for (i, p) in cloud.points.iter().enumerate() {
        let x = (margin + p[0] * (1.0 - 2.0 * margin)) * w as f32;
        let y = (margin + p[1] * (1.0 - 2.0 * margin)) * h as f32;
        let rank = 1.0 - i as f32 / n;
        let bright = 0.45 + 0.55 * rank;
        let (x0, x1) = (
            (x - r - 1.0).floor().max(0.0) as u32,
            ((x + r + 1.0).ceil() as u32).min(w - 1),
        );
        let (y0, y1) = (
            (y - r - 1.0).floor().max(0.0) as u32,
            ((y + r + 1.0).ceil() as u32).min(h - 1),
        );
        for py in y0..=y1 {
            for px in x0..=x1 {
                let d = ((px as f32 + 0.5 - x).powi(2) + (py as f32 + 0.5 - y).powi(2)).sqrt();
                let a = ((r - d) / 1.2).clamp(0.0, 1.0) * bright;
                if a <= 0.0 {
                    continue;
                }
                let px_ = img.get_pixel_mut(px, py);
                let mix = |c: u8, t: u8| (c as f32 + (t as f32 - c as f32) * a).min(255.0) as u8;
                *px_ = image::Rgba([mix(px_[0], 255), mix(px_[1], 138), mix(px_[2], 60), 255]);
            }
        }
    }
    img
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn preview_keeps_the_aspect_and_lights_the_points() {
        let cloud = Cloud {
            version: illustration::VERSION,
            name: "t".into(),
            description: String::new(),
            prompt: None,
            generated: None,
            aspect: 2.0,
            points: Arc::new(vec![[0.5, 0.5]]),
        };
        let img = render_preview(&cloud, 200);
        assert_eq!(img.dimensions(), (100, 200));
        let centre = img.get_pixel(50, 100);
        assert!(centre[0] > 200, "centre is not lit: {centre:?}");
        assert!(img.get_pixel(2, 2)[0] < 20);
    }

    #[test]
    fn prompt_names_the_subject_and_the_look() {
        let p = image_prompt("A server in a rack");
        assert!(p.starts_with("A server in a rack."));
        assert!(p.contains("black background"));
    }
}
