use eframe::egui;
use std::path::PathBuf;
use std::time::Instant;

use crate::parser::{self, Block, Inline, Presentation};
use crate::render;
use crate::render::image_cache::ImageCache;
use crate::render::transition::{ActiveTransition, TransitionDirection, TransitionKind};
use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq)]
enum AppMode {
    Presentation,
    Grid { selected: usize },
}

struct PresentationApp {
    presentation: Presentation,
    #[allow(dead_code)]
    file_path: PathBuf,
    current_slide: usize,
    mode: AppMode,
    theme: Theme,
    default_transition: TransitionKind,
    transition: Option<ActiveTransition>,
    image_cache: ImageCache,
    show_hud: bool,
    toast: Option<Toast>,
    last_ctrl_c: Option<Instant>,
    reveal_steps: Vec<usize>,
    max_steps: Vec<usize>,
    frame_count: u32,
    fps: f32,
    fps_update: Instant,
}

struct Toast {
    message: String,
    start: Instant,
}

impl Toast {
    fn new(message: String) -> Self {
        Self {
            message,
            start: Instant::now(),
        }
    }

    fn opacity(&self) -> f32 {
        let elapsed = self.start.elapsed().as_secs_f32();
        let duration = 1.5;
        let fade_start = 1.0;
        if elapsed < fade_start {
            1.0
        } else if elapsed < duration {
            1.0 - (elapsed - fade_start) / (duration - fade_start)
        } else {
            0.0
        }
    }

    fn is_expired(&self) -> bool {
        self.start.elapsed().as_secs_f32() >= 1.5
    }
}

impl PresentationApp {
    fn new(file: PathBuf, presentation: Presentation, windowed: bool) -> Self {
        let _ = windowed; // used at window creation time

        let theme_name = presentation.meta.theme.as_deref().unwrap_or("light");
        let theme = Theme::from_name(theme_name);

        let transition_name = presentation.meta.transition.as_deref().unwrap_or("slide");
        let default_transition = TransitionKind::from_name(transition_name);

        let base_path = file
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .to_path_buf();
        let image_cache = ImageCache::new(base_path);

        let max_steps: Vec<usize> = presentation
            .slides
            .iter()
            .map(|s| parser::compute_max_steps(&s.blocks))
            .collect();
        let reveal_steps = vec![0; presentation.slides.len()];

        let now = Instant::now();
        Self {
            presentation,
            file_path: file,
            current_slide: 0,
            mode: AppMode::Presentation,
            theme,
            default_transition,
            transition: None,
            image_cache,
            show_hud: false,
            toast: None,
            last_ctrl_c: None,
            reveal_steps,
            max_steps,
            frame_count: 0,
            fps: 0.0,
            fps_update: now,
        }
    }

    fn slide_count(&self) -> usize {
        self.presentation.slides.len()
    }

    fn navigate_forward(&mut self) {
        if self.transition.is_some() {
            return;
        }

        let idx = self.current_slide;

        // If we have reveal steps remaining, reveal next item
        if self.reveal_steps[idx] < self.max_steps[idx] {
            self.reveal_steps[idx] += 1;
            return;
        }

        // Otherwise advance to the next slide
        if idx >= self.slide_count().saturating_sub(1) {
            return;
        }

        self.transition = Some(ActiveTransition::new(
            idx,
            idx + 1,
            self.default_transition,
            TransitionDirection::Forward,
        ));
    }

    fn navigate_backward(&mut self) {
        if self.transition.is_some() {
            return;
        }

        let idx = self.current_slide;

        // If we've revealed items, un-reveal
        if self.reveal_steps[idx] > 0 {
            self.reveal_steps[idx] -= 1;
            return;
        }

        // Otherwise go to previous slide (fully revealed)
        if idx == 0 {
            return;
        }

        let prev = idx - 1;
        // Show previous slide fully revealed
        self.reveal_steps[prev] = self.max_steps[prev];

        self.transition = Some(ActiveTransition::new(
            idx,
            prev,
            self.default_transition,
            TransitionDirection::Backward,
        ));
    }

    fn jump_to_slide(&mut self, index: usize) {
        if index < self.slide_count() && self.transition.is_none() {
            self.current_slide = index;
        }
    }

    fn toggle_theme(&mut self) {
        self.theme = self.theme.toggled();
        self.toast = Some(Toast::new(format!("Theme: {}", self.theme.name)));
    }

    fn update_fps(&mut self) {
        self.frame_count += 1;
        let elapsed = self.fps_update.elapsed().as_secs_f32();
        if elapsed >= 0.5 {
            self.fps = self.frame_count as f32 / elapsed;
            self.frame_count = 0;
            self.fps_update = Instant::now();
        }
    }

    fn draw_slide(&self, ui: &egui::Ui, index: usize, rect: egui::Rect, opacity: f32, scale: f32) {
        if index < self.presentation.slides.len() {
            let reveal = self.reveal_steps.get(index).copied().unwrap_or(0);
            render::render_slide(
                ui,
                &self.presentation.slides[index],
                &self.theme,
                rect,
                opacity,
                &self.image_cache,
                reveal,
                scale,
            );
        }
    }

    fn grid_columns(&self) -> usize {
        let count = self.slide_count();
        if count <= 4 {
            2
        } else if count <= 9 {
            3
        } else {
            4
        }
    }

    fn compute_scale(rect: egui::Rect) -> f32 {
        let ref_w = 1920.0;
        let ref_h = 1080.0;
        (rect.width() / ref_w).min(rect.height() / ref_h)
    }
}

impl eframe::App for PresentationApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_fps();

        let mode = self.mode;

        // Collect viewport commands to send AFTER the input closure
        // (sending inside ctx.input() causes RwLock deadlock)
        let mut viewport_cmds: Vec<egui::ViewportCommand> = Vec::new();

        // Handle keyboard input
        ctx.input(|i| {
            // Quit: Q from any mode
            if i.key_pressed(egui::Key::Q) {
                viewport_cmds.push(egui::ViewportCommand::Close);
                return;
            }

            // Ctrl+C double-tap to quit
            if i.modifiers.ctrl && i.key_pressed(egui::Key::C) {
                if let Some(last) = self.last_ctrl_c {
                    if last.elapsed().as_secs_f32() < 1.0 {
                        viewport_cmds.push(egui::ViewportCommand::Close);
                        return;
                    }
                }
                self.last_ctrl_c = Some(Instant::now());
                self.toast = Some(Toast::new("Press Ctrl+C again to quit".to_string()));
                return;
            }

            match mode {
                AppMode::Presentation => {
                    // Forward: Right, N, Space
                    if i.key_pressed(egui::Key::ArrowRight)
                        || i.key_pressed(egui::Key::N)
                        || i.key_pressed(egui::Key::Space)
                    {
                        self.navigate_forward();
                    }
                    // Backward: Left, P
                    if i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::P) {
                        self.navigate_backward();
                    }
                    // Toggle theme: T
                    if i.key_pressed(egui::Key::T) {
                        self.toggle_theme();
                    }
                    // Toggle HUD: H
                    if i.key_pressed(egui::Key::H) {
                        self.show_hud = !self.show_hud;
                    }
                    // Fullscreen toggle: F
                    if i.key_pressed(egui::Key::F) {
                        viewport_cmds.push(egui::ViewportCommand::Fullscreen(
                            !i.viewport().fullscreen.unwrap_or(false),
                        ));
                    }
                    // Home/End
                    if i.key_pressed(egui::Key::Home) {
                        self.jump_to_slide(0);
                    }
                    if i.key_pressed(egui::Key::End) {
                        self.jump_to_slide(self.slide_count().saturating_sub(1));
                    }
                    // ESC: enter grid mode and exit fullscreen
                    if i.key_pressed(egui::Key::Escape) {
                        self.mode = AppMode::Grid {
                            selected: self.current_slide,
                        };
                        self.show_hud = false;
                        // Exit fullscreen when entering grid
                        viewport_cmds.push(egui::ViewportCommand::Fullscreen(false));
                    }
                }
                AppMode::Grid { selected } => {
                    let cols = self.grid_columns();
                    let count = self.slide_count();

                    // Arrow navigation in grid
                    if i.key_pressed(egui::Key::ArrowRight) {
                        let next = (selected + 1).min(count.saturating_sub(1));
                        self.mode = AppMode::Grid { selected: next };
                    }
                    if i.key_pressed(egui::Key::ArrowLeft) {
                        let prev = selected.saturating_sub(1);
                        self.mode = AppMode::Grid { selected: prev };
                    }
                    if i.key_pressed(egui::Key::ArrowDown) {
                        let next = (selected + cols).min(count.saturating_sub(1));
                        self.mode = AppMode::Grid { selected: next };
                    }
                    if i.key_pressed(egui::Key::ArrowUp) {
                        let prev = selected.saturating_sub(cols);
                        self.mode = AppMode::Grid { selected: prev };
                    }

                    // Enter: jump to selected slide
                    if i.key_pressed(egui::Key::Enter) {
                        self.current_slide = selected;
                        self.mode = AppMode::Presentation;
                    }
                    // ESC: back to presentation mode
                    if i.key_pressed(egui::Key::Escape) {
                        self.mode = AppMode::Presentation;
                    }
                    // Theme toggle works in grid too
                    if i.key_pressed(egui::Key::T) {
                        self.toggle_theme();
                    }
                }
            }
        });

        // Send collected viewport commands outside the input closure
        for cmd in viewport_cmds {
            ctx.send_viewport_cmd(cmd);
        }

        // Advance transition
        if let Some(ref t) = self.transition {
            if t.is_complete() {
                let to = t.to;
                self.transition = None;
                self.current_slide = to;
            }
        }

        // Expire toast
        if self.toast.as_ref().is_some_and(|t| t.is_expired()) {
            self.toast = None;
        }

        let bg = self.theme.background;

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(bg).inner_margin(0.0))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                ui.painter().rect_filled(rect, 0.0, bg);

                let scale = Self::compute_scale(rect);

                match self.mode {
                    AppMode::Presentation => {
                        self.draw_presentation(ui, ctx, rect, scale);
                    }
                    AppMode::Grid { selected } => {
                        self.draw_grid(ui, rect, selected, scale);
                    }
                }

                // Toast notification (shown in both modes)
                if let Some(ref toast) = self.toast {
                    let opacity = toast.opacity();
                    if opacity > 0.0 {
                        let toast_color = Theme::with_opacity(self.theme.foreground, opacity * 0.9);
                        let toast_bg =
                            Theme::with_opacity(self.theme.code_background, opacity * 0.9);
                        let galley = ui.painter().layout_no_wrap(
                            toast.message.clone(),
                            egui::FontId::proportional(20.0 * scale),
                            toast_color,
                        );
                        let padding = 16.0 * scale;
                        let toast_rect = egui::Rect::from_min_size(
                            egui::pos2(
                                rect.center().x - galley.rect.width() / 2.0 - padding,
                                rect.bottom() - 80.0 * scale,
                            ),
                            egui::vec2(
                                galley.rect.width() + padding * 2.0,
                                galley.rect.height() + padding * 2.0,
                            ),
                        );
                        ui.painter().rect_filled(toast_rect, 8.0 * scale, toast_bg);
                        let text_pos =
                            egui::pos2(toast_rect.left() + padding, toast_rect.top() + padding);
                        ui.painter().galley(text_pos, galley, toast_color);
                        ctx.request_repaint();
                    }
                }

                // HUD overlay (presentation mode only)
                if self.show_hud && self.mode == AppMode::Presentation {
                    draw_hud(ui, &self.theme, rect, scale);
                }
            });
    }
}

impl PresentationApp {
    fn draw_presentation(&self, ui: &egui::Ui, ctx: &egui::Context, rect: egui::Rect, scale: f32) {
        if let Some(ref t) = self.transition {
            let kind = t.kind;
            let from = t.from;
            let to = t.to;
            let progress = t.progress();
            let direction = t.direction;

            match kind {
                TransitionKind::Fade => {
                    self.draw_slide(ui, from, rect, 1.0 - progress, scale);
                    self.draw_slide(ui, to, rect, progress, scale);
                }
                TransitionKind::SlideHorizontal => {
                    let w = rect.width();
                    let sign = match direction {
                        TransitionDirection::Forward => -1.0,
                        TransitionDirection::Backward => 1.0,
                    };
                    let from_offset = sign * progress * w;
                    let to_offset = from_offset - sign * w;

                    let from_rect = rect.translate(egui::vec2(from_offset, 0.0));
                    let to_rect = rect.translate(egui::vec2(to_offset, 0.0));

                    self.draw_slide(ui, from, from_rect, 1.0, scale);
                    self.draw_slide(ui, to, to_rect, 1.0, scale);
                }
                TransitionKind::None => {
                    self.draw_slide(ui, to, rect, 1.0, scale);
                }
            }
            ctx.request_repaint();
        } else {
            self.draw_slide(ui, self.current_slide, rect, 1.0, scale);
        }

        // Footer
        if let Some(ref footer) = self.presentation.meta.footer {
            let footer_color = Theme::with_opacity(self.theme.foreground, 0.4);
            let galley = ui.painter().layout_no_wrap(
                footer.clone(),
                egui::FontId::proportional(14.0 * scale),
                footer_color,
            );
            let pos = egui::pos2(
                rect.center().x - galley.rect.width() / 2.0,
                rect.bottom() - 30.0 * scale,
            );
            ui.painter().galley(pos, galley, footer_color);
        }

        // Slide counter
        let counter_text = format!("{} / {}", self.current_slide + 1, self.slide_count());
        let counter_color = Theme::with_opacity(self.theme.foreground, 0.3);
        let counter_galley = ui.painter().layout_no_wrap(
            counter_text,
            egui::FontId::monospace(14.0 * scale),
            counter_color,
        );
        let counter_pos = egui::pos2(
            rect.right() - counter_galley.rect.width() - 16.0 * scale,
            rect.bottom() - 30.0 * scale,
        );
        ui.painter()
            .galley(counter_pos, counter_galley, counter_color);

        // FPS overlay
        let fps_text = format!("{:.0} fps", self.fps);
        let fps_color = Theme::with_opacity(self.theme.foreground, 0.3);
        let fps_galley =
            ui.painter()
                .layout_no_wrap(fps_text, egui::FontId::monospace(14.0 * scale), fps_color);
        let fps_pos = egui::pos2(
            rect.right() - fps_galley.rect.width() - 12.0 * scale,
            rect.top() + 10.0 * scale,
        );
        ui.painter().galley(fps_pos, fps_galley, fps_color);
    }

    fn draw_grid(&self, ui: &egui::Ui, rect: egui::Rect, selected: usize, scale: f32) {
        let cols = self.grid_columns();
        let count = self.slide_count();
        let rows = count.div_ceil(cols);

        let padding = 24.0 * scale;
        let gap = 12.0 * scale;

        // Title
        let title_color = Theme::with_opacity(self.theme.heading_color, 0.9);
        let title_galley = ui.painter().layout_no_wrap(
            "Slide Overview".to_string(),
            egui::FontId::proportional(24.0 * scale),
            title_color,
        );
        let title_pos = egui::pos2(rect.left() + padding, rect.top() + padding);
        ui.painter().galley(title_pos, title_galley, title_color);

        let grid_top = rect.top() + padding + 40.0 * scale;
        let grid_width = rect.width() - padding * 2.0;
        let grid_height = rect.bottom() - grid_top - padding;

        let cell_width = (grid_width - gap * (cols as f32 - 1.0)) / cols as f32;
        // 16:9 aspect ratio for cells
        let cell_height_max = (grid_height - gap * (rows as f32 - 1.0)) / rows as f32;
        let cell_height = cell_height_max.min(cell_width * 9.0 / 16.0);

        let text_color = Theme::with_opacity(self.theme.foreground, 0.7);
        let selected_border = self.theme.accent;
        let cell_bg = Theme::with_opacity(self.theme.code_background, 0.5);

        for i in 0..count {
            let col = i % cols;
            let row = i / cols;
            let x = rect.left() + padding + col as f32 * (cell_width + gap);
            let y = grid_top + row as f32 * (cell_height + gap);

            let cell_rect =
                egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(cell_width, cell_height));

            // Background
            ui.painter().rect_filled(cell_rect, 4.0 * scale, cell_bg);

            // Selected border
            if i == selected {
                ui.painter().rect_stroke(
                    cell_rect,
                    4.0 * scale,
                    egui::Stroke::new(3.0 * scale, selected_border),
                    egui::StrokeKind::Outside,
                );
            }

            // Slide number
            let num_galley = ui.painter().layout_no_wrap(
                format!("{}", i + 1),
                egui::FontId::monospace(14.0 * scale),
                text_color,
            );
            ui.painter().galley(
                egui::pos2(
                    cell_rect.left() + 8.0 * scale,
                    cell_rect.top() + 6.0 * scale,
                ),
                num_galley,
                text_color,
            );

            // First heading text (if any)
            if let Some(heading_text) = self.slide_heading_text(i) {
                let heading_color = Theme::with_opacity(self.theme.heading_color, 0.8);
                let max_text_width = cell_width - 16.0 * scale;
                let galley = ui.painter().layout(
                    heading_text,
                    egui::FontId::proportional(14.0 * scale),
                    heading_color,
                    max_text_width,
                );
                ui.painter().galley(
                    egui::pos2(
                        cell_rect.left() + 8.0 * scale,
                        cell_rect.top() + 26.0 * scale,
                    ),
                    galley,
                    heading_color,
                );
            }

            // Layout type label at bottom
            let layout_label = format!("{:?}", self.presentation.slides[i].layout);
            let label_color = Theme::with_opacity(self.theme.foreground, 0.3);
            let label_galley = ui.painter().layout_no_wrap(
                layout_label,
                egui::FontId::monospace(11.0 * scale),
                label_color,
            );
            ui.painter().galley(
                egui::pos2(
                    cell_rect.left() + 8.0 * scale,
                    cell_rect.bottom() - label_galley.rect.height() - 6.0 * scale,
                ),
                label_galley,
                label_color,
            );
        }

        // Navigation hint at bottom
        let hint = "Arrow keys: navigate  |  Enter: select  |  Esc: back  |  Q: quit";
        let hint_color = Theme::with_opacity(self.theme.foreground, 0.4);
        let hint_galley = ui.painter().layout_no_wrap(
            hint.to_string(),
            egui::FontId::proportional(14.0 * scale),
            hint_color,
        );
        let hint_pos = egui::pos2(
            rect.center().x - hint_galley.rect.width() / 2.0,
            rect.bottom() - 30.0 * scale,
        );
        ui.painter().galley(hint_pos, hint_galley, hint_color);
    }

    fn slide_heading_text(&self, index: usize) -> Option<String> {
        if index >= self.presentation.slides.len() {
            return None;
        }
        for block in &self.presentation.slides[index].blocks {
            if let Block::Heading { inlines, .. } = block {
                let text = inlines_to_text(inlines);
                if !text.is_empty() {
                    return Some(text);
                }
            }
        }
        None
    }
}

fn inlines_to_text(inlines: &[Inline]) -> String {
    parser::inlines_to_text(inlines)
}

fn draw_hud(ui: &egui::Ui, theme: &Theme, rect: egui::Rect, scale: f32) {
    let shortcuts = [
        ("Space / N / \u{2192}", "Next slide / reveal"),
        ("P / \u{2190}", "Previous slide / hide"),
        ("T", "Toggle theme"),
        ("F", "Toggle fullscreen"),
        ("H", "Toggle this HUD"),
        ("Esc", "Slide overview"),
        ("Q", "Quit"),
        ("Home", "First slide"),
        ("End", "Last slide"),
    ];

    let bg = Theme::with_opacity(theme.code_background, 0.9);
    let text_color = Theme::with_opacity(theme.foreground, 0.9);
    let key_color = Theme::with_opacity(theme.accent, 0.9);

    let padding = 24.0 * scale;
    let line_height = 32.0 * scale;
    let hud_height = shortcuts.len() as f32 * line_height + padding * 2.0 + 40.0 * scale;
    let hud_width = 360.0 * scale;

    let hud_rect = egui::Rect::from_center_size(rect.center(), egui::vec2(hud_width, hud_height));

    ui.painter().rect_filled(hud_rect, 12.0 * scale, bg);

    // Title
    let title_galley = ui.painter().layout_no_wrap(
        "Keyboard Shortcuts".to_string(),
        egui::FontId::proportional(20.0 * scale),
        Theme::with_opacity(theme.heading_color, 0.9),
    );
    let title_pos = egui::pos2(hud_rect.left() + padding, hud_rect.top() + padding);
    ui.painter().galley(title_pos, title_galley, text_color);

    let mut y = hud_rect.top() + padding + 40.0 * scale;

    for (key, desc) in &shortcuts {
        let key_galley = ui.painter().layout_no_wrap(
            key.to_string(),
            egui::FontId::monospace(15.0 * scale),
            key_color,
        );
        ui.painter().galley(
            egui::pos2(hud_rect.left() + padding, y),
            key_galley,
            key_color,
        );

        let desc_galley = ui.painter().layout_no_wrap(
            desc.to_string(),
            egui::FontId::proportional(15.0 * scale),
            text_color,
        );
        ui.painter().galley(
            egui::pos2(hud_rect.left() + padding + 170.0 * scale, y),
            desc_galley,
            text_color,
        );

        y += line_height;
    }
}

pub fn run(file: PathBuf, windowed: bool) -> anyhow::Result<()> {
    let content = std::fs::read_to_string(&file)?;
    let base_path = file.parent().unwrap_or(std::path::Path::new("."));
    let presentation = parser::parse(&content, base_path);

    if presentation.slides.is_empty() {
        anyhow::bail!("No slides found in {}", file.display());
    }

    let title = presentation.meta.title.clone().unwrap_or_else(|| {
        format!(
            "presemd \u{2014} {}",
            file.file_name().unwrap_or_default().to_string_lossy()
        )
    });

    let viewport = if windowed {
        egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_title(&title)
    } else {
        egui::ViewportBuilder::default()
            .with_fullscreen(true)
            .with_title(&title)
    };

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        &title,
        options,
        Box::new(move |_cc| Ok(Box::new(PresentationApp::new(file, presentation, windowed)))),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))
}
