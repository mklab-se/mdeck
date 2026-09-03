use eframe::egui::{self, Pos2};

use crate::parser::{Block, Slide};
use crate::render::image_cache::ImageCache;
use crate::render::layouts::{SLIDE_PADDING, centered_left, centered_top, content_width};
use crate::render::text;
use crate::theme::Theme;

const COLUMN_GAP: f32 = 40.0;

/// Blocks of a two-column slide split into their regions.
struct Columns<'a> {
    heading: Vec<&'a Block>,
    left: Vec<&'a Block>,
    right: Vec<&'a Block>,
}

/// Split blocks at the `ColumnSeparator`. A leading H1/H2 spans both columns.
fn split_columns(blocks: &[Block]) -> Columns<'_> {
    let mut columns = Columns {
        heading: Vec::new(),
        left: Vec::new(),
        right: Vec::new(),
    };
    let mut in_right = false;

    for block in blocks {
        if matches!(block, Block::ColumnSeparator) {
            in_right = true;
            continue;
        }
        if !in_right
            && columns.left.is_empty()
            && matches!(
                block,
                Block::Heading { level: 1, .. } | Block::Heading { level: 2, .. }
            )
        {
            columns.heading.push(block);
            continue;
        }
        if in_right {
            columns.right.push(block);
        } else {
            columns.left.push(block);
        }
    }

    columns
}

struct Geometry {
    content_rect: egui::Rect,
    col_width: f32,
    gap: f32,
}

fn geometry(rect: egui::Rect, scale: f32) -> Geometry {
    let v_padding = SLIDE_PADDING * scale;
    let width = content_width(crate::parser::Layout::TwoColumn, rect, scale);
    let left = centered_left(rect, width);
    let content_rect = egui::Rect::from_min_max(
        egui::pos2(left, rect.top() + v_padding),
        egui::pos2(left + width, rect.bottom() - v_padding),
    );
    let gap = COLUMN_GAP * scale;
    Geometry {
        content_rect,
        col_width: (width - gap) / 2.0,
        gap,
    }
}

/// Heading height including the spacing that follows it.
fn heading_height(ui: &egui::Ui, columns: &Columns, theme: &Theme, width: f32, scale: f32) -> f32 {
    columns
        .heading
        .iter()
        .map(|b| {
            text::measure_single_block_height(ui, b, theme, width, scale)
                + text::block_spacing(b, theme, scale)
        })
        .sum()
}

/// Content height of a two-column slide: heading plus the taller column, each
/// laid out at the width it is drawn with.
pub fn measure_content_height(
    ui: &egui::Ui,
    slide: &Slide,
    theme: &Theme,
    rect: egui::Rect,
    scale: f32,
) -> f32 {
    let geo = geometry(rect, scale);
    let columns = split_columns(&slide.blocks);
    let heading = heading_height(ui, &columns, theme, geo.content_rect.width(), scale);
    let left = text::measure_blocks_height(
        ui,
        columns.left.iter().copied(),
        theme,
        geo.col_width,
        scale,
    );
    let right = text::measure_blocks_height(
        ui,
        columns.right.iter().copied(),
        theme,
        geo.col_width,
        scale,
    );
    heading + left.max(right)
}

#[allow(clippy::too_many_arguments)]
pub fn render(
    ui: &egui::Ui,
    slide: &Slide,
    theme: &Theme,
    rect: egui::Rect,
    opacity: f32,
    image_cache: &ImageCache,
    reveal_step: usize,
    scale: f32,
) {
    let geo = geometry(rect, scale);
    let content_rect = geo.content_rect;
    let columns = split_columns(&slide.blocks);

    let total_height = measure_content_height(ui, slide, theme, rect, scale);
    let mut y = centered_top(content_rect.top(), content_rect.height(), total_height);

    // Draw heading spanning full width
    for block in &columns.heading {
        if let Block::Heading { level, inlines } = block {
            let h = text::draw_heading(
                ui,
                inlines,
                *level,
                theme,
                Pos2::new(content_rect.left(), y),
                content_rect.width(),
                opacity,
                scale,
            );
            y += h + text::block_spacing(block, theme, scale);
        }
    }

    // Draw both columns
    text::draw_blocks(
        ui,
        columns.left.iter().copied(),
        theme,
        Pos2::new(content_rect.left(), y),
        geo.col_width,
        opacity,
        image_cache,
        reveal_step,
        scale,
    );
    text::draw_blocks(
        ui,
        columns.right.iter().copied(),
        theme,
        Pos2::new(content_rect.left() + geo.col_width + geo.gap, y),
        geo.col_width,
        opacity,
        image_cache,
        reveal_step,
        scale,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{Inline, Layout};
    use crate::render::test_support::with_ui;

    fn para(s: &str) -> Block {
        Block::Paragraph {
            inlines: vec![Inline::Text(s.to_string())],
        }
    }

    #[test]
    fn measures_heading_plus_tallest_column_not_the_sum() {
        with_ui(|ui| {
            let theme = Theme::dark();
            let rect = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(1920.0, 1080.0));
            let long = "A long paragraph that wraps over a few rows in a narrow column, so its \
                        height is clearly larger than a single short line of text.";
            let slide = Slide {
                directives: vec![],
                blocks: vec![
                    Block::Heading {
                        level: 1,
                        inlines: vec![Inline::Text("Title".into())],
                    },
                    para(long),
                    para(long),
                    Block::ColumnSeparator,
                    para("short"),
                ],
                layout: Layout::TwoColumn,
                raw_source: String::new(),
                notes: None,
            };

            let geo = geometry(rect, 1.0);
            let columns = split_columns(&slide.blocks);
            assert_eq!(columns.heading.len(), 1);
            assert_eq!(columns.left.len(), 2);
            assert_eq!(columns.right.len(), 1);

            let heading = heading_height(ui, &columns, &theme, geo.content_rect.width(), 1.0);
            let left = text::measure_blocks_height(
                ui,
                columns.left.iter().copied(),
                &theme,
                geo.col_width,
                1.0,
            );
            let right = text::measure_blocks_height(
                ui,
                columns.right.iter().copied(),
                &theme,
                geo.col_width,
                1.0,
            );
            assert!(left > right);

            let measured = measure_content_height(ui, &slide, &theme, rect, 1.0);
            assert!((measured - (heading + left)).abs() < 0.01);
            // Stacking everything would be taller than the real layout.
            let stacked =
                text::measure_blocks_height(ui, &slide.blocks, &theme, geo.col_width, 1.0);
            assert!(stacked > measured);
        });
    }
}
