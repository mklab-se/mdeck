use eframe::egui::{self, Pos2};

use crate::parser::{Block, Slide};
use crate::render::BlockCx;
use crate::render::text;

/// Gallery slide layout: multiple images arranged in a grid.
/// - 2 images: side by side
/// - 3 images: top row of 2, bottom row of 1 centered
/// - 4 images: 2x2 grid
/// - 5+ images: rows of 3 (or 2 for remainder)
pub fn render(cx: &BlockCx, slide: &Slide, rect: egui::Rect) {
    let scale = cx.scale;
    let padding = 50.0 * scale;
    let gap = 16.0 * scale;

    // Collect heading and image blocks
    let mut heading: Option<&Block> = None;
    let mut images: Vec<&Block> = Vec::new();

    for block in &slide.blocks {
        match block {
            Block::Heading { .. } if heading.is_none() && images.is_empty() => {
                heading = Some(block);
            }
            Block::Image { .. } => {
                images.push(block);
            }
            _ => {}
        }
    }

    if images.is_empty() {
        return;
    }

    let content_width = rect.width() - padding * 2.0;
    let mut y = rect.top() + padding;

    // Draw heading if present
    if let Some(Block::Heading { level, inlines }) = heading {
        let pos = Pos2::new(rect.left() + padding, y);
        let h = text::draw_heading(&cx.text(), inlines, *level, pos, content_width);
        y += h + 20.0 * scale;
    }

    let gallery_height = rect.bottom() - y - padding;
    let gallery_left = rect.left() + padding;

    // Compute grid layout based on image count
    let cells = compute_grid(images.len(), content_width, gallery_height, gap);

    for (i, block) in images.iter().enumerate() {
        if i >= cells.len() {
            break;
        }

        let cell = &cells[i];
        let cell_rect = egui::Rect::from_min_size(
            Pos2::new(gallery_left + cell.x, y + cell.y),
            egui::vec2(cell.w, cell.h),
        );

        if let Block::Image {
            alt,
            path,
            directives,
        } = block
        {
            text::draw_image_in_area(cx, path, alt, directives, cell_rect);
        }
    }
}

struct Cell {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

/// A `cols` by `rows` grid of equal cells filling `width` by `height`.
fn uniform_grid(cols: usize, rows: usize, width: f32, height: f32, gap: f32) -> Vec<Cell> {
    let cell_w = (width - (cols - 1) as f32 * gap) / cols as f32;
    let cell_h = (height - (rows - 1) as f32 * gap) / rows as f32;
    (0..rows)
        .flat_map(|row| (0..cols).map(move |col| (row, col)))
        .map(|(row, col)| Cell {
            x: col as f32 * (cell_w + gap),
            y: row as f32 * (cell_h + gap),
            w: cell_w,
            h: cell_h,
        })
        .collect()
}

fn compute_grid(count: usize, width: f32, height: f32, gap: f32) -> Vec<Cell> {
    match count {
        0 => Vec::new(),
        // Single image centered
        1 => uniform_grid(1, 1, width, height, gap),
        // Side by side
        2 => uniform_grid(2, 1, width, height, gap),
        3 => {
            // Top row: 2 images, bottom row: 1 centered
            let row_h = (height - gap) / 2.0;
            let top_w = (width - gap) / 2.0;
            let bot_w = top_w; // centered, same width as the top cells
            let bot_x = (width - bot_w) / 2.0;
            vec![
                Cell {
                    x: 0.0,
                    y: 0.0,
                    w: top_w,
                    h: row_h,
                },
                Cell {
                    x: top_w + gap,
                    y: 0.0,
                    w: top_w,
                    h: row_h,
                },
                Cell {
                    x: bot_x,
                    y: row_h + gap,
                    w: bot_w,
                    h: row_h,
                },
            ]
        }
        // 2x2 grid
        4 => uniform_grid(2, 2, width, height, gap),
        _ => {
            // Generic grid: rows of 3, last row may have fewer
            let cols = 3;
            let rows = count.div_ceil(cols);
            let cell_w = (width - (cols - 1) as f32 * gap) / cols as f32;
            let cell_h = (height - (rows - 1) as f32 * gap) / rows as f32;

            (0..count)
                .map(|i| {
                    let col = i % cols;
                    let row = i / cols;
                    // Center the last row if it has fewer items
                    let items_in_row = if row == rows - 1 {
                        count - row * cols
                    } else {
                        cols
                    };
                    let row_width = items_in_row as f32 * cell_w + (items_in_row - 1) as f32 * gap;
                    let row_offset = (width - row_width) / 2.0;
                    let col_in_row = col; // col index within this row

                    Cell {
                        x: row_offset + col_in_row as f32 * (cell_w + gap),
                        y: row as f32 * (cell_h + gap),
                        w: cell_w,
                        h: cell_h,
                    }
                })
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_image_grid_uses_equal_cell_widths() {
        let cells = compute_grid(3, 1000.0, 600.0, 20.0);
        assert_eq!(cells.len(), 3);
        assert_eq!(cells[0].w, cells[1].w);
        assert_eq!(cells[2].w, cells[0].w);
        // Bottom cell is centred
        assert!((cells[2].x + cells[2].w / 2.0 - 500.0).abs() < 0.01);
        assert_eq!(cells[2].y, cells[0].h + 20.0);
    }

    #[test]
    fn uniform_grids_tile_the_area_row_by_row() {
        let one = compute_grid(1, 1000.0, 600.0, 20.0);
        assert_eq!(
            (one[0].x, one[0].y, one[0].w, one[0].h),
            (0.0, 0.0, 1000.0, 600.0)
        );
        let two = compute_grid(2, 1000.0, 600.0, 20.0);
        assert_eq!(
            (two[1].x, two[1].y, two[1].w, two[1].h),
            (510.0, 0.0, 490.0, 600.0)
        );
        let four = compute_grid(4, 1000.0, 600.0, 20.0);
        let at: Vec<(f32, f32)> = four.iter().map(|c| (c.x, c.y)).collect();
        assert_eq!(at, [(0.0, 0.0), (510.0, 0.0), (0.0, 310.0), (510.0, 310.0)]);
        assert!(four.iter().all(|c| c.w == 490.0 && c.h == 290.0));
    }

    #[test]
    fn generic_grid_centres_short_last_row() {
        let cells = compute_grid(5, 1000.0, 600.0, 10.0);
        assert_eq!(cells.len(), 5);
        let cell_w = (1000.0 - 20.0) / 3.0;
        // Last row has 2 cells, centred as a group
        let row_w = 2.0 * cell_w + 10.0;
        assert!((cells[3].x - (1000.0 - row_w) / 2.0).abs() < 0.01);
    }
}
