//! What is drawn over a slide: annotations, scroll fades, the HUD and the
//! raw markdown overlay.

use std::time::Instant;

use eframe::egui;

use crate::theme::Theme;

use super::input::ActiveDraw;
use super::keys::SHORTCUTS;
use super::{DRAW_FADE_DURATION, PresentationApp, RawOverlaySide};

impl PresentationApp {
    /// Pen colour, from the theme's `annotations.pen`.
    pub(super) fn pen_color(&self, opacity: f32) -> egui::Color32 {
        Theme::with_opacity(self.theme.pen, opacity * (230.0 / 255.0))
    }

    /// Pen outline colour (`annotations.pen-outline`).
    pub(super) fn pen_outline_color(&self, opacity: f32) -> egui::Color32 {
        Theme::with_opacity(self.theme.pen_outline, opacity * (140.0 / 255.0))
    }

    /// Arrow colour (`annotations.arrow`).
    pub(super) fn arrow_color(&self, opacity: f32) -> egui::Color32 {
        Theme::with_opacity(self.theme.arrow, opacity * (230.0 / 255.0))
    }

    /// Arrow outline colour (`annotations.arrow-outline`).
    pub(super) fn arrow_outline_color(&self, opacity: f32) -> egui::Color32 {
        Theme::with_opacity(self.theme.arrow_outline, opacity * (140.0 / 255.0))
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
    let shortcuts = SHORTCUTS;

    let bg = Theme::with_opacity(theme.code_background, 0.9);
    let text_color = Theme::with_opacity(theme.foreground, 0.9);
    let key_color = Theme::with_opacity(theme.accent, 0.9);

    let padding = 24.0 * scale;
    let line_height = 32.0 * scale;
    let hud_height = shortcuts.len() as f32 * line_height + padding * 2.0 + 40.0 * scale;
    let hud_width = 440.0 * scale;

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

    for (key, desc) in shortcuts {
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
            egui::pos2(hud_rect.left() + padding + 210.0 * scale, y),
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
