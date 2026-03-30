use eframe::egui;
use std::time::Instant;

use crate::render;
use crate::theme::Theme;

use super::{
    ActiveDraw, DRAW_FADE_DURATION, OVERVIEW_TRANSITION_DURATION, PresentationApp, RawOverlaySide,
};

impl PresentationApp {
    pub(super) fn draw_slide(
        &self,
        ui: &egui::Ui,
        index: usize,
        rect: egui::Rect,
        opacity: f32,
        scale: f32,
    ) {
        if index < self.presentation.slides.len() {
            let reveal = self.reveal_steps.get(index).copied().unwrap_or(0);
            let timestamp = self.reveal_timestamps.get(index).copied().flatten();
            render::render_slide(
                ui,
                &self.presentation.slides[index],
                &self.theme,
                rect,
                opacity,
                &self.image_cache,
                reveal,
                timestamp,
                scale,
            );
        }
    }

    /// Draw a slide at full reveal (all steps visible). Used by grid view.
    pub(super) fn draw_slide_fully_revealed(
        &self,
        ui: &egui::Ui,
        index: usize,
        rect: egui::Rect,
        opacity: f32,
        scale: f32,
    ) {
        if index < self.presentation.slides.len() {
            let reveal = self.max_steps.get(index).copied().unwrap_or(0);
            render::render_slide(
                ui,
                &self.presentation.slides[index],
                &self.theme,
                rect,
                opacity,
                &self.image_cache,
                reveal,
                None,
                scale,
            );
        }
    }

    pub(super) fn draw_end_slide(&mut self, ui: &egui::Ui, rect: egui::Rect, scale: f32) {
        // Draw ESC hint at top like regular slides
        let hint_color = egui::Color32::from_gray(100);
        let hint_galley = ui.painter().layout_no_wrap(
            "Press ESC to exit".to_string(),
            egui::FontId::proportional(14.0 * scale),
            hint_color,
        );
        ui.painter().galley(
            egui::pos2(
                rect.center().x - hint_galley.rect.width() / 2.0,
                rect.top() + 20.0 * scale,
            ),
            hint_galley,
            hint_color,
        );

        // "The End" centered — large enough to read from distance
        let title_color = egui::Color32::from_gray(220);
        let galley = ui.painter().layout_no_wrap(
            "The End".to_string(),
            egui::FontId::proportional(140.0 * scale),
            title_color,
        );
        let title_pos = egui::pos2(
            rect.center().x - galley.rect.width() / 2.0,
            rect.center().y - galley.rect.height() / 2.0 - 40.0 * scale,
        );
        ui.painter().galley(title_pos, galley, title_color);

        // Bottom-right attribution block: logo + text
        let margin = 32.0 * scale;
        let logo_height = 48.0 * scale;

        // Load logo texture lazily
        if self.end_logo_texture.is_none() {
            static LOGO_BYTES: &[u8] = include_bytes!("../../media/logo-small.png");
            if let Ok(img) = image::load_from_memory(LOGO_BYTES) {
                let rgba = img.to_rgba8();
                let (w, h) = (rgba.width() as usize, rgba.height() as usize);
                let pixels = rgba.into_raw();
                let color_image = egui::ColorImage::from_rgba_unmultiplied([w, h], &pixels);
                let texture = ui.ctx().load_texture(
                    "mdeck-end-logo",
                    color_image,
                    egui::TextureOptions::LINEAR,
                );
                self.end_logo_texture = Some(texture);
            }
        }

        let text_color = egui::Color32::from_gray(140);
        let url_color = egui::Color32::from_gray(100);

        let powered_galley = ui.painter().layout_no_wrap(
            "Powered by MDeck".to_string(),
            egui::FontId::proportional(14.0 * scale),
            text_color,
        );
        let url_galley = ui.painter().layout_no_wrap(
            "https://github.com/mklab-se/mdeck".to_string(),
            egui::FontId::proportional(11.0 * scale),
            url_color,
        );

        // Position: bottom-right corner
        let text_block_width = powered_galley.rect.width().max(url_galley.rect.width());
        let logo_aspect = 192.0 / 128.0; // width/height of the embedded logo
        let logo_width = logo_height * logo_aspect;

        let block_width = logo_width + 10.0 * scale + text_block_width;
        let block_x = rect.right() - margin - block_width;
        let block_y = rect.bottom() - margin - logo_height;

        // Draw logo
        if let Some(ref texture) = self.end_logo_texture {
            let logo_rect = egui::Rect::from_min_size(
                egui::pos2(block_x, block_y),
                egui::vec2(logo_width, logo_height),
            );
            // Rounded clip for the logo
            ui.painter()
                .rect_filled(logo_rect, 6.0 * scale, egui::Color32::from_gray(30));
            let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            ui.painter()
                .image(texture.id(), logo_rect, uv, egui::Color32::WHITE);
        }

        // Draw text lines to the right of logo
        let text_x = block_x + logo_width + 10.0 * scale;
        let text_y = block_y
            + (logo_height - powered_galley.rect.height() - url_galley.rect.height() - 4.0 * scale)
                / 2.0;

        ui.painter()
            .galley(egui::pos2(text_x, text_y), powered_galley, text_color);
        ui.painter().galley(
            egui::pos2(text_x, text_y + 14.0 * scale + 4.0 * scale),
            url_galley,
            url_color,
        );
    }

    pub(super) fn draw_presentation_with_scroll(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        rect: egui::Rect,
        scale: f32,
    ) {
        // Cache slide rect for mouse coordinate conversion
        self.last_slide_rect = rect;

        // During transitions, just render normally (no scroll)
        if self.transition.is_some() {
            self.draw_presentation(ui, ctx, rect, scale);
            self.draw_annotations(ui, scale);
            return;
        }

        let idx = self.current_slide;
        let slide = &self.presentation.slides[idx];
        let (content_height, available_height) =
            render::measure_slide_content_height(ui, slide, &self.theme, rect, scale);
        let overflow = content_height - available_height;

        if overflow <= 0.0 {
            // No overflow — render normally, reset scroll
            self.scroll_offsets[idx] = 0.0;
            self.scroll_targets[idx] = 0.0;
            self.draw_presentation(ui, ctx, rect, scale);
            self.draw_annotations(ui, scale);
            return;
        }

        // Clamp target
        self.scroll_targets[idx] = self.scroll_targets[idx].clamp(0.0, overflow);

        // Animate: lerp current offset toward target
        let target = self.scroll_targets[idx];
        let current = self.scroll_offsets[idx];
        let diff = target - current;
        if diff.abs() < 0.5 {
            self.scroll_offsets[idx] = target;
        } else {
            // Smooth ease: move 15% of remaining distance each frame
            self.scroll_offsets[idx] = current + diff * 0.15;
            ctx.request_repaint();
        }
        let scroll_offset = self.scroll_offsets[idx];

        // Render slide inside a clipped child UI so content doesn't bleed outside
        let scrolled_rect = rect.translate(egui::vec2(0.0, -scroll_offset));
        let reveal = self.reveal_steps.get(idx).copied().unwrap_or(0);
        let timestamp = self.reveal_timestamps.get(idx).copied().flatten();
        let child_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect).id_salt("scroll_clip"));
        render::render_slide(
            &child_ui,
            slide,
            &self.theme,
            scrolled_rect,
            1.0,
            &self.image_cache,
            reveal,
            timestamp,
            scale,
        );

        // Draw fade-out gradient at bottom
        let fade_h = 80.0 * scale;
        if scroll_offset < overflow - 0.5 {
            draw_fade_gradient(ui, rect, fade_h, &self.theme, false);
        }
        // Draw fade-in gradient at top when scrolled
        if scroll_offset > 0.5 {
            draw_fade_gradient(ui, rect, fade_h, &self.theme, true);
        }

        // Draw scroll indicators
        let indicator_color = Theme::with_opacity(self.theme.foreground, 0.35);
        let indicator_font = egui::FontId::proportional(self.theme.body_size * 0.4 * scale);
        if scroll_offset < overflow - 0.5 {
            let galley = ui.painter().layout_no_wrap(
                "\u{25BC}".to_string(),
                indicator_font.clone(),
                indicator_color,
            );
            let pos = egui::pos2(
                rect.center().x - galley.rect.width() / 2.0,
                rect.bottom() - 40.0 * scale,
            );
            ui.painter().galley(pos, galley, indicator_color);
        }
        if scroll_offset > 0.5 {
            let galley = ui.painter().layout_no_wrap(
                "\u{25B2}".to_string(),
                indicator_font,
                indicator_color,
            );
            let pos = egui::pos2(
                rect.center().x - galley.rect.width() / 2.0,
                rect.top() + 10.0 * scale,
            );
            ui.painter().galley(pos, galley, indicator_color);
        }

        // Draw annotations on top of slide content
        self.draw_annotations(ui, scale);

        // Footer, counter, FPS
        self.draw_presentation_chrome(ui, rect, scale);
    }

    pub(super) fn draw_presentation(
        &self,
        ui: &egui::Ui,
        ctx: &egui::Context,
        rect: egui::Rect,
        scale: f32,
    ) {
        if let Some(ref t) = self.transition {
            let kind = t.kind;
            let from = t.from;
            let to = t.to;
            let progress = t.progress();
            let direction = t.direction;

            match kind {
                crate::render::transition::TransitionKind::Fade => {
                    self.draw_slide(ui, from, rect, 1.0 - progress, scale);
                    self.draw_slide(ui, to, rect, progress, scale);
                }
                crate::render::transition::TransitionKind::SlideHorizontal => {
                    let w = rect.width();
                    let sign = match direction {
                        crate::render::transition::TransitionDirection::Forward => -1.0,
                        crate::render::transition::TransitionDirection::Backward => 1.0,
                    };
                    let from_offset = sign * progress * w;
                    let to_offset = from_offset - sign * w;

                    let from_rect = rect.translate(egui::vec2(from_offset, 0.0));
                    let to_rect = rect.translate(egui::vec2(to_offset, 0.0));

                    self.draw_slide(ui, from, from_rect, 1.0, scale);
                    self.draw_slide(ui, to, to_rect, 1.0, scale);
                }
                crate::render::transition::TransitionKind::Spatial => {
                    let (dx, dy) = t.spatial_direction(self.grid_columns());
                    let w = rect.width();
                    let h = rect.height();

                    let from_rect =
                        rect.translate(egui::vec2(-dx * progress * w, -dy * progress * h));
                    let to_rect = rect.translate(egui::vec2(
                        dx * (1.0 - progress) * w,
                        dy * (1.0 - progress) * h,
                    ));

                    self.draw_slide(ui, from, from_rect, 1.0, scale);
                    self.draw_slide(ui, to, to_rect, 1.0, scale);
                }
                crate::render::transition::TransitionKind::None => {
                    self.draw_slide(ui, to, rect, 1.0, scale);
                }
            }
            ctx.request_repaint();
        } else {
            self.draw_slide(ui, self.current_slide, rect, 1.0, scale);
        }

        self.draw_presentation_chrome(ui, rect, scale);
    }

    pub(super) fn draw_presentation_chrome(&self, ui: &egui::Ui, rect: egui::Rect, scale: f32) {
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

    pub(super) fn draw_grid(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        rect: egui::Rect,
        selected: usize,
        scale: f32,
    ) {
        let count = self.slide_count();
        let padding = 24.0 * scale;

        // --- Grid scrolling ---
        let content_h = self.grid_content_height(rect, scale);
        let available_h = self.grid_available_height(rect, scale);
        let overflow = (content_h - available_h).max(0.0);

        // Mouse wheel scrolling in grid
        let scroll_delta = ctx.input(|i| i.smooth_scroll_delta.y);
        if scroll_delta != 0.0 && overflow > 0.0 {
            self.grid_scroll_target = (self.grid_scroll_target - scroll_delta).clamp(0.0, overflow);
        }

        // Clamp target
        self.grid_scroll_target = self.grid_scroll_target.clamp(0.0, overflow);

        // Animate scroll
        let diff = self.grid_scroll_target - self.grid_scroll_offset;
        if diff.abs() < 0.5 {
            self.grid_scroll_offset = self.grid_scroll_target;
        } else {
            self.grid_scroll_offset += diff * 0.15;
            ctx.request_repaint();
        }

        let scroll = self.grid_scroll_offset;

        // --- Mouse hover detection ---
        let hover_pos = ctx.input(|i| i.pointer.hover_pos());
        let mut hovered: Option<usize> = None;
        // Clip area for grid cells (below title, above hint)
        let grid_top = rect.top() + padding + 40.0 * scale;
        let grid_bottom = rect.bottom() - padding;
        let clip_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), grid_top),
            egui::pos2(rect.right(), grid_bottom),
        );

        // Detect whether the mouse has actually moved since last frame
        let mouse_moved = match (hover_pos, self.last_hover_pos) {
            (Some(cur), Some(prev)) => cur.distance(prev) > 1.0,
            (Some(_), None) => true,
            _ => false,
        };
        self.last_hover_pos = hover_pos;

        if let Some(hp) = hover_pos {
            for i in 0..count {
                let cell_rect = self.grid_cell_rect(i, rect, scale, scroll);
                let visible = cell_rect.intersects(clip_rect);
                if visible && cell_rect.contains(hp) && clip_rect.contains(hp) {
                    hovered = Some(i);
                    break;
                }
            }
        }
        if hovered.is_some() {
            self.hover_slide = hovered;
            // Only re-enable hover when the mouse has actually moved
            if mouse_moved {
                self.use_hover = true;
            }
        } else if hover_pos.is_some() {
            self.hover_slide = None;
        }

        // --- Mouse click detection ---
        let clicked = ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary));
        if clicked {
            if let Some(hi) = self.hover_slide {
                // Click on a grid cell → zoom into that slide
                self.mode = super::AppMode::OverviewTransition {
                    selected: hi,
                    entering: false,
                };
                self.overview_transition_start = Some(Instant::now());
                return;
            }
        }

        // --- Ensure selected cell is visible when using keyboard ---
        if !self.use_hover && overflow > 0.0 {
            let sel_rect = self.grid_cell_rect(selected, rect, scale, scroll);
            if sel_rect.top() < grid_top {
                self.grid_scroll_target -= grid_top - sel_rect.top() + padding;
                self.grid_scroll_target = self.grid_scroll_target.max(0.0);
            } else if sel_rect.bottom() > grid_bottom {
                self.grid_scroll_target += sel_rect.bottom() - grid_bottom + padding;
                self.grid_scroll_target = self.grid_scroll_target.min(overflow);
            }
        }

        // Title
        let title_color = Theme::with_opacity(self.theme.heading_color, 0.9);
        let title_galley = ui.painter().layout_no_wrap(
            self.display_title(),
            egui::FontId::proportional(24.0 * scale),
            title_color,
        );
        let title_pos = egui::pos2(rect.left() + padding, rect.top() + padding);
        ui.painter().galley(title_pos, title_galley, title_color);

        // Render grid cells clipped to the grid area
        let mut grid_child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(clip_rect)
                .id_salt("grid_clip"),
        );

        for i in 0..count {
            let cell_rect = self.grid_cell_rect(i, rect, scale, scroll);

            // Skip cells entirely outside the visible area
            if !cell_rect.intersects(clip_rect) {
                continue;
            }

            let cell_scale = (cell_rect.width() / 1920.0).min(cell_rect.height() / 1080.0);

            // Fill cell with theme background
            grid_child
                .painter()
                .rect_filled(cell_rect, 4.0 * scale, self.theme.background);

            // Render slide at full reveal (all steps visible) in grid
            let child_ui = grid_child.new_child(
                egui::UiBuilder::new()
                    .max_rect(cell_rect)
                    .id_salt(("grid_cell", i)),
            );
            self.draw_slide_fully_revealed(&child_ui, i, cell_rect, 1.0, cell_scale);

            // Slide number badge overlay
            self.draw_slide_badge(&grid_child, cell_rect, i, scale, 1.0);

            // Hover highlight (subtle glow, distinct from selection)
            if self.use_hover && self.hover_slide == Some(i) && i != selected {
                let hover_color = Theme::with_opacity(self.theme.accent, 0.12);
                grid_child
                    .painter()
                    .rect_filled(cell_rect, 4.0 * scale, hover_color);
                grid_child.painter().rect_stroke(
                    cell_rect.expand(2.0 * scale),
                    4.0 * scale,
                    egui::Stroke::new(2.0 * scale, Theme::with_opacity(self.theme.accent, 0.5)),
                    egui::StrokeKind::Outside,
                );
            }

            // Selected border (drawn AFTER preview so it's on top)
            if i == selected {
                grid_child.painter().rect_stroke(
                    cell_rect,
                    4.0 * scale,
                    egui::Stroke::new(3.0 * scale, self.theme.accent),
                    egui::StrokeKind::Outside,
                );
            }
        }

        // Fade gradients at screen edges when scrolled
        let fade_h = 60.0 * scale;
        if scroll > 0.5 {
            draw_fade_gradient(ui, rect, fade_h, &self.theme, true);
        }
        if scroll < overflow - 0.5 {
            draw_fade_gradient(ui, rect, fade_h, &self.theme, false);
        }

        // Navigation hint at bottom
        let hint = "Arrows/Mouse: navigate  |  Enter/Click: select  |  Q: quit";
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

    pub(super) fn draw_slide_badge(
        &self,
        ui: &egui::Ui,
        cell_rect: egui::Rect,
        index: usize,
        scale: f32,
        opacity: f32,
    ) {
        if opacity < 0.01 {
            return;
        }
        let badge_bg = Theme::with_opacity(self.theme.code_background, 0.7 * opacity);
        let badge_text_color = Theme::with_opacity(self.theme.foreground, 0.9 * opacity);
        let badge_galley = ui.painter().layout_no_wrap(
            format!(" {} ", index + 1),
            egui::FontId::monospace(12.0 * scale),
            badge_text_color,
        );
        let badge_rect = egui::Rect::from_min_size(
            cell_rect.min + egui::vec2(4.0 * scale, 4.0 * scale),
            badge_galley.rect.size() + egui::vec2(4.0 * scale, 2.0 * scale),
        );
        ui.painter().rect_filled(badge_rect, 3.0 * scale, badge_bg);
        ui.painter().galley(
            badge_rect.min + egui::vec2(2.0 * scale, 1.0 * scale),
            badge_galley,
            badge_text_color,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_overview_transition(
        &self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        rect: egui::Rect,
        scale: f32,
        selected: usize,
        entering: bool,
    ) {
        let elapsed = self
            .overview_transition_start
            .map(|s| s.elapsed().as_secs_f32())
            .unwrap_or(0.0);
        let raw_t = (elapsed / OVERVIEW_TRANSITION_DURATION).clamp(0.0, 1.0);
        let t = crate::render::transition::ease_in_out(raw_t);

        // grid_amount: 0 = fullscreen presentation, 1 = grid view
        let grid_amount = if entering { t } else { 1.0 - t };

        let hero_index = if entering {
            self.current_slide
        } else {
            selected
        };
        let hero_cell_rect = self.grid_cell_rect(hero_index, rect, scale, 0.0);
        let hero_rect = super::lerp_rect(rect, hero_cell_rect, grid_amount);
        let hero_scale = (hero_rect.width() / 1920.0).min(hero_rect.height() / 1080.0);

        let count = self.slide_count();

        // Draw non-hero slides at their grid positions with fading opacity
        for i in 0..count {
            if i == hero_index {
                continue;
            }
            let cell_rect = self.grid_cell_rect(i, rect, scale, 0.0);
            let cell_scale = (cell_rect.width() / 1920.0).min(cell_rect.height() / 1080.0);

            ui.painter()
                .rect_filled(cell_rect, 4.0 * scale, self.theme.background);

            let child_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(cell_rect)
                    .id_salt(("overview_cell", i)),
            );
            self.draw_slide_fully_revealed(&child_ui, i, cell_rect, grid_amount, cell_scale);

            self.draw_slide_badge(ui, cell_rect, i, scale, grid_amount);

            if i == selected {
                let border_color = Theme::with_opacity(self.theme.accent, grid_amount);
                ui.painter().rect_stroke(
                    cell_rect,
                    4.0 * scale,
                    egui::Stroke::new(3.0 * scale, border_color),
                    egui::StrokeKind::Outside,
                );
            }
        }

        // Draw hero slide on top (interpolating from full-screen to grid cell)
        ui.painter()
            .rect_filled(hero_rect, 4.0 * scale * grid_amount, self.theme.background);

        let hero_child_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(hero_rect)
                .id_salt("overview_hero"),
        );
        self.draw_slide(&hero_child_ui, hero_index, hero_rect, 1.0, hero_scale);

        self.draw_slide_badge(ui, hero_rect, hero_index, scale, grid_amount);

        if hero_index == selected {
            let border_color = Theme::with_opacity(self.theme.accent, grid_amount);
            ui.painter().rect_stroke(
                hero_rect,
                4.0 * scale * grid_amount,
                egui::Stroke::new(3.0 * scale, border_color),
                egui::StrokeKind::Outside,
            );
        }

        // Title and navigation hints fade in/out
        if grid_amount > 0.01 {
            let padding = 24.0 * scale;

            let title_color = Theme::with_opacity(self.theme.heading_color, 0.9 * grid_amount);
            let title_galley = ui.painter().layout_no_wrap(
                self.display_title(),
                egui::FontId::proportional(24.0 * scale),
                title_color,
            );
            let title_pos = egui::pos2(rect.left() + padding, rect.top() + padding);
            ui.painter().galley(title_pos, title_galley, title_color);

            let hint = "Arrows/Mouse: navigate  |  Enter/Click: select  |  Q: quit";
            let hint_color = Theme::with_opacity(self.theme.foreground, 0.4 * grid_amount);
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

        ctx.request_repaint();
    }

    /// Pen color: cyan/blue tones
    pub(super) fn pen_color(&self, opacity: f32) -> egui::Color32 {
        if self.theme.name == "dark" {
            egui::Color32::from_rgba_unmultiplied(80, 200, 255, (opacity * 230.0) as u8)
        } else {
            egui::Color32::from_rgba_unmultiplied(30, 80, 200, (opacity * 230.0) as u8)
        }
    }

    /// Pen outline color: darker cyan/blue
    pub(super) fn pen_outline_color(&self, opacity: f32) -> egui::Color32 {
        if self.theme.name == "dark" {
            egui::Color32::from_rgba_unmultiplied(30, 130, 180, (opacity * 140.0) as u8)
        } else {
            egui::Color32::from_rgba_unmultiplied(15, 40, 130, (opacity * 140.0) as u8)
        }
    }

    /// Arrow color: yellow-orange / red tones
    pub(super) fn arrow_color(&self, opacity: f32) -> egui::Color32 {
        if self.theme.name == "dark" {
            egui::Color32::from_rgba_unmultiplied(255, 200, 50, (opacity * 230.0) as u8)
        } else {
            egui::Color32::from_rgba_unmultiplied(220, 40, 40, (opacity * 230.0) as u8)
        }
    }

    /// Arrow outline color: darker orange / red
    pub(super) fn arrow_outline_color(&self, opacity: f32) -> egui::Color32 {
        if self.theme.name == "dark" {
            egui::Color32::from_rgba_unmultiplied(200, 140, 0, (opacity * 140.0) as u8)
        } else {
            egui::Color32::from_rgba_unmultiplied(150, 20, 20, (opacity * 140.0) as u8)
        }
    }

    /// Compute fade opacity for an annotation (1.0 for most of its life, fading in last 2s)
    pub(super) fn annotation_opacity(start: Instant) -> f32 {
        let elapsed = start.elapsed().as_secs_f32();
        let fade_start = DRAW_FADE_DURATION - 2.0;
        if elapsed < fade_start {
            1.0
        } else if elapsed < DRAW_FADE_DURATION {
            1.0 - (elapsed - fade_start) / 2.0
        } else {
            0.0
        }
    }

    /// Draw all pen strokes and arrow annotations for the current slide
    pub(super) fn draw_annotations(&self, ui: &egui::Ui, scale: f32) {
        let idx = self.current_slide;
        let pen_width = 6.0 * scale;
        let pen_outline_width = pen_width + 2.0 * scale;
        let arrow_width = 5.0 * scale;
        let arrow_outline_width = arrow_width + 2.0 * scale;
        let arrow_size = 22.0 * scale;
        let arrow_outline_size = arrow_size + 3.0 * scale;

        // Draw completed pen strokes
        for stroke in &self.pen_strokes {
            if stroke.slide_index != idx || stroke.points.len() < 2 {
                continue;
            }
            let opacity = Self::annotation_opacity(stroke.start);
            if opacity < 0.01 {
                continue;
            }
            let outline_color = self.pen_outline_color(opacity);
            let color = self.pen_color(opacity);
            let screen_points: Vec<egui::Pos2> = stroke
                .points
                .iter()
                .map(|p| self.local_to_screen(*p))
                .collect();
            // Outline pass
            ui.painter().add(egui::Shape::line(
                screen_points.clone(),
                egui::Stroke::new(pen_outline_width, outline_color),
            ));
            // Main pass
            ui.painter().add(egui::Shape::line(
                screen_points,
                egui::Stroke::new(pen_width, color),
            ));
        }

        // Draw completed arrows
        for arrow in &self.arrows {
            if arrow.slide_index != idx {
                continue;
            }
            let opacity = Self::annotation_opacity(arrow.start);
            if opacity < 0.01 {
                continue;
            }
            let outline_color = self.arrow_outline_color(opacity);
            let color = self.arrow_color(opacity);
            let from = self.local_to_screen(arrow.from);
            let to = self.local_to_screen(arrow.to);
            // Outline pass
            self.draw_arrow_shape(
                ui,
                from,
                to,
                arrow_outline_width,
                arrow_outline_size,
                outline_color,
            );
            // Main pass
            self.draw_arrow_shape(ui, from, to, arrow_width, arrow_size, color);
        }

        // Draw active drawing in progress
        match &self.active_draw {
            ActiveDraw::PenDrawing { points } if points.len() >= 2 => {
                let outline_color = self.pen_outline_color(1.0);
                let color = self.pen_color(1.0);
                let screen_points: Vec<egui::Pos2> =
                    points.iter().map(|p| self.local_to_screen(*p)).collect();
                ui.painter().add(egui::Shape::line(
                    screen_points.clone(),
                    egui::Stroke::new(pen_outline_width, outline_color),
                ));
                ui.painter().add(egui::Shape::line(
                    screen_points,
                    egui::Stroke::new(pen_width, color),
                ));
            }
            ActiveDraw::ArrowDrawing { from, current } => {
                let outline_color = self.arrow_outline_color(1.0);
                let color = self.arrow_color(1.0);
                let screen_from = self.local_to_screen(*from);
                let screen_to = self.local_to_screen(*current);
                self.draw_arrow_shape(
                    ui,
                    screen_from,
                    screen_to,
                    arrow_outline_width,
                    arrow_outline_size,
                    outline_color,
                );
                self.draw_arrow_shape(ui, screen_from, screen_to, arrow_width, arrow_size, color);
            }
            _ => {}
        }
    }

    /// Draw an arrow from `from` to `to` with a filled triangular arrowhead
    pub(super) fn draw_arrow_shape(
        &self,
        ui: &egui::Ui,
        from: egui::Pos2,
        to: egui::Pos2,
        stroke_width: f32,
        arrow_size: f32,
        color: egui::Color32,
    ) {
        let delta = to - from;
        let len = delta.length();
        if len < 1.0 {
            return;
        }
        let dir = delta / len;
        let perp = egui::vec2(-dir.y, dir.x);

        // Arrowhead triangle points (wider spread)
        let p1 = to - dir * arrow_size + perp * arrow_size * 0.45;
        let p2 = to - dir * arrow_size - perp * arrow_size * 0.45;

        // Shaft (stop further back from head to avoid blunt overlap)
        ui.painter().line_segment(
            [from, to - dir * arrow_size * 0.7],
            egui::Stroke::new(stroke_width, color),
        );
        // Arrowhead
        ui.painter().add(egui::Shape::convex_polygon(
            vec![to, p1, p2],
            color,
            egui::Stroke::NONE,
        ));
    }
}

/// Draw a fade gradient at the top or bottom of a rect.
pub(super) fn draw_fade_gradient(
    ui: &egui::Ui,
    rect: egui::Rect,
    fade_h: f32,
    theme: &Theme,
    top: bool,
) {
    let bg = theme.background;
    let transparent = egui::Color32::from_rgba_unmultiplied(bg.r(), bg.g(), bg.b(), 0);
    let opaque = bg;

    let fade_rect = if top {
        egui::Rect::from_min_max(
            egui::pos2(rect.left(), rect.top()),
            egui::pos2(rect.right(), rect.top() + fade_h),
        )
    } else {
        egui::Rect::from_min_max(
            egui::pos2(rect.left(), rect.bottom() - fade_h),
            egui::pos2(rect.right(), rect.bottom()),
        )
    };

    let mut mesh = egui::Mesh::default();
    // Four vertices: top-left, top-right, bottom-left, bottom-right
    let (top_color, bottom_color) = if top {
        (opaque, transparent)
    } else {
        (transparent, opaque)
    };

    mesh.colored_vertex(fade_rect.left_top(), top_color);
    mesh.colored_vertex(fade_rect.right_top(), top_color);
    mesh.colored_vertex(fade_rect.left_bottom(), bottom_color);
    mesh.colored_vertex(fade_rect.right_bottom(), bottom_color);
    // Two triangles: (0,1,2) and (1,3,2)
    mesh.add_triangle(0, 2, 1);
    mesh.add_triangle(1, 2, 3);

    ui.painter().add(egui::Shape::mesh(mesh));
}

pub(super) fn draw_hud(ui: &egui::Ui, theme: &Theme, rect: egui::Rect, scale: f32) {
    let shortcuts = [
        ("Space / N / \u{2192}", "Next slide / reveal"),
        ("P / \u{2190}", "Previous slide / hide"),
        ("\u{2191} / \u{2193} / Wheel", "Scroll slide content"),
        ("Left click", "Next slide"),
        ("Right click", "Previous slide"),
        ("Left drag", "Freehand pen (blue)"),
        ("Right drag", "Draw arrow (orange)"),
        ("Esc", "Clear drawings / \u{00d7}2 exit"),
        ("G", "Grid view / overview"),
        ("T", "Cycle transition"),
        ("\u{21e7}T", "Cycle theme"),
        ("F", "Toggle fullscreen"),
        ("M", "Move to next monitor"),
        ("H", "Toggle this HUD"),
        (".", "Blackout screen"),
        ("R", "Debug overlay (L/R/off)"),
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

pub(super) fn draw_raw_markdown_overlay(
    ui: &egui::Ui,
    raw: &str,
    debug_info: Option<&str>,
    side: RawOverlaySide,
    theme: &Theme,
    rect: egui::Rect,
    scale: f32,
) {
    let bg = Theme::with_opacity(theme.code_background, 0.78);
    let text_color = Theme::with_opacity(theme.code_foreground, 0.95);
    let title_color = Theme::with_opacity(theme.heading_color, 0.9);

    let padding = 20.0 * scale;
    let panel_width = rect.width() * 0.25;

    let overlay_rect = match side {
        RawOverlaySide::Left => {
            egui::Rect::from_min_size(rect.left_top(), egui::vec2(panel_width, rect.height()))
        }
        RawOverlaySide::Right => egui::Rect::from_min_size(
            egui::pos2(rect.right() - panel_width, rect.top()),
            egui::vec2(panel_width, rect.height()),
        ),
        RawOverlaySide::Off => return,
    };
    ui.painter().rect_filled(overlay_rect, 0.0, bg);

    // Title
    let title_galley = ui.painter().layout_no_wrap(
        "Raw Markdown".to_string(),
        egui::FontId::proportional(16.0 * scale),
        title_color,
    );
    let title_pos = egui::pos2(overlay_rect.left() + padding, overlay_rect.top() + padding);
    ui.painter().galley(title_pos, title_galley, title_color);

    // Hint text
    let hint_color = Theme::with_opacity(theme.foreground, 0.5);
    let hint_text = match side {
        RawOverlaySide::Left => "R: move right | RR: close",
        RawOverlaySide::Right => "R: close",
        RawOverlaySide::Off => "",
    };
    let hint_galley = ui.painter().layout_no_wrap(
        hint_text.to_string(),
        egui::FontId::proportional(11.0 * scale),
        hint_color,
    );
    let hint_pos = egui::pos2(
        overlay_rect.right() - padding - hint_galley.rect.width(),
        overlay_rect.top() + padding + 3.0 * scale,
    );
    ui.painter().galley(hint_pos, hint_galley, hint_color);

    // Markdown content in monospace font
    let text_top = overlay_rect.top() + padding + 28.0 * scale;
    let text_width = overlay_rect.width() - padding * 2.0;
    let font = egui::FontId::monospace(11.0 * scale);

    let galley = ui
        .painter()
        .layout(raw.to_string(), font.clone(), text_color, text_width);
    let text_pos = egui::pos2(overlay_rect.left() + padding, text_top);
    let raw_bottom = text_pos.y + galley.rect.height();
    ui.painter().galley(text_pos, galley, text_color);

    // Debug section (if diagram info is available)
    if let Some(info) = debug_info {
        let sep_y = raw_bottom + 12.0 * scale;
        let sep_color = Theme::with_opacity(theme.foreground, 0.3);
        ui.painter().line_segment(
            [
                egui::pos2(overlay_rect.left() + padding, sep_y),
                egui::pos2(overlay_rect.right() - padding, sep_y),
            ],
            egui::Stroke::new(1.0 * scale, sep_color),
        );

        let debug_title_pos = egui::pos2(overlay_rect.left() + padding, sep_y + 8.0 * scale);
        let debug_title = ui.painter().layout_no_wrap(
            "Routing Debug".to_string(),
            egui::FontId::proportional(14.0 * scale),
            title_color,
        );
        let debug_content_top = debug_title_pos.y + debug_title.rect.height() + 6.0 * scale;
        ui.painter()
            .galley(debug_title_pos, debug_title, title_color);

        let debug_galley = ui
            .painter()
            .layout(info.to_string(), font, text_color, text_width);
        let debug_pos = egui::pos2(overlay_rect.left() + padding, debug_content_top);
        ui.painter().galley(debug_pos, debug_galley, text_color);
    }
}
