//! `mdeck theme preview`: export a sampler deck in a theme: one slide per
//! design in the catalogue (THM-19, THM-20), so a designer sees every
//! arrangement at once.

use std::path::{Path, PathBuf};

use anyhow::Result;
use eframe::egui::Color32;

use super::here;
use crate::theme::Theme;

/// The sampler deck: its slides are the designs of the catalogue in order
/// (`Design::ALL`), each written so the recogniser gives it that design.
const SAMPLER: &str = r#"---
title: Theme preview
author: mdeck
---

# Theme preview

Every slide design, in this theme

---

## Part two

---

## Why a theme

A theme is a whole look: colours, type, how every design is arranged and the engine behind the slides.

---

## Bullets and inline styles

The list design, with a lead paragraph.

- Body text with **bold**, *emphasis* and `inline code`
- A [link](https://example.com) and a longer line that wraps onto a second line to show the leading
- Nested items
  - stay readable
  - at a smaller indent

---

## Text beside a picture

![A picture in the theme's colours](sample-1.png)

- The split design puts text beside one image
- Paragraphs or a list, the picture on the other side

---

## One picture

![A wide picture](sample-2.png)

A caption under the picture

---

## Gallery

![First](sample-1.png)

![Second](sample-2.png)

![Third](sample-3.png)

---

## A quote

> Simplicity is the ultimate sophistication.

-- Leonardo da Vinci

---

## Code

A short lead before the code.

```rust
/// Greets the room.
fn main() {
    let name = "world";
    println!("Hello, {name}!");
}
```

---

## Chart

```@bar
- Design: 42
- Build: 68
- Launch: 55
- Grow: 81
```

---

## Columns

Before

- Slides drawn by hand
- Every deck looks different

+++

After

- Markdown in, slides out
- One theme for every deck

---

## Table

| Role | Colour | Used for |
|---|---|---|
| Accent | the brand colour | links, highlights |
| Text | a quiet step | body copy |
| Heading | the brightest | titles |

---

## Anything else

The content design holds whatever no other design does, in reading order.

- a list
- after a paragraph

> [!TIP]
> And a callout at the end.
"#;

/// The sample pictures the sampler shows: gradients in the theme's colours.
fn write_pictures(dir: &Path, theme: &Theme) -> Result<()> {
    let pairs = [
        (theme.accent, theme.background),
        (theme.series[1], theme.series[2]),
        (theme.secondary, theme.series[3]),
    ];
    for (i, (a, b)) in pairs.into_iter().enumerate() {
        picture(a, b, 1200, 800).save(dir.join(format!("sample-{}.png", i + 1)))?;
    }
    Ok(())
}

/// A diagonal gradient from `a` to `b` with a soft disc, `w` by `h` px.
pub(crate) fn picture(a: Color32, b: Color32, w: u32, h: u32) -> image::RgbImage {
    image::RgbImage::from_fn(w, h, |x, y| {
        let t = (x as f32 / w as f32 * 0.6 + y as f32 / h as f32 * 0.4).clamp(0.0, 1.0);
        let (dx, dy) = (x as f32 - w as f32 * 0.66, y as f32 - h as f32 * 0.4);
        let disc = (1.0 - (dx * dx + dy * dy).sqrt() / (h as f32 * 0.32)).clamp(0.0, 1.0);
        let mix = |p: u8, q: u8| {
            let v = p as f32 + (q as f32 - p as f32) * t;
            (v + (255.0 - v) * disc * 0.35) as u8
        };
        image::Rgb([mix(a.r(), b.r()), mix(a.g(), b.g()), mix(a.b(), b.b())])
    })
}

pub(super) fn preview(name: &str, output_dir: PathBuf, width: u32, height: u32) -> Result<()> {
    let built = here().load(name).map_err(|e| anyhow::anyhow!("{e}"))?;
    for w in &built.warnings {
        eprintln!("warning: theme: {w}");
    }
    let dir = std::env::temp_dir().join(format!("mdeck-theme-preview-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let deck = dir.join("theme-preview.md");
    std::fs::write(&deck, SAMPLER)?;
    write_pictures(&dir, &built.theme)?;
    let result = crate::commands::export::run(crate::commands::export::ExportArgs {
        file: deck,
        output_dir,
        width,
        height,
        debug: false,
        slide: None,
        range: None,
        format: crate::commands::export::Format::Png,
        notes: false,
        theme: crate::commands::export::ThemeChoice::Given(Box::new(built.theme)),
        engine: None,
        at: None,
        moment: None,
        presenter_view: false,
    });
    let _ = std::fs::remove_dir_all(&dir);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Design;

    #[test]
    fn the_sampler_shows_every_design_once_in_catalogue_order() {
        let p = crate::parser::parse(SAMPLER);
        let designs: Vec<Design> = p.slides.iter().map(|s| s.design).collect();
        assert_eq!(designs, Design::ALL);
    }

    #[test]
    fn sample_pictures_are_drawn_in_the_theme() {
        let t = Theme::dark();
        let img = picture(t.accent, t.background, 60, 40);
        assert_eq!(img.dimensions(), (60, 40));
        let p = img.get_pixel(0, 39).0;
        assert!(p.iter().any(|c| *c > 0));
    }
}
