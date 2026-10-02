//! Syntax-highlighted code blocks.

use eframe::egui::{self, Pos2};

use crate::render::TextCx;
use crate::theme::Theme;

pub(crate) const CODE_PADDING: f32 = 16.0;

/// Draw a code block with syntax highlighting. Returns height used.
pub fn draw_code_block(
    cx: &TextCx,
    code: &str,
    language: Option<&str>,
    highlight_lines: &[usize],
    pos: Pos2,
    max_width: f32,
) -> f32 {
    let (theme, opacity, scale) = (cx.theme, cx.opacity, cx.scale);
    let padding = CODE_PADDING * scale;
    let bg_color = Theme::with_opacity(theme.code_background, opacity);

    // Build syntax-highlighted layout
    let job = crate::render::syntax::highlight_code(
        code,
        language,
        theme.code_size * scale,
        opacity,
        theme,
        max_width - padding * 2.0,
    );
    let code_galley = cx.ui.painter().layout_job(job);

    let total_height = code_galley.rect.height() + padding * 2.0;

    // Draw background
    let bg_rect = egui::Rect::from_min_size(pos, egui::vec2(max_width, total_height));
    cx.ui
        .painter()
        .rect_filled(bg_rect, theme.radius * scale, bg_color);

    // Draw line highlights using actual galley row positions
    if !highlight_lines.is_empty() {
        let accent = Theme::with_opacity(theme.accent, opacity * 0.15);
        let code_top = pos.y + padding;

        // Each row in the galley corresponds to a visual line.
        // `ends_with_newline` tells us when a source line ends.
        let mut source_line = 1usize;
        for row in &code_galley.rows {
            let row_rect = row.rect();

            if highlight_lines.contains(&source_line) {
                let hl_rect = egui::Rect::from_min_max(
                    Pos2::new(pos.x + padding * 0.5, code_top + row_rect.top()),
                    Pos2::new(
                        pos.x + max_width - padding * 0.5,
                        code_top + row_rect.bottom(),
                    ),
                );
                cx.ui.painter().rect_filled(hl_rect, 4.0 * scale, accent);
            }

            if row.ends_with_newline {
                source_line += 1;
            }
        }
    }

    // Draw code
    let code_pos = Pos2::new(pos.x + padding, pos.y + padding);
    let fallback = Theme::with_opacity(theme.code_foreground, opacity);
    crate::render::math::galley(cx.ui.painter(), code_pos, code_galley, fallback);

    total_height
}

/// Width in points of the widest line of `code` at the theme's code size,
/// unwrapped: what the block would need to show every line whole.
pub(crate) fn widest_code_line(ui: &egui::Ui, code: &str, theme: &Theme, scale: f32) -> f32 {
    let font = egui::FontId::new(theme.code_size * scale, theme.mono_family());
    code.lines()
        .map(|line| {
            // laid out the way the block draws it (no ligatures)
            let mut job = egui::text::LayoutJob::default();
            super::mono::append_code(
                &mut job,
                &line.replace('\t', "    "),
                egui::text::TextFormat {
                    font_id: font.clone(),
                    ..Default::default()
                },
            );
            ui.painter().layout_job(job).rect.width()
        })
        .fold(0.0, f32::max)
}

/// Measure a code block exactly as [`draw_code_block`] lays it out.
pub(crate) fn measure_code_block_height(
    ui: &egui::Ui,
    code: &str,
    language: Option<&str>,
    theme: &Theme,
    max_width: f32,
    scale: f32,
) -> f32 {
    let padding = CODE_PADDING * scale;
    let job = crate::render::syntax::highlight_code(
        code,
        language,
        theme.code_size * scale,
        1.0,
        theme,
        max_width - padding * 2.0,
    );
    ui.painter().layout_job(job).rect.height() + padding * 2.0
}
