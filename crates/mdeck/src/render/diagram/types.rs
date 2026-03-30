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

/// Parsed metadata from parenthetical notation like `(icon: database, pos: 1,2, prompt: "...")`.
pub(super) struct NodeMetadata<'a> {
    pub(super) before: &'a str,
    pub(super) icon: String,
    pub(super) grid_pos: Option<(u32, u32)>,
    pub(super) prompt: Option<String>,
}

pub(super) struct NodeLayout {
    pub(super) center_x: f32,
    pub(super) center_y: f32,
    pub(super) width: f32,
    pub(super) height: f32,
}
