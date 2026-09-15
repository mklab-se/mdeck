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
        IllustrationCommands::Contribute { name, no_open } => contribute(&name, no_open, quiet),
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

// ---------------------------------------------------------------------------
// Contributing
// ---------------------------------------------------------------------------

/// Where contributions go.
const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
/// Longest issue link a browser and GitHub reliably accept.
const MAX_URL_LEN: usize = 8000;

/// Offer a deck or user illustration as a built-in: write a `.json` copy
/// GitHub will accept as an attachment, and open a new-issue page with the
/// title and body filled in. The person drags the file onto the issue and
/// submits; nothing needs to be installed.
fn contribute(name: &str, no_open: bool, quiet: bool) -> Result<()> {
    let Some((src, cloud)) =
        illustration::resolve(name, Some(Path::new("."))).map_err(|e| anyhow::anyhow!(e))?
    else {
        bail!("no illustration named `{name}` (run `mdeck illustration list`)");
    };
    let path = match &src {
        Source::Deck(p) | Source::User(p) => p.clone(),
        Source::Builtin => bail!("`{name}` is already a built-in illustration"),
    };
    // GitHub attaches .json, not .mdpc
    let attachment = path.with_extension("mdpc.json");
    std::fs::copy(&path, &attachment)
        .with_context(|| format!("writing {}", attachment.display()))?;

    let title = format!("Illustration: {name}");
    let body = issue_body(
        &cloud,
        attachment
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("the file"),
    );
    let url = issue_url(REPOSITORY, &title, &body);

    if !quiet {
        println!(
            "{} {} is ready to attach ({} points, aspect {:.2})",
            "✓".green().bold(),
            attachment.display(),
            cloud.points.len(),
            cloud.aspect
        );
        println!();
        println!("{}", braille(&cloud, 48, 24));
        println!();
        println!("Drag that file onto the issue text and submit:");
        println!("  {}", url.cyan());
    }
    if !no_open && open_in_browser(&url).is_err() && !quiet {
        println!("  (could not open a browser; paste the link yourself)");
    }
    Ok(())
}

/// The issue text: what the cloud is, how it was made, a braille sketch and
/// what to do next.
pub fn issue_body(cloud: &Cloud, attachment: &str) -> String {
    let mut b = String::new();
    b.push_str(&format!("**Name:** `{}`\n", cloud.name));
    b.push_str(&format!("**Description:** {}\n", cloud.description));
    b.push_str(&format!(
        "**Points:** {}, aspect {:.2}\n",
        cloud.points.len(),
        cloud.aspect
    ));
    if let Some(p) = &cloud.prompt {
        b.push_str(&format!("**Prompt:** {p}\n"));
    }
    // small: every braille cell costs nine bytes in the link
    b.push_str("\n```\n");
    b.push_str(&braille(cloud, 32, 12));
    b.push_str("\n```\n\n");
    b.push_str(&format!(
        "**Attach `{attachment}`** by dragging it onto this text box, then submit.\n\n"
    ));
    b.push_str(
        "I made this illustration and contribute it under the project's license, for \
         inclusion in MDeck's built-in set.\n",
    );
    b
}

/// `<repository>/issues/new?...` with the title and body filled in. The body
/// is trimmed if the link would grow too long for a browser.
pub fn issue_url(repository: &str, title: &str, body: &str) -> String {
    let base = format!(
        "{}/issues/new?labels=illustration&title={}&body=",
        repository.trim_end_matches('/'),
        percent_encode(title)
    );
    let mut body = body.to_string();
    let mut url = format!("{base}{}", percent_encode(&body));
    while url.len() > MAX_URL_LEN && body.len() > 200 {
        let cut = body.len() - 200;
        let cut = body
            .char_indices()
            .map(|(i, _)| i)
            .take_while(|&i| i <= cut)
            .last()
            .unwrap_or(0);
        body.truncate(cut);
        url = format!("{base}{}", percent_encode(&body));
    }
    url
}

fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// The cloud as braille dots, at most `cols` characters wide and `rows`
/// tall, keeping the cloud's aspect (a braille cell is two dots wide and
/// four tall, and a terminal cell is about twice as tall as it is wide).
pub fn braille(cloud: &Cloud, cols: usize, rows: usize) -> String {
    let (cols, rows) = (cols.max(1), rows.max(1));
    // dot grid that keeps the aspect within the box
    let mut w = (cols * 2) as f32;
    let mut h = w * cloud.aspect;
    if h > (rows * 4) as f32 {
        h = (rows * 4) as f32;
        w = h / cloud.aspect.max(0.01);
    }
    let (w, h) = ((w as usize).max(2), (h as usize).max(4));
    let (cw, ch) = (w.div_ceil(2), h.div_ceil(4));
    let mut cells = vec![0u8; cw * ch];
    for p in cloud.points.iter() {
        let x = ((p[0] * (w - 1) as f32).round() as usize).min(w - 1);
        let y = ((p[1] * (h - 1) as f32).round() as usize).min(h - 1);
        let (cx, cy) = (x / 2, y / 4);
        let bit = match (x % 2, y % 4) {
            (0, 0) => 0x01,
            (0, 1) => 0x02,
            (0, 2) => 0x04,
            (1, 0) => 0x08,
            (1, 1) => 0x10,
            (1, 2) => 0x20,
            (0, _) => 0x40,
            _ => 0x80,
        };
        cells[cy * cw + cx] |= bit;
    }
    cells
        .chunks(cw)
        .map(|row| {
            row.iter()
                .map(|&bits| char::from_u32(0x2800 + bits as u32).unwrap_or(' '))
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn open_in_browser(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    let mut cmd = {
        let mut c = std::process::Command::new("open");
        c.arg(url);
        c
    };
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", "start", "", url]);
        c
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let mut cmd = {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(url);
        c
    };
    cmd.stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .and_then(|s| {
            if s.success() {
                Ok(())
            } else {
                Err(std::io::Error::other("browser did not open"))
            }
        })
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

    fn ring_cloud(aspect: f32) -> Cloud {
        Cloud {
            version: illustration::VERSION,
            name: "ring".into(),
            description: "A ring".into(),
            prompt: Some("A ring. Sparse particles.".into()),
            generated: None,
            aspect,
            points: Arc::new(
                (0..200)
                    .map(|i| {
                        let a = i as f32 / 200.0 * std::f32::consts::TAU;
                        [0.5 + 0.5 * a.cos(), 0.5 + 0.5 * a.sin()]
                    })
                    .collect(),
            ),
        }
    }

    #[test]
    fn braille_keeps_the_aspect_and_draws_only_the_ring() {
        let art = braille(&ring_cloud(1.0), 40, 20);
        let lines: Vec<&str> = art.lines().collect();
        // 40 cols x 2 = 80 dots wide, square -> 80 dots tall = 20 rows
        assert_eq!(lines.len(), 20);
        assert!(lines.iter().all(|l| l.chars().count() == 40));
        let blank = char::from_u32(0x2800).unwrap();
        // the centre of the ring is empty, the edge is not
        assert_eq!(lines[10].chars().nth(20), Some(blank));
        assert_ne!(lines[0].chars().nth(20), Some(blank));
        // a tall cloud is capped by the row count instead
        let tall = braille(&ring_cloud(3.0), 40, 20);
        let lines: Vec<&str> = tall.lines().collect();
        assert_eq!(lines.len(), 20);
        assert!(lines[0].chars().count() < 40);
    }

    #[test]
    fn issue_body_and_link_carry_the_facts_and_stay_short() {
        let cloud = ring_cloud(1.0);
        let body = issue_body(&cloud, "ring.mdpc.json");
        assert!(body.contains("`ring`"));
        assert!(body.contains("A ring"));
        assert!(body.contains("Sparse particles"));
        assert!(body.contains("ring.mdpc.json"));
        assert!(body.contains("license"));
        let url = issue_url(
            "https://github.com/mklab-se/mdeck",
            "Illustration: ring",
            &body,
        );
        assert!(url.starts_with("https://github.com/mklab-se/mdeck/issues/new?labels=illustration&title=Illustration%3A%20ring&body="));
        assert!(url.len() <= MAX_URL_LEN);
        // a huge prompt is trimmed rather than producing an unusable link
        let mut big = cloud.clone();
        big.prompt = Some("x".repeat(20_000));
        let url = issue_url(
            "https://github.com/mklab-se/mdeck",
            "t",
            &issue_body(&big, "f"),
        );
        assert!(url.len() <= MAX_URL_LEN);
        assert_eq!(percent_encode("a b/c"), "a%20b%2Fc");
    }

    #[test]
    fn prompt_names_the_subject_and_the_look() {
        let p = image_prompt("A server in a rack");
        assert!(p.starts_with("A server in a rack."));
        assert!(p.contains("black background"));
    }
}
