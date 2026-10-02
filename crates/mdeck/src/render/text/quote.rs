//! Quotes and callouts: blocks of their own inside an accent bar or a
//! tinted panel. Quotes keep their paragraphs, lists and nested quotes;
//! GitHub alerts (`> [!NOTE]`) are callouts with their label on top.

use eframe::egui::{self, FontId, Pos2};

use super::block::{block_spacing, draw_block, measure_single_block_height};
use super::inline::{draw_inlines, measure_inlines};
use crate::parser::{Alert, Block};
use crate::render::BlockCx;
use crate::theme::Theme;

pub(super) const QUOTE_BAR_WIDTH: f32 = 4.0;
pub(super) const QUOTE_BAR_PADDING: f32 = 16.0;
/// Quote paragraphs read a little larger than body text.
const QUOTE_TEXT: f32 = 1.1;
const CALLOUT_PADDING: f32 = 20.0;
const CALLOUT_LABEL_GAP: f32 = 8.0;

/// The width a quote's own blocks get inside its bar.
fn quote_inner(max_width: f32, scale: f32) -> (f32, f32) {
    let offset = (QUOTE_BAR_WIDTH + QUOTE_BAR_PADDING) * scale;
    (offset, max_width - offset)
}

/// Draw a quote's blocks beside its accent bar. Returns the height used.
pub fn draw_quote(cx: &BlockCx, blocks: &[Block], pos: Pos2, max_width: f32) -> f32 {
    let (offset, width) = quote_inner(max_width, cx.scale);
    let height = quote_flow(cx, blocks, Pos2::new(pos.x + offset, pos.y), width);
    let accent = Theme::with_opacity(cx.theme.accent, cx.opacity);
    let bar = egui::Rect::from_min_size(pos, egui::vec2(QUOTE_BAR_WIDTH * cx.scale, height));
    cx.ui.painter().rect_filled(bar, 2.0, accent);
    height
}

/// Measure a quote exactly as [`draw_quote`] draws it.
pub fn measure_quote(
    ui: &egui::Ui,
    blocks: &[Block],
    theme: &Theme,
    max_width: f32,
    scale: f32,
) -> f32 {
    let (_, width) = quote_inner(max_width, scale);
    let mut total = 0.0;
    let mut gap = 0.0;
    for block in blocks {
        total += gap;
        total += match block {
            Block::Paragraph { inlines } => measure_inlines(
                ui,
                inlines,
                theme.body_size * QUOTE_TEXT * scale,
                width,
                theme,
            ),
            other => measure_single_block_height(ui, other, theme, width, scale),
        };
        gap = block_spacing(block, theme, scale);
    }
    total
}

/// A quote's own blocks, paragraphs at quote size.
fn quote_flow(cx: &BlockCx, blocks: &[Block], pos: Pos2, width: f32) -> f32 {
    let color = Theme::with_opacity(cx.theme.foreground, cx.opacity);
    let mut y = 0.0;
    let mut gap = 0.0;
    for block in blocks {
        y += gap;
        let at = Pos2::new(pos.x, pos.y + y);
        y += match block {
            Block::Paragraph { inlines } => draw_inlines(
                &cx.text(),
                inlines,
                at,
                cx.theme.body_size * QUOTE_TEXT * cx.scale,
                color,
                width,
            ),
            other => draw_block(cx, other, at, width),
        };
        gap = block_spacing(block, cx.theme, cx.scale);
    }
    y
}

fn callout_label(
    ui: &egui::Ui,
    kind: Alert,
    theme: &Theme,
    color: egui::Color32,
    scale: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        &kind.label().to_uppercase(),
        0.0,
        egui::TextFormat {
            font_id: FontId::new(theme.body_size * 0.62 * scale, theme.body_family()),
            color,
            extra_letter_spacing: 2.0 * scale,
            ..Default::default()
        },
    );
    ui.painter().layout_job(job)
}

/// Draw a callout: a tinted panel with an accent edge, its label, then its
/// blocks. Returns the height used.
pub fn draw_callout(cx: &BlockCx, kind: Alert, blocks: &[Block], pos: Pos2, max_width: f32) -> f32 {
    let (scale, theme) = (cx.scale, cx.theme);
    let pad = CALLOUT_PADDING * scale;
    let height = measure_callout(cx.ui, kind, blocks, theme, max_width, scale);
    let rect = egui::Rect::from_min_size(pos, egui::vec2(max_width, height));
    let painter = cx.ui.painter();
    painter.rect_filled(
        rect,
        theme.radius * scale,
        Theme::with_opacity(theme.accent, cx.opacity * 0.10),
    );
    let edge = egui::Rect::from_min_size(pos, egui::vec2(4.0 * scale, height));
    painter.rect_filled(
        edge,
        2.0 * scale,
        Theme::with_opacity(theme.accent, cx.opacity),
    );

    let label = callout_label(
        cx.ui,
        kind,
        theme,
        Theme::with_opacity(theme.accent, cx.opacity),
        scale,
    );
    let label_h = label.rect.height();
    painter.galley(
        Pos2::new(pos.x + pad, pos.y + pad),
        label,
        egui::Color32::WHITE,
    );
    let inner = Pos2::new(
        pos.x + pad,
        pos.y + pad + label_h + CALLOUT_LABEL_GAP * scale,
    );
    super::block::draw_blocks(cx, blocks, inner, max_width - 2.0 * pad);
    height
}

/// Measure a callout exactly as [`draw_callout`] draws it.
pub fn measure_callout(
    ui: &egui::Ui,
    kind: Alert,
    blocks: &[Block],
    theme: &Theme,
    max_width: f32,
    scale: f32,
) -> f32 {
    let pad = CALLOUT_PADDING * scale;
    let label = callout_label(ui, kind, theme, theme.accent, scale);
    let inner =
        super::block::measure_blocks_height(ui, blocks, theme, max_width - 2.0 * pad, scale);
    pad + label.rect.height() + CALLOUT_LABEL_GAP * scale + inner + pad
}
