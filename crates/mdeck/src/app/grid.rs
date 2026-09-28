//! Where the overview grid (`G`) puts each slide: pure geometry shared by
//! drawing, mouse hit-testing and scrolling the selection into view.

use eframe::egui::{Rect, pos2, vec2};

/// The grid of `count` slides laid out in `rect` at `scale`.
#[derive(Debug, Clone, Copy)]
pub(super) struct GridLayout {
    count: usize,
    cols: usize,
    rect: Rect,
    scale: f32,
    padding: f32,
    gap: f32,
    cell_width: f32,
    /// Cells keep the 16:9 slide shape unless the whole grid fits the view,
    /// when they may be a little shorter.
    cell_height: f32,
}

impl GridLayout {
    pub(super) fn new(count: usize, rect: Rect, scale: f32) -> Self {
        let cols = Self::columns(count);
        let rows = count.div_ceil(cols) as f32;
        let padding = 24.0 * scale;
        let gap = 12.0 * scale;
        let grid_width = rect.width() - padding * 2.0;
        let cell_width = (grid_width - gap * (cols as f32 - 1.0)) / cols as f32;
        let natural_height = cell_width * 9.0 / 16.0;
        let mut grid = Self {
            count,
            cols,
            rect,
            scale,
            padding,
            gap,
            cell_width,
            cell_height: natural_height,
        };
        // If the natural layout fits in the viewport, clamp to the viewport;
        // otherwise use the natural size and scroll.
        let available = grid.available_height();
        if grid.content_height() <= available {
            let max = (available - gap * (rows - 1.0)) / rows;
            grid.cell_height = max.min(natural_height);
        }
        grid
    }

    /// Columns for a deck of `count` slides.
    pub(super) fn columns(count: usize) -> usize {
        match count {
            0..=4 => 2,
            5..=9 => 3,
            _ => 4,
        }
    }

    /// Top edge of the first row (below the grid's title line).
    fn top(&self) -> f32 {
        self.rect.top() + self.padding + 40.0 * self.scale
    }

    /// Height the grid can show without scrolling.
    pub(super) fn available_height(&self) -> f32 {
        self.rect.bottom() - self.top() - self.padding
    }

    /// Height of every row at the natural 16:9 cell size (for scrolling,
    /// which only happens when that does not fit).
    pub(super) fn content_height(&self) -> f32 {
        let rows = self.count.div_ceil(self.cols) as f32;
        let natural = self.cell_width * 9.0 / 16.0;
        rows * natural + (rows - 1.0) * self.gap
    }

    /// Slide `index`'s cell with the grid scrolled by `scroll`.
    pub(super) fn cell(&self, index: usize, scroll: f32) -> Rect {
        let col = (index % self.cols) as f32;
        let row = (index / self.cols) as f32;
        let x = self.rect.left() + self.padding + col * (self.cell_width + self.gap);
        let y = self.top() + row * (self.cell_height + self.gap) - scroll;
        Rect::from_min_size(pos2(x, y), vec2(self.cell_width, self.cell_height))
    }

    /// The scroll offset that brings `index` into view, starting from `current`.
    pub(super) fn scroll_to_show(&self, index: usize, current: f32) -> f32 {
        let overflow = (self.content_height() - self.available_height()).max(0.0);
        if overflow <= 0.0 {
            return 0.0;
        }
        let top = self.top();
        let bottom = self.rect.bottom() - self.padding;
        let cell = self.cell(index, current);
        let target = if cell.top() < top {
            current - (top - cell.top() + self.padding)
        } else if cell.bottom() > bottom {
            current + (cell.bottom() - bottom + self.padding)
        } else {
            current
        };
        target.clamp(0.0, overflow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen() -> Rect {
        Rect::from_min_size(pos2(0.0, 0.0), vec2(1920.0, 1080.0))
    }

    #[test]
    fn columns_grow_with_the_deck() {
        assert_eq!(GridLayout::columns(3), 2);
        assert_eq!(GridLayout::columns(9), 3);
        assert_eq!(GridLayout::columns(40), 4);
    }

    #[test]
    fn cells_run_left_to_right_then_down_without_overlap() {
        let g = GridLayout::new(8, screen(), 1.0);
        let (a, b, d) = (g.cell(0, 0.0), g.cell(1, 0.0), g.cell(3, 0.0));
        assert_eq!(a.top(), b.top());
        assert!(b.left() > a.right());
        assert!(d.top() > a.bottom());
        assert_eq!(d.left(), a.left());
    }

    #[test]
    fn a_small_deck_fits_and_never_scrolls() {
        let g = GridLayout::new(6, screen(), 1.0);
        assert!(g.content_height() <= g.available_height());
        assert_eq!(g.scroll_to_show(5, 0.0), 0.0);
        assert!(g.cell(5, 0.0).bottom() <= screen().bottom());
    }

    #[test]
    fn a_large_deck_scrolls_the_selection_into_view() {
        let g = GridLayout::new(40, screen(), 1.0);
        let overflow = g.content_height() - g.available_height();
        assert!(overflow > 0.0);
        let last = g.scroll_to_show(39, 0.0);
        assert!(last > 0.0 && last <= overflow);
        assert!(g.cell(39, last).bottom() <= screen().bottom());
        // back to the top for the first slide
        assert_eq!(g.scroll_to_show(0, last), 0.0);
    }
}
