//! `mdeck ai generate <file>`: scan a presentation for AI image markers and generate them.

mod names;
mod scan;
mod style;

use std::path::{Path, PathBuf};

use anyhow::Result;
use colored::Colorize;

use crate::commands::ai;
use crate::config::Config;
use crate::parser;
use crate::prompt;

use scan::{IconMarker, ImageMarker};

pub const DEFAULT_IMAGE_STYLE: &str = prompt::DEFAULT_IMAGE_STYLE;
pub const DEFAULT_ICON_STYLE: &str = prompt::DEFAULT_ICON_STYLE;

/// Images generated at the same time.
const MAX_CONCURRENT: usize = 4;

/// One picture to generate: a marker's prompt, combined with its style.
struct Task {
    /// The prompt text used (the marker's own, without the style).
    prompt_text: String,
    /// The full prompt sent to the image model.
    combined: String,
    /// 0-indexed line number in the raw file.
    line_index: usize,
    /// The original line text.
    old_line: String,
    /// Whether this is an icon (vs. a regular image).
    is_icon: bool,
}

/// Where generated pictures go and how progress is reported.
struct Output {
    images_dir: PathBuf,
    icons_dir: PathBuf,
    has_chat: bool,
    quiet: bool,
}

pub async fn run(
    file: PathBuf,
    force: bool,
    style_override: Option<String>,
    quiet: bool,
) -> Result<()> {
    if !ai::has_capability("image") {
        anyhow::bail!(
            "Image generation not configured. Run `mdeck ai config` to set up an image provider."
        );
    }

    let content = std::fs::read_to_string(&file)?;
    let lines: Vec<&str> = content.lines().collect();
    let base_path = file.parent().unwrap_or(Path::new("."));

    // Parse the presentation for layout info
    let presentation = parser::parse(&content, base_path);

    let config = Config::load_or_default();
    let styles = style::resolve_styles(&config, &presentation.meta, style_override.as_deref());

    let (mut image_markers, icon_markers) = scan::scan_markers(&lines, &presentation)?;

    if image_markers.is_empty() && icon_markers.is_empty() {
        if !quiet {
            println!("No image-generation markers found in {}.", file.display());
        }
        return Ok(());
    }

    let has_chat = ai::has_capability("chat");
    fill_empty_prompts(&mut image_markers, &presentation, has_chat).await?;

    if !force && !confirm_plan(&image_markers, &icon_markers)? {
        println!("Aborted.");
        return Ok(());
    }

    let output = Output {
        images_dir: base_path.join("images"),
        icons_dir: base_path.join("media").join("diagram-icons"),
        has_chat,
        quiet,
    };
    if !image_markers.is_empty() {
        std::fs::create_dir_all(&output.images_dir)?;
    }
    if !icon_markers.is_empty() {
        std::fs::create_dir_all(&output.icons_dir)?;
    }

    let client = ailloy::Client::for_capability("image")?;
    let tasks = build_tasks(&lines, &image_markers, &icon_markers, &styles);
    let total = tasks.len();
    if !quiet {
        println!("  Generating {} image(s)...\n", total,);
    }

    let replacements = generate_all(&client, tasks, &output).await?;
    let success_count = replacements.len();

    if success_count > 0 {
        std::fs::write(&file, scan::rewrite_lines(&content, &replacements))?;
    }

    if !quiet {
        print_summary(success_count, total);
        if success_count > 0 {
            println!("Updated: {}", file.display());
        }
    }

    Ok(())
}

/// Ask the chat model for a prompt wherever an image marker has no alt text.
async fn fill_empty_prompts(
    image_markers: &mut [ImageMarker],
    presentation: &parser::Presentation,
    has_chat: bool,
) -> Result<()> {
    for marker in image_markers {
        if marker.alt_text.is_empty() {
            if !has_chat {
                anyhow::bail!(
                    "Slide {} has an image with no prompt (empty alt text). \
                     Chat capability is required for auto-prompting. \
                     Either add alt text or configure a chat provider.",
                    marker.slide_number
                );
            }
            marker.alt_text = names::auto_prompt(presentation, marker.slide_number).await?;
        }
    }
    Ok(())
}

/// List what will be generated and ask to go ahead.
fn confirm_plan(image_markers: &[ImageMarker], icon_markers: &[IconMarker]) -> Result<bool> {
    println!(
        "Found {} image(s) and {} icon(s) to generate:",
        image_markers.len(),
        icon_markers.len()
    );
    for m in image_markers {
        let prompt_preview = truncate(&m.alt_text, 60);
        println!(
            "  Slide {}: \"{}\" ({:?})",
            m.slide_number, prompt_preview, m.orientation
        );
    }
    for m in icon_markers {
        let prompt_preview = truncate(&m.prompt_text, 60);
        println!(
            "  Slide {}: icon \"{}\" (Square)",
            m.slide_number, prompt_preview
        );
    }
    println!();

    Ok(inquire::Confirm::new("Generate these images?")
        .with_default(true)
        .prompt()?)
}

/// Images first, then icons, each with its full prompt.
fn build_tasks(
    lines: &[&str],
    image_markers: &[ImageMarker],
    icon_markers: &[IconMarker],
    styles: &style::Styles,
) -> Vec<Task> {
    let images = image_markers.iter().map(|marker| Task {
        prompt_text: marker.alt_text.clone(),
        combined: prompt::build_image_prompt(&styles.image, &marker.alt_text, marker.orientation),
        line_index: marker.line_index,
        old_line: lines[marker.line_index].to_string(),
        is_icon: false,
    });
    let icons = icon_markers.iter().map(|marker| Task {
        prompt_text: marker.prompt_text.clone(),
        combined: prompt::build_icon_prompt(&styles.icon, &marker.prompt_text),
        line_index: marker.line_index,
        old_line: lines[marker.line_index].to_string(),
        is_icon: true,
    });
    images.chain(icons).collect()
}

/// Run the tasks concurrently, save each picture as it arrives and return
/// the rewritten marker lines of the ones that succeeded.
async fn generate_all(
    client: &ailloy::Client,
    tasks: Vec<Task>,
    output: &Output,
) -> Result<Vec<(usize, String)>> {
    use futures::stream::StreamExt;
    use std::io::Write;

    let total = tasks.len();
    let mut buffered = futures::stream::iter(tasks.into_iter().map(|task| async move {
        let result = client.generate_image(&task.combined).await;
        (task, result)
    }))
    .buffer_unordered(MAX_CONCURRENT);

    let mut replacements = Vec::new();
    let mut completed = 0usize;
    while let Some((task, result)) = buffered.next().await {
        completed += 1;
        let kind = if task.is_icon { "icon " } else { "" };
        let prompt_preview = truncate(&task.prompt_text, 50);

        match result {
            Ok(response) => {
                let (filepath, new_line) = save_picture(&task, &response, output).await?;
                if !output.quiet {
                    println!(
                        "  [{}/{}] {}{} {}",
                        completed,
                        total,
                        kind,
                        prompt_preview,
                        "✓".green().bold()
                    );
                    ai::display_image_result(&filepath);
                }
                replacements.push((task.line_index, new_line));
            }
            Err(e) => {
                println!(
                    "  [{}/{}] {}{} {}",
                    completed,
                    total,
                    kind,
                    prompt_preview,
                    "✗".red().bold()
                );
                eprintln!("    Error: {e}");
            }
        }
        let _ = std::io::stdout().flush();
    }
    Ok(replacements)
}

/// Write a generated picture under a fresh name. Returns its path and the
/// marker line rewritten to point at it.
async fn save_picture(
    task: &Task,
    response: &ailloy::ImageResponse,
    output: &Output,
) -> Result<(PathBuf, String)> {
    let ext = ai::image_ext(&response.format);
    let (dir, prefix) = if task.is_icon {
        (&output.icons_dir, "")
    } else {
        (&output.images_dir, "images/")
    };
    let filename = names::generate_filename(output.has_chat, &task.prompt_text, ext, dir).await;
    let filepath = dir.join(&filename);
    std::fs::write(&filepath, &response.data)?;

    let new_line = if task.is_icon {
        let icon_name = filename
            .strip_suffix(&format!(".{ext}"))
            .unwrap_or(&filename);
        scan::replace_icon_marker(&task.old_line, icon_name)
    } else {
        scan::replace_image_marker(&task.old_line, &format!("{prefix}{filename}"))
    };
    Ok((filepath, new_line))
}

fn print_summary(success_count: usize, total: usize) {
    println!();
    if success_count == total {
        println!(
            "{} Generated {}/{} images successfully.",
            "✓".green().bold(),
            success_count,
            total
        );
    } else {
        println!(
            "{} Generated {}/{} images ({} failed).",
            "!".yellow().bold(),
            success_count,
            total,
            total - success_count
        );
    }
}

fn truncate(s: &str, max: usize) -> String {
    crate::commands::util::truncate_chars(s, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("a very long string here", 10), "a very ...");
    }

    #[test]
    fn test_truncate_non_ascii_does_not_panic() {
        // Regression: byte slicing panicked on multi-byte chars
        assert_eq!(truncate("Räksmörgås på sommaren", 10), "Räksmör...");
        assert_eq!(
            truncate("Plan \u{2014} build \u{2014} ship 🚀", 8),
            "Plan ..."
        );
        assert_eq!(truncate("🚀🚀🚀🚀", 4), "🚀🚀🚀🚀");
    }

    #[test]
    fn build_tasks_puts_images_before_icons() {
        let lines = [
            "![a cat](image-generation)",
            "- Db (icon: generate-image, prompt: \"db\")",
        ];
        let images = [ImageMarker {
            line_index: 0,
            alt_text: "a cat".into(),
            slide_number: 1,
            orientation: prompt::Orientation::Horizontal,
        }];
        let icons = [IconMarker {
            line_index: 1,
            prompt_text: "db".into(),
            slide_number: 1,
        }];
        let styles = style::Styles {
            image: "S".into(),
            icon: "I".into(),
        };
        let tasks = build_tasks(&lines, &images, &icons, &styles);
        assert_eq!(tasks.len(), 2);
        assert!(!tasks[0].is_icon && tasks[1].is_icon);
        assert_eq!(
            tasks[0].combined,
            prompt::build_image_prompt("S", "a cat", prompt::Orientation::Horizontal)
        );
        assert_eq!(tasks[1].combined, prompt::build_icon_prompt("I", "db"));
        assert_eq!(tasks[1].old_line, lines[1]);
    }
}
