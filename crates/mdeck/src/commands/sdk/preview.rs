//! `mdeck sdk preview` (EXT-33): an extension on a built-in preview deck,
//! every design, a chart, images and a picture, plus the countdown and the
//! end, exported as PNGs with the engine and theme given by name.
//!
//! Each export runs this same binary as a child process (`mdeck export`),
//! so a custom build's extension engines are found by name, and every
//! export gets a window of its own (an event loop is made once per process).

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use colored::Colorize;
use eframe::egui::Color32;

/// The preview deck: one slide per design in catalogue order, then a slide
/// with a picture on the design's stage.
pub(crate) const DECK: &str = r#"---
title: Extension preview
author: mdeck sdk preview
---

# Extension preview

Every design, a chart, images, a picture and both moments

---

## Part two

---

## One idea, set large

A statement slide: a heading and a sentence, nothing else.

---

## Points

A lead paragraph before the list.

- Body text with **bold**, *emphasis* and `inline code`
- A longer line that wraps onto a second line, to show the leading
- Nested items
  - at a smaller indent

---

## Text beside an image

![A picture](preview-1.png)

- The split design puts text beside one image
- The engine draws around both

---

## One image

![A wide picture](preview-2.png)

A caption under the image

---

## Gallery

![First](preview-1.png)

![Second](preview-2.png)

![Third](preview-3.png)

---

## A quote

> Simplicity is prerequisite for reliability.

-- Edsger Dijkstra

---

## Code

```rust
fn main() {
    println!("Hello from an extension");
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

+++

After

- Markdown in, slides out

---

## Table

| Engine | Draws | Where |
|---|---|---|
| plain | nothing | under the slide |
| yours | anything | under or around it |

---

## Anything else

A paragraph, then a list and a callout: the content design.

- a list

> [!TIP]
> And a callout at the end.

---

## A picture
<!-- picture: rocket -->

- Engines that draw pictures show one here
- A point cloud, or the slide's artwork on an art engine
"#;

/// The images the deck shows: neutral gradients, written next to it.
fn write_images(dir: &Path) -> Result<()> {
    let pairs = [
        (
            Color32::from_rgb(232, 98, 52),
            Color32::from_rgb(24, 26, 32),
        ),
        (
            Color32::from_rgb(64, 132, 200),
            Color32::from_rgb(40, 160, 120),
        ),
        (
            Color32::from_rgb(210, 160, 60),
            Color32::from_rgb(120, 70, 160),
        ),
    ];
    for (i, (a, b)) in pairs.into_iter().enumerate() {
        crate::commands::theme::preview::picture(a, b, 1200, 800)
            .save(dir.join(format!("preview-{}.png", i + 1)))?;
    }
    Ok(())
}

/// What `mdeck sdk preview` is asked for.
pub struct PreviewArgs {
    pub engine: Option<String>,
    pub theme: Option<String>,
    pub output_dir: PathBuf,
    pub width: u32,
    pub height: u32,
    pub quiet: bool,
}

/// The arguments of one `mdeck export` child.
fn export_args(
    deck: &Path,
    out: &Path,
    a: &PreviewArgs,
    moment: Option<(&str, usize)>,
) -> Vec<String> {
    let mut args = vec![
        "export".to_string(),
        "--quiet".to_string(),
        deck.display().to_string(),
        "--output-dir".to_string(),
        out.display().to_string(),
        "--width".to_string(),
        a.width.to_string(),
        "--height".to_string(),
        a.height.to_string(),
    ];
    if let Some(t) = &a.theme {
        args.extend(["--theme".to_string(), t.clone()]);
    }
    if let Some(e) = &a.engine {
        args.extend(["--engine".to_string(), e.clone()]);
    }
    if let Some((m, slide)) = moment {
        args.extend([
            "--moment".to_string(),
            m.to_string(),
            "--slide".to_string(),
            slide.to_string(),
        ]);
    }
    args
}

fn export(exe: &Path, args: &[String]) -> Result<()> {
    let status = Command::new(exe)
        .args(args)
        .status()
        .with_context(|| format!("running {}", exe.display()))?;
    if !status.success() {
        bail!("`mdeck {}` failed", args.join(" "));
    }
    Ok(())
}

/// `mdeck sdk preview`.
pub fn run(a: PreviewArgs) -> Result<()> {
    if let Some(name) = &a.engine
        && crate::engines::EngineId::find(name).is_none()
    {
        bail!(
            "no engine named `{name}` in this mdeck (it has: {}); an extension's engine needs a build with it (`mdeck build --with`)",
            crate::engines::names()
        );
    }
    let exe = std::env::current_exe().context("finding this mdeck")?;
    let dir = crate::extensions::packs::tempdir("sdk-preview")?;
    let result = (|| {
        let deck = dir.join("preview.md");
        std::fs::write(&deck, DECK)?;
        write_images(&dir)?;
        std::fs::create_dir_all(&a.output_dir)?;
        if !a.quiet {
            eprintln!(
                "Previewing {} with {} ...",
                a.engine.as_deref().unwrap_or("the theme's engine").bold(),
                a.theme.as_deref().unwrap_or("the default theme").bold()
            );
        }
        export(&exe, &export_args(&deck, &a.output_dir, &a, None))?;
        let slides = crate::parser::parse(DECK).slides.len();
        for (moment, slide) in [("countdown", 1), ("end", slides)] {
            let out = dir.join(moment);
            export(&exe, &export_args(&deck, &out, &a, Some((moment, slide))))?;
            let png = std::fs::read_dir(&out)?
                .flatten()
                .map(|e| e.path())
                .find(|p| p.extension().is_some_and(|x| x == "png"))
                .with_context(|| format!("the {moment} export wrote no image"))?;
            std::fs::copy(&png, a.output_dir.join(format!("{moment}.png")))?;
        }
        if !a.quiet {
            println!(
                "Wrote {slides} slides, countdown.png and end.png to {}",
                a.output_dir.display()
            );
        }
        Ok(())
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::CheckCategory;
    use crate::parser::Design;

    #[test]
    fn the_deck_covers_every_design_then_a_picture() {
        let p = crate::parser::parse(DECK);
        let designs: Vec<Design> = p.slides.iter().map(|s| s.design).collect();
        assert_eq!(&designs[..Design::ALL.len()], &Design::ALL);
        let last = p.slides.last().unwrap();
        assert_eq!(last.illustration.as_deref(), Some("rocket"));
        assert!(DECK.contains("```@bar"));
    }

    #[test]
    fn the_deck_has_no_check_problems() {
        let dir = crate::extensions::packs::tempdir("sdk-preview-check").unwrap();
        write_images(&dir).unwrap();
        // on a theme whose designs have a stage, so the picture slide shows
        let text = DECK.replacen("---\n", "---\ntheme: ember\n", 1);
        let file = dir.join("preview.md");
        std::fs::write(&file, &text).unwrap();
        let p = crate::parser::parse(&text);
        let report = crate::commands::check::collect(&file, &text, &p, &dir, None).unwrap();
        let problems: Vec<String> = report
            .warnings()
            .filter(|w| {
                // a build without the particles engine has no built-in clouds
                cfg!(feature = "particles") || w.category != CheckCategory::PointCloud
            })
            .filter(|w| cfg!(feature = "particles") || w.category != CheckCategory::Engine)
            .map(|w| format!("{}: {}", w.slide, w.message))
            .collect();
        assert!(problems.is_empty(), "{problems:?}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn exports_pass_the_engine_theme_and_moment() {
        let a = PreviewArgs {
            engine: Some("glow".into()),
            theme: Some("dark".into()),
            output_dir: PathBuf::from("out"),
            width: 1920,
            height: 1080,
            quiet: true,
        };
        let args = export_args(Path::new("d.md"), Path::new("o"), &a, Some(("end", 15)));
        let s = args.join(" ");
        assert!(s.starts_with("export --quiet d.md --output-dir o"), "{s}");
        assert!(
            s.contains("--theme dark --engine glow --moment end --slide 15"),
            "{s}"
        );
    }

    #[test]
    fn an_unknown_engine_is_refused_before_exporting() {
        let err = run(PreviewArgs {
            engine: Some("no-such-engine".into()),
            theme: None,
            output_dir: PathBuf::from("unused"),
            width: 10,
            height: 10,
            quiet: true,
        })
        .unwrap_err()
        .to_string();
        assert!(err.contains("no engine named `no-such-engine`"), "{err}");
        assert!(!Path::new("unused").exists());
    }
}
