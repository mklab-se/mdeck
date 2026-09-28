//! `mdeck export`: slides to PNG files, or to one PDF (optionally with
//! speaker notes pages).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use eframe::egui;

use crate::commands::util::slide_number_width;
use crate::deck::{self, Deck};
use crate::parser;
use crate::render;

mod app;
mod canvas;
mod cursor;
mod notes;
mod pdf;
mod rehearsal;

use app::{ExportApp, NotesPages, Output};
use cursor::Job;

/// What `mdeck export` writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum Format {
    /// One PNG per slide
    #[default]
    Png,
    /// One PDF with a page per slide
    Pdf,
}

/// Build `slide-NN.png` / `slide-NN-step-MM.png`, padding both numbers so
/// files sort correctly for decks with 100+ slides or steps.
pub(super) fn export_filename(
    slide_index: usize,
    slide_count: usize,
    step: Option<usize>,
    max_step: usize,
) -> String {
    let sw = slide_number_width(slide_count);
    match step {
        Some(step) => {
            let stw = slide_number_width(max_step);
            format!(
                "slide-{:0sw$}-step-{:0stw$}.png",
                slide_index + 1,
                step,
                sw = sw,
                stw = stw
            )
        }
        None => format!("slide-{:0sw$}.png", slide_index + 1, sw = sw),
    }
}

/// Which slides (0-based) `--slide` / `--range` select out of `count`.
pub fn select_slides(
    slide: Option<usize>,
    range: Option<&str>,
    count: usize,
) -> anyhow::Result<Vec<usize>> {
    if let Some(n) = slide {
        if n == 0 || n > count {
            anyhow::bail!("slide {n} is outside 1-{count}");
        }
        return Ok(vec![n - 1]);
    }
    if let Some(r) = range {
        let (a, b) = r
            .split_once('-')
            .ok_or_else(|| anyhow::anyhow!("range must look like 3-7"))?;
        let a: usize = a
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("range start"))?;
        let b: usize = b.trim().parse().map_err(|_| anyhow::anyhow!("range end"))?;
        if a == 0 || b < a || b > count {
            anyhow::bail!("range {r} is outside 1-{count}");
        }
        return Ok((a - 1..b).collect());
    }
    Ok((0..count).collect())
}

/// Which theme an export uses.
pub enum ThemeChoice {
    /// What the deck asks for (`@theme`, then the config default).
    Deck,
    /// A theme looked up by name from the deck's folder (`--theme`).
    Named(String),
    /// A theme already resolved by the caller.
    Given(Box<crate::theme::Theme>),
}

/// What `mdeck export` is asked to do.
pub struct ExportArgs {
    pub file: PathBuf,
    pub output_dir: PathBuf,
    pub width: u32,
    pub height: u32,
    /// Export every reveal step of every slide.
    pub debug: bool,
    /// Only this slide (1-based).
    pub slide: Option<usize>,
    /// Only these slides, e.g. `3-7` (1-based, inclusive).
    pub range: Option<String>,
    pub format: Format,
    /// PDF only: a notes page after every slide.
    pub notes: bool,
    pub theme: ThemeChoice,
    /// `--engine`: overrides `@engine` and the theme's.
    pub engine: Option<String>,
}

pub fn run(args: ExportArgs) -> anyhow::Result<()> {
    let ExportArgs {
        file,
        output_dir,
        width,
        height,
        debug,
        slide,
        range,
        format,
        notes,
        theme,
        engine,
    } = args;
    if width == 0 || height == 0 {
        anyhow::bail!("Export width and height must be greater than zero");
    }
    if notes && format != Format::Pdf {
        anyhow::bail!("--notes needs --format pdf (notes pages only exist in PDF export)");
    }

    let content = std::fs::read_to_string(&file)?;
    let base_path = file
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .to_path_buf();
    let presentation = parser::parse(&content);

    if presentation.slides.is_empty() {
        anyhow::bail!("No slides found in {}", file.display());
    }
    crate::commands::check::warn_missing_cjk_font(&presentation);

    // Same precedence as presenting: --theme, then @theme, then the config
    // default, then the built-in default. An explicit --theme must exist.
    let themes = crate::theme::lookup::Lookup::for_deck(Some(&base_path));
    let theme = match theme {
        ThemeChoice::Given(theme) => *theme,
        ThemeChoice::Named(name) => {
            let built = themes.load(&name).map_err(|e| anyhow::anyhow!("{e}"))?;
            deck::report_theme_problems(&built.warnings);
            built.theme
        }
        ThemeChoice::Deck => {
            let defaults = crate::config::Config::load_or_default()
                .defaults
                .unwrap_or_default();
            deck::deck_theme(&themes, &presentation, defaults.theme.as_deref()).0
        }
    };

    // --engine, then @engine, then the theme's own.
    let (cli_engine, _) =
        crate::engines::choose(engine.as_deref(), None).map_err(|e| anyhow::anyhow!("{e}"))?;
    let theme =
        crate::engines::with_engine(theme, deck::deck_engine(cli_engine, &presentation, false));
    let deck = Deck::open(file.clone(), presentation, &theme, false, false);
    if let Some(line) =
        crate::engines::unsupported_summary(theme.engine, &deck.presentation, &deck.with_story())
    {
        eprintln!("warning: {line}");
    }

    std::fs::create_dir_all(&output_dir)?;

    let slide_total = deck.slide_count();
    let targets = select_slides(slide, range.as_deref(), slide_total)?;
    let slide_count = targets.len();
    let which = if slide_count == slide_total {
        format!("{slide_count} slides")
    } else {
        let span = if slide_count > 1 {
            format!("-{}", targets[slide_count - 1] + 1)
        } else {
            String::new()
        };
        format!(
            "{slide_count} of {slide_total} slides ({}{span})",
            targets[0] + 1
        )
    };
    let pdf_path = (format == Format::Pdf).then(|| output_dir.join(pdf_filename(&file, notes)));
    let target = pdf_path.as_ref().unwrap_or(&output_dir);
    eprintln!(
        "{} {which} to {} ({width}x{height}{})",
        if debug { "Debug export:" } else { "Exporting" },
        target.display(),
        if notes { ", with speaker notes" } else { "" },
    );

    let meta = &deck.presentation.meta;
    let title = meta
        .title
        .clone()
        .unwrap_or_else(|| "mdeck export".to_string());
    let pdf_meta = pdf::Meta {
        title: meta.title.clone(),
        author: meta.author.clone(),
    };

    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([width as f32, height as f32])
        .with_title(&title)
        .with_decorations(false);

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    let doc = Arc::new(Mutex::new(pdf::PdfDoc::new()));
    let output = match format {
        Format::Png => Output::Png {
            dir: output_dir.clone(),
        },
        Format::Pdf => Output::Pdf {
            doc: doc.clone(),
            notes: notes.then(|| Box::new(NotesPages::new())),
        },
    };
    let job = Job {
        width,
        height,
        debug,
        targets,
    };
    let error: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let error_clone = error.clone();
    eframe::run_native(
        &title,
        options,
        Box::new(move |cc| {
            render::fonts::install(&cc.egui_ctx);
            Ok(Box::new(ExportApp::new(
                deck,
                theme,
                output,
                job,
                error_clone,
            )))
        }),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;

    // A failed save must not look like success: propagate it as an error
    let failed = error.lock().unwrap_or_else(|p| p.into_inner()).take();
    if let Some(e) = failed {
        anyhow::bail!("Export failed: {e}");
    }

    if let Some(path) = pdf_path {
        let doc = std::mem::take(&mut *doc.lock().unwrap_or_else(|p| p.into_inner()));
        if doc.page_count() == 0 {
            anyhow::bail!("Export failed: no pages were rendered");
        }
        let pages = doc.page_count();
        std::fs::write(&path, doc.finish(&pdf_meta))
            .map_err(|e| anyhow::anyhow!("Failed to write {}: {e}", path.display()))?;
        eprintln!(
            "Wrote {} ({pages} page{}).",
            path.display(),
            if pages == 1 { "" } else { "s" }
        );
    }

    eprintln!("Export complete.");
    Ok(())
}

/// `<deck>.pdf`, or `<deck>-notes.pdf` with speaker notes pages.
fn pdf_filename(deck: &std::path::Path, notes: bool) -> String {
    let stem = deck
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "slides".into());
    if notes {
        format!("{stem}-notes.pdf")
    } else {
        format!("{stem}.pdf")
    }
}

#[cfg(test)]
mod tests {
    use super::canvas::{TileCanvas, tile_count};
    use super::*;

    fn solid_image(w: usize, h: usize, c: egui::Color32) -> egui::ColorImage {
        egui::ColorImage::new([w, h], vec![c; w * h])
    }

    #[test]
    fn slide_and_range_select_zero_based_indices() {
        assert_eq!(select_slides(None, None, 4).unwrap(), vec![0, 1, 2, 3]);
        assert_eq!(select_slides(Some(3), None, 4).unwrap(), vec![2]);
        assert_eq!(select_slides(None, Some("2-3"), 4).unwrap(), vec![1, 2]);
        assert!(select_slides(Some(0), None, 4).is_err());
        assert!(select_slides(Some(5), None, 4).is_err());
        assert!(select_slides(None, Some("3-2"), 4).is_err());
        assert!(select_slides(None, Some("x"), 4).is_err());
    }

    #[test]
    fn filenames_pad_to_two_digits_for_small_decks() {
        assert_eq!(export_filename(0, 5, None, 0), "slide-01.png");
        assert_eq!(export_filename(98, 99, None, 0), "slide-99.png");
    }

    #[test]
    fn filenames_pad_to_three_digits_for_large_decks() {
        assert_eq!(export_filename(0, 100, None, 0), "slide-001.png");
        assert_eq!(export_filename(119, 120, None, 0), "slide-120.png");
    }

    #[test]
    fn debug_filenames_include_padded_step() {
        assert_eq!(export_filename(2, 10, Some(0), 4), "slide-03-step-00.png");
        assert_eq!(export_filename(2, 10, Some(3), 4), "slide-03-step-03.png");
        assert_eq!(
            export_filename(2, 150, Some(7), 120),
            "slide-003-step-007.png"
        );
    }

    #[test]
    fn pdf_filenames_follow_the_deck() {
        let deck = std::path::Path::new("talks/intro.md");
        assert_eq!(pdf_filename(deck, false), "intro.pdf");
        assert_eq!(pdf_filename(deck, true), "intro-notes.pdf");
    }

    #[test]
    fn tiles_advance_in_row_order_then_finish() {
        use super::canvas::next_tile;
        assert_eq!(next_tile((0, 0), (2, 2)), Some((1, 0)));
        assert_eq!(next_tile((1, 0), (2, 2)), Some((0, 1)));
        assert_eq!(next_tile((1, 1), (2, 2)), None);
        assert_eq!(next_tile((0, 0), (1, 1)), None);
    }

    #[test]
    fn tile_count_covers_area() {
        assert_eq!(tile_count(1920, 1920), 1);
        assert_eq!(tile_count(1920, 3024), 1);
        assert_eq!(tile_count(3840, 3024), 2);
        assert_eq!(tile_count(2160, 1964), 2);
        assert_eq!(tile_count(100, 0), 1);
    }

    #[test]
    fn blit_stitches_tiles_and_clips_overflow() {
        // Canvas 4x3, tiles of 3x2: 2x2 tiles, the right/bottom ones overflow.
        let mut canvas = TileCanvas::new(4, 3);
        let red = egui::Color32::RED;
        let blue = egui::Color32::BLUE;
        canvas.blit(&solid_image(3, 2, red), 0, 0);
        canvas.blit(&solid_image(3, 2, blue), 3, 0);
        canvas.blit(&solid_image(3, 2, blue), 0, 2);
        canvas.blit(&solid_image(3, 2, red), 3, 2);

        let px = |x: u32, y: u32| {
            let i = ((y * 4 + x) * 4) as usize;
            [canvas.pixels[i], canvas.pixels[i + 1], canvas.pixels[i + 2]]
        };
        assert_eq!(px(0, 0), [255, 0, 0]);
        assert_eq!(px(2, 1), [255, 0, 0]);
        assert_eq!(px(3, 0), [0, 0, 255]);
        assert_eq!(px(0, 2), [0, 0, 255]);
        assert_eq!(px(3, 2), [255, 0, 0]);
        assert_eq!(canvas.pixels.len(), 4 * 3 * 4);
    }

    #[test]
    fn blit_ignores_tiles_outside_canvas() {
        let mut canvas = TileCanvas::new(2, 2);
        canvas.blit(&solid_image(2, 2, egui::Color32::WHITE), 5, 5);
        assert!(canvas.pixels.iter().all(|&b| b == 0));
    }
}
