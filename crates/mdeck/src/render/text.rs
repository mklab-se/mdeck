use crate::parser::{Block, ImageDirectives, Inline, ListItem, ListMarker};
use crate::render::diagram::draw_diagram_sized;
use crate::render::image_cache::ImageCache;
use crate::theme::Theme;
use eframe::egui::{self, Color32, FontFamily, FontId, Pos2, Stroke};

// ---------------------------------------------------------------------------
// Spacing rules shared by layouts and measurement
// ---------------------------------------------------------------------------

/// Vertical gap between a heading and the block that follows it.
/// One rule for every layout: half the heading's font size.
pub fn heading_spacing(theme: &Theme, level: u8, scale: f32) -> f32 {
    theme.heading_size(level) * 0.5 * scale
}

/// Vertical gap that follows `block` when blocks are stacked vertically.
/// Used by both drawing and measurement so overflow detection stays exact.
pub fn block_spacing(block: &Block, theme: &Theme, scale: f32) -> f32 {
    match block {
        Block::Heading { level, .. } => heading_spacing(theme, *level, scale),
        Block::HorizontalRule => 10.0 * scale,
        _ => 20.0 * scale,
    }
}

// ---------------------------------------------------------------------------
// Inline text
// ---------------------------------------------------------------------------

/// Colours used when laying out inline runs. Derived from the theme and the
/// caller's base colour so that fade opacity carries through to every run.
#[derive(Clone, Copy)]
struct InlineStyle {
    /// Base text colour (already carries the fade opacity in its alpha).
    color: Color32,
    /// Colour for `**bold**` runs. Only a light face is bundled, so bold is
    /// emphasised with a brighter colour in addition to the size bump.
    strong: Color32,
    /// Colour for link text.
    link: Color32,
    /// Background tint for inline code.
    code_bg: Color32,
}

impl InlineStyle {
    fn new(theme: &Theme, color: Color32) -> Self {
        let alpha = color.a();
        Self {
            color,
            strong: with_alpha(strong_color(theme), alpha),
            link: with_alpha(theme.accent, alpha),
            code_bg: with_alpha(theme.accent, (alpha as f32 * 0.12) as u8),
        }
    }
}

fn with_alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

/// Relative luminance (0..1) of a colour, ignoring alpha.
fn luminance(c: Color32) -> f32 {
    (0.2126 * c.r() as f32 + 0.7152 * c.g() as f32 + 0.0722 * c.b() as f32) / 255.0
}

/// Colour used to emphasise bold text. The heading colour is used when it is
/// visibly brighter than the body colour (dark themes); otherwise the accent
/// colour is used so bold still stands out (light theme, where heading and body
/// colours are nearly identical).
pub fn strong_color(theme: &Theme) -> Color32 {
    let diff = luminance(theme.heading_color) - luminance(theme.foreground);
    if diff > 0.08 {
        theme.heading_color
    } else {
        theme.accent
    }
}

/// Create a LayoutJob from inline elements.
pub fn inlines_to_job(
    inlines: &[Inline],
    font_size: f32,
    color: Color32,
    max_width: f32,
    theme: &Theme,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = max_width;
    let style = InlineStyle::new(theme, color);
    append_inlines(&mut job, inlines, font_size, style, false, false);
    job
}

fn append_inlines(
    job: &mut egui::text::LayoutJob,
    inlines: &[Inline],
    font_size: f32,
    style: InlineStyle,
    bold: bool,
    italic: bool,
) {
    for inline in inlines {
        match inline {
            Inline::Text(s) => {
                let (size, color) = if bold {
                    (font_size + 1.0, style.strong)
                } else {
                    (font_size, style.color)
                };
                let format = egui::text::TextFormat {
                    font_id: FontId::new(size, FontFamily::Proportional),
                    color,
                    italics: italic,
                    ..Default::default()
                };
                job.append(s, 0.0, format);
            }
            Inline::Bold(children) => {
                append_inlines(job, children, font_size, style, true, italic);
            }
            Inline::Italic(children) => {
                append_inlines(job, children, font_size, style, bold, true);
            }
            Inline::Strikethrough(children) => {
                let mut inner_job = egui::text::LayoutJob::default();
                append_inlines(&mut inner_job, children, font_size, style, bold, italic);
                // Apply strikethrough to all sections
                for section in &inner_job.sections {
                    let mut format = section.format.clone();
                    format.strikethrough = Stroke::new(1.0_f32, format.color);
                    job.append(&inner_job.text[section.byte_range.clone()], 0.0, format);
                }
            }
            Inline::Code(s) => {
                let format = egui::text::TextFormat {
                    font_id: FontId::new(font_size * 0.85, FontFamily::Monospace),
                    color: style.color,
                    background: style.code_bg,
                    ..Default::default()
                };
                job.append(s, 0.0, format);
            }
            Inline::Link { text, .. } => {
                // Render link text in the theme accent colour
                let link_style = InlineStyle {
                    color: style.link,
                    strong: style.link,
                    ..style
                };
                append_inlines(job, text, font_size, link_style, bold, italic);
            }
        }
    }
}

/// Measure the height of inlines laid out at `max_width` without painting.
fn measure_inlines(
    ui: &egui::Ui,
    inlines: &[Inline],
    font_size: f32,
    max_width: f32,
    theme: &Theme,
) -> f32 {
    let job = inlines_to_job(inlines, font_size, theme.foreground, max_width, theme);
    ui.painter().layout_job(job).rect.height()
}

/// Layout and paint inlines, returning the height used.
#[allow(clippy::too_many_arguments)]
pub fn draw_inlines(
    ui: &egui::Ui,
    inlines: &[Inline],
    pos: Pos2,
    font_size: f32,
    color: Color32,
    max_width: f32,
    theme: &Theme,
) -> f32 {
    let job = inlines_to_job(inlines, font_size, color, max_width, theme);
    let galley = ui.painter().layout_job(job);
    let height = galley.rect.height();
    ui.painter().galley(pos, galley, color);
    height
}

/// Draw a heading block. Returns height used.
#[allow(clippy::too_many_arguments)]
pub fn draw_heading(
    ui: &egui::Ui,
    inlines: &[Inline],
    level: u8,
    theme: &Theme,
    pos: Pos2,
    max_width: f32,
    opacity: f32,
    scale: f32,
) -> f32 {
    let size = theme.heading_size(level) * scale;
    let color = Theme::with_opacity(theme.heading_color, opacity);
    draw_inlines(ui, inlines, pos, size, color, max_width, theme)
}

/// Draw a paragraph. Returns height used.
pub fn draw_paragraph(
    ui: &egui::Ui,
    inlines: &[Inline],
    theme: &Theme,
    pos: Pos2,
    max_width: f32,
    opacity: f32,
    scale: f32,
) -> f32 {
    let color = Theme::with_opacity(theme.foreground, opacity);
    draw_inlines(
        ui,
        inlines,
        pos,
        theme.body_size * scale,
        color,
        max_width,
        theme,
    )
}

// ---------------------------------------------------------------------------
// Lists
// ---------------------------------------------------------------------------

const LIST_INDENT: f32 = 30.0;
const LIST_MARKER_WIDTH: f32 = 45.0;
const LIST_ITEM_SPACING: f32 = 8.0;
/// Deeply nested items never get less than this much room for their text.
const LIST_MIN_TEXT_WIDTH: f32 = 200.0;

/// Horizontal metrics for a list at a given nesting level:
/// `(indent, marker_width, text_width)`.
fn list_metrics(max_width: f32, indent_level: usize, scale: f32) -> (f32, f32, f32) {
    let indent = LIST_INDENT * scale * indent_level as f32;
    let marker_width = LIST_MARKER_WIDTH * scale;
    let text_width = (max_width - indent - marker_width).max(LIST_MIN_TEXT_WIDTH * scale);
    (indent, marker_width, text_width)
}

/// Draw a list with incremental reveal support. Returns height used.
#[allow(clippy::too_many_arguments)]
pub fn draw_list(
    ui: &egui::Ui,
    items: &[ListItem],
    ordered: bool,
    theme: &Theme,
    pos: Pos2,
    max_width: f32,
    opacity: f32,
    indent_level: usize,
    reveal_step: usize,
    scale: f32,
) -> f32 {
    let mut step_counter = 0usize;
    draw_list_inner(
        ui,
        items,
        ordered,
        theme,
        pos,
        max_width,
        opacity,
        indent_level,
        reveal_step,
        &mut step_counter,
        scale,
    )
}

#[allow(clippy::too_many_arguments)]
fn draw_list_inner(
    ui: &egui::Ui,
    items: &[ListItem],
    ordered: bool,
    theme: &Theme,
    pos: Pos2,
    max_width: f32,
    opacity: f32,
    indent_level: usize,
    reveal_step: usize,
    step_counter: &mut usize,
    scale: f32,
) -> f32 {
    let color = Theme::with_opacity(theme.foreground, opacity);
    let (indent, marker_width, text_width) = list_metrics(max_width, indent_level, scale);
    let item_spacing = LIST_ITEM_SPACING * scale;
    let font_size = theme.body_size * scale;
    let mut y_offset = 0.0;

    for (idx, item) in items.iter().enumerate() {
        // Compute this item's reveal step
        let item_step = match item.marker {
            ListMarker::Static | ListMarker::Ordered => 0,
            ListMarker::NextStep => {
                *step_counter += 1;
                *step_counter
            }
            ListMarker::WithPrev => *step_counter,
        };

        // Skip items not yet revealed
        if item_step > reveal_step {
            continue;
        }

        // Draw marker
        let marker_text = if ordered {
            format!("{}.", idx + 1)
        } else {
            match item.marker {
                ListMarker::Static | ListMarker::NextStep | ListMarker::WithPrev => {
                    "\u{2022}".to_string()
                }
                ListMarker::Ordered => format!("{}.", idx + 1),
            }
        };

        let marker_pos = Pos2::new(pos.x + indent, pos.y + y_offset);
        let marker_galley =
            ui.painter()
                .layout_no_wrap(marker_text, FontId::proportional(font_size), color);
        ui.painter().galley(marker_pos, marker_galley, color);

        // Draw item text
        let text_pos = Pos2::new(pos.x + indent + marker_width, pos.y + y_offset);
        let text_height = draw_inlines(
            ui,
            &item.inlines,
            text_pos,
            font_size,
            color,
            text_width,
            theme,
        );

        y_offset += text_height + item_spacing;

        // Draw children
        if !item.children.is_empty() {
            let children_ordered = item
                .children
                .first()
                .is_some_and(|c| c.marker == ListMarker::Ordered);
            let child_height = draw_list_inner(
                ui,
                &item.children,
                children_ordered,
                theme,
                Pos2::new(pos.x, pos.y + y_offset),
                max_width,
                opacity,
                indent_level + 1,
                reveal_step,
                step_counter,
                scale,
            );
            y_offset += child_height;
        }
    }

    y_offset
}

/// Measure the height a list will occupy when fully revealed, using the same
/// wrapping and spacing as [`draw_list`].
pub fn measure_list_height(
    ui: &egui::Ui,
    items: &[ListItem],
    theme: &Theme,
    max_width: f32,
    indent_level: usize,
    scale: f32,
) -> f32 {
    let (_, _, text_width) = list_metrics(max_width, indent_level, scale);
    let item_spacing = LIST_ITEM_SPACING * scale;
    let font_size = theme.body_size * scale;
    let mut total = 0.0;

    for item in items {
        total += measure_inlines(ui, &item.inlines, font_size, text_width, theme) + item_spacing;
        if !item.children.is_empty() {
            total += measure_list_height(
                ui,
                &item.children,
                theme,
                max_width,
                indent_level + 1,
                scale,
            );
        }
    }

    total
}

// ---------------------------------------------------------------------------
// Code blocks
// ---------------------------------------------------------------------------

const CODE_PADDING: f32 = 16.0;

/// Draw a code block with syntax highlighting. Returns height used.
#[allow(clippy::too_many_arguments)]
pub fn draw_code_block(
    ui: &egui::Ui,
    code: &str,
    language: Option<&str>,
    highlight_lines: &[usize],
    theme: &Theme,
    pos: Pos2,
    max_width: f32,
    opacity: f32,
    scale: f32,
) -> f32 {
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
    let code_galley = ui.painter().layout_job(job);

    let total_height = code_galley.rect.height() + padding * 2.0;

    // Draw background
    let bg_rect = egui::Rect::from_min_size(pos, egui::vec2(max_width, total_height));
    ui.painter().rect_filled(bg_rect, 8.0 * scale, bg_color);

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
                ui.painter().rect_filled(hl_rect, 4.0 * scale, accent);
            }

            if row.ends_with_newline {
                source_line += 1;
            }
        }
    }

    // Draw code
    let code_pos = Pos2::new(pos.x + padding, pos.y + padding);
    let fallback = Theme::with_opacity(theme.code_foreground, opacity);
    ui.painter().galley(code_pos, code_galley, fallback);

    total_height
}

/// Measure a code block exactly as [`draw_code_block`] lays it out.
fn measure_code_block_height(
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

// ---------------------------------------------------------------------------
// Tables
// ---------------------------------------------------------------------------

/// Resolved geometry for a table: font size and per-column widths.
struct TableLayout {
    font_size: f32,
    /// Width of each column, including the inter-column gap (`cell_padding`).
    col_widths: Vec<f32>,
    cell_padding: f32,
}

impl TableLayout {
    /// Text width available inside column `col`.
    fn text_width(&self, col: usize) -> f32 {
        self.col_widths[col] - self.cell_padding
    }

    /// Left edge of column `col` relative to the table's left edge.
    fn col_offset(&self, col: usize) -> f32 {
        self.cell_padding + self.col_widths[..col].iter().sum::<f32>()
    }
}

/// Compute column widths from content. Columns get their natural (unwrapped)
/// width, capped so one long column cannot starve the others; the font shrinks
/// (down to 0.7x body) when the natural width exceeds `max_width`, and any
/// remaining space is distributed proportionally.
fn layout_table(
    ui: &egui::Ui,
    headers: &[Vec<Inline>],
    rows: &[Vec<Vec<Inline>>],
    theme: &Theme,
    max_width: f32,
    scale: f32,
) -> TableLayout {
    let cell_padding = 12.0 * scale;
    let num_cols = headers.len().max(1);
    let available = (max_width - cell_padding * 2.0).max(cell_padding * num_cols as f32);
    let max_col_width = available * 0.6;

    let natural_widths = |font_size: f32| -> Vec<f32> {
        let mut widths = vec![cell_padding; num_cols];
        let all_rows = std::iter::once(headers).chain(rows.iter().map(Vec::as_slice));
        for row in all_rows {
            for (col, cell) in row.iter().enumerate().take(num_cols) {
                let job = inlines_to_job(cell, font_size, theme.foreground, f32::INFINITY, theme);
                let w = ui.painter().layout_job(job).rect.width() + cell_padding;
                widths[col] = widths[col].max(w).min(max_col_width);
            }
        }
        widths
    };

    let base_font = theme.body_size * 0.85 * scale;
    let min_font = theme.body_size * 0.7 * scale;

    let mut font_size = base_font;
    let mut widths = natural_widths(font_size);
    let mut total: f32 = widths.iter().sum();
    if total > available {
        font_size = (font_size * available / total).max(min_font);
        widths = natural_widths(font_size);
        total = widths.iter().sum();
    }

    // Scale to fit (wrapping if still too wide) or hand out the slack.
    let factor = available / total;
    for w in &mut widths {
        *w *= factor;
    }

    TableLayout {
        font_size,
        col_widths: widths,
        cell_padding,
    }
}

/// Height of the tallest cell in a row (cells beyond the header count are ignored).
fn table_row_height(
    ui: &egui::Ui,
    cells: &[Vec<Inline>],
    layout: &TableLayout,
    theme: &Theme,
) -> f32 {
    cells
        .iter()
        .enumerate()
        .take(layout.col_widths.len())
        .map(|(col, cell)| {
            measure_inlines(ui, cell, layout.font_size, layout.text_width(col), theme)
        })
        .fold(0.0, f32::max)
}

/// Total table height for a given layout (shared by draw and measure).
fn table_height(
    ui: &egui::Ui,
    headers: &[Vec<Inline>],
    rows: &[Vec<Vec<Inline>>],
    layout: &TableLayout,
    theme: &Theme,
    scale: f32,
) -> f32 {
    let pad = layout.cell_padding;
    let mut h = table_row_height(ui, headers, layout, theme) + pad * 2.0;
    h += TABLE_ROW_SPACING * scale;
    for row in rows {
        h += table_row_height(ui, row, layout, theme) + pad * 1.5;
    }
    h
}

const TABLE_ROW_SPACING: f32 = 4.0;

/// Measure a table exactly as [`draw_table`] lays it out.
pub fn measure_table_height(
    ui: &egui::Ui,
    headers: &[Vec<Inline>],
    rows: &[Vec<Vec<Inline>>],
    theme: &Theme,
    max_width: f32,
    scale: f32,
) -> f32 {
    let layout = layout_table(ui, headers, rows, theme, max_width, scale);
    table_height(ui, headers, rows, &layout, theme, scale)
}

/// Draw a table. Returns height used.
#[allow(clippy::too_many_arguments)]
pub fn draw_table(
    ui: &egui::Ui,
    headers: &[Vec<Inline>],
    rows: &[Vec<Vec<Inline>>],
    theme: &Theme,
    pos: Pos2,
    max_width: f32,
    opacity: f32,
    scale: f32,
) -> f32 {
    let color = Theme::with_opacity(theme.foreground, opacity);
    let heading_color = Theme::with_opacity(theme.heading_color, opacity);
    let accent = Theme::with_opacity(theme.accent, opacity);
    let header_bg = Theme::with_opacity(theme.accent, opacity * 0.12);
    let zebra = Theme::with_opacity(theme.foreground, opacity * 0.04);

    let layout = layout_table(ui, headers, rows, theme, max_width, scale);
    let pad = layout.cell_padding;
    let num_cols = layout.col_widths.len();
    let rounding = 6.0 * scale;
    let band_left = pos.x + pad * 0.5;
    let band_right = pos.x + max_width - pad * 0.5;

    let mut y = pos.y;

    // Header row: subtle accent band behind the header text
    let header_h = table_row_height(ui, headers, &layout, theme);
    let header_rect = egui::Rect::from_min_max(
        Pos2::new(band_left, y),
        Pos2::new(band_right, y + header_h + pad * 2.0),
    );
    ui.painter().rect_filled(header_rect, rounding, header_bg);
    for (col, header) in headers.iter().enumerate().take(num_cols) {
        let cell_pos = Pos2::new(pos.x + layout.col_offset(col), y + pad);
        draw_inlines(
            ui,
            header,
            cell_pos,
            layout.font_size,
            heading_color,
            layout.text_width(col),
            theme,
        );
    }
    y += header_h + pad * 2.0;

    // Separator line
    let row_spacing = TABLE_ROW_SPACING * scale;
    let line_y = y + row_spacing / 2.0;
    ui.painter().line_segment(
        [
            Pos2::new(pos.x + pad, line_y),
            Pos2::new(pos.x + max_width - pad, line_y),
        ],
        Stroke::new(1.0_f32, accent),
    );
    y += row_spacing;

    // Data rows with a faint zebra stripe on every other row
    for (row_idx, row) in rows.iter().enumerate() {
        let row_h = table_row_height(ui, row, &layout, theme);
        let band_h = row_h + pad * 1.5;
        if row_idx % 2 == 1 {
            let band = egui::Rect::from_min_max(
                Pos2::new(band_left, y),
                Pos2::new(band_right, y + band_h),
            );
            ui.painter().rect_filled(band, rounding, zebra);
        }
        for (col, cell) in row.iter().enumerate().take(num_cols) {
            let cell_pos = Pos2::new(pos.x + layout.col_offset(col), y + pad * 0.75);
            draw_inlines(
                ui,
                cell,
                cell_pos,
                layout.font_size,
                color,
                layout.text_width(col),
                theme,
            );
        }
        y += band_h;
    }

    y - pos.y
}

// ---------------------------------------------------------------------------
// Block quotes
// ---------------------------------------------------------------------------

const QUOTE_BAR_WIDTH: f32 = 4.0;
const QUOTE_BAR_PADDING: f32 = 16.0;

/// Draw a blockquote. Returns height used.
pub fn draw_blockquote(
    ui: &egui::Ui,
    inlines: &[Inline],
    theme: &Theme,
    pos: Pos2,
    max_width: f32,
    opacity: f32,
    scale: f32,
) -> f32 {
    let accent = Theme::with_opacity(theme.accent, opacity);
    let color = Theme::with_opacity(theme.foreground, opacity);
    let bar_width = QUOTE_BAR_WIDTH * scale;
    let bar_padding = QUOTE_BAR_PADDING * scale;
    let font_size = theme.body_size * 1.1 * scale;

    let text_pos = Pos2::new(pos.x + bar_width + bar_padding, pos.y);
    let text_width = max_width - bar_width - bar_padding;

    let height = draw_inlines(ui, inlines, text_pos, font_size, color, text_width, theme);

    // Draw accent bar
    let bar_rect = egui::Rect::from_min_size(pos, egui::vec2(bar_width, height));
    ui.painter().rect_filled(bar_rect, 2.0, accent);

    height
}

// ---------------------------------------------------------------------------
// Block sequences
// ---------------------------------------------------------------------------

/// Draw blocks sequentially with the shared spacing rule. Returns the total
/// height used (spacing is added between blocks only, matching
/// [`measure_blocks_height`]).
#[allow(clippy::too_many_arguments)]
pub fn draw_blocks<'a>(
    ui: &egui::Ui,
    blocks: impl IntoIterator<Item = &'a Block>,
    theme: &Theme,
    pos: Pos2,
    max_width: f32,
    opacity: f32,
    image_cache: &ImageCache,
    reveal_step: usize,
    scale: f32,
) -> f32 {
    let mut y_offset = 0.0;
    let mut pending_spacing = 0.0;

    for block in blocks {
        y_offset += pending_spacing;
        let block_pos = Pos2::new(pos.x, pos.y + y_offset);
        let height = draw_block(
            ui,
            block,
            theme,
            block_pos,
            max_width,
            opacity,
            image_cache,
            reveal_step,
            scale,
        );
        y_offset += height;
        pending_spacing = block_spacing(block, theme, scale);
    }

    y_offset
}

/// Measure total height of a block sequence without drawing.
pub fn measure_blocks_height<'a>(
    ui: &egui::Ui,
    blocks: impl IntoIterator<Item = &'a Block>,
    theme: &Theme,
    max_width: f32,
    scale: f32,
) -> f32 {
    let mut total = 0.0;
    let mut pending_spacing = 0.0;
    for block in blocks {
        total += pending_spacing;
        total += measure_single_block_height(ui, block, theme, max_width, scale);
        pending_spacing = block_spacing(block, theme, scale);
    }
    total
}

/// Measure the height of a single block without drawing. Text-like blocks are
/// laid out exactly as their drawing counterparts; visualizations are sized by
/// the layout that hosts them and get a nominal height here.
pub fn measure_single_block_height(
    ui: &egui::Ui,
    block: &Block,
    theme: &Theme,
    max_width: f32,
    scale: f32,
) -> f32 {
    match block {
        Block::Heading { level, inlines } => {
            let size = theme.heading_size(*level) * scale;
            measure_inlines(ui, inlines, size, max_width, theme)
        }
        Block::Paragraph { inlines } => {
            measure_inlines(ui, inlines, theme.body_size * scale, max_width, theme)
        }
        Block::BlockQuote { inlines } => {
            let text_width = max_width - (QUOTE_BAR_WIDTH + QUOTE_BAR_PADDING) * scale;
            measure_inlines(
                ui,
                inlines,
                theme.body_size * 1.1 * scale,
                text_width,
                theme,
            )
        }
        Block::List { items, .. } => measure_list_height(ui, items, theme, max_width, 0, scale),
        Block::CodeBlock { code, language, .. } => {
            measure_code_block_height(ui, code, language.as_deref(), theme, max_width, scale)
        }
        Block::Table { headers, rows } => {
            measure_table_height(ui, headers, rows, theme, max_width, scale)
        }
        Block::HorizontalRule => 20.0 * scale,
        Block::Diagram { .. }
        | Block::WordCloud { .. }
        | Block::Timeline { .. }
        | Block::PieChart { .. }
        | Block::BarChart { .. }
        | Block::LineChart { .. }
        | Block::DonutChart { .. }
        | Block::KpiCards { .. }
        | Block::FunnelChart { .. }
        | Block::RadarChart { .. }
        | Block::StackedBar { .. }
        | Block::VennDiagram { .. }
        | Block::ProgressBars { .. }
        | Block::ScatterPlot { .. }
        | Block::OrgChart { .. }
        | Block::GanttChart { .. }
        | Block::GitGraph { .. } => 500.0 * scale, // visualizations fill available space
        Block::Image { .. } => IMAGE_MAX_HEIGHT * scale,
        Block::ColumnSeparator => 0.0,
    }
}

/// Draw a single block. Returns height used.
#[allow(clippy::too_many_arguments)]
pub fn draw_block(
    ui: &egui::Ui,
    block: &Block,
    theme: &Theme,
    pos: Pos2,
    max_width: f32,
    opacity: f32,
    image_cache: &ImageCache,
    reveal_step: usize,
    scale: f32,
) -> f32 {
    match block {
        Block::Heading { level, inlines } => {
            draw_heading(ui, inlines, *level, theme, pos, max_width, opacity, scale)
        }
        Block::Paragraph { inlines } => {
            draw_paragraph(ui, inlines, theme, pos, max_width, opacity, scale)
        }
        Block::List { ordered, items } => draw_list(
            ui,
            items,
            *ordered,
            theme,
            pos,
            max_width,
            opacity,
            0,
            reveal_step,
            scale,
        ),
        Block::CodeBlock {
            language,
            code,
            highlight_lines,
        } => draw_code_block(
            ui,
            code,
            language.as_deref(),
            highlight_lines,
            theme,
            pos,
            max_width,
            opacity,
            scale,
        ),
        Block::BlockQuote { inlines } => {
            draw_blockquote(ui, inlines, theme, pos, max_width, opacity, scale)
        }
        Block::Table { headers, rows } => {
            draw_table(ui, headers, rows, theme, pos, max_width, opacity, scale)
        }
        Block::Image {
            alt,
            path,
            directives,
        } => draw_image(
            ui,
            path,
            alt,
            directives,
            theme,
            pos,
            max_width,
            opacity,
            image_cache,
            scale,
        ),
        Block::Diagram { content } => draw_diagram_sized(
            ui,
            content,
            theme,
            pos,
            max_width,
            0.0,
            opacity,
            image_cache,
            reveal_step,
            None,
            scale,
        ),
        Block::WordCloud { content } => crate::render::visualizations::word_cloud::draw_word_cloud(
            ui,
            content,
            theme,
            pos,
            max_width,
            0.0,
            opacity,
            reveal_step,
            scale,
        ),
        Block::Timeline { content } => crate::render::visualizations::timeline::draw_timeline(
            ui,
            content,
            theme,
            pos,
            max_width,
            0.0,
            opacity,
            reveal_step,
            scale,
        ),
        Block::PieChart { content } => crate::render::visualizations::pie_chart::draw_pie_chart(
            ui,
            content,
            theme,
            pos,
            max_width,
            0.0,
            opacity,
            reveal_step,
            None,
            scale,
        ),
        Block::BarChart { content } => crate::render::visualizations::bar_chart::draw_bar_chart(
            ui,
            content,
            theme,
            pos,
            max_width,
            0.0,
            opacity,
            reveal_step,
            None,
            scale,
        ),
        Block::LineChart { content } => crate::render::visualizations::line_chart::draw_line_chart(
            ui,
            content,
            theme,
            pos,
            max_width,
            0.0,
            opacity,
            reveal_step,
            None,
            scale,
        ),
        Block::DonutChart { content } => {
            crate::render::visualizations::donut_chart::draw_donut_chart(
                ui,
                content,
                theme,
                pos,
                max_width,
                0.0,
                opacity,
                reveal_step,
                None,
                scale,
            )
        }
        Block::KpiCards { content } => crate::render::visualizations::kpi_cards::draw_kpi_cards(
            ui,
            content,
            theme,
            pos,
            max_width,
            0.0,
            opacity,
            reveal_step,
            None,
            scale,
        ),
        Block::FunnelChart { content } => {
            crate::render::visualizations::funnel_chart::draw_funnel_chart(
                ui,
                content,
                theme,
                pos,
                max_width,
                0.0,
                opacity,
                reveal_step,
                None,
                scale,
            )
        }
        Block::RadarChart { content } => {
            crate::render::visualizations::radar_chart::draw_radar_chart(
                ui,
                content,
                theme,
                pos,
                max_width,
                0.0,
                opacity,
                reveal_step,
                None,
                scale,
            )
        }
        Block::StackedBar { content } => {
            crate::render::visualizations::stacked_bar::draw_stacked_bar(
                ui,
                content,
                theme,
                pos,
                max_width,
                0.0,
                opacity,
                reveal_step,
                None,
                scale,
            )
        }
        Block::VennDiagram { content } => {
            crate::render::visualizations::venn_diagram::draw_venn_diagram(
                ui,
                content,
                theme,
                pos,
                max_width,
                0.0,
                opacity,
                reveal_step,
                None,
                scale,
            )
        }
        Block::ProgressBars { content } => {
            crate::render::visualizations::progress_bars::draw_progress_bars(
                ui,
                content,
                theme,
                pos,
                max_width,
                0.0,
                opacity,
                reveal_step,
                None,
                scale,
            )
        }
        Block::ScatterPlot { content } => {
            crate::render::visualizations::scatter_plot::draw_scatter_plot(
                ui,
                content,
                theme,
                pos,
                max_width,
                0.0,
                opacity,
                reveal_step,
                None,
                scale,
            )
        }
        Block::OrgChart { content } => crate::render::visualizations::org_chart::draw_org_chart(
            ui,
            content,
            theme,
            pos,
            max_width,
            0.0,
            opacity,
            reveal_step,
            None,
            scale,
        ),
        Block::GanttChart { content } => {
            crate::render::visualizations::gantt_chart::draw_gantt_chart(
                ui,
                content,
                theme,
                pos,
                max_width,
                0.0,
                opacity,
                reveal_step,
                None,
                scale,
            )
        }
        Block::GitGraph { content } => crate::render::visualizations::git_graph::draw_gitgraph(
            ui,
            content,
            theme,
            pos,
            max_width,
            0.0,
            opacity,
            reveal_step,
            scale,
        ),
        Block::HorizontalRule => {
            let color = Theme::with_opacity(theme.accent, opacity * 0.5);
            let y = pos.y + 10.0 * scale;
            ui.painter().line_segment(
                [Pos2::new(pos.x, y), Pos2::new(pos.x + max_width, y)],
                Stroke::new(1.0_f32, color),
            );
            20.0 * scale
        }
        Block::ColumnSeparator => 0.0, // handled by two-column layout
    }
}

// ---------------------------------------------------------------------------
// Images
// ---------------------------------------------------------------------------

/// Maximum height (at reference resolution) of an image drawn inline in a block flow.
const IMAGE_MAX_HEIGHT: f32 = 400.0;

/// Draw an image, loading from cache. Falls back to placeholder if unavailable.
#[allow(clippy::too_many_arguments)]
pub fn draw_image(
    ui: &egui::Ui,
    path: &str,
    alt: &str,
    directives: &ImageDirectives,
    theme: &Theme,
    pos: Pos2,
    max_width: f32,
    opacity: f32,
    image_cache: &ImageCache,
    scale: f32,
) -> f32 {
    let max_height = IMAGE_MAX_HEIGHT * scale;
    let available = egui::Rect::from_min_size(pos, egui::vec2(max_width, max_height));
    let drawn = draw_image_in_area(
        ui,
        path,
        alt,
        directives,
        theme,
        available,
        opacity,
        image_cache,
        scale,
    );
    drawn.height()
}

/// Draw an image with full control over the available area (used by image_slide layout).
/// Returns the actual drawn rect.
#[allow(clippy::too_many_arguments)]
pub fn draw_image_in_area(
    ui: &egui::Ui,
    path: &str,
    alt: &str,
    directives: &ImageDirectives,
    theme: &Theme,
    available: egui::Rect,
    opacity: f32,
    image_cache: &ImageCache,
    scale: f32,
) -> egui::Rect {
    if let Some(texture) = image_cache.get_or_load(ui, path) {
        let tex_size = texture.size_vec2();
        let draw_rect = compute_image_rect(directives, tex_size, available, scale);
        let alpha = (opacity * 255.0) as u8;
        let tint = Color32::from_rgba_unmultiplied(255, 255, 255, alpha);
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        ui.painter().image(texture.id(), draw_rect, uv, tint);
        draw_rect
    } else {
        let height = draw_image_placeholder(
            ui,
            alt,
            directives,
            theme,
            available.left_top(),
            available.width(),
            opacity,
            scale,
        );
        egui::Rect::from_min_size(available.left_top(), egui::vec2(available.width(), height))
    }
}

/// Compute where an image of `tex_size` pixels is drawn inside `available`.
///
/// `scale` is the resolution scale factor: images are never upscaled beyond
/// their reference-resolution size (`tex_size * scale`), so a picture looks the
/// same at 1080p and in a 4K export.
fn compute_image_rect(
    directives: &ImageDirectives,
    tex_size: egui::Vec2,
    available: egui::Rect,
    scale: f32,
) -> egui::Rect {
    let avail_w = available.width();
    let avail_h = available.height();

    let factor = if directives.fill {
        // Cover: scale to fill, center, may crop
        (avail_w / tex_size.x).max(avail_h / tex_size.y)
    } else if let Some(ref width_str) = directives.width {
        // Explicit width, still bounded by the available height
        let target_w = parse_size(width_str, avail_w, scale);
        (target_w / tex_size.x).min(avail_h / tex_size.y)
    } else {
        // Contain: fit within available area, preserve aspect ratio
        (avail_w / tex_size.x).min(avail_h / tex_size.y).min(scale)
    };

    let draw_w = tex_size.x * factor;
    let draw_h = tex_size.y * factor;
    let offset_x = (avail_w - draw_w) / 2.0;
    let offset_y = (avail_h - draw_h) / 2.0;
    egui::Rect::from_min_size(
        egui::pos2(available.left() + offset_x, available.top() + offset_y),
        egui::vec2(draw_w, draw_h),
    )
}

/// Parse a size directive. Percentages are relative to `reference`; pixel
/// values (with or without a `px` suffix) are in reference-resolution pixels
/// and are multiplied by `scale`.
fn parse_size(s: &str, reference: f32, scale: f32) -> f32 {
    let s = s.trim();
    if let Some(pct) = s.strip_suffix('%') {
        if let Ok(v) = pct.trim().parse::<f32>() {
            return reference * v / 100.0;
        }
    }
    let px = s.strip_suffix("px").unwrap_or(s).trim();
    match px.parse::<f32>() {
        Ok(v) => v * scale,
        Err(_) => reference * 0.8,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_image_placeholder(
    ui: &egui::Ui,
    alt: &str,
    _directives: &crate::parser::ImageDirectives,
    theme: &Theme,
    pos: Pos2,
    max_width: f32,
    opacity: f32,
    scale: f32,
) -> f32 {
    let height = 200.0 * scale;
    let bg = Theme::with_opacity(theme.code_background, opacity);
    let color = Theme::with_opacity(theme.foreground, opacity * 0.6);

    let rect = egui::Rect::from_min_size(pos, egui::vec2(max_width, height));
    ui.painter().rect_filled(rect, 8.0 * scale, bg);
    ui.painter().rect_stroke(
        rect,
        8.0 * scale,
        Stroke::new(1.0_f32, color),
        egui::StrokeKind::Outside,
    );

    let label = if alt.is_empty() {
        "[Image]".to_string()
    } else {
        format!("[Image: {alt}]")
    };
    let galley = ui.painter().layout(
        label,
        FontId::proportional(theme.body_size * 0.8 * scale),
        color,
        max_width,
    );
    let text_pos = Pos2::new(
        pos.x + (max_width - galley.rect.width()) / 2.0,
        pos.y + (height - galley.rect.height()) / 2.0,
    );
    ui.painter().galley(text_pos, galley, color);

    height
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::test_support::with_ui;

    fn text(s: &str) -> Vec<Inline> {
        vec![Inline::Text(s.to_string())]
    }

    fn item(s: &str, children: Vec<ListItem>) -> ListItem {
        ListItem {
            marker: ListMarker::Static,
            inlines: text(s),
            children,
        }
    }

    const LONG: &str = "This is a deliberately long list item whose text is guaranteed to wrap \
        onto several rows when laid out inside a narrow column, so that the measured height \
        depends on real galley layout rather than a per-item constant.";

    #[test]
    fn wrapped_list_measures_exactly_as_drawn() {
        with_ui(|ui| {
            let theme = Theme::dark();
            let cache = ImageCache::new(std::path::PathBuf::new());
            let items = vec![
                item(LONG, vec![item("child", vec![])]),
                item("short", vec![]),
            ];
            let block = Block::List {
                ordered: false,
                items,
            };
            let width = 700.0;

            let measured = measure_single_block_height(ui, &block, &theme, width, 1.0);
            let drawn = draw_block(
                ui,
                &block,
                &theme,
                Pos2::ZERO,
                width,
                1.0,
                &cache,
                usize::MAX,
                1.0,
            );
            assert!((measured - drawn).abs() < 0.01, "{measured} vs {drawn}");

            // The old estimate was `count * (font_size + 8)`; a wrapped item must exceed it.
            let naive = 3.0 * (theme.body_size + 8.0);
            assert!(measured > naive * 1.5, "{measured} should reflect wrapping");
        });
    }

    #[test]
    fn deeply_nested_list_keeps_minimum_text_width() {
        let (_, _, w) = list_metrics(300.0, 20, 1.0);
        assert_eq!(w, LIST_MIN_TEXT_WIDTH);
        let (_, _, w) = list_metrics(1000.0, 0, 1.0);
        assert_eq!(w, 1000.0 - LIST_MARKER_WIDTH);
    }

    #[test]
    fn block_sequence_measures_exactly_as_drawn() {
        with_ui(|ui| {
            let theme = Theme::dark();
            let cache = ImageCache::new(std::path::PathBuf::new());
            let blocks = vec![
                Block::Heading {
                    level: 1,
                    inlines: text("A heading that is long enough to wrap in a narrow column"),
                },
                Block::Paragraph {
                    inlines: text(LONG),
                },
                Block::List {
                    ordered: true,
                    items: vec![item(LONG, vec![]), item("two", vec![])],
                },
                Block::CodeBlock {
                    language: Some("rust".into()),
                    code: "fn main() {\n    println!(\"hi\");\n}".into(),
                    highlight_lines: vec![],
                },
                Block::Table {
                    headers: vec![text("Name"), text("Value")],
                    rows: vec![vec![text("a"), text(LONG)], vec![text("b"), text("2")]],
                },
                Block::BlockQuote {
                    inlines: text(LONG),
                },
                Block::HorizontalRule,
            ];
            let width = 640.0;
            let measured = measure_blocks_height(ui, &blocks, &theme, width, 1.0);
            let drawn = draw_blocks(
                ui,
                &blocks,
                &theme,
                Pos2::ZERO,
                width,
                1.0,
                &cache,
                usize::MAX,
                1.0,
            );
            assert!((measured - drawn).abs() < 0.01, "{measured} vs {drawn}");
        });
    }

    #[test]
    fn table_columns_size_to_content_and_never_exceed_width() {
        with_ui(|ui| {
            let theme = Theme::dark();
            let headers = vec![text("Id"), text("Description")];
            let rows = vec![vec![text("1"), text(LONG)], vec![text("2"), text("x")]];
            let layout = layout_table(ui, &headers, &rows, &theme, 1000.0, 1.0);
            assert!(layout.col_widths[1] > layout.col_widths[0]);
            let total: f32 = layout.col_widths.iter().sum::<f32>() + layout.cell_padding * 2.0;
            assert!((total - 1000.0).abs() < 0.5, "columns fill width: {total}");
        });
    }

    #[test]
    fn wide_table_shrinks_font_and_clamps_extra_cells() {
        with_ui(|ui| {
            let theme = Theme::dark();
            let headers: Vec<_> = (0..8).map(|i| text(&format!("Column {i}"))).collect();
            // A row with more cells than headers must not affect the width.
            let rows = vec![(0..12).map(|i| text(&format!("value {i}"))).collect()];
            let layout = layout_table(ui, &headers, &rows, &theme, 900.0, 1.0);
            assert_eq!(layout.col_widths.len(), 8);
            assert!(layout.font_size < theme.body_size * 0.85);
            assert!(layout.font_size >= theme.body_size * 0.7 - 0.01);
            let total: f32 = layout.col_widths.iter().sum::<f32>() + layout.cell_padding * 2.0;
            assert!(total <= 900.5, "{total}");
        });
    }

    #[test]
    fn bold_and_links_use_theme_colours_with_incoming_alpha() {
        let theme = Theme::dark();
        let faded = Color32::from_rgba_unmultiplied(200, 200, 200, 128);
        let inlines = vec![
            Inline::Bold(text("bold")),
            Inline::Link {
                text: text("link"),
                url: "https://example.com".into(),
            },
            Inline::Code("code".into()),
        ];
        let job = inlines_to_job(&inlines, 20.0, faded, 1000.0, &theme);
        let bold = &job.sections[0].format;
        assert_eq!(bold.color, with_alpha(theme.heading_color, 128));
        assert_eq!(bold.font_id.size, 21.0);
        let link = &job.sections[1].format;
        assert_eq!(link.color, with_alpha(theme.accent, 128));
        let code = &job.sections[2].format;
        assert_eq!(
            code.background,
            with_alpha(theme.accent, (128.0 * 0.12) as u8)
        );
    }

    #[test]
    fn strong_colour_uses_heading_only_when_visibly_brighter() {
        // Dark: white headings vs grey body — clearly brighter.
        assert_eq!(strong_color(&Theme::dark()), Theme::dark().heading_color);
        // Light and nord: heading and body colours are nearly identical, so
        // the accent is used to make bold visible.
        assert_eq!(strong_color(&Theme::light()), Theme::light().accent);
        assert_eq!(strong_color(&Theme::nord()), Theme::nord().accent);
    }

    #[test]
    fn contain_mode_upscales_only_to_reference_size() {
        let d = ImageDirectives::default();
        let tex = egui::vec2(400.0, 200.0);
        let area = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(2000.0, 2000.0));
        // 1080p: never larger than native pixels
        assert_eq!(compute_image_rect(&d, tex, area, 1.0).width(), 400.0);
        // 4K export: twice the pixels, same apparent size
        assert_eq!(compute_image_rect(&d, tex, area, 2.0).width(), 800.0);
        // Still bounded by the available area
        let small = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(100.0, 2000.0));
        assert_eq!(compute_image_rect(&d, tex, small, 2.0).width(), 100.0);
    }

    #[test]
    fn width_directive_scales_with_resolution() {
        assert_eq!(parse_size("300px", 1000.0, 2.0), 600.0);
        assert_eq!(parse_size("300", 1000.0, 2.0), 600.0);
        assert_eq!(parse_size("50%", 1000.0, 2.0), 500.0);
        assert_eq!(parse_size("garbage", 1000.0, 2.0), 800.0);

        let d = ImageDirectives {
            width: Some("300px".into()),
            ..Default::default()
        };
        let tex = egui::vec2(600.0, 300.0);
        let area = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(2000.0, 2000.0));
        assert_eq!(compute_image_rect(&d, tex, area, 2.0).width(), 600.0);
    }

    #[test]
    fn heading_spacing_is_half_the_heading_size() {
        let theme = Theme::dark();
        assert_eq!(heading_spacing(&theme, 1, 1.0), theme.h1_size * 0.5);
        assert_eq!(heading_spacing(&theme, 2, 2.0), theme.h2_size);
        let h = Block::Heading {
            level: 3,
            inlines: vec![],
        };
        assert_eq!(block_spacing(&h, &theme, 1.0), theme.h3_size * 0.5);
    }
}
