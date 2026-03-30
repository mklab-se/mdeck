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

fn layout_grid(
    nodes: &[DiagramNode],
    area_width: f32,
    area_height: f32,
    origin_x: f32,
    origin_y: f32,
    scale: f32,
) -> (Vec<NodeLayout>, GridInfo) {
    // Find grid dimensions from pos values
    let mut max_col: u32 = 1;
    let mut max_row: u32 = 1;
    for node in nodes {
        if let Some((col, row)) = node.grid_pos {
            max_col = max_col.max(col);
            max_row = max_row.max(row);
        }
    }

    let cell_w = area_width / max_col as f32;
    let cell_h = area_height / max_row as f32;

    // Responsive node sizes: fill a fraction of each cell, with min/max bounds
    let node_w = (cell_w * 0.65).clamp(100.0 * scale, 220.0 * scale);
    let node_h = (cell_h * 0.6).clamp(80.0 * scale, 160.0 * scale);

    // Assign unpositioned nodes to first available cells
    let mut occupied: Vec<(u32, u32)> = nodes.iter().filter_map(|n| n.grid_pos).collect();
    let mut next_unplaced = 0u32;

    let layouts = nodes
        .iter()
        .map(|node| {
            let (col, row) = node.grid_pos.unwrap_or_else(|| {
                // Find next unoccupied cell
                loop {
                    let c = next_unplaced % max_col + 1;
                    let r = next_unplaced / max_col + 1;
                    next_unplaced += 1;
                    if !occupied.contains(&(c, r)) {
                        occupied.push((c, r));
                        return (c, r);
                    }
                }
            });

            let cx = (col as f32 - 0.5) * cell_w;
            let cy = (row as f32 - 0.5) * cell_h;

            NodeLayout {
                center_x: cx,
                center_y: cy,
                width: node_w,
                height: node_h,
            }
        })
        .collect();

    // Build occupied set (convert 1-based grid_pos to 0-based)
    let occupied_set: HashSet<(usize, usize)> = occupied
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

    // For small node counts, use a single row
    if n <= 5 {
        // Responsive: size nodes to fill available space
        let max_node_w = (area_width / n as f32 * 0.6).clamp(100.0 * scale, 240.0 * scale);
        let node_h = (area_height * 0.4).clamp(80.0 * scale, 220.0 * scale);
        let node_w = max_node_w.min(node_h * 1.4); // keep reasonable aspect ratio

        let gap = if n > 1 {
            ((area_width - n as f32 * node_w) / (n - 1) as f32).max(20.0 * scale)
        } else {
            0.0
        };
        let total_w = n as f32 * node_w + (n - 1).max(0) as f32 * gap;
        let start_x = (area_width - total_w) / 2.0 + node_w / 2.0;

        let cell_w = if n > 1 {
            area_width / n as f32
        } else {
            area_width
        };

        let layouts = nodes
            .iter()
            .enumerate()
            .map(|(i, _)| NodeLayout {
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

        return (layouts, grid_info);
    }

    // For larger counts, arrange in a grid pattern
    let cols = ((n as f32).sqrt().ceil() as usize).max(2);
    let rows = n.div_ceil(cols);

    let cell_w = area_width / cols as f32;
    let cell_h = area_height / rows as f32;

    // Responsive node sizes
    let node_w = (cell_w * 0.65).clamp(100.0 * scale, 220.0 * scale);
    let node_h = (cell_h * 0.6).clamp(80.0 * scale, 160.0 * scale);

    let layouts = nodes
        .iter()
        .enumerate()
        .map(|(i, _)| {
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
