//! The export window: renders each slide (and, for PDF with notes, each
//! notes page) at exactly one point per pixel, in window-sized tiles that are
//! stitched into a canvas of the requested size, and hands finished pages to
//! the output (PNG files or the PDF being assembled).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use eframe::egui;

use super::canvas::{TileCanvas, next_tile, tile_count};
use super::cursor::{Cursor, Job};
use super::notes::{self, Footer, NotesJob};
use super::pdf::PdfDoc;
use super::rehearsal::Rehearsal;
use crate::deck::{Deck, EngineFrame, SlideFrame};
use crate::parser;
use crate::render;
use crate::theme::Theme;

/// Frames to let the viewport settle (pixels-per-point change, fonts) before
/// the first screenshot is requested.
const WARMUP_FRAMES: u32 = 2;

/// Where finished pages go.
pub(super) enum Output {
    /// One PNG per slide (or per reveal step with `--debug`).
    Png { dir: PathBuf },
    /// Pages of a PDF, each slide followed by its notes pages when `notes`
    /// is set.
    Pdf {
        doc: Arc<Mutex<PdfDoc>>,
        notes: Option<Box<NotesPages>>,
    },
}

/// Speaker notes pages, printed in the light theme after each slide.
pub(super) struct NotesPages {
    theme: Theme,
    /// The current slide's notes and the page being drawn, once its slide
    /// has been captured.
    current: Option<(NotesJob, usize)>,
}

impl NotesPages {
    pub(super) fn new() -> Self {
        Self {
            theme: Theme::light(),
            current: None,
        }
    }
}

pub(super) struct ExportApp {
    deck: Deck,
    theme: Theme,
    output: Output,
    width: u32,
    height: u32,
    canvas: TileCanvas,
    cursor: Cursor,
    /// Tile currently being rendered (column, row).
    tile: (u32, u32),
    /// Canvas offset of the tile whose screenshot is pending.
    pending_tile_origin: (u32, u32),
    screenshot_requested: bool,
    warmup_frames: u32,
    done: bool,
    /// First error, shared with `run()` so the exit code reflects it.
    error: Arc<Mutex<Option<String>>>,
    /// Frames rendered for the current page; content hints arrive one frame
    /// late, so the screenshot waits for the engine's settle frames.
    frames_on_slide: u32,
    /// `--at` / `--moment`: a still of the engine's motion.
    rehearsal: Rehearsal,
    /// `--presenter-view`: draw the presenter's cockpit instead of slides.
    presenter_view: bool,
}

impl ExportApp {
    pub(super) fn new(
        deck: Deck,
        theme: Theme,
        output: Output,
        job: Job,
        error: Arc<Mutex<Option<String>>>,
    ) -> Self {
        if job.debug {
            let total: usize = job
                .targets
                .iter()
                .map(|&i| deck.max_steps.get(i).copied().unwrap_or(0) + 1)
                .sum();
            eprintln!("  {total} reveal steps in total");
        }
        Self {
            deck,
            theme,
            output,
            width: job.width,
            height: job.height,
            canvas: TileCanvas::new(job.width, job.height),
            cursor: Cursor::new(job.targets, job.debug),
            tile: (0, 0),
            pending_tile_origin: (0, 0),
            screenshot_requested: false,
            warmup_frames: WARMUP_FRAMES,
            done: false,
            error,
            frames_on_slide: 0,
            rehearsal: job.rehearsal,
            presenter_view: job.presenter_view,
        }
    }

    /// The current slide's last reveal step.
    fn max_step(&self) -> usize {
        self.deck
            .max_steps
            .get(self.cursor.slide())
            .copied()
            .unwrap_or(0)
    }

    /// The notes page being drawn, if the canvas holds one.
    fn notes_page(&self) -> Option<usize> {
        match &self.output {
            Output::Pdf { notes: Some(n), .. } => n.current.as_ref().map(|(_, page)| *page),
            _ => None,
        }
    }

    /// File name for the current slide/step, zero-padded to the deck size.
    fn output_filename(&self) -> String {
        super::export_filename(
            self.cursor.slide(),
            self.deck.slide_count(),
            self.cursor.debug.then_some(self.cursor.step),
            self.deck.max_steps.iter().copied().max().unwrap_or(0),
        )
    }

    /// Outline entry for the current slide: its first heading, else its number.
    /// Only the first page of a slide gets one.
    fn bookmark(&self) -> Option<String> {
        if self.cursor.debug && self.cursor.step > 0 {
            return None;
        }
        let index = self.cursor.slide();
        let slide = self.deck.presentation.slides.get(index)?;
        let n = index + 1;
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
        let bookmark = self.bookmark();
        let slide = self.cursor.slide();
        let filename = self.output_filename();
        let (width, height) = (self.width, self.height);
        match &mut self.output {
            Output::Png { dir } => {
                if let Err(e) = self.canvas.save(&dir.join(&filename)) {
                    self.fail(ctx, e.to_string());
                    return false;
                }
                eprintln!("  Saved {filename}");
            }
            Output::Pdf { doc, notes: None } => {
                add_page(doc, &self.canvas, bookmark);
                eprintln!("  Rendered slide {}", slide + 1);
            }
            Output::Pdf {
                notes: Some(notes), ..
            } if notes.current.is_none() => {
                // Keep the slide and lay out its notes page(s) next.
                let geo = notes::Geometry::new(width, width, height, &notes.theme);
                let (w, h) = (geo.width, geo.height);
                let text = self
                    .deck
                    .presentation
                    .slides
                    .get(slide)
                    .and_then(|s| s.notes.as_deref());
                let job = NotesJob {
                    geo,
                    blocks: notes::blocks(text),
                    pages: Vec::new(),
                    slide: std::mem::take(&mut self.canvas.pixels),
                };
                notes.current = Some((job, 0));
                self.next_canvas(w, h);
                return true;
            }
            Output::Pdf {
                doc,
                notes: Some(notes),
            } => {
                let Some((job, page)) = notes.current.as_mut() else {
                    return false;
                };
                if *page == 0 {
                    notes::composite(
                        &mut self.canvas.pixels,
                        self.canvas.width,
                        &job.slide,
                        width,
                        height,
                        job.geo.slide,
                    );
                }
                add_page(doc, &self.canvas, if *page == 0 { bookmark } else { None });
                if *page + 1 < job.pages.len() {
                    *page += 1;
                    let (w, h) = (job.geo.width, job.geo.height);
                    self.next_canvas(w, h);
                    return true;
                }
                let pages = job.pages.len();
                eprintln!(
                    "  Rendered slide {} with notes{}",
                    slide + 1,
                    if pages > 1 {
                        format!(" ({pages} pages)")
                    } else {
                        String::new()
                    }
                );
                notes.current = None;
            }
        }
        self.next_canvas(width, height);
        let max_step = self.max_step();
        self.cursor.advance(max_step)
    }

    fn draw_slide(&mut self, ui: &mut egui::Ui, origin: (u32, u32)) {
        // export shows each block's palette as written
        let palette = self.deck.presentation.meta.palette.as_deref();
        render::thermal::set_deck_palette(
            ui.ctx(),
            palette.and_then(render::thermal::Palette::from_name),
        );
        render::thermal::set_live_palette(ui.ctx(), None);
        let bg = self.theme.background;
        ui.painter().rect_filled(ui.max_rect(), 0.0, bg);

        // The full slide rect, shifted so the current tile is visible.
        let rect = egui::Rect::from_min_size(
            egui::pos2(-(origin.0 as f32), -(origin.1 as f32)),
            egui::vec2(self.width as f32, self.height as f32),
        );
        let scale = (rect.width() / 1920.0).min(rect.height() / 1080.0);
        // A theme's page puts the slide on a sheet.
        let rect = render::page::draw(ui.painter(), rect, &self.theme, scale);
        let scale = (rect.width() / 1920.0).min(rect.height() / 1080.0);

        let idx = self.cursor.slide();
        if idx >= self.deck.slide_count() {
            return;
        }
        let reveal = self.cursor.reveal(self.max_step());
        if self.presenter_view {
            let full = egui::Rect::from_min_size(
                egui::pos2(-(origin.0 as f32), -(origin.1 as f32)),
                egui::vec2(self.width as f32, self.height as f32),
            );
            let view = crate::app::presenter::View {
                deck: &self.deck,
                theme: &self.theme,
                index: idx,
                reveal,
                end: false,
                // a fixed time, so the export is reproducible
                elapsed: std::time::Duration::from_secs(754),
            };
            crate::app::presenter::draw(ui, full, &view);
            return;
        }
        let radius = self.theme.page.as_ref().map_or(0.0, |p| p.radius * scale);
        self.deck.draw_background(
            &ui.painter().with_clip_rect(rect),
            rect,
            idx,
            1.0,
            radius,
            false,
        );
        // Stills of an engine's motion (see doc/engines.md): `--at <seconds>`
        // rehearses the engine from a cold start, `--moment countdown|end`
        // shows the countdown or the end.
        let rehearsal = self.rehearsal;
        if self.theme.engine.paints() {
            let frame = EngineFrame {
                rect,
                scale,
                index: idx,
                reveal,
                end: rehearsal.end,
                countdown: rehearsal.countdown,
                still: true,
            };
            self.deck.engine_layer(ui, &self.theme, frame, rehearsal.at);
        }
        if rehearsal.moment() {
            return;
        }
        let cx = render::SlideContext {
            index: idx,
            count: self.deck.slide_count(),
            deck_title: self.deck.presentation.meta.title.clone(),
            author: self.deck.presentation.meta.author.clone(),
            hold_copy: false,
            animate: false,
            engine_drew: true,
        };
        let frame = SlideFrame {
            rect,
            opacity: 1.0,
            reveal,
            reveal_timestamp: None, // no animation in export
            scale,
        };
        self.deck.draw_slide(ui, &self.theme, idx, frame, &cx);
        self.deck.draw_logo(ui.painter(), rect, idx, scale);
        // the footer and counter, as the window draws them
        self.deck
            .draw_chrome(ui.painter(), &self.theme, rect, &cx, scale);
    }

    fn draw_notes(&mut self, ui: &egui::Ui, origin: (u32, u32)) {
        let footer = Footer {
            deck_title: self.deck.presentation.meta.title.as_deref().unwrap_or(""),
            slide: self.cursor.slide() + 1,
            count: self.deck.slide_count(),
        };
        if let Output::Pdf {
            notes: Some(notes), ..
        } = &mut self.output
            && let Some((job, page)) = notes.current.as_mut()
        {
            job.draw(ui, origin, *page, &notes.theme, &footer);
        }
    }
}

/// Append the canvas to the PDF as a page.
fn add_page(doc: &Arc<Mutex<PdfDoc>>, canvas: &TileCanvas, bookmark: Option<String>) {
    doc.lock().unwrap_or_else(|p| p.into_inner()).add_page(
        &canvas.pixels,
        canvas.width,
        canvas.height,
        bookmark,
    );
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
        let tiles = (
            tile_count(self.canvas.width, tile_w),
            tile_count(self.canvas.height, tile_h),
        );

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
            match next_tile(self.tile, tiles) {
                Some(tile) => self.tile = tile,
                None => {
                    if !self.page_done(&ctx) {
                        if !self.done {
                            self.done = true;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        return;
                    }
                }
            }
        }

        let (tx, ty) = self.tile;
        let origin = (tx * tile_w, ty * tile_h);
        self.pending_tile_origin = origin;

        let notes_page = self.notes_page().is_some();
        let bg = if notes_page {
            egui::Color32::WHITE
        } else {
            self.theme.background
        };
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(bg).inner_margin(0.0))
            .show(root_ui, |ui| {
                if notes_page {
                    ui.painter().rect_filled(ui.max_rect(), 0.0, bg);
                    self.draw_notes(ui, origin);
                } else {
                    self.draw_slide(ui, origin);
                }
            });

        // Request screenshot after rendering (will arrive next frame), but not
        // while images are still decoding in the background, and for engine
        // slides only once the engine has seen the slide's geometry.
        self.frames_on_slide += 1;
        let settled = notes_page
            || self.presenter_view
            || self.frames_on_slide >= self.theme.engine.settle_frames();
        if !self.screenshot_requested && !self.deck.image_cache.is_loading() && settled {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            self.screenshot_requested = true;
        }

        ctx.request_repaint();
    }
}
