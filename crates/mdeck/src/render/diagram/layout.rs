use std::collections::HashSet;

use super::types::*;

pub(super) fn layout_nodes(
    nodes: &[DiagramNode],
    area_width: f32,
    area_height: f32,
    origin_x: f32,
    origin_y: f32,
    scale: f32,
) -> (Vec<NodeLayout>, GridInfo) {
    let has_grid = nodes.iter().any(|n| n.grid_pos.is_some());

    if has_grid {
        layout_grid(nodes, area_width, area_height, origin_x, origin_y, scale)
    } else {
        layout_auto(nodes, area_width, area_height, origin_x, origin_y, scale)
    }
}

/// The grid size (columns, rows) spanned by explicit `pos:` values, at least 1x1.
pub(super) fn grid_extent(nodes: &[DiagramNode]) -> (u32, u32) {
    let mut max_col: u32 = 1;
    let mut max_row: u32 = 1;
    for node in nodes {
        if let Some((col, row)) = node.grid_pos {
            max_col = max_col.max(col);
            max_row = max_row.max(row);
        }
    }
    (max_col, max_row)
}

/// The 1-based (col, row) cell of every node on an explicit grid: a node's own
/// `pos:`, or else the first cell (row by row) no other node claims.
pub(super) fn grid_cells(nodes: &[DiagramNode], max_col: u32) -> Vec<(u32, u32)> {
    let mut occupied: Vec<(u32, u32)> = nodes.iter().filter_map(|n| n.grid_pos).collect();
    let mut next_unplaced = 0u32;
    nodes
        .iter()
        .map(|node| {
            node.grid_pos.unwrap_or_else(|| {
                loop {
                    let c = next_unplaced % max_col + 1;
                    let r = next_unplaced / max_col + 1;
                    next_unplaced += 1;
                    if !occupied.contains(&(c, r)) {
                        occupied.push((c, r));
                        return (c, r);
                    }
                }
            })
        })
        .collect()
}

/// Columns of the automatic layout: one row up to five nodes, then a
/// near-square grid of at least two columns.
pub(super) fn auto_columns(n: usize) -> usize {
    if n <= 5 {
        n
    } else {
        ((n as f32).sqrt().ceil() as usize).max(2)
    }
}

fn layout_grid(
    nodes: &[DiagramNode],
    area_width: f32,
    area_height: f32,
    origin_x: f32,
    origin_y: f32,
    scale: f32,
) -> (Vec<NodeLayout>, GridInfo) {
    let (max_col, max_row) = grid_extent(nodes);

    let cell_w = area_width / max_col as f32;
    let cell_h = area_height / max_row as f32;

    // Responsive node sizes: fill a fraction of each cell, with min/max bounds
    let node_w = (cell_w * 0.65).clamp(100.0 * scale, 220.0 * scale);
    let node_h = (cell_h * 0.6).clamp(80.0 * scale, 160.0 * scale);

    let cells = grid_cells(nodes, max_col);
    let layouts = cells
        .iter()
        .map(|&(col, row)| NodeLayout {
            center_x: (col as f32 - 0.5) * cell_w,
            center_y: (row as f32 - 0.5) * cell_h,
            width: node_w,
            height: node_h,
        })
        .collect();

    // Build occupied set (convert 1-based grid_pos to 0-based)
    let occupied_set: HashSet<(usize, usize)> = cells
        .iter()
        .map(|&(c, r)| ((c - 1) as usize, (r - 1) as usize))
        .collect();

    let grid_info = GridInfo {
        cols: max_col as usize,
        rows: max_row as usize,
        cell_w,
        cell_h,
        origin_x,
        origin_y,
        occupied: occupied_set,
    };

    (layouts, grid_info)
}

fn layout_auto(
    nodes: &[DiagramNode],
    area_width: f32,
    area_height: f32,
    origin_x: f32,
    origin_y: f32,
    scale: f32,
) -> (Vec<NodeLayout>, GridInfo) {
    let n = nodes.len();
    if n == 0 {
        let grid_info = GridInfo {
            cols: 1,
            rows: 1,
            cell_w: area_width,
            cell_h: area_height,
            origin_x,
            origin_y,
            occupied: HashSet::new(),
        };
        return (Vec::new(), grid_info);
    }

    // For small node counts, use a single row; for larger ones, a grid.
    if n <= 5 {
        layout_row(n, area_width, area_height, origin_x, origin_y, scale)
    } else {
        layout_auto_grid(n, area_width, area_height, origin_x, origin_y, scale)
    }
}

/// `n` (1 to 5) nodes in one row, spread across the area and sized to fill it.
fn layout_row(
    n: usize,
    area_width: f32,
    area_height: f32,
    origin_x: f32,
    origin_y: f32,
    scale: f32,
) -> (Vec<NodeLayout>, GridInfo) {
    // Responsive: size nodes to fill available space
    let max_node_w = (area_width / n as f32 * 0.6).clamp(100.0 * scale, 240.0 * scale);
    let node_h = (area_height * 0.4).clamp(80.0 * scale, 220.0 * scale);
    let node_w = max_node_w.min(node_h * 1.4); // keep reasonable aspect ratio

    let gap = if n > 1 {
        ((area_width - n as f32 * node_w) / (n - 1) as f32).max(20.0 * scale)
    } else {
        0.0
    };
    let total_w = n as f32 * node_w + n.saturating_sub(1) as f32 * gap;
    let start_x = (area_width - total_w) / 2.0 + node_w / 2.0;

    let cell_w = if n > 1 {
        area_width / n as f32
    } else {
        area_width
    };

    let layouts = (0..n)
        .map(|i| NodeLayout {
            center_x: start_x + i as f32 * (node_w + gap),
            center_y: area_height / 2.0,
            width: node_w,
            height: node_h,
        })
        .collect();

    let grid_info = GridInfo {
        cols: n,
        rows: 1,
        cell_w,
        cell_h: area_height,
        origin_x,
        origin_y,
        occupied: (0..n).map(|i| (i, 0)).collect(),
    };

    (layouts, grid_info)
}

/// `n` nodes (more than five) in a near-square grid, row by row.
fn layout_auto_grid(
    n: usize,
    area_width: f32,
    area_height: f32,
    origin_x: f32,
    origin_y: f32,
    scale: f32,
) -> (Vec<NodeLayout>, GridInfo) {
    let cols = auto_columns(n);
    let rows = n.div_ceil(cols);

    let cell_w = area_width / cols as f32;
    let cell_h = area_height / rows as f32;

    // Responsive node sizes
    let node_w = (cell_w * 0.65).clamp(100.0 * scale, 220.0 * scale);
    let node_h = (cell_h * 0.6).clamp(80.0 * scale, 160.0 * scale);

    let layouts = (0..n)
        .map(|i| {
            let col = i % cols;
            let row = i / cols;

            NodeLayout {
                center_x: (col as f32 + 0.5) * cell_w,
                center_y: (row as f32 + 0.5) * cell_h,
                width: node_w,
                height: node_h,
            }
        })
        .collect();

    // In auto-layout, all cells up to n are occupied; remaining may be empty
    let occupied: HashSet<(usize, usize)> = (0..n).map(|i| (i % cols, i / cols)).collect();

    let grid_info = GridInfo {
        cols,
        rows,
        cell_w,
        cell_h,
        origin_x,
        origin_y,
        occupied,
    };

    (layouts, grid_info)
}

// ─── Scale to fit ────────────────────────────────────────────────────────────

/// The uniform scale the `scale` directive asks for. `Fit` shrinks a layout
/// whose nodes overflow the area (large diagrams of 3+ rows, where minimum
/// node sizes add up), never below 0.3.
pub(super) fn fit_scale(directive: DiagramScale, layouts: &[NodeLayout], area_height: f32) -> f32 {
    match directive {
        DiagramScale::Fit => {
            if area_height > 0.0 {
                let mut bbox_bottom = 0.0f32;
                for layout in layouts {
                    let bottom = layout.center_y + layout.height / 2.0;
                    bbox_bottom = bbox_bottom.max(bottom);
                }
                if bbox_bottom > area_height {
                    (area_height / bbox_bottom).clamp(0.3, 1.0)
                } else {
                    1.0
                }
            } else {
                1.0
            }
        }
        DiagramScale::Factor(f) => f,
        DiagramScale::Scroll => 1.0,
    }
}

/// Scale nodes and grid cells by `fit` about the centre of the area.
pub(super) fn apply_fit(
    layouts: &mut [NodeLayout],
    grid: &mut GridInfo,
    fit: f32,
    area_width: f32,
    area_height: f32,
) {
    if (fit - 1.0).abs() <= 0.001 {
        return;
    }
    let center_x = area_width / 2.0;
    let center_y = area_height / 2.0;
    for layout in layouts {
        layout.center_x = center_x + (layout.center_x - center_x) * fit;
        layout.center_y = center_y + (layout.center_y - center_y) * fit;
        layout.width *= fit;
        layout.height *= fit;
    }
    grid.cell_w *= fit;
    grid.cell_h *= fit;
}

#[cfg(test)]
mod tests {
    use super::super::parsing::parse_diagram;
    use super::*;

    fn layout(center_y: f32, height: f32) -> NodeLayout {
        NodeLayout {
            center_x: 50.0,
            center_y,
            width: 100.0,
            height,
        }
    }

    #[test]
    fn grid_cells_fill_free_cells_in_row_order() {
        let (nodes, _, _) = parse_diagram("- A (pos: 1,1)\n- B\n- C (pos: 2,1)\n- D");
        let (max_col, max_row) = grid_extent(&nodes);
        assert_eq!((max_col, max_row), (2, 1));
        // B and D skip the cells A and C claim
        assert_eq!(
            grid_cells(&nodes, max_col),
            [(1, 1), (1, 2), (2, 1), (2, 2)]
        );
    }

    #[test]
    fn auto_columns_single_row_then_square() {
        assert_eq!(auto_columns(1), 1);
        assert_eq!(auto_columns(5), 5);
        assert_eq!(auto_columns(6), 3);
        assert_eq!(auto_columns(10), 4);
    }

    #[test]
    fn a_row_is_centred_and_evenly_spaced() {
        let (layouts, info) = layout_row(3, 1200.0, 600.0, 10.0, 20.0, 1.0);
        assert_eq!(layouts.len(), 3);
        assert_eq!((info.cols, info.rows), (3, 1));
        assert_eq!(info.cell_w, 400.0);
        assert_eq!((info.origin_x, info.origin_y), (10.0, 20.0));
        assert_eq!(info.occupied.len(), 3);
        let step = layouts[1].center_x - layouts[0].center_x;
        assert!((layouts[2].center_x - layouts[1].center_x - step).abs() < 1e-3);
        assert!((layouts[0].center_x + layouts[2].center_x - 1200.0).abs() < 1e-3);
        assert!(layouts.iter().all(|l| l.center_y == 300.0));
    }

    #[test]
    fn a_single_node_row_sits_in_the_middle() {
        let (layouts, info) = layout_row(1, 800.0, 400.0, 0.0, 0.0, 1.0);
        assert_eq!(layouts[0].center_x, 400.0);
        assert_eq!(info.cell_w, 800.0);
    }

    #[test]
    fn a_larger_auto_layout_fills_a_grid_row_by_row() {
        let (layouts, info) = layout_auto_grid(7, 900.0, 600.0, 0.0, 0.0, 1.0);
        assert_eq!((info.cols, info.rows), (3, 3));
        assert_eq!((info.cell_w, info.cell_h), (300.0, 200.0));
        assert_eq!(layouts.len(), 7);
        assert_eq!((layouts[4].center_x, layouts[4].center_y), (450.0, 300.0));
        assert!(info.occupied.contains(&(0, 2)) && !info.occupied.contains(&(1, 2)));
    }

    #[test]
    fn fit_scale_shrinks_only_overflow() {
        let fits = [layout(50.0, 100.0)];
        assert_eq!(fit_scale(DiagramScale::Fit, &fits, 200.0), 1.0);
        let overflows = [layout(300.0, 200.0)]; // bottom at 400
        assert_eq!(fit_scale(DiagramScale::Fit, &overflows, 200.0), 0.5);
        let huge = [layout(3000.0, 200.0)];
        assert_eq!(fit_scale(DiagramScale::Fit, &huge, 200.0), 0.3);
        assert_eq!(fit_scale(DiagramScale::Fit, &overflows, 0.0), 1.0);
        assert_eq!(fit_scale(DiagramScale::Factor(0.7), &fits, 200.0), 0.7);
        assert_eq!(fit_scale(DiagramScale::Scroll, &overflows, 200.0), 1.0);
    }

    #[test]
    fn apply_fit_scales_about_the_area_centre() {
        let (nodes, _, _) = parse_diagram("- A\n- B");
        let (_, mut grid) = layout_nodes(&nodes, 400.0, 200.0, 0.0, 0.0, 1.0);
        let mut layouts = vec![layout(0.0, 100.0)];
        apply_fit(&mut layouts, &mut grid, 0.5, 400.0, 200.0);
        assert_eq!(layouts[0].center_x, 125.0); // 200 + (50 - 200) * 0.5
        assert_eq!(layouts[0].center_y, 50.0);
        assert_eq!(layouts[0].height, 50.0);
        assert_eq!(grid.cell_w, 100.0);

        let mut unchanged = vec![layout(0.0, 100.0)];
        apply_fit(&mut unchanged, &mut grid, 1.0005, 400.0, 200.0);
        assert_eq!(unchanged[0].center_x, 50.0);
    }
}
