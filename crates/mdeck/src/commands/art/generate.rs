//! Writing scenes and drawing pictures: the chat request for scenes, the
//! image prompt in a style, the image call with retries, and keeping the
//! result small.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};

use crate::parser::{Block, Inline, Presentation, Slide};
use crate::render::art;
use crate::render::art::style::{Reference, Style};

/// Tries per picture (throttling and transient errors back off in between).
const TRIES: u32 = 6;

pub(super) const SCENE_PROMPT: &str = r#"You write scenes for illustrations in a presentation. Each slide gets one picture, drawn beside its text. For every slide you are given, write the scene the picture shows.

Rules:
- One scene, one clear focal subject, simply composed. Never a collage or a grid of vignettes.
- A concrete, visual metaphor for the slide's point, not a literal diagram of it. People, places, objects and machines, doing something.
- Keep the deck's world (setting, era, recurring characters) when one is given, and keep recurring characters looking the same.
- No text anywhere in the picture: no signs, labels, screens with words, letters or numbers.
- One or two sentences, 20 to 45 words, describing only what is seen. Do not name a drawing style or medium; that is added separately.

Answer with a JSON object only: {"scenes": [{"slide": <number>, "scene": "<scene>"}]}"#;

/// Plain text of a slide's copy, for the scene prompt.
pub(super) fn slide_text(slide: &Slide) -> String {
    fn inlines(v: &[Inline]) -> String {
        v.iter()
            .map(|i| match i {
                Inline::Text(s) | Inline::Code(s) => s.clone(),
                Inline::Bold(c) | Inline::Italic(c) | Inline::Strikethrough(c) => inlines(c),
                Inline::Link { text, .. } => inlines(text),
                Inline::Math { tex, .. } => tex.clone(),
            })
            .collect()
    }
    let mut out = String::new();
    for b in &slide.blocks {
        match b {
            Block::Heading { inlines: v, .. } => {
                out.push_str(&format!("# {}\n", inlines(v)));
            }
            Block::Paragraph { inlines: v } | Block::BlockQuote { inlines: v } => {
                out.push_str(&inlines(v));
                out.push('\n');
            }
            Block::List { items, .. } => {
                for it in items {
                    out.push_str(&format!("- {}\n", inlines(&it.inlines)));
                }
            }
            _ => {}
        }
    }
    out.trim().to_string()
}

/// The chat request for the scenes of `slides` (0-based).
pub fn scene_request(pres: &Presentation, slides: &[usize]) -> String {
    let mut msg = String::new();
    if let Some(t) = &pres.meta.title {
        msg.push_str(&format!("Deck: {t}\n"));
    }
    if let Some(world) = &pres.meta.art {
        msg.push_str(&format!("The deck's world: {world}\n"));
    }
    msg.push('\n');
    for &i in slides {
        let slide = &pres.slides[i];
        msg.push_str(&format!("Slide {}:\n{}\n", i + 1, slide_text(slide)));
        if let Some(notes) = slide
            .notes
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty())
        {
            msg.push_str(&format!("Speaker notes: {notes}\n"));
        }
        msg.push('\n');
    }
    msg.push_str("Write the JSON now.");
    msg
}

/// The full image prompt for a scene in a style.
pub fn image_prompt(scene: &str, style: &Style, with_references: bool) -> String {
    let technique = if with_references {
        " Match the drawing technique of the reference images exactly, but draw a completely new subject: do not copy their objects or composition."
    } else {
        ""
    };
    format!(
        "{}{technique} {}",
        scene.trim().trim_end_matches('.').to_string() + ".",
        style.prompt
    )
}

/// Reference images as files the image client can send.
pub(super) fn reference_files(style: &Style) -> Result<Vec<PathBuf>> {
    let dir = std::env::temp_dir().join("mdeck-art-references");
    std::fs::create_dir_all(&dir)?;
    style
        .references
        .iter()
        .map(|r| match r {
            Reference::File(p) => Ok(p.clone()),
            Reference::Bundled { name, bytes } => {
                let p = dir.join(name);
                if !p.exists() {
                    std::fs::write(&p, bytes)?;
                }
                Ok(p)
            }
        })
        .collect()
}

/// A picture's file name: the deck, the slide, the style and a stamp, so
/// regenerating never overwrites a picture another entry points at.
pub(super) fn file_name(deck: &Path, index: usize, style: &Style) -> String {
    let stem = deck
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "deck".into());
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() % 1_000_000)
        .unwrap_or(0);
    format!("{stem}-{:02}-{}-{stamp:06}.jpg", index + 1, style.name)
}

/// Keep a picture small next to the deck: at most 1024 px, JPEG quality 82
/// (about 200 KB for detailed line art). What cannot be decoded is kept as it came.
pub(super) fn compact(bytes: Vec<u8>) -> Vec<u8> {
    let Ok(img) = image::load_from_memory(&bytes) else {
        return bytes;
    };
    let img = if img.width().max(img.height()) > 1024 {
        img.resize(1024, 1024, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };
    let mut out = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 82);
    match img.to_rgb8().write_with_encoder(encoder) {
        Ok(()) if out.len() < bytes.len() => out,
        _ => bytes,
    }
}

/// Is this error worth waiting out (throttling, timeouts, a busy service)?
pub(super) fn transient(e: &str) -> bool {
    let e = e.to_ascii_lowercase();
    [
        "429",
        "rate",
        "throttl",
        "timeout",
        "timed out",
        "503",
        "502",
        "500",
        "overloaded",
        "temporarily",
    ]
    .iter()
    .any(|k| e.contains(k))
}

/// Generate one picture with retries. Returns the JPEG bytes.
pub(super) async fn draw_one(
    client: &ailloy::Client,
    prompt_with: &str,
    prompt_without: &str,
    references: &[PathBuf],
    use_refs: &std::sync::atomic::AtomicBool,
) -> Result<Vec<u8>> {
    use std::sync::atomic::Ordering;
    let mut last = String::new();
    for attempt in 0..TRIES {
        let with = use_refs.load(Ordering::Relaxed) && !references.is_empty();
        let mut options = ailloy::ImageOptions::builder()
            .size(1024, 1024)
            .quality("medium")
            .output_format(ailloy::ImageFormat::Jpeg)
            .compression(88);
        if with {
            options = options.reference_images(references.to_vec());
        }
        let options = options.build();
        let prompt = if with { prompt_with } else { prompt_without };
        match client.generate_images_with(prompt, &options).await {
            Ok(images) => {
                let image = images
                    .into_iter()
                    .next()
                    .context("the model returned no image")?;
                return Ok(compact(image.data));
            }
            Err(e) => {
                last = format!("{e:#}");
                if with && !transient(&last) {
                    // this model does not take reference images: go on without
                    use_refs.store(false, Ordering::Relaxed);
                    continue;
                }
                if !transient(&last) || attempt + 1 == TRIES {
                    break;
                }
                tokio::time::sleep(Duration::from_secs(10 * (attempt as u64 + 1))).await;
            }
        }
    }
    bail!(
        "{}",
        last.lines().last().unwrap_or("image generation failed")
    )
}

/// Scenes for `slides`: a slide's own `@art:`, or written by the chat model.
pub(super) async fn scenes(
    pres: &Presentation,
    slides: &[usize],
    quiet: bool,
) -> Result<Vec<(usize, String)>> {
    let mut out: Vec<(usize, String)> = Vec::new();
    let mut ask = Vec::new();
    for &i in slides {
        match art::slide_scene(&pres.slides[i]) {
            Some(s) => out.push((i, s.to_string())),
            None => ask.push(i),
        }
    }
    if !ask.is_empty() {
        if !quiet {
            eprintln!(
                "Writing {} scene{} from the slides…",
                ask.len(),
                if ask.len() == 1 { "" } else { "s" }
            );
        }
        let client = ailloy::Client::for_capability("chat")?;
        let messages = [
            ailloy::Message::system(SCENE_PROMPT),
            ailloy::Message::user(scene_request(pres, &ask)),
        ];
        let response = client.chat(&messages).await.context("AI request failed")?;
        let json = crate::commands::story::extract_json(&response.content);
        let parsed: serde_json::Value =
            serde_json::from_str(json).context("the chat model did not answer with JSON")?;
        let list = parsed["scenes"]
            .as_array()
            .context("no scenes in the answer")?;
        for &i in &ask {
            let scene = list
                .iter()
                .find(|s| s["slide"].as_u64() == Some(i as u64 + 1))
                .and_then(|s| s["scene"].as_str())
                .map(str::to_string)
                .unwrap_or_else(|| {
                    // no scene came back: draw the slide's title
                    pres.slides[i]
                        .title()
                        .unwrap_or_else(|| "a quiet workshop".into())
                });
            out.push((i, scene));
        }
    }
    out.sort_by_key(|(i, _)| *i);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;

    fn deck() -> Presentation {
        crate::parser::parse(
            "---\ntitle: Harbour\n@art: a Victorian harbour town that builds software\n---\n# Launch\n\nWe ship today\n\n# Why\n@art: a lighthouse keeper with a laptop\n\n- one\n\n???\nTell the story of the storm.\n",
            Path::new("."),
        )
    }

    #[test]
    fn the_scene_request_carries_world_copy_and_notes() {
        let pres = deck();
        let r = scene_request(&pres, &[0, 1]);
        assert!(r.contains("Deck: Harbour"));
        assert!(r.contains("Victorian harbour"));
        assert!(r.contains("Slide 1:\n# Launch\nWe ship today"));
        assert!(r.contains("Speaker notes: Tell the story of the storm."));
        assert!(r.trim_end().ends_with("Write the JSON now."));
    }

    #[test]
    fn pictures_are_kept_small() {
        // noise, so the PNG is large (a smooth gradient compresses better
        // as PNG than any JPEG, and is then rightly kept as it came)
        let img = image::RgbImage::from_fn(1500, 900, |x, y| {
            let v = (crate::engines::hash01(x * 7919 + y * 104_729) * 255.0) as u8;
            image::Rgb([v, v / 2, 128])
        });
        let mut png = Vec::new();
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let small = compact(png.clone());
        let back = image::load_from_memory(&small).unwrap();
        assert_eq!(back.width(), 1024);
        assert!(small.len() < png.len());
        assert_eq!(compact(b"not an image".to_vec()), b"not an image");
    }

    #[test]
    fn the_image_prompt_is_scene_then_style() {
        let theme = Theme::light();
        let style = Style::for_medium(&crate::engines::blueprint::MEDIUM, &theme);
        let p = image_prompt("a lighthouse keeper", &style, true);
        assert!(p.starts_with("a lighthouse keeper. Match the drawing technique"));
        assert!(p.ends_with(&style.prompt));
        assert!(!image_prompt("x", &style, false).contains("reference"));
        assert!(transient("HTTP 429 Too Many Requests"));
        assert!(!transient("400 invalid parameter: image"));
    }
}
