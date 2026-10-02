//! Speaker notes in the presenter view and the one-display overlay: the
//! notes markdown parsed, fitted and drawn on the cockpit's panel.

use std::time::Duration;

use eframe::egui::{self, Color32, FontId, Rect, Stroke, pos2, vec2};

use super::{format_elapsed, ink, spaced};
use crate::parser::{self, Block};
use crate::render::{self, image_cache::ImageCache, text};
use crate::theme::Theme;

/// The part of a slide's notes that reads as text: headings, paragraphs,
/// lists, code, quotes, tables and rules (charts, diagrams and images in
/// notes are left out). Notes are markdown; this is how they are shown in
/// the presenter view and printed on PDF notes pages.
pub fn notes_blocks(notes: Option<&str>) -> Vec<Block> {
    let Some(notes) = notes else {
        return Vec::new();
    };
    parser::blocks::parse(notes)
        .into_iter()
        .filter(|b| {
            matches!(
                b,
                Block::Heading { .. }
                    | Block::Paragraph { .. }
                    | Block::List { .. }
                    | Block::CodeBlock { .. }
                    | Block::BlockQuote { .. }
                    | Block::Callout { .. }
                    | Block::Table { .. }
                    | Block::HorizontalRule
            )
        })
        .collect()
}

/// The notes theme: the built-in dark theme, readable on the cockpit panel.
fn notes_theme() -> &'static Theme {
    static NOTES: std::sync::LazyLock<Theme> = std::sync::LazyLock::new(|| {
        let mut t = Theme::dark();
        t.foreground = Color32::from_rgb(0xD9, 0xDB, 0xE1);
        t.heading_color = ink::TEXT;
        t.strong = Color32::WHITE;
        t.code_background = Color32::from_rgb(0x22, 0x25, 0x2C);
        t
    });
    &NOTES
}

/// Draw `blocks` in `area`, body text about `body_px` high; notes that do
/// not fit shrink a little, then fade out at the bottom.
pub fn draw_notes(ui: &mut egui::Ui, area: Rect, blocks: &[Block], body_px: f32) {
    let theme = notes_theme();
    if blocks.is_empty() {
        ui.painter().text(
            area.left_top(),
            egui::Align2::LEFT_TOP,
            "No notes for this slide.",
            FontId::proportional(body_px * 0.85),
            ink::MUTED,
        );
        return;
    }
    let base = body_px / theme.body_size;
    let height = |ui: &egui::Ui, scale: f32| -> f32 {
        blocks
            .iter()
            .map(|b| {
                text::measure_single_block_height(ui, b, theme, area.width(), scale)
                    + text::block_spacing(b, theme, scale)
            })
            .sum()
    };
    let mut scale = base;
    while scale > base * 0.7 && height(ui, scale) > area.height() {
        scale *= 0.92;
    }
    let overflow = height(ui, scale) > area.height();
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(area)
            .id_salt("presenter-notes"),
    );
    child.shrink_clip_rect(area);
    let images = ImageCache::new(std::path::PathBuf::new());
    let cx = render::BlockCx {
        ui: &child,
        theme,
        opacity: 1.0,
        scale,
        image_cache: &images,
        reveal_step: usize::MAX,
        reveal_timestamp: None,
    };
    text::draw_blocks(&cx, blocks, area.left_top(), area.width());
    if overflow {
        fade_bottom(ui.painter(), area, ink::PANEL, 48.0 * scale / base);
    }
}

/// A gradient to `color` over the last `h` points of `area`.
fn fade_bottom(painter: &egui::Painter, area: Rect, color: Color32, h: f32) {
    let top = area.bottom() - h;
    let mut mesh = egui::Mesh::default();
    let clear = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 0);
    mesh.colored_vertex(pos2(area.left(), top), clear);
    mesh.colored_vertex(pos2(area.right(), top), clear);
    mesh.colored_vertex(pos2(area.right(), area.bottom()), color);
    mesh.colored_vertex(pos2(area.left(), area.bottom()), color);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(egui::Shape::mesh(mesh));
}

/// The one-display notes overlay: a translucent band at the bottom of the
/// slide with the notes and the elapsed time.
pub fn draw_overlay(ui: &mut egui::Ui, rect: Rect, notes: &[Block], elapsed: Duration) {
    let hints = render::hints::enabled(ui.ctx());
    render::hints::set_enabled(ui.ctx(), false);
    let u = (rect.width() / 1920.0).min(rect.height() / 1080.0);
    let band = Rect::from_min_max(
        pos2(rect.left() + 40.0 * u, rect.bottom() - rect.height() * 0.36),
        pos2(rect.right() - 40.0 * u, rect.bottom() - 40.0 * u),
    );
    let painter = &ui.painter().clone();
    painter.rect_filled(band, 14.0 * u, ink::DESK.gamma_multiply(0.94));
    painter.rect_stroke(
        band,
        14.0 * u,
        Stroke::new(1.0, ink::HAIRLINE),
        egui::StrokeKind::Inside,
    );
    let pad = 28.0 * u;
    let head = painter.text(
        band.left_top() + vec2(pad, pad * 0.8),
        egui::Align2::LEFT_TOP,
        spaced("NOTES"),
        FontId::monospace(16.0 * u),
        ink::ACCENT,
    );
    painter.text(
        pos2(band.right() - pad, head.top()),
        egui::Align2::RIGHT_TOP,
        format_elapsed(elapsed),
        FontId::monospace(30.0 * u),
        ink::TEXT,
    );
    let area = Rect::from_min_max(
        pos2(band.left() + pad, head.bottom() + 14.0 * u),
        band.max - vec2(pad, pad * 0.8),
    );
    draw_notes(ui, area, notes, 30.0 * u);
    render::hints::set_enabled(ui.ctx(), hints);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_are_markdown_without_visuals() {
        let b = notes_blocks(Some(
            "## Remember\n\nSay **this**, then `that`.\n\n- one\n- two\n\n```@bar\n- A: 1\n```\n\n![x](y.png)\n",
        ));
        assert!(matches!(b[0], Block::Heading { level: 2, .. }), "{b:?}");
        assert!(matches!(b[1], Block::Paragraph { .. }));
        assert!(matches!(b[2], Block::List { .. }));
        assert_eq!(b.len(), 3, "{b:?}");
        assert!(notes_blocks(None).is_empty());
    }
}
