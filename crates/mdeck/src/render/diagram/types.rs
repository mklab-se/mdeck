use std::collections::HashSet;

use eframe::egui::Pos2;

// ─── Diagram data structures ─────────────────────────────────────────────────

/// How the diagram should handle overflow / sizing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum DiagramScale {
    /// Auto-scale to fit available area (default).
    Fit,
    /// Explicit scale factor relative to normal size (e.g. 0.7).
    Factor(f32),
    /// Allow scrolling instead of scaling.
    Scroll,
}

/// Reveal marker for diagram elements (mirrors ListMarker semantics).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum DiagramReveal {
    /// Always visible (prefix `-` or no prefix).
    Static,
    /// Appears on the next reveal step (prefix `+`).
    NextStep,
    /// Appears together with the previous `+` element (prefix `*`).
    WithPrev,
}

pub(super) struct DiagramNode {
    pub(super) name: String,
    pub(super) label: String,
    pub(super) icon: String,
    pub(super) grid_pos: Option<(u32, u32)>,
    pub(super) prompt: Option<String>,
    pub(super) reveal: DiagramReveal,
    pub(super) parse_order: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum ArrowKind {
    Forward,       // ->
    Reverse,       // <-
    Bidirectional, // <->
    DashedLine,    // --
    DashedArrow,   // -->
}

impl ArrowKind {
    /// The arrow as written in the diagram source.
    pub(super) fn symbol(self) -> &'static str {
        match self {
            ArrowKind::Forward => "->",
            ArrowKind::Reverse => "<-",
            ArrowKind::Bidirectional => "<->",
            ArrowKind::DashedLine => "--",
            ArrowKind::DashedArrow => "-->",
        }
    }

    pub(super) fn is_dashed(self) -> bool {
        matches!(self, ArrowKind::DashedLine | ArrowKind::DashedArrow)
    }

    /// Whether an arrowhead sits at the target end.
    pub(super) fn has_end_arrow(self) -> bool {
        matches!(
            self,
            ArrowKind::Forward | ArrowKind::DashedArrow | ArrowKind::Bidirectional
        )
    }

    /// Whether an arrowhead sits at the source end.
    pub(super) fn has_start_arrow(self) -> bool {
        matches!(self, ArrowKind::Reverse | ArrowKind::Bidirectional)
    }
}

pub(super) struct DiagramEdge {
    pub(super) from: String,
    pub(super) to: String,
    pub(super) label: String,
    pub(super) arrow: ArrowKind,
    pub(super) reveal: DiagramReveal,
    pub(super) parse_order: usize,
}

// ─── Orthogonal routing ─────────────────────────────────────────────────────

/// Information about the grid layout for routing.
///
/// Corridors live in the gaps between grid cells:
///   - Horizontal corridor `i` runs at y = origin_y + i * cell_h (between row i-1 and row i)
///   - Vertical corridor `j` runs at x = origin_x + j * cell_w (between col j-1 and col j)
///
/// Corridor index 0 is the edge before the first row/col; index N is after the last.
pub(super) struct GridInfo {
    pub(super) cols: usize,
    pub(super) rows: usize,
    pub(super) cell_w: f32,
    pub(super) cell_h: f32,
    pub(super) origin_x: f32,
    pub(super) origin_y: f32,
    /// Grid cells that contain a node (0-indexed: col 0..cols-1, row 0..rows-1).
    pub(super) occupied: HashSet<(usize, usize)>,
}

impl GridInfo {
    /// Y position of horizontal corridor at given index (raw cell boundary).
    #[cfg(test)]
    pub(super) fn h_corridor_y(&self, index: usize) -> f32 {
        self.origin_y + index as f32 * self.cell_h
    }

    /// X position of vertical corridor at given index (raw cell boundary).
    #[cfg(test)]
    pub(super) fn v_corridor_x(&self, index: usize) -> f32 {
        self.origin_x + index as f32 * self.cell_w
    }

    /// Return the grid cell (col, row) containing a point, if within bounds.
    pub(super) fn cell_at(&self, pos: Pos2) -> Option<(usize, usize)> {
        let col = ((pos.x - self.origin_x) / self.cell_w).floor() as isize;
        let row = ((pos.y - self.origin_y) / self.cell_h).floor() as isize;
        if col >= 0 && (col as usize) < self.cols && row >= 0 && (row as usize) < self.rows {
            Some((col as usize, row as usize))
        } else {
            None
        }
    }

    /// Check if a grid cell has no node in it.
    #[cfg(test)]
    pub(super) fn is_cell_empty(&self, col: usize, row: usize) -> bool {
        !self.occupied.contains(&(col, row))
    }
}

/// Which face of a node to exit/enter from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Face {
    Right,
    Left,
    Bottom,
    Top,
}

pub(super) struct NodeLayout {
    pub(super) center_x: f32,
    pub(super) center_y: f32,
    pub(super) width: f32,
    pub(super) height: f32,
}

/// Edge sizes in design pixels, scaled for the slide.
#[derive(Debug, Clone, Copy)]
pub(super) struct EdgeMetrics {
    pub(super) line_width: f32,
    pub(super) arrow_size: f32,
    /// Length of the straight ramp between a node face and the first bend.
    pub(super) node_margin: f32,
    pub(super) corner_radius: f32,
    /// Distance between neighbouring lanes in a corridor.
    pub(super) lane_spacing: f32,
    /// Distance between fallback connections sharing a node face.
    pub(super) port_spacing: f32,
    pub(super) dash_len: f32,
    pub(super) gap_len: f32,
    pub(super) label_pad_h: f32,
    pub(super) label_pad_v: f32,
}

impl EdgeMetrics {
    pub(super) fn new(scale: f32) -> Self {
        Self {
            line_width: 4.0 * scale,
            arrow_size: 20.0 * scale,
            node_margin: 10.0 * scale,
            corner_radius: 10.0 * scale,
            lane_spacing: 20.0 * scale,
            port_spacing: 22.0 * scale,
            dash_len: 8.0 * scale,
            gap_len: 5.0 * scale,
            label_pad_h: 10.0 * scale,
            label_pad_v: 5.0 * scale,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(origin: (f32, f32), occupied: HashSet<(usize, usize)>) -> GridInfo {
        GridInfo {
            cols: 3,
            rows: 2,
            cell_w: 100.0,
            cell_h: 80.0,
            origin_x: origin.0,
            origin_y: origin.1,
            occupied,
        }
    }

    #[test]
    fn test_cell_at_origin() {
        let grid = grid((0.0, 0.0), HashSet::new());
        assert_eq!(grid.cell_at(Pos2::new(10.0, 10.0)), Some((0, 0)));
    }

    #[test]
    fn test_cell_at_center_cell() {
        let grid = grid((0.0, 0.0), HashSet::new());
        assert_eq!(grid.cell_at(Pos2::new(150.0, 40.0)), Some((1, 0)));
    }

    #[test]
    fn test_cell_at_out_of_bounds() {
        let grid = grid((0.0, 0.0), HashSet::new());
        assert_eq!(grid.cell_at(Pos2::new(-10.0, 10.0)), None);
        assert_eq!(grid.cell_at(Pos2::new(10.0, -10.0)), None);
        assert_eq!(grid.cell_at(Pos2::new(310.0, 10.0)), None);
        assert_eq!(grid.cell_at(Pos2::new(10.0, 170.0)), None);
    }

    #[test]
    fn test_is_cell_empty() {
        let grid = grid((0.0, 0.0), HashSet::from([(1, 0)]));
        assert!(grid.is_cell_empty(0, 0));
        assert!(!grid.is_cell_empty(1, 0));
        assert!(grid.is_cell_empty(2, 0));
    }

    #[test]
    fn test_corridor_positions() {
        let grid = grid((10.0, 20.0), HashSet::new());
        assert_eq!(grid.h_corridor_y(0), 20.0);
        assert_eq!(grid.h_corridor_y(1), 100.0);
        assert_eq!(grid.h_corridor_y(2), 180.0);
        assert_eq!(grid.v_corridor_x(0), 10.0);
        assert_eq!(grid.v_corridor_x(1), 110.0);
        assert_eq!(grid.v_corridor_x(3), 310.0);
    }

    #[test]
    fn test_arrow_kind_symbols_and_heads() {
        let all = [
            ArrowKind::Forward,
            ArrowKind::Reverse,
            ArrowKind::Bidirectional,
            ArrowKind::DashedLine,
            ArrowKind::DashedArrow,
        ];
        let symbols: Vec<&str> = all.iter().map(|a| a.symbol()).collect();
        assert_eq!(symbols, ["->", "<-", "<->", "--", "-->"]);
        let dashed: Vec<bool> = all.iter().map(|a| a.is_dashed()).collect();
        assert_eq!(dashed, [false, false, false, true, true]);
        let end: Vec<bool> = all.iter().map(|a| a.has_end_arrow()).collect();
        assert_eq!(end, [true, false, true, false, true]);
        let start: Vec<bool> = all.iter().map(|a| a.has_start_arrow()).collect();
        assert_eq!(start, [false, true, true, false, false]);
    }
}
