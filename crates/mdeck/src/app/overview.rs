//! The overview grid (`G`) and the zoom between it and a slide.

use eframe::egui;
use std::time::Instant;

use super::keys::{SCROLL_SMOOTH_RATE, smooth_factor};
use super::overlays::draw_fade_gradient;
use super::*;

/// Navigation hint shown at the bottom of the grid view.
const GRID_HINT: &str = "Arrows/Mouse: navigate  |  Enter/Click: select  |  Q \u{00d7}2: quit";

impl PresentationApp {
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
        let content_h = self.grid(rect, scale).content_height();
        let available_h = self.grid(rect, scale).available_height();
        let overflow = (content_h - available_h).max(0.0);

        // Mouse wheel scrolling in grid
        let scroll_delta = ctx.input(|i| i.smooth_scroll_delta.y);
        if scroll_delta != 0.0 && overflow > 0.0 {
            self.grid_scroll_target = (self.grid_scroll_target - scroll_delta).clamp(0.0, overflow);
        }

        // Clamp target
        self.grid_scroll_target = self.grid_scroll_target.clamp(0.0, overflow);

        // Animate scroll (frame-rate independent)
        let diff = self.grid_scroll_target - self.grid_scroll_offset;
        if diff.abs() < 0.5 {
            self.grid_scroll_offset = self.grid_scroll_target;
        } else {
            let dt = ctx.input(|i| i.stable_dt);
            self.grid_scroll_offset += diff * smooth_factor(dt, SCROLL_SMOOTH_RATE);
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
                let cell_rect = self.grid(rect, scale).cell(i, scroll);
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
        if clicked && let Some(hi) = self.hover_slide {
            // Click on a grid cell → zoom into that slide
            self.mode = super::AppMode::OverviewTransition {
                selected: hi,
                entering: false,
            };
            self.overview_transition_start = Some(Instant::now());
            return;
        }

        // --- Ensure selected cell is visible when using keyboard ---
        if !self.use_hover && overflow > 0.0 {
            self.grid_scroll_target = self.grid(rect, scale).scroll_to_show(selected, scroll);
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

        // Render grid cells clipped to the grid area. `max_rect` only affects
        // layout, so the clip rect must be set explicitly.
        let mut grid_child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(clip_rect)
                .id_salt("grid_clip"),
        );
        grid_child.shrink_clip_rect(clip_rect);

        for i in 0..count {
            let cell_rect = self.grid(rect, scale).cell(i, scroll);

            // Skip cells entirely outside the visible area
            if !cell_rect.intersects(clip_rect) {
                continue;
            }

            let cell_scale = (cell_rect.width() / 1920.0).min(cell_rect.height() / 1080.0);

            // Fill cell with theme background
            grid_child
                .painter()
                .rect_filled(cell_rect, 4.0 * scale, self.theme.background);

            // Render slide at full reveal (all steps visible), clipped to its cell
            // so overflowing slides don't bleed into neighbours
            let mut child_ui = grid_child.new_child(
                egui::UiBuilder::new()
                    .max_rect(cell_rect)
                    .id_salt(("grid_cell", i)),
            );
            child_ui.shrink_clip_rect(cell_rect);
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
        let hint = GRID_HINT;
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
        // Use the live grid scroll so cells below the fold animate to/from
        // where they will actually be drawn in the grid.
        let grid_scroll = self.grid_scroll_offset;
        let hero_cell_rect = self.grid(rect, scale).cell(hero_index, grid_scroll);
        let hero_rect = super::helpers::lerp_rect(rect, hero_cell_rect, grid_amount);
        let hero_scale = (hero_rect.width() / 1920.0).min(hero_rect.height() / 1080.0);
        // Ease the hero's slide scroll out as it shrinks into its (unscrolled) cell
        let hero_scroll = self.view(hero_index).scroll * (1.0 - grid_amount);

        let count = self.slide_count();
        let padding = 24.0 * scale;
        let grid_clip = egui::Rect::from_min_max(
            egui::pos2(rect.left(), rect.top() + padding + 40.0 * scale),
            egui::pos2(rect.right(), rect.bottom() - padding),
        );

        // Draw non-hero slides at their grid positions with fading opacity
        for i in 0..count {
            if i == hero_index {
                continue;
            }
            let cell_rect = self.grid(rect, scale).cell(i, grid_scroll);
            if !cell_rect.intersects(grid_clip) {
                continue;
            }
            let cell_scale = (cell_rect.width() / 1920.0).min(cell_rect.height() / 1080.0);

            ui.painter()
                .rect_filled(cell_rect, 4.0 * scale, self.theme.background);

            let mut child_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(cell_rect)
                    .id_salt(("overview_cell", i)),
            );
            child_ui.shrink_clip_rect(cell_rect.intersect(grid_clip));
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

        let mut hero_child_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(hero_rect)
                .id_salt("overview_hero"),
        );
        hero_child_ui.shrink_clip_rect(hero_rect);
        self.draw_slide(
            &mut hero_child_ui,
            hero_index,
            hero_rect,
            1.0,
            hero_scale,
            hero_scroll,
        );

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

            let hint = GRID_HINT;
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
}
