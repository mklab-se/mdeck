//! A background image behind a slide: the deck's (`background` in the
//! frontmatter) on every slide, or a slide's own `background`, which
//! replaces it there (`none` turns it off). `background-opacity` tones it
//! down at either level. The image covers the slide over the theme's
//! background colour and under the engine layer and the content, so a low
//! opacity blends it toward the theme's own colour. Drawn the same way when
//! presenting and when exporting.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use eframe::egui::{self, Color32, Pos2, Rect};

use crate::parser::{self, Presentation};

/// Quiet enough that text stays readable on any theme.
pub const DEFAULT_OPACITY: f32 = 0.3;
/// How long a background takes to fade in once its image has loaded.
const APPEAR_SECONDS: f32 = 0.4;
/// The image files a background can be.
const EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "svg"];

/// A background to draw.
#[derive(Debug, Clone, PartialEq)]
pub struct Background {
    /// As written, relative to the deck (the image cache resolves it).
    pub path: String,
    pub opacity: f32,
}

/// Something in the deck's background keys that did not work.
#[derive(Debug, Clone, PartialEq)]
pub struct Problem {
    /// 1-based slide, or 0 for the frontmatter.
    pub slide: usize,
    /// Deck file line; 0 for the frontmatter (`--check` finds it).
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.slide {
            0 => write!(f, "{}", self.message),
            n => write!(f, "slide {n}: {}", self.message),
        }
    }
}

/// The background on every slide.
#[derive(Debug, Clone, Default)]
pub struct Backgrounds {
    slides: Vec<Option<Background>>,
}

impl Backgrounds {
    /// The background of slide `idx` (0-based); none past the last slide.
    pub fn get(&self, idx: usize) -> Option<&Background> {
        self.slides.get(idx).and_then(Option::as_ref)
    }

    /// Every image the deck uses, once each.
    pub fn distinct_paths(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        for b in self.slides.iter().flatten() {
            if !out.contains(&b.path.as_str()) {
                out.push(&b.path);
            }
        }
        out
    }
}

/// Resolve the deck's background and each slide's over it. A slide's
/// `background` replaces the deck's image, `none` removes it, and a slide's
/// `background-opacity` applies to whichever image the slide shows. A file
/// that cannot be used falls back to the deck's image; an opacity that does
/// not parse falls back to the deck's or the default.
pub fn resolve(presentation: &Presentation, deck_dir: &Path) -> (Backgrounds, Vec<Problem>) {
    let mut problems = Vec::new();
    let meta = &presentation.meta;
    let deck_opacity = meta
        .background_opacity
        .as_deref()
        .and_then(|o| opacity(o, 0, 0, &mut problems))
        .unwrap_or(DEFAULT_OPACITY);
    let deck = match meta.background.as_deref().map(str::trim) {
        None | Some("") | Some("none") => None,
        Some(file) => match image_file(deck_dir, file) {
            Ok(()) => Some(Background {
                path: file.to_string(),
                opacity: deck_opacity,
            }),
            Err(message) => {
                problems.push(Problem {
                    slide: 0,
                    line: 0,
                    message,
                });
                None
            }
        },
    };
    let slides = presentation
        .slides
        .iter()
        .enumerate()
        .map(|(i, slide)| {
            let value = |name| {
                parser::setting(&slide.settings, name)
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
            };
            let own_opacity = value("background-opacity").and_then(|o| {
                opacity(
                    o,
                    i + 1,
                    slide.setting_line("background-opacity"),
                    &mut problems,
                )
            });
            let path = match value("background") {
                None => deck.as_ref().map(|d| d.path.clone()),
                Some("none") => None,
                Some(file) => match image_file(deck_dir, file) {
                    Ok(()) => Some(file.to_string()),
                    Err(message) => {
                        problems.push(Problem {
                            slide: i + 1,
                            line: slide.setting_line("background"),
                            message,
                        });
                        deck.as_ref().map(|d| d.path.clone())
                    }
                },
            };
            if path.is_none() && own_opacity.is_some() && value("background") != Some("none") {
                problems.push(Problem {
                    slide: i + 1,
                    line: slide.setting_line("background-opacity"),
                    message: "background-opacity has no background image to apply to".into(),
                });
            }
            path.map(|path| Background {
                path,
                opacity: own_opacity.unwrap_or(deck_opacity),
            })
        })
        .collect();
    (Backgrounds { slides }, problems)
}

/// `30%` or `0.3`, or a problem on `slide` (0 for the frontmatter).
fn opacity(value: &str, slide: usize, line: usize, problems: &mut Vec<Problem>) -> Option<f32> {
    let parsed = super::logo::parse_opacity(value);
    if parsed.is_none() {
        problems.push(Problem {
            slide,
            line,
            message: format!("background-opacity: '{value}' must be 0 to 1 (or 0% to 100%)"),
        });
    }
    parsed
}

/// Whether `file` (relative to the deck) is an image a background can show.
fn image_file(deck_dir: &Path, file: &str) -> Result<(), String> {
    let path = deck_dir.join(file);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    if !ext.is_some_and(|e| EXTENSIONS.contains(&e.as_str())) {
        Err(format!(
            "background: '{file}' must be a .png, .jpg, .webp or .svg file"
        ))
    } else if !path.is_file() {
        Err(format!("background: {} was not found", path.display()))
    } else {
        Ok(())
    }
}

/// The part of an image of `image_aspect` (width / height) that covers a
/// rect of `rect_aspect`: centred, cropped on the long side, never stretched.
pub fn cover_uv(image_aspect: f32, rect_aspect: f32) -> Rect {
    if !(image_aspect > 0.0 && rect_aspect > 0.0) {
        return Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
    }
    if image_aspect > rect_aspect {
        // wider than the slide: crop the sides
        let w = rect_aspect / image_aspect;
        Rect::from_min_max(
            Pos2::new((1.0 - w) / 2.0, 0.0),
            Pos2::new((1.0 + w) / 2.0, 1.0),
        )
    } else {
        let h = image_aspect / rect_aspect;
        Rect::from_min_max(
            Pos2::new(0.0, (1.0 - h) / 2.0),
            Pos2::new(1.0, (1.0 + h) / 2.0),
        )
    }
}

/// Draw `texture` covering `rect` at `opacity`, with rounded corners of
/// `radius` (a page theme's sheet).
pub fn draw(
    painter: &egui::Painter,
    rect: Rect,
    texture: &egui::TextureHandle,
    opacity: f32,
    radius: f32,
) {
    let [w, h] = texture.size();
    if w == 0 || h == 0 || rect.height() <= 0.0 || opacity <= 0.0 {
        return;
    }
    let uv = cover_uv(w as f32 / h as f32, rect.width() / rect.height());
    let tint = Color32::from_white_alpha((opacity.clamp(0.0, 1.0) * 255.0).round() as u8);
    painter.add(egui::epaint::RectShape::filled(rect, radius, tint).with_texture(texture.id(), uv));
}

/// When each background image first appeared, so it fades in instead of
/// popping up when its decode finishes.
#[derive(Debug, Default)]
pub struct FadeIn(RefCell<HashMap<String, Instant>>);

impl FadeIn {
    /// How far `path` has faded in at `now`, 0 to 1 (first call: 0).
    pub fn amount(&self, path: &str, now: Instant) -> f32 {
        let start = *self.0.borrow_mut().entry(path.to_string()).or_insert(now);
        let t = (now.saturating_duration_since(start).as_secs_f32() / APPEAR_SECONDS).min(1.0);
        // ease out
        1.0 - (1.0 - t) * (1.0 - t)
    }

    pub fn clear(&self) {
        self.0.borrow_mut().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("mdeck-bg-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        for f in ["deck.jpg", "own.png", "notes.txt"] {
            std::fs::write(d.join(f), b"x").unwrap();
        }
        d
    }

    #[test]
    fn a_slide_replaces_or_removes_the_deck_background() {
        let d = tmp("slides");
        let md = "---\nbackground: deck.jpg\nbackground-opacity: 25%\n---\n\n\
                  # One\n\n- a\n\n\
                  # Two\n<!-- background: own.png -->\n\n- b\n\n\
                  # Three\n<!-- background: none -->\n\n- c\n\n\
                  # Four\n<!-- background-opacity: 0.8 -->\n\n- d\n\n\
                  # Five\n<!-- background: own.png -->\n<!-- background-opacity: 100% -->\n\n- e\n";
        let pres = crate::parser::parse(md);
        let (bgs, problems) = resolve(&pres, &d);
        assert!(problems.is_empty(), "{problems:?}");
        let deck = Background {
            path: "deck.jpg".into(),
            opacity: 0.25,
        };
        assert_eq!(bgs.get(0), Some(&deck));
        assert_eq!(
            bgs.get(1),
            Some(&Background {
                path: "own.png".into(),
                opacity: 0.25
            }),
            "a slide's image keeps the deck's opacity"
        );
        assert_eq!(bgs.get(2), None, "none turns it off");
        assert_eq!(
            bgs.get(3),
            Some(&Background {
                path: "deck.jpg".into(),
                opacity: 0.8
            }),
            "opacity alone reuses the deck's image"
        );
        assert_eq!(bgs.get(4).unwrap().opacity, 1.0);
        assert_eq!(bgs.get(5), None, "nothing past the last slide");
        assert_eq!(bgs.distinct_paths(), ["deck.jpg", "own.png"]);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn a_slide_can_have_a_background_without_a_deck_default() {
        let d = tmp("own");
        let md = "# One\n\n- a\n\n# Two\n<!-- background: own.png -->\n\n- b\n";
        let (bgs, problems) = resolve(&crate::parser::parse(md), &d);
        assert!(problems.is_empty());
        assert_eq!(bgs.get(0), None);
        assert_eq!(
            bgs.get(1),
            Some(&Background {
                path: "own.png".into(),
                opacity: DEFAULT_OPACITY
            })
        );
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn problems_name_their_slide_and_line_and_fall_back() {
        let d = tmp("problems");
        let md = "---\nbackground: deck.jpg\nbackground-opacity: lots\n---\n\n\
                  # One\n<!-- background: gone.png -->\n\n- a\n\n\
                  # Two\n<!-- background: notes.txt -->\n\n- b\n\n\
                  # Three\n<!-- background-opacity: 2 -->\n\n- c\n";
        let pres = crate::parser::parse(md);
        let (bgs, problems) = resolve(&pres, &d);
        let deck = Background {
            path: "deck.jpg".into(),
            opacity: DEFAULT_OPACITY,
        };
        assert_eq!(bgs.get(0), Some(&deck), "a missing file falls back");
        assert_eq!(bgs.get(1), Some(&deck), "a non-image falls back");
        assert_eq!(bgs.get(2), Some(&deck), "a bad opacity falls back");
        let found: Vec<(usize, usize)> = problems.iter().map(|p| (p.slide, p.line)).collect();
        assert_eq!(found, [(0, 0), (1, 7), (2, 12), (3, 17)], "{problems:?}");
        assert!(problems[1].message.contains("gone.png"));
        assert!(problems[2].to_string().starts_with("slide 2: background:"));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn opacity_without_an_image_is_reported() {
        let d = tmp("orphan");
        let md = "# One\n<!-- background-opacity: 50% -->\n\n- a\n";
        let (bgs, problems) = resolve(&crate::parser::parse(md), &d);
        assert_eq!(bgs.get(0), None);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].message.contains("no background image"));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn cover_crops_the_long_side_and_keeps_the_centre() {
        let full = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        assert_eq!(cover_uv(16.0 / 9.0, 16.0 / 9.0), full);
        // a square image on a 2:1 slide shows its middle half, full width
        let uv = cover_uv(1.0, 2.0);
        assert_eq!((uv.min.x, uv.max.x), (0.0, 1.0));
        assert!((uv.min.y - 0.25).abs() < 1e-6 && (uv.max.y - 0.75).abs() < 1e-6);
        // a 4:1 panorama on a 2:1 slide shows its middle half, full height
        let uv = cover_uv(4.0, 2.0);
        assert_eq!((uv.min.y, uv.max.y), (0.0, 1.0));
        assert!((uv.min.x - 0.25).abs() < 1e-6 && (uv.max.x - 0.75).abs() < 1e-6);
        assert_eq!(cover_uv(0.0, 1.0), full);
    }

    #[test]
    fn fade_in_starts_at_zero_and_settles_at_one() {
        let f = FadeIn::default();
        let t0 = Instant::now();
        assert_eq!(f.amount("a.png", t0), 0.0);
        let mid = f.amount("a.png", t0 + std::time::Duration::from_millis(200));
        assert!(mid > 0.5 && mid < 1.0, "{mid}");
        assert_eq!(
            f.amount("a.png", t0 + std::time::Duration::from_secs(1)),
            1.0
        );
        f.clear();
        assert_eq!(
            f.amount("a.png", t0 + std::time::Duration::from_secs(2)),
            0.0
        );
    }
}
