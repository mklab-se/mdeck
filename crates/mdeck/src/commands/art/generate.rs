//! Writing scenes and drawing pictures: the chat request for scenes, the
//! image prompt in a style, the image call with retries, and keeping the
//! result small.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
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
            Block::Paragraph { inlines: v } => {
                out.push_str(&inlines(v));
                out.push('\n');
            }
            Block::BlockQuote { blocks } | Block::Callout { blocks, .. } => {
                for p in Block::quote_paragraphs(blocks) {
                    out.push_str(&inlines(&p));
                    out.push('\n');
                }
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
    if let Some(world) = &pres.meta.art_world {
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

/// What an image model turned out to accept, learned on the first picture
/// and shared by the rest of the run. Models differ: gpt-image takes
/// quality, JPEG output and reference images; MAI Image takes a size only.
pub struct Fit {
    /// Quality, JPEG output and compression (gpt-image only).
    rich: AtomicBool,
    /// Reference images (the style swatches).
    refs: AtomicBool,
}

impl Fit {
    pub fn new() -> Self {
        Self {
            rich: AtomicBool::new(true),
            refs: AtomicBool::new(true),
        }
    }

    /// Whether the style swatches were sent (for the note at the end).
    pub fn took_references(&self) -> bool {
        self.refs.load(Ordering::Relaxed)
    }
}

/// What to give up after a failed request, in order: the gpt-image options
/// first (the picture is re-encoded here anyway), then the reference images.
/// `None`: nothing left to give up, or the error is worth waiting out.
#[derive(Debug, PartialEq, Eq)]
enum Fallback {
    Options,
    References,
}

fn fallback(error: &str, rich: bool, refs: bool) -> Option<Fallback> {
    if transient(error) {
        return None;
    }
    if rich {
        Some(Fallback::Options)
    } else if refs {
        Some(Fallback::References)
    } else {
        None
    }
}

/// The request: always a square picture; the rest only where it is taken.
fn options(rich: bool, references: &[PathBuf]) -> ailloy::ImageOptions {
    let mut b = ailloy::ImageOptions::builder().size(1024, 1024);
    if rich {
        b = b
            .quality("medium")
            .output_format(ailloy::ImageFormat::Jpeg)
            .compression(88);
    }
    if !references.is_empty() {
        b = b.reference_images(references.to_vec());
    }
    b.build()
}

/// Generate one picture, with retries, giving up what the model does not
/// take. Returns compact JPEG bytes.
pub(super) async fn draw_one(
    client: &ailloy::Client,
    prompt_with: &str,
    prompt_without: &str,
    references: &[PathBuf],
    fit: &Fit,
) -> Result<Vec<u8>> {
    let mut last = String::new();
    let mut waits = 0;
    // each give-up is one extra try on top of the waits for throttling
    for _ in 0..TRIES + 2 {
        let rich = fit.rich.load(Ordering::Relaxed);
        let with = fit.refs.load(Ordering::Relaxed) && !references.is_empty();
        let refs: &[PathBuf] = if with { references } else { &[] };
        let prompt = if with { prompt_with } else { prompt_without };
        match client
            .generate_images_with(prompt, &options(rich, refs))
            .await
        {
            Ok(images) => {
                let image = images
                    .into_iter()
                    .next()
                    .context("the model returned no image")?;
                return Ok(compact(image.data));
            }
            Err(e) => {
                last = format!("{e:#}");
                match fallback(&last, rich, with) {
                    Some(Fallback::Options) => fit.rich.store(false, Ordering::Relaxed),
                    Some(Fallback::References) => fit.refs.store(false, Ordering::Relaxed),
                    None if transient(&last) && waits + 1 < TRIES => {
                        waits += 1;
                        tokio::time::sleep(Duration::from_secs(10 * waits as u64)).await;
                    }
                    None => break,
                }
            }
        }
    }
    bail!(
        "{}",
        last.lines().last().unwrap_or("image generation failed")
    )
}

/// The scene list in a chat reply: a JSON object with a `scenes` array, or
/// what is wrong with it.
fn parse_scenes(reply: &str) -> Result<Vec<serde_json::Value>, String> {
    let json = crate::commands::ai_reply::json_object(reply);
    let parsed: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("the answer is not a JSON object ({e})"))?;
    parsed["scenes"]
        .as_array()
        .cloned()
        .ok_or_else(|| "the JSON has no `scenes` array".to_string())
}

/// The follow-up request when the scene list is malformed.
fn fix_scenes_message(error: &str) -> String {
    format!(
        "That answer could not be read: {error}. Reply with only the JSON object \
         {{\"scenes\": [{{\"slide\": <number>, \"scene\": \"...\"}}]}}."
    )
}

/// Scenes for `slides`: a slide's own `picture-prompt`, or written by the chat model.
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
        let messages = vec![
            ailloy::Message::system(SCENE_PROMPT),
            ailloy::Message::user(scene_request(pres, &ask)),
        ];
        // a malformed answer is sent back once with what is wrong (GEN-07)
        let list = crate::commands::ai_reply::chat_client_validated(
            &client,
            messages,
            parse_scenes,
            fix_scenes_message,
            "the chat model's scenes",
        )
        .await?;
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
            "---\ntitle: Harbour\nart-world: a Victorian harbour town that builds software\n---\n# Launch\n\nWe ship today\n\n# Why\n<!-- picture-prompt: a lighthouse keeper with a laptop -->\n\n- one\n\n```@notes\nTell the story of the storm.\n```\n",
        )
    }

    #[test]
    fn a_malformed_scene_list_is_asked_for_again_with_the_error() {
        assert!(parse_scenes("Sure! Here are some ideas.").is_err());
        let e = parse_scenes("{\"slides\": []}").unwrap_err();
        assert!(e.contains("`scenes`"), "{e}");
        let ok =
            parse_scenes("```json\n{\"scenes\": [{\"slide\": 1, \"scene\": \"a ship\"}]}\n```")
                .unwrap();
        assert_eq!(ok[0]["scene"], "a ship");
        assert!(fix_scenes_message(&e).contains("`scenes`"));
        // the loop: a bad answer, then the error, then a good answer
        let mut replies = vec![
            "{\"scenes\": [{\"slide\": 1, \"scene\": \"a lighthouse\"}]}".to_string(),
            "not json".to_string(),
        ];
        let mut seen = Vec::new();
        let list = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(crate::commands::ai_reply::chat_validated(
                async |h: &[ailloy::Message]| {
                    seen.push(h.len());
                    Ok(replies.pop().unwrap())
                },
                vec![ailloy::Message::user("scenes please")],
                parse_scenes,
                fix_scenes_message,
                "scenes",
            ))
            .unwrap();
        assert_eq!(list[0]["scene"], "a lighthouse");
        // the second request carries the bad answer and the fix
        assert_eq!(seen, [1, 3]);
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
    fn a_model_that_refuses_options_gets_a_plain_request() {
        // Issue #18: MAI Image takes a size only. Its refusal gives up the
        // gpt-image options first, then the reference images, then stops.
        let mai = "The following parameters are not supported by MAI image models ('MAI-Image-2.6-Flash'): quality, compression.";
        assert_eq!(fallback(mai, true, true), Some(Fallback::Options));
        assert_eq!(fallback(mai, false, true), Some(Fallback::References));
        assert_eq!(fallback(mai, false, false), None);
        // throttling is waited out, never given up on
        assert_eq!(fallback("HTTP 429 Too Many Requests", true, true), None);
        let plain = options(false, &[]);
        assert!(plain.quality.is_none() && plain.compression.is_none());
        assert!(plain.output_format.is_none() && plain.reference_images.is_empty());
        assert_eq!(plain.size, Some((1024, 1024)));
        let rich = options(true, &[PathBuf::from("a.jpg")]);
        assert_eq!(rich.quality.as_deref(), Some("medium"));
        assert_eq!(rich.reference_images.len(), 1);
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
        // a line medium, as the line engine's (which a build may leave out)
        let medium = art::Medium {
            name: "line",
            kind: art::ArtKind::Line,
            tonal: &art::style::LINE,
            tonal_strategy: art::prepare::Strategy::Hatch,
        };
        let style = Style::for_medium(&medium, &theme);
        let p = image_prompt("a lighthouse keeper", &style, true);
        assert!(p.starts_with("a lighthouse keeper. Match the drawing technique"));
        assert!(p.ends_with(&style.prompt));
        assert!(!image_prompt("x", &style, false).contains("reference"));
        assert!(transient("HTTP 429 Too Many Requests"));
        assert!(!transient("400 invalid parameter: image"));
    }
}
