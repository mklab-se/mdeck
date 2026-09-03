use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use eframe::egui;

use crate::commands::util::slide_number_width;
use crate::parser::{self, Presentation};
use crate::render;
use crate::render::image_cache::ImageCache;
use crate::theme::Theme;

struct ExportApp {
    presentation: Presentation,
    theme: Theme,
    image_cache: ImageCache,
    output_dir: PathBuf,
    current_slide: usize,
    current_step: usize,
    screenshot_requested: bool,
    max_steps: Vec<usize>,
    debug: bool,
    done: bool,
    /// First save error, shared with `run()` so the exit code reflects it.
    error: Arc<Mutex<Option<String>>>,
}

impl ExportApp {
    fn new(
        presentation: Presentation,
        base_path: &Path,
        output_dir: PathBuf,
        debug: bool,
        error: Arc<Mutex<Option<String>>>,
    ) -> Self {
        let theme_name = presentation.meta.theme.as_deref().unwrap_or("light");
        let theme = Theme::from_name(theme_name);
        let image_cache = ImageCache::new(base_path.to_path_buf());
        let max_steps: Vec<usize> = presentation
            .slides
            .iter()
            .map(|s| parser::compute_max_steps(&s.blocks))
            .collect();

        Self {
            presentation,
            theme,
            image_cache,
            output_dir,
            current_slide: 0,
            current_step: 0,
            screenshot_requested: false,
            max_steps,
            debug,
            done: false,
            error,
        }
    }

    fn slide_count(&self) -> usize {
        self.presentation.slides.len()
    }

    /// File name for the current slide/step, zero-padded to the deck size.
    fn output_filename(&self) -> String {
        export_filename(
            self.current_slide,
            self.slide_count(),
            self.debug.then_some(self.current_step),
            self.max_steps.iter().copied().max().unwrap_or(0),
        )
    }
}

/// Build `slide-NN.png` / `slide-NN-step-MM.png`, padding both numbers so
/// files sort correctly for decks with 100+ slides or steps.
fn export_filename(
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

impl eframe::App for ExportApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.done {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        // Check for screenshot result from previous frame
        let mut got_screenshot = false;
        let mut save_error: Option<String> = None;
        ctx.input(|i| {
            for event in &i.events {
                if let egui::Event::Screenshot { image, .. } = event {
                    let filename = self.output_filename();
                    let path = self.output_dir.join(&filename);
                    match save_color_image(image, &path) {
                        Ok(()) => eprintln!("  Saved {filename}"),
                        Err(e) => save_error = Some(e),
                    }
                    got_screenshot = true;
                }
            }
        });

        if let Some(e) = save_error {
            // Stop at the first failure so it cannot be mistaken for success
            eprintln!("  {e}");
            *self.error.lock().unwrap() = Some(e);
            self.done = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        if got_screenshot {
            self.screenshot_requested = false;

            if self.debug {
                // In debug mode, advance through each reveal step
                let max = self.max_steps.get(self.current_slide).copied().unwrap_or(0);
                if self.current_step < max {
                    self.current_step += 1;
                } else {
                    self.current_step = 0;
                    self.current_slide += 1;
                }
            } else {
                self.current_slide += 1;
            }

            if self.current_slide >= self.slide_count() {
                self.done = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
        }

        let bg = self.theme.background;

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(bg).inner_margin(0.0))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                ui.painter().rect_filled(rect, 0.0, bg);

                let scale = {
                    let ref_w = 1920.0;
                    let ref_h = 1080.0;
                    (rect.width() / ref_w).min(rect.height() / ref_h)
                };

                let idx = self.current_slide;
                if idx < self.presentation.slides.len() {
                    let reveal = if self.debug {
                        self.current_step
                    } else {
                        self.max_steps.get(idx).copied().unwrap_or(0)
                    };
                    render::render_slide(
                        ui,
                        &self.presentation.slides[idx],
                        &self.theme,
                        rect,
                        1.0,
                        &self.image_cache,
                        reveal,
                        None, // no animation in export
                        scale,
                    );
                }
            });

        // Request screenshot after rendering (will arrive next frame)
        if !self.screenshot_requested {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            self.screenshot_requested = true;
        }

        ctx.request_repaint();
    }
}

fn save_color_image(image: &egui::ColorImage, path: &Path) -> Result<(), String> {
    let width = image.width() as u32;
    let height = image.height() as u32;
    let pixels: Vec<u8> = image
        .pixels
        .iter()
        .flat_map(|c| [c.r(), c.g(), c.b(), c.a()])
        .collect();

    image::save_buffer(path, &pixels, width, height, image::ColorType::Rgba8)
        .map_err(|e| format!("Failed to save {}: {e}", path.display()))
}

pub fn run(
    file: PathBuf,
    output_dir: PathBuf,
    width: u32,
    height: u32,
    debug: bool,
) -> anyhow::Result<()> {
    let content = std::fs::read_to_string(&file)?;
    let base_path = file
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .to_path_buf();
    let presentation = parser::parse(&content, &base_path);

    if presentation.slides.is_empty() {
        anyhow::bail!("No slides found in {}", file.display());
    }

    std::fs::create_dir_all(&output_dir)?;

    let slide_count = presentation.slides.len();
    if debug {
        let total_steps: usize = presentation
            .slides
            .iter()
            .map(|s| parser::compute_max_steps(&s.blocks) + 1)
            .sum();
        eprintln!(
            "Debug export: {} slides, {} total steps to {} ({}x{})",
            slide_count,
            total_steps,
            output_dir.display(),
            width,
            height,
        );
    } else {
        eprintln!(
            "Exporting {} slides to {} ({}x{})",
            slide_count,
            output_dir.display(),
            width,
            height,
        );
    }

    let title = presentation
        .meta
        .title
        .clone()
        .unwrap_or_else(|| "mdeck export".to_string());

    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([width as f32, height as f32])
        .with_title(&title)
        .with_decorations(false);

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    let output_dir_clone = output_dir.clone();
    let error: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let error_clone = error.clone();
    eframe::run_native(
        &title,
        options,
        Box::new(move |_cc| {
            Ok(Box::new(ExportApp::new(
                presentation,
                &base_path,
                output_dir_clone,
                debug,
                error_clone,
            )))
        }),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;

    // A failed save must not look like success: propagate it as an error
    let failed = error.lock().unwrap().take();
    if let Some(e) = failed {
        anyhow::bail!("Export failed: {e}");
    }

    eprintln!("Export complete.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
