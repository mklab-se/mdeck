//! The plate: the image, gallery, code, table or visuals a design gives its
//! own region. Natural heights (code, tables) are measured exactly as they
//! draw; images and visuals fill the space they are given.

use std::sync::Arc;

use eframe::egui::{self, Pos2, Rect};

use super::copy::Lay;
use super::parts::Role;
use super::style;
use crate::parser::Block;

/// One thing on the plate, placed.
#[derive(Clone)]
pub struct Placed<'s> {
    pub block: &'s Block,
    pub rect: Rect,
    /// A caption under a gallery image (from its alt text).
    pub caption: Option<(Arc<egui::Galley>, Pos2)>,
}

/// Whether a plate block fills whatever height it is given.
fn fills(block: &Block) -> bool {
    matches!(
        block,
        Block::Image { .. } | Block::Chart { .. } | Block::Diagram { .. }
    )
}

/// The plate's own height at `width`, or `None` when it fills its region.
pub fn natural_height(lay: &Lay, blocks: &[&Block], width: f32, gap: f32) -> Option<f32> {
    if blocks.iter().any(|b| fills(b)) {
        return None;
    }
    let s = lay.scale;
    let total: f32 = blocks
        .iter()
        .map(|b| crate::render::text::measure_single_block_height(lay.ui, b, lay.theme, width, s))
        .sum();
    Some(total + gap * blocks.len().saturating_sub(1) as f32)
}

/// Place `blocks` in `rect`.
pub fn place<'s>(lay: &Lay, blocks: &[&'s Block], rect: Rect, gap: f32) -> Vec<Placed<'s>> {
    let images: Vec<&Block> = blocks
        .iter()
        .copied()
        .filter(|b| matches!(b, Block::Image { .. }))
        .collect();
    if images.len() == blocks.len() && images.len() > 1 {
        return gallery(lay, blocks, rect, gap);
    }
    if blocks.iter().all(|b| fills(b)) && blocks.len() > 1 {
        // DES-08: several visuals share the plate, side by side when it is
        // wide, else stacked
        let n = blocks.len() as f32;
        let wide = rect.width() / rect.height() > 1.6;
        return blocks
            .iter()
            .enumerate()
            .map(|(i, b)| {
                let i = i as f32;
                let r = if wide {
                    let w = (rect.width() - gap * (n - 1.0)) / n;
                    Rect::from_min_size(
                        Pos2::new(rect.left() + i * (w + gap), rect.top()),
                        egui::vec2(w, rect.height()),
                    )
                } else {
                    let h = (rect.height() - gap * (n - 1.0)) / n;
                    Rect::from_min_size(
                        Pos2::new(rect.left(), rect.top() + i * (h + gap)),
                        egui::vec2(rect.width(), h),
                    )
                };
                Placed {
                    block: b,
                    rect: r,
                    caption: None,
                }
            })
            .collect();
    }
    // a stack: natural heights, a filling block takes what is left
    let s = lay.scale;
    let natural: Vec<Option<f32>> = blocks
        .iter()
        .map(|b| {
            (!fills(b)).then(|| {
                crate::render::text::measure_single_block_height(
                    lay.ui,
                    b,
                    lay.theme,
                    rect.width(),
                    s,
                )
            })
        })
        .collect();
    let fixed: f32 =
        natural.iter().flatten().sum::<f32>() + gap * blocks.len().saturating_sub(1) as f32;
    let fillers = natural.iter().filter(|n| n.is_none()).count().max(1) as f32;
    let fill_h = ((rect.height() - fixed) / fillers).max(160.0 * s);
    let mut y = rect.top();
    let mut out = Vec::new();
    for (b, n) in blocks.iter().zip(natural) {
        let h = n.unwrap_or(fill_h);
        out.push(Placed {
            block: b,
            rect: Rect::from_min_size(Pos2::new(rect.left(), y), egui::vec2(rect.width(), h)),
            caption: None,
        });
        y += h + gap;
    }
    out
}

/// Images in a grid, each with its alt text as a caption.
fn gallery<'s>(lay: &Lay, blocks: &[&'s Block], rect: Rect, gap: f32) -> Vec<Placed<'s>> {
    let style = lay.style(Role::Caption);
    let r = lay.resolve(style, 0);
    let cells = grid(blocks.len(), rect.width(), rect.height(), gap);
    blocks
        .iter()
        .zip(cells)
        .map(|(b, c)| {
            let cell = Rect::from_min_size(
                Pos2::new(rect.left() + c.x, rect.top() + c.y),
                egui::vec2(c.w, c.h),
            );
            let alt = match b {
                Block::Image { alt, .. } => alt.trim(),
                _ => "",
            };
            if alt.is_empty() {
                return Placed {
                    block: b,
                    rect: cell,
                    caption: None,
                };
            }
            let job = style::text_job(&r, alt, egui::Align::Center);
            let mut job = job;
            job.wrap.max_width = cell.width();
            let galley = lay.ui.painter().layout_job(job);
            let ch = galley.rect.height() + 10.0 * lay.scale;
            let image = Rect::from_min_max(cell.min, Pos2::new(cell.right(), cell.bottom() - ch));
            let pos = Pos2::new(cell.center().x, cell.bottom() - galley.rect.height());
            Placed {
                block: b,
                rect: image,
                caption: Some((galley, pos)),
            }
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// A `cols` by `rows` grid of equal cells filling `width` by `height`.
fn uniform(cols: usize, rows: usize, width: f32, height: f32, gap: f32) -> Vec<Cell> {
    let cw = (width - (cols - 1) as f32 * gap) / cols as f32;
    let chh = (height - (rows - 1) as f32 * gap) / rows as f32;
    (0..rows)
        .flat_map(|row| (0..cols).map(move |col| (row, col)))
        .map(|(row, col)| Cell {
            x: col as f32 * (cw + gap),
            y: row as f32 * (chh + gap),
            w: cw,
            h: chh,
        })
        .collect()
}

/// Gallery cells: two side by side, three as two over one (centred), four
/// in a 2x2 grid, more in rows of three with a short last row centred.
pub fn grid(count: usize, width: f32, height: f32, gap: f32) -> Vec<Cell> {
    match count {
        0 => Vec::new(),
        1 => uniform(1, 1, width, height, gap),
        2 => uniform(2, 1, width, height, gap),
        3 => {
            let row_h = (height - gap) / 2.0;
            let w = (width - gap) / 2.0;
            vec![
                Cell {
                    x: 0.0,
                    y: 0.0,
                    w,
                    h: row_h,
                },
                Cell {
                    x: w + gap,
                    y: 0.0,
                    w,
                    h: row_h,
                },
                Cell {
                    x: (width - w) / 2.0,
                    y: row_h + gap,
                    w,
                    h: row_h,
                },
            ]
        }
        4 => uniform(2, 2, width, height, gap),
        _ => {
            let cols = 3;
            let rows = count.div_ceil(cols);
            let cw = (width - (cols - 1) as f32 * gap) / cols as f32;
            let chh = (height - (rows - 1) as f32 * gap) / rows as f32;
            (0..count)
                .map(|i| {
                    let (row, col) = (i / cols, i % cols);
                    let in_row = if row == rows - 1 {
                        count - row * cols
                    } else {
                        cols
                    };
                    let row_w = in_row as f32 * cw + (in_row - 1) as f32 * gap;
                    Cell {
                        x: (width - row_w) / 2.0 + col as f32 * (cw + gap),
                        y: row as f32 * (chh + gap),
                        w: cw,
                        h: chh,
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
    fn grids_tile_the_area() {
        let three = grid(3, 1000.0, 600.0, 20.0);
        assert_eq!(three[0].w, three[2].w);
        assert!((three[2].x + three[2].w / 2.0 - 500.0).abs() < 0.01);
        let four = grid(4, 1000.0, 600.0, 20.0);
        assert!(four.iter().all(|c| c.w == 490.0 && c.h == 290.0));
        let five = grid(5, 1000.0, 600.0, 10.0);
        let cw = (1000.0 - 20.0) / 3.0;
        assert!((five[3].x - (1000.0 - (2.0 * cw + 10.0)) / 2.0).abs() < 0.01);
    }
}
