//! The overview grid (`G`) and the zoom between it and a slide.

use std::time::Instant;

use eframe::egui;

use crate::theme::Theme;

use super::grid::GridLayout;
use super::keys::{SCROLL_SMOOTH_RATE, smooth_factor};
use super::overlays::draw_fade_gradient;
use super::{OVERVIEW_TRANSITION_DURATION, PresentationApp};

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
        let grid = self.grid_layout(rect, scale);
        let overflow = (grid.content_height() - grid.available_height()).max(0.0);
        let scroll = self.animate_grid_scroll(ctx, overflow);
        let clip = grid.clip();

        if self.track_grid_pointer(ctx, &grid, scroll) {
            return;
        }
        // Keep the selected cell visible when using the keyboard
        if !self.grid.use_hover && overflow > 0.0 {
            self.grid.scroll_target = grid.scroll_to_show(selected, scroll);
        }

        self.draw_grid_title(ui, rect, scale, 1.0);

        // Render grid cells clipped to the grid area. `max_rect` only affects
        // layout, so the clip rect must be set explicitly.
        let mut grid_child =
            ui.new_child(egui::UiBuilder::new().max_rect(clip).id_salt("grid_clip"));
        grid_child.shrink_clip_rect(clip);
        for i in 0..self.slide_count() {
            let cell_rect = grid.cell(i, scroll);
            // Skip cells entirely outside the visible area
            if cell_rect.intersects(clip) {
                self.draw_grid_cell(&mut grid_child, i, cell_rect, selected, scale);
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
        self.draw_grid_hint(ui, rect, scale, 1.0);
    }

    /// Follow the wheel and ease the grid's scroll toward its target (frame-rate
    /// independent). Returns the scroll offset to draw with.
    fn animate_grid_scroll(&mut self, ctx: &egui::Context, overflow: f32) -> f32 {
        let scroll_delta = ctx.input(|i| i.smooth_scroll_delta.y);
        if scroll_delta != 0.0 && overflow > 0.0 {
            self.grid.scroll_target = (self.grid.scroll_target - scroll_delta).clamp(0.0, overflow);
        }
        self.grid.scroll_target = self.grid.scroll_target.clamp(0.0, overflow);
        let diff = self.grid.scroll_target - self.grid.scroll;
        if diff.abs() < 0.5 {
            self.grid.scroll = self.grid.scroll_target;
        } else {
            let dt = ctx.input(|i| i.stable_dt);
            self.grid.scroll += diff * smooth_factor(dt, SCROLL_SMOOTH_RATE);
            ctx.request_repaint();
        }
        self.grid.scroll
    }

    /// Hover follows the mouse once it moves; a click zooms into the hovered
    /// slide. Returns true when a click started that zoom.
    fn track_grid_pointer(&mut self, ctx: &egui::Context, grid: &GridLayout, scroll: f32) -> bool {
        let hover_pos = ctx.input(|i| i.pointer.hover_pos());
        // Only a mouse that actually moved takes over from the keyboard
        let mouse_moved = match (hover_pos, self.grid.last_hover_pos) {
            (Some(cur), Some(prev)) => cur.distance(prev) > 1.0,
            (Some(_), None) => true,
            _ => false,
        };
        self.grid.last_hover_pos = hover_pos;

        let clip = grid.clip();
        let hovered = hover_pos.and_then(|hp| {
            (0..self.slide_count()).find(|&i| {
                let cell = grid.cell(i, scroll);
                cell.intersects(clip) && cell.contains(hp) && clip.contains(hp)
            })
        });
        if hovered.is_some() {
            self.grid.hover = hovered;
            if mouse_moved {
                self.grid.use_hover = true;
            }
        } else if hover_pos.is_some() {
            self.grid.hover = None;
        }

        let clicked = ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary));
        let Some(target) = self.grid.hover.filter(|_| clicked) else {
            return false;
        };
        self.mode = super::AppMode::OverviewTransition {
            selected: target,
            entering: false,
        };
        self.overview_transition_start = Some(Instant::now());
        true
    }

    /// One grid cell: the slide fully revealed, its number, and the hover
    /// and selection outlines.
    fn draw_grid_cell(
        &self,
        ui: &mut egui::Ui,
        index: usize,
        cell_rect: egui::Rect,
        selected: usize,
        scale: f32,
    ) {
        let cell_scale = (cell_rect.width() / 1920.0).min(cell_rect.height() / 1080.0);
        ui.painter()
            .rect_filled(cell_rect, 4.0 * scale, self.theme.background);
        // Clipped to its cell so overflowing slides don't bleed into neighbours
        let mut child_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(cell_rect)
                .id_salt(("grid_cell", index)),
        );
        child_ui.shrink_clip_rect(cell_rect);
        self.draw_slide_fully_revealed(&child_ui, index, cell_rect, 1.0, cell_scale);
        self.draw_slide_badge(ui, cell_rect, index, scale, 1.0);

        // Hover highlight (subtle glow, distinct from selection)
        if self.grid.use_hover && self.grid.hover == Some(index) && index != selected {
            let hover_color = Theme::with_opacity(self.theme.accent, 0.12);
            ui.painter()
                .rect_filled(cell_rect, 4.0 * scale, hover_color);
            ui.painter().rect_stroke(
                cell_rect.expand(2.0 * scale),
                4.0 * scale,
                egui::Stroke::new(2.0 * scale, Theme::with_opacity(self.theme.accent, 0.5)),
                egui::StrokeKind::Outside,
            );
        }
        // Selected border (drawn after the preview so it's on top)
        if index == selected {
            ui.painter().rect_stroke(
                cell_rect,
                4.0 * scale,
                egui::Stroke::new(3.0 * scale, self.theme.accent),
                egui::StrokeKind::Outside,
            );
        }
    }

    /// The deck's title above the grid, at `opacity` (fading with the zoom).
    fn draw_grid_title(&self, ui: &egui::Ui, rect: egui::Rect, scale: f32, opacity: f32) {
        let padding = 24.0 * scale;
        let title_color = Theme::with_opacity(self.theme.heading_color, 0.9 * opacity);
        let title_galley = ui.painter().layout_no_wrap(
            self.display_title(),
            egui::FontId::proportional(24.0 * scale),
            title_color,
        );
        let title_pos = egui::pos2(rect.left() + padding, rect.top() + padding);
        ui.painter().galley(title_pos, title_galley, title_color);
    }

    /// The navigation hint below the grid, at `opacity`.
    fn draw_grid_hint(&self, ui: &egui::Ui, rect: egui::Rect, scale: f32, opacity: f32) {
        let hint_color = Theme::with_opacity(self.theme.foreground, 0.4 * opacity);
        let hint_galley = ui.painter().layout_no_wrap(
            GRID_HINT.to_string(),
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

    /// The zoom between a slide and the grid: `entering` zooms out from the
    /// current slide, otherwise back in to `selected`.
    pub(super) fn draw_overview_transition(
        &self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        rect: egui::Rect,
        scale: f32,
        (selected, entering): (usize, bool),
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
        let grid = self.grid_layout(rect, scale);
        // Use the live grid scroll so cells below the fold animate to/from
        // where they will actually be drawn in the grid.
        let grid_scroll = self.grid.scroll;
        let grid_clip = grid.clip();

        // Non-hero slides at their grid positions with fading opacity
        for i in (0..self.slide_count()).filter(|&i| i != hero_index) {
            let cell_rect = grid.cell(i, grid_scroll);
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
                self.draw_fading_selection(ui, cell_rect, 4.0 * scale, scale, grid_amount);
            }
        }

        // The hero slide on top, interpolating from full screen to its cell
        let hero_cell_rect = grid.cell(hero_index, grid_scroll);
        let hero_rect = super::helpers::lerp_rect(rect, hero_cell_rect, grid_amount);
        let hero_scale = (hero_rect.width() / 1920.0).min(hero_rect.height() / 1080.0);
        // Ease the hero's slide scroll out as it shrinks into its (unscrolled) cell
        let hero_scroll = self.view(hero_index).scroll * (1.0 - grid_amount);
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
            let rounding = 4.0 * scale * grid_amount;
            self.draw_fading_selection(ui, hero_rect, rounding, scale, grid_amount);
        }

        // Title and navigation hints fade in/out
        if grid_amount > 0.01 {
            self.draw_grid_title(ui, rect, scale, grid_amount);
            self.draw_grid_hint(ui, rect, scale, grid_amount);
        }
        ctx.request_repaint();
    }

    /// The selection outline while zooming, fading with the grid.
    fn draw_fading_selection(
        &self,
        ui: &egui::Ui,
        rect: egui::Rect,
        rounding: f32,
        scale: f32,
        opacity: f32,
    ) {
        let border_color = Theme::with_opacity(self.theme.accent, opacity);
        ui.painter().rect_stroke(
            rect,
            rounding,
            egui::Stroke::new(3.0 * scale, border_color),
            egui::StrokeKind::Outside,
        );
    }
}
