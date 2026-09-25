//! The export window: renders each slide (and, for PDF with notes, each
//! notes page) at exactly one point per pixel, in window-sized tiles that are
//! stitched into a canvas of the requested size, and hands finished pages to
//! the output (PNG files or the PDF being assembled).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use eframe::egui;

use super::canvas::{TileCanvas, tile_count};
use super::notes::{self, NotesJob};
use super::pdf::PdfDoc;
use crate::app::ember::EmberState;
use crate::parser::{self, Presentation};
use crate::render;
use crate::render::image_cache::ImageCache;
use crate::render::story::{self, sidecar::Resolved};
use crate::theme::Theme;

/// Frames to let the viewport settle (pixels-per-point change, fonts) before
/// the first screenshot is requested.
const WARMUP_FRAMES: u32 = 2;

/// Where finished pages go.
pub(super) enum Output {
    /// One PNG per slide (or per reveal step with `--debug`).
    Png { dir: PathBuf },
    /// Pages of a PDF; with `notes`, every slide becomes a notes page.
    Pdf {
        doc: Arc<Mutex<PdfDoc>>,
        notes: bool,
    },
}

/// What the canvas currently holds.
enum Pass {
    Slide,
    /// Notes page `page` of the current slide.
    Notes {
        page: usize,
    },
}

pub(super) struct ExportApp {
    presentation: Presentation,
    theme: Theme,
    image_cache: ImageCache,
    output: Output,
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
    /// Slide indices to export, in order (the whole deck by default).
    targets: Vec<usize>,
    /// Position in `targets`.
    target_pos: usize,
    debug: bool,
    done: bool,
    /// First error, shared with `run()` so the exit code reflects it.
    error: Arc<Mutex<Option<String>>>,
    /// Ember's particle field, settled per slide so exports are still frames.
    ember: EmberState,
    /// Frames rendered for the current page; content hints arrive one frame
    /// late, so the screenshot waits for the second frame.
    frames_on_slide: u32,
    /// Story per slide (sidecar), for the Ember theme.
    stories: Vec<Option<Resolved>>,
    /// Point cloud illustrations resolved for this deck.
    illustrations: render::illustration::Library,
    pass: Pass,
    notes: Option<NotesJob>,
    /// Theme notes pages are printed in.
    notes_theme: Theme,
}

impl ExportApp {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        presentation: Presentation,
        deck: &Path,
        base_path: &Path,
        output: Output,
        width: u32,
        height: u32,
        debug: bool,
        targets: Vec<usize>,
        error: Arc<Mutex<Option<String>>>,
    ) -> Self {
        let theme_name = presentation.meta.theme.as_deref().unwrap_or("light");
        let theme = Theme::from_name(theme_name);
        let image_cache = ImageCache::new(base_path.to_path_buf());
        let stories = match story::sidecar::load(deck) {
            Ok(sc) => story::sidecar::resolve(&presentation, sc.as_ref()).0,
            Err(e) => {
                eprintln!("Warning: story sidecar ignored: {e}");
                story::sidecar::resolve(&presentation, None).0
            }
        };
        let max_steps: Vec<usize> = presentation
            .slides
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let beats = if theme.is_ember() {
                    stories
                        .get(i)
                        .and_then(|r| r.as_ref())
                        .map(|r| r.script.extra_steps())
                        .unwrap_or(0)
                } else {
                    0
                };
                parser::compute_max_steps(&s.blocks).max(beats)
            })
            .collect();

        if debug {
            let total: usize = targets
                .iter()
                .map(|&i| max_steps.get(i).copied().unwrap_or(0) + 1)
                .sum();
            eprintln!("  {total} reveal steps in total (story beats included)");
        }
        let illustrations = render::illustration::Library::for_deck(deck.parent());
        Self {
            presentation,
            theme,
            image_cache,
            illustrations,
            output,
            width,
            height,
            canvas: TileCanvas::new(width, height),
            current_slide: targets.first().copied().unwrap_or(0),
            targets,
            target_pos: 0,
            current_step: 0,
            tile: (0, 0),
            pending_tile_origin: (0, 0),
            screenshot_requested: false,
            warmup_frames: WARMUP_FRAMES,
            max_steps,
            debug,
            done: false,
            error,
            ember: EmberState::new(),
            frames_on_slide: 0,
            stories,
            pass: Pass::Slide,
            notes: None,
            notes_theme: Theme::light(),
        }
    }

    fn slide_count(&self) -> usize {
        self.presentation.slides.len()
    }

    /// File name for the current slide/step, zero-padded to the deck size.
    fn output_filename(&self) -> String {
        super::export_filename(
            self.current_slide,
            self.slide_count(),
            self.debug.then_some(self.current_step),
            self.max_steps.iter().copied().max().unwrap_or(0),
        )
    }

    /// Outline entry for the current slide: its first heading, else its number.
    /// Only the first page of a slide gets one.
    fn bookmark(&self) -> Option<String> {
        if self.debug && self.current_step > 0 {
            return None;
        }
        let slide = self.presentation.slides.get(self.current_slide)?;
        let n = self.current_slide + 1;
        let heading = slide.blocks.iter().find_map(|b| match b {
            parser::Block::Heading { inlines, .. } => {
                let t = parser::inlines_to_text(inlines);
                let t = t.trim();
                (!t.is_empty()).then(|| t.to_string())
            }
            _ => None,
        });
        Some(match heading {
            Some(h) => format!("{n}. {h}"),
            None => format!("Slide {n}"),
        })
    }

    /// Advance to the next reveal step or target slide. Returns false when finished.
    fn advance(&mut self) -> bool {
        if self.debug {
            let max = self.max_steps.get(self.current_slide).copied().unwrap_or(0);
            if self.current_step < max {
                self.current_step += 1;
                return true;
            }
            self.current_step = 0;
        }
        self.target_pos += 1;
        match self.targets.get(self.target_pos) {
            Some(&idx) => {
                self.current_slide = idx;
                true
            }
            None => false,
        }
    }

    fn fail(&mut self, ctx: &egui::Context, e: String) {
        // Stop at the first failure so it cannot be mistaken for success
        eprintln!("  {e}");
        *self.error.lock().unwrap_or_else(|p| p.into_inner()) = Some(e);
        self.done = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    /// Start a fresh canvas of `width`×`height` for the next page.
    fn next_canvas(&mut self, width: u32, height: u32) {
        self.canvas = TileCanvas::new(width, height);
        self.tile = (0, 0);
        self.frames_on_slide = 0;
    }

    /// Every tile of the current page is captured: deliver it and move on.
    /// Returns false when the export is finished.
    fn page_done(&mut self, ctx: &egui::Context) -> bool {
        match (&self.pass, &self.output) {
            (Pass::Slide, Output::Png { dir }) => {
                let filename = self.output_filename();
                if let Err(e) = self.canvas.save(&dir.join(&filename)) {
                    self.fail(ctx, e);
                    return false;
                }
                eprintln!("  Saved {filename}");
            }
            (Pass::Slide, Output::Pdf { doc, notes: false }) => {
                let bookmark = self.bookmark();
                doc.lock().unwrap_or_else(|p| p.into_inner()).add_page(
                    &self.canvas.pixels,
                    self.canvas.width,
                    self.canvas.height,
                    bookmark,
                );
                eprintln!("  Rendered slide {}", self.current_slide + 1);
            }
            (Pass::Slide, Output::Pdf { notes: true, .. }) => {
                // Keep the slide and lay out its notes page(s) next.
                let slide = self.presentation.slides.get(self.current_slide);
                let geo =
                    notes::Geometry::new(self.width, self.width, self.height, &self.notes_theme);
                let (w, h) = (geo.width, geo.height);
                self.notes = Some(NotesJob {
                    geo,
                    blocks: notes::blocks(slide.and_then(|s| s.notes.as_deref())),
                    pages: Vec::new(),
                    slide: std::mem::take(&mut self.canvas.pixels),
                });
                self.pass = Pass::Notes { page: 0 };
                self.next_canvas(w, h);
                return true;
            }
            (Pass::Notes { page }, Output::Pdf { doc, .. }) => {
                let page = *page;
                let Some(job) = self.notes.as_ref() else {
                    return false;
                };
                if page == 0 {
                    notes::composite(
                        &mut self.canvas.pixels,
                        self.canvas.width,
                        &job.slide,
                        self.width,
                        self.height,
                        job.geo.slide,
                    );
                }
                let bookmark = if page == 0 { self.bookmark() } else { None };
                doc.lock().unwrap_or_else(|p| p.into_inner()).add_page(
                    &self.canvas.pixels,
                    self.canvas.width,
                    self.canvas.height,
                    bookmark,
                );
                if page + 1 < job.pages.len() {
                    let (w, h) = (job.geo.width, job.geo.height);
                    self.pass = Pass::Notes { page: page + 1 };
                    self.next_canvas(w, h);
                    return true;
                }
                let pages = job.pages.len();
                eprintln!(
                    "  Rendered slide {} with notes{}",
                    self.current_slide + 1,
                    if pages > 1 {
                        format!(" ({pages} pages)")
                    } else {
                        String::new()
                    }
                );
                self.notes = None;
                self.pass = Pass::Slide;
            }
            (Pass::Notes { .. }, Output::Png { .. }) => unreachable!("notes pages are PDF only"),
        }
        let (w, h) = (self.width, self.height);
        self.next_canvas(w, h);
        self.advance()
    }

    fn draw_slide(&mut self, ui: &mut egui::Ui, origin: (u32, u32)) {
        let bg = self.theme.background;
        ui.painter().rect_filled(ui.max_rect(), 0.0, bg);

        // The full slide rect, shifted so the current tile is visible.
        let rect = egui::Rect::from_min_size(
            egui::pos2(-(origin.0 as f32), -(origin.1 as f32)),
            egui::vec2(self.width as f32, self.height as f32),
        );
        let scale = (rect.width() / 1920.0).min(rect.height() / 1080.0);

        let idx = self.current_slide;
        if idx >= self.presentation.slides.len() {
            return;
        }
        let reveal = if self.debug {
            self.current_step
        } else {
            self.max_steps.get(idx).copied().unwrap_or(0)
        };
        if self.theme.is_ember() {
            let slide = &self.presentation.slides[idx];
            let story = self
                .stories
                .get(idx)
                .and_then(|r| r.as_ref())
                .map(|r| r.script.clone());
            let theme = self.theme.clone();
            self.ember.frame(
                ui,
                rect,
                Some(slide),
                story.as_ref(),
                0,
                idx,
                reveal,
                false,
                None,
                &theme,
                scale,
                1.0,
                true,
                &mut self.illustrations,
            );
        }
        let cx = render::SlideContext {
            index: idx,
            count: self.presentation.slides.len(),
            deck_title: self.presentation.meta.title.clone(),
            author: self.presentation.meta.author.clone(),
            hold_copy: false,
            animate: false,
            beats: None,
            say: None,
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
            &cx,
        );
    }

    fn draw_notes(&mut self, ui: &egui::Ui, origin: (u32, u32), page: usize) {
        let n = self.current_slide + 1;
        let count = self.presentation.slides.len();
        let title = self.presentation.meta.title.clone().unwrap_or_default();
        if let Some(job) = self.notes.as_mut() {
            job.draw(ui, origin, page, &self.notes_theme, &title, n, count);
        }
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
        let tiles_x = tile_count(self.canvas.width, tile_w);
        let tiles_y = tile_count(self.canvas.height, tile_h);

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

            // Next tile, or finish the page once every tile is captured.
            let (tx, ty) = self.tile;
            if tx + 1 < tiles_x {
                self.tile = (tx + 1, ty);
            } else if ty + 1 < tiles_y {
                self.tile = (0, ty + 1);
            } else if !self.page_done(&ctx) {
                if !self.done {
                    self.done = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                return;
            }
        }

        let (tx, ty) = self.tile;
        let origin = (tx * tile_w, ty * tile_h);
        self.pending_tile_origin = origin;

        let bg = match self.pass {
            Pass::Slide => self.theme.background,
            Pass::Notes { .. } => egui::Color32::WHITE,
        };
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(bg).inner_margin(0.0))
            .show(root_ui, |ui| match self.pass {
                Pass::Slide => self.draw_slide(ui, origin),
                Pass::Notes { page } => {
                    ui.painter().rect_filled(ui.max_rect(), 0.0, bg);
                    self.draw_notes(ui, origin, page);
                }
            });

        // Request screenshot after rendering (will arrive next frame), but not
        // while images are still decoding in the background, and for Ember
        // slides only once the field has seen the slide's geometry.
        self.frames_on_slide += 1;
        let settled = match self.pass {
            Pass::Slide => !self.theme.is_ember() || self.frames_on_slide >= 2,
            Pass::Notes { .. } => true,
        };
        if !self.screenshot_requested && !self.image_cache.is_loading() && settled {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            self.screenshot_requested = true;
        }

        ctx.request_repaint();
    }
}
