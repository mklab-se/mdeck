use std::path::{Path, PathBuf};

use eframe::egui;

use crate::parser::{self, Presentation};
use crate::render;
use crate::render::image_cache::ImageCache;
use crate::theme::Theme;

/// Frames to let the viewport settle (pixels-per-point change, fonts) before
/// the first screenshot is requested.
const WARMUP_FRAMES: u32 = 2;

/// Accumulates screenshot tiles into a single RGBA canvas of the requested size.
///
/// The export window can never be larger than the screen, and on HiDPI
/// displays its pixel size differs from its logical size. Instead of trusting
/// the window, the slide is rendered at one point per pixel into a virtual
/// canvas of exactly `width`×`height`, one window-sized tile at a time, and
/// the tiles are stitched together here.
struct TileCanvas {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl TileCanvas {
    fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; (width as usize) * (height as usize) * 4],
        }
    }

    fn clear(&mut self) {
        self.pixels.fill(0);
    }

    /// Copy a screenshot captured for the tile whose top-left canvas pixel is
    /// `(ox, oy)`. Pixels outside the canvas are ignored.
    fn blit(&mut self, image: &egui::ColorImage, ox: u32, oy: u32) {
        let copy_w = image.width().min(self.width.saturating_sub(ox) as usize);
        let copy_h = image.height().min(self.height.saturating_sub(oy) as usize);
        for y in 0..copy_h {
            let src_row = y * image.width();
            let dst_row = ((oy as usize + y) * self.width as usize + ox as usize) * 4;
            for x in 0..copy_w {
                let c = image.pixels[src_row + x];
                let d = dst_row + x * 4;
                self.pixels[d..d + 4].copy_from_slice(&[c.r(), c.g(), c.b(), c.a()]);
            }
        }
    }

    fn save(&self, path: &Path) {
        image::save_buffer(
            path,
            &self.pixels,
            self.width,
            self.height,
            image::ColorType::Rgba8,
        )
        .unwrap_or_else(|e| eprintln!("Failed to save {}: {e}", path.display()));
    }
}

/// Number of tiles needed to cover `total` pixels with tiles of `tile` pixels.
fn tile_count(total: u32, tile: u32) -> u32 {
    if tile == 0 {
        return 1;
    }
    total.div_ceil(tile).max(1)
}

struct ExportApp {
    presentation: Presentation,
    theme: Theme,
    image_cache: ImageCache,
    output_dir: PathBuf,
    width: u32,
    height: u32,
    canvas: TileCanvas,
    current_slide: usize,
    current_step: usize,
    /// Tile currently being rendered (column, row).
    tile: (u32, u32),
    /// Canvas offset of the tile whose screenshot is pending.
    pending_tile_origin: (u32, u32),
    screenshot_requested: bool,
    warmup_frames: u32,
    max_steps: Vec<usize>,
    debug: bool,
    done: bool,
}

impl ExportApp {
    fn new(
        presentation: Presentation,
        base_path: &Path,
        output_dir: PathBuf,
        width: u32,
        height: u32,
        debug: bool,
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
            width,
            height,
            canvas: TileCanvas::new(width, height),
            current_slide: 0,
            current_step: 0,
            tile: (0, 0),
            pending_tile_origin: (0, 0),
            screenshot_requested: false,
            warmup_frames: WARMUP_FRAMES,
            max_steps,
            debug,
            done: false,
        }
    }

    fn slide_count(&self) -> usize {
        self.presentation.slides.len()
    }

    fn output_filename(&self) -> String {
        if self.debug {
            format!(
                "slide-{:02}-step-{:02}.png",
                self.current_slide + 1,
                self.current_step
            )
        } else {
            format!("slide-{:02}.png", self.current_slide + 1)
        }
    }

    /// Advance to the next reveal step or slide. Returns false when finished.
    fn advance(&mut self) -> bool {
        if self.debug {
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
        self.current_slide < self.slide_count()
    }
}

impl eframe::App for ExportApp {
    fn ui(&mut self, root_ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = root_ui.ctx().clone();
        if self.done {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        // Render at exactly one point per pixel so canvas coordinates map 1:1
        // to screenshot pixels regardless of the display's DPI.
        if (ctx.pixels_per_point() - 1.0).abs() > 0.001 {
            ctx.set_pixels_per_point(1.0);
            ctx.request_repaint();
            return;
        }
        if self.warmup_frames > 0 {
            self.warmup_frames -= 1;
            ctx.request_repaint();
            return;
        }

        let window = ctx.viewport_rect().size();
        let tile_w = (window.x.floor() as u32).max(1);
        let tile_h = (window.y.floor() as u32).max(1);
        let tiles_x = tile_count(self.width, tile_w);
        let tiles_y = tile_count(self.height, tile_h);

        // Collect the screenshot of the previously rendered tile.
        let mut got_screenshot = false;
        ctx.input(|i| {
            for event in &i.events {
                if let egui::Event::Screenshot { image, .. } = event {
                    let (ox, oy) = self.pending_tile_origin;
                    self.canvas.blit(image, ox, oy);
                    got_screenshot = true;
                }
            }
        });

        if got_screenshot {
            self.screenshot_requested = false;

            // Next tile, or finish the slide once every tile is captured.
            let (tx, ty) = self.tile;
            if tx + 1 < tiles_x {
                self.tile = (tx + 1, ty);
            } else if ty + 1 < tiles_y {
                self.tile = (0, ty + 1);
            } else {
                let filename = self.output_filename();
                self.canvas.save(&self.output_dir.join(&filename));
                eprintln!("  Saved {filename}");
                self.canvas.clear();
                self.tile = (0, 0);

                if !self.advance() {
                    self.done = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    return;
                }
            }
        }

        let bg = self.theme.background;
        let (tx, ty) = self.tile;
        let origin = (tx * tile_w, ty * tile_h);
        self.pending_tile_origin = origin;

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(bg).inner_margin(0.0))
            .show(root_ui, |ui| {
                ui.painter().rect_filled(ui.max_rect(), 0.0, bg);

                // The full slide rect, shifted so the current tile is visible.
                let rect = egui::Rect::from_min_size(
                    egui::pos2(-(origin.0 as f32), -(origin.1 as f32)),
                    egui::vec2(self.width as f32, self.height as f32),
                );
                let scale = (rect.width() / 1920.0).min(rect.height() / 1080.0);

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

        // Request screenshot after rendering (will arrive next frame), but not
        // while images are still decoding in the background.
        if !self.screenshot_requested && !self.image_cache.is_loading() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            self.screenshot_requested = true;
        }

        ctx.request_repaint();
    }
}

pub fn run(
    file: PathBuf,
    output_dir: PathBuf,
    width: u32,
    height: u32,
    debug: bool,
) -> anyhow::Result<()> {
    if width == 0 || height == 0 {
        anyhow::bail!("Export width and height must be greater than zero");
    }

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
    eframe::run_native(
        &title,
        options,
        Box::new(move |_cc| {
            Ok(Box::new(ExportApp::new(
                presentation,
                &base_path,
                output_dir_clone,
                width,
                height,
                debug,
            )))
        }),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;

    eprintln!("Export complete.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid_image(w: usize, h: usize, c: egui::Color32) -> egui::ColorImage {
        egui::ColorImage::new([w, h], vec![c; w * h])
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
