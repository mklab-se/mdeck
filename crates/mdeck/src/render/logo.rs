//! A logo in a corner of every slide: from the theme (`logo:`), from the
//! deck (`logo:`), or both (the deck wins). A slide's own `logo` hides it
//! there (`none`) or shows another file. PNG and SVG; drawn the same way
//! when presenting and when exporting.

use std::path::{Path, PathBuf};

use anyhow::{Context, anyhow};
use eframe::egui::{self, Color32, Pos2, Rect, Vec2};

use crate::parser::{Presentation, PresentationMeta};
use crate::theme::Theme;

/// Which corner the logo sits in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Corner {
    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "top-left" => Some(Corner::TopLeft),
            "top-right" => Some(Corner::TopRight),
            "bottom-left" => Some(Corner::BottomLeft),
            "bottom-right" => Some(Corner::BottomRight),
            _ => None,
        }
    }
}

/// Defaults: quiet, in the corner no chrome uses.
pub const DEFAULT_CORNER: Corner = Corner::TopRight;
/// Height in px on a 1920x1080 slide.
pub const DEFAULT_HEIGHT: f32 = 56.0;
pub const DEFAULT_OPACITY: f32 = 0.6;
/// Distance from the slide edges, px on a 1920x1080 slide.
const MARGIN: f32 = 44.0;
/// SVGs are rasterised this tall and scaled down, sharp up to 4K exports.
const RASTER_HEIGHT: u32 = 512;

/// A logo to draw.
#[derive(Debug, Clone, PartialEq)]
pub struct Logo {
    pub path: PathBuf,
    pub corner: Corner,
    pub height: f32,
    pub opacity: f32,
}

/// `0.6`, `60%` → 0.6. Values outside 0..=1 are rejected.
pub fn parse_opacity(s: &str) -> Option<f32> {
    let t = s.trim();
    let v = match t.strip_suffix('%') {
        Some(p) => p.trim().parse::<f32>().ok()? / 100.0,
        None => t.parse::<f32>().ok()?,
    };
    (0.0..=1.0).contains(&v).then_some(v)
}

/// A logo height: px on a 1920x1080 slide, 8 to 400.
pub fn valid_height(h: f32) -> bool {
    (8.0..=400.0).contains(&h)
}

fn is_logo_file(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .as_deref(),
        Some("png" | "svg")
    )
}

/// The logo a deck shows: `logo` (relative to the deck) over the theme's,
/// with `logo-position`, `logo-opacity` and `logo-height` over the
/// theme's settings. `logo: none` hides the theme's logo. Returns the logo
/// and any problems (a bad value falls back to the default).
pub fn resolve(
    theme: &Theme,
    meta: &PresentationMeta,
    deck_dir: &Path,
) -> (Option<Logo>, Vec<String>) {
    let mut problems = Vec::new();
    let mut style_problems = Vec::new();
    let style = style(theme, meta, &mut style_problems);
    let logo = match meta.logo.as_deref().map(str::trim) {
        Some("none") | Some("") => None,
        Some(file) => match logo_file(deck_dir, file) {
            Ok(path) => Some(Logo { path, ..style }),
            Err(p) => {
                problems.push(p.to_string());
                theme.logo.clone().map(|t| Logo {
                    path: t.path,
                    ..style
                })
            }
        },
        None => theme.logo.clone().map(|t| Logo {
            path: t.path,
            ..style
        }),
    };
    problems.extend(style_problems);
    (logo, problems)
}

/// The logo on every slide: the deck's, unless a slide's own `logo` hides it
/// (`none`) or shows another file, placed like the deck's logo.
#[derive(Debug, Clone, Default)]
pub struct Logos {
    slides: Vec<Option<Logo>>,
}

impl Logos {
    /// The logo on slide `idx` (0-based); none past the last slide.
    pub fn get(&self, idx: usize) -> Option<&Logo> {
        self.slides.get(idx).and_then(Option::as_ref)
    }

    /// Every distinct logo in the deck, for checking that the files load.
    pub fn distinct(&self) -> Vec<&Logo> {
        let mut out: Vec<&Logo> = Vec::new();
        for logo in self.slides.iter().flatten() {
            if !out.iter().any(|l| l.path == logo.path) {
                out.push(logo);
            }
        }
        out
    }
}

/// Resolve the deck's logo ([`resolve`]) and each slide's `logo` over it.
/// Problems name the slide they come from.
pub fn resolve_slides(
    theme: &Theme,
    presentation: &Presentation,
    deck_dir: &Path,
) -> (Logos, Vec<String>) {
    let (deck, mut problems) = resolve(theme, &presentation.meta, deck_dir);
    let style = style(theme, &presentation.meta, &mut Vec::new());
    let slides = presentation
        .slides
        .iter()
        .enumerate()
        .map(|(i, slide)| match slide.logo.as_deref() {
            None => deck.clone(),
            Some("none") => None,
            Some(file) => match logo_file(deck_dir, file) {
                Ok(path) => Some(Logo {
                    path,
                    ..deck.clone().unwrap_or_else(|| style.clone())
                }),
                Err(p) => {
                    problems.push(format!("slide {}: {p}", i + 1));
                    deck.clone()
                }
            },
        })
        .collect();
    (Logos { slides }, problems)
}

/// A logo file relative to the deck, or why it cannot be used.
fn logo_file(deck_dir: &Path, file: &str) -> anyhow::Result<PathBuf> {
    let path = deck_dir.join(file);
    if !is_logo_file(&path) {
        Err(anyhow!("logo: '{file}' must be a .png or .svg file"))
    } else if !path.is_file() {
        Err(anyhow!("logo: {} was not found", path.display()))
    } else {
        Ok(path)
    }
}

/// Where and how a logo is drawn: the theme's settings (or the defaults) with
/// the deck's `logo-position`, `logo-opacity` and `logo-height` over them.
/// The path is left empty.
fn style(theme: &Theme, meta: &PresentationMeta, problems: &mut Vec<String>) -> Logo {
    let mut logo = match &theme.logo {
        Some(t) => Logo {
            path: PathBuf::new(),
            ..t.clone()
        },
        None => Logo {
            path: PathBuf::new(),
            corner: DEFAULT_CORNER,
            height: DEFAULT_HEIGHT,
            opacity: DEFAULT_OPACITY,
        },
    };
    if let Some(p) = &meta.logo_position {
        match Corner::from_name(p) {
            Some(c) => logo.corner = c,
            None => problems.push(format!(
                "logo-position: '{p}' is not top-left, top-right, bottom-left or bottom-right"
            )),
        }
    }
    if let Some(o) = &meta.logo_opacity {
        match parse_opacity(o) {
            Some(v) => logo.opacity = v,
            None => problems.push(format!(
                "logo-opacity: '{o}' must be 0 to 1 (or 0% to 100%)"
            )),
        }
    }
    if let Some(h) = &meta.logo_height {
        match h.trim().trim_end_matches("px").parse::<f32>() {
            Ok(v) if valid_height(v) => logo.height = v,
            _ => problems.push(format!(
                "logo-height: '{h}' must be 8 to 400 (px at 1920x1080)"
            )),
        }
    }
    logo
}

/// Decode a PNG or rasterise an SVG.
pub fn load_image(path: &Path) -> anyhow::Result<egui::ColorImage> {
    let bytes = std::fs::read(path).map_err(|e| anyhow!("{}: {e}", path.display()))?;
    let svg = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("svg"));
    if svg {
        rasterize_svg(&bytes, SvgSize::Height(RASTER_HEIGHT))
            .map_err(|e| anyhow!("{}: {e}", path.display()))
    } else {
        let img = image::load_from_memory(&bytes)
            .map_err(|e| anyhow!("{}: {e}", path.display()))?
            .to_rgba8();
        let size = [img.width() as usize, img.height() as usize];
        Ok(egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()))
    }
}

/// How large to rasterise an SVG.
#[derive(Debug, Clone, Copy)]
pub enum SvgSize {
    /// This many pixels tall (a logo).
    Height(u32),
    /// This many pixels on the longest side (a picture).
    Longest(u32),
}

/// Rasterise an SVG at `size`, keeping its aspect.
pub fn rasterize_svg(bytes: &[u8], size: SvgSize) -> anyhow::Result<egui::ColorImage> {
    use resvg::{tiny_skia, usvg};
    let tree = usvg::Tree::from_data(bytes, &usvg::Options::default())
        .map_err(|e| anyhow!("not a readable SVG ({e})"))?;
    let natural = tree.size();
    if natural.width() <= 0.0 || natural.height() <= 0.0 {
        anyhow::bail!("the SVG has no size");
    }
    let scale = match size {
        SvgSize::Height(h) => h as f32 / natural.height(),
        SvgSize::Longest(l) => l as f32 / natural.width().max(natural.height()),
    };
    let w = ((natural.width() * scale).round() as u32).clamp(1, 4096);
    let h = ((natural.height() * scale).round() as u32).clamp(1, 4096);
    let mut pixmap = tiny_skia::Pixmap::new(w, h).context("the SVG is too large")?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    // tiny-skia stores premultiplied RGBA, which is what egui wants.
    Ok(egui::ColorImage::from_rgba_premultiplied(
        [w as usize, h as usize],
        pixmap.data(),
    ))
}

/// A loaded logo texture, or `None` when the file could not be loaded
/// (`--check` and the theme report why; drawing just leaves it out).
#[derive(Clone)]
struct Loaded(Option<egui::TextureHandle>);

/// The texture for `path`, loaded once per context.
fn texture(ctx: &egui::Context, path: &Path) -> Option<egui::TextureHandle> {
    let id = egui::Id::new(("mdeck-logo", path));
    if let Some(Loaded(t)) = ctx.data(|d| d.get_temp::<Loaded>(id)) {
        return t;
    }
    let loaded = load_image(path).ok().map(|img| {
        ctx.load_texture(
            format!("logo:{}", path.display()),
            img,
            egui::TextureOptions::LINEAR,
        )
    });
    ctx.data_mut(|d| d.insert_temp(id, Loaded(loaded.clone())));
    loaded
}

/// Where a logo of `aspect` (width / height) goes on `slide`.
pub fn placement(slide: Rect, logo: &Logo, aspect: f32, scale: f32) -> Rect {
    let h = logo.height * scale;
    let size = Vec2::new(h * aspect, h);
    let m = MARGIN * scale;
    let min = match logo.corner {
        Corner::TopLeft => Pos2::new(slide.left() + m, slide.top() + m),
        Corner::TopRight => Pos2::new(slide.right() - m - size.x, slide.top() + m),
        Corner::BottomLeft => Pos2::new(slide.left() + m, slide.bottom() - m - size.y),
        Corner::BottomRight => Pos2::new(slide.right() - m - size.x, slide.bottom() - m - size.y),
    };
    Rect::from_min_size(min, size)
}

/// Draw `logo` on `slide`. `fade` multiplies its opacity (transitions).
pub fn draw(painter: &egui::Painter, slide: Rect, logo: &Logo, scale: f32, fade: f32) {
    let Some(tex) = texture(painter.ctx(), &logo.path) else {
        return;
    };
    let [w, h] = tex.size();
    if h == 0 {
        return;
    }
    let rect = placement(slide, logo, w as f32 / h as f32, scale);
    let tint = Color32::from_white_alpha(((logo.opacity * fade).clamp(0.0, 1.0) * 255.0) as u8);
    painter.image(
        tex.id(),
        rect,
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        tint,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    const SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100" width="200" height="100"><rect x="0" y="0" width="100" height="100" fill="#ff0000"/></svg>"##;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("mdeck-logo-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn svg_is_rasterised_at_its_aspect_with_transparency() {
        let img = rasterize_svg(SVG.as_bytes(), SvgSize::Height(RASTER_HEIGHT)).unwrap();
        assert_eq!(img.size, [1024, 512]);
        let big = rasterize_svg(SVG.as_bytes(), SvgSize::Longest(3000)).unwrap();
        assert_eq!(big.size, [3000, 1500]);
        // left half red, right half transparent
        assert_eq!(img.pixels[512 * 1024 / 2 + 10].a(), 255);
        assert_eq!(img.pixels[512 * 1024 / 2 + 1000].a(), 0);
        assert!(rasterize_svg(b"<nope", SvgSize::Height(RASTER_HEIGHT)).is_err());
    }

    #[test]
    fn png_keeps_its_alpha() {
        let d = tmp("png");
        let p = d.join("l.png");
        let mut img = image::RgbaImage::new(4, 2);
        img.put_pixel(0, 0, image::Rgba([255, 0, 0, 255]));
        img.save(&p).unwrap();
        let c = load_image(&p).unwrap();
        assert_eq!(c.size, [4, 2]);
        assert_eq!(c.pixels[0].a(), 255);
        assert_eq!(c.pixels[1].a(), 0);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn placement_hugs_the_chosen_corner() {
        let slide = Rect::from_min_size(Pos2::ZERO, Vec2::new(1920.0, 1080.0));
        let mut logo = Logo {
            path: PathBuf::new(),
            corner: Corner::TopRight,
            height: 50.0,
            opacity: 1.0,
        };
        let r = placement(slide, &logo, 2.0, 1.0);
        assert_eq!(
            r,
            Rect::from_min_size(
                Pos2::new(1920.0 - 44.0 - 100.0, 44.0),
                Vec2::new(100.0, 50.0)
            )
        );
        logo.corner = Corner::BottomLeft;
        let r = placement(slide, &logo, 2.0, 0.5);
        assert_eq!(r.min, Pos2::new(22.0, 1080.0 - 22.0 - 25.0));
    }

    #[test]
    fn opacity_accepts_fractions_and_percentages() {
        assert_eq!(parse_opacity("0.4"), Some(0.4));
        assert_eq!(parse_opacity("40%"), Some(0.4));
        assert_eq!(parse_opacity("1.5"), None);
        assert_eq!(parse_opacity("lots"), None);
    }

    #[test]
    fn the_deck_logo_overrides_the_theme_and_none_hides_it() {
        let d = tmp("resolve");
        std::fs::write(d.join("deck.svg"), SVG).unwrap();
        let mut theme = Theme::light();
        theme.logo = Some(Logo {
            path: d.join("theme.svg"),
            corner: Corner::BottomLeft,
            height: 40.0,
            opacity: 0.3,
        });
        let mut meta = PresentationMeta::default();
        // theme only
        let (l, p) = resolve(&theme, &meta, &d);
        assert_eq!(l.as_ref().unwrap().corner, Corner::BottomLeft);
        assert!(p.is_empty());
        // deck file keeps the theme's placement; deck keys override it
        meta.logo = Some("deck.svg".into());
        meta.logo_opacity = Some("80%".into());
        let (l, _) = resolve(&theme, &meta, &d);
        let l = l.unwrap();
        assert_eq!(l.path, d.join("deck.svg"));
        assert_eq!(
            (l.corner, l.height, l.opacity),
            (Corner::BottomLeft, 40.0, 0.8)
        );
        // none hides it
        meta.logo = Some("none".into());
        assert!(resolve(&theme, &meta, &d).0.is_none());
        // no theme logo: defaults
        let plain = Theme::light();
        meta.logo = Some("deck.svg".into());
        meta.logo_opacity = None;
        let l = resolve(&plain, &meta, &d).0.unwrap();
        assert_eq!(
            (l.corner, l.height, l.opacity),
            (DEFAULT_CORNER, DEFAULT_HEIGHT, DEFAULT_OPACITY)
        );
        // problems are reported and fall back
        meta.logo = Some("missing.svg".into());
        meta.logo_position = Some("middle".into());
        let (l, p) = resolve(&plain, &meta, &d);
        assert!(l.is_none());
        assert!(p[0].contains("missing.svg"));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn a_slide_logo_hides_or_replaces_the_deck_logo() {
        let d = tmp("slides");
        std::fs::write(d.join("deck.svg"), SVG).unwrap();
        std::fs::write(d.join("partner.svg"), SVG).unwrap();
        let md = "---\nlogo: deck.svg\nlogo-position: bottom-left\n---\n\n\
                  # One\n\n- a\n\n# Two\n<!-- logo: none -->\n\n- b\n\n\
                  # Three\n<!-- logo: partner.svg -->\n\n- c\n\n# Four\n<!-- logo: gone.svg -->\n\n- d\n";
        let pres = crate::parser::parse(md);
        let (logos, problems) = resolve_slides(&Theme::light(), &pres, &d);
        assert_eq!(logos.get(0).unwrap().path, d.join("deck.svg"));
        assert!(logos.get(1).is_none(), "logo: none hides it on that slide");
        let partner = logos.get(2).unwrap();
        assert_eq!(partner.path, d.join("partner.svg"));
        assert_eq!(
            partner.corner,
            Corner::BottomLeft,
            "placed like the deck's logo"
        );
        assert_eq!(
            logos.get(3).unwrap().path,
            d.join("deck.svg"),
            "bad file falls back"
        );
        assert!(problems[0].starts_with("slide 4: logo:"), "{problems:?}");
        assert!(logos.get(4).is_none(), "no logo past the last slide");
        assert_eq!(logos.distinct().len(), 2);

        // A deck that hides the theme's logo can still show one on a slide.
        let md =
            "---\nlogo: none\n---\n\n# One\n\n- a\n\n# Two\n<!-- logo: partner.svg -->\n\n- b\n";
        let pres = crate::parser::parse(md);
        let (logos, _) = resolve_slides(&Theme::light(), &pres, &d);
        assert!(logos.get(0).is_none());
        assert_eq!(logos.get(1).unwrap().corner, DEFAULT_CORNER);
        std::fs::remove_dir_all(&d).ok();
    }
}
