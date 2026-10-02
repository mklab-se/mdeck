use std::collections::HashMap;

use eframe::egui::{self, Pos2};

use super::layout::{apply_fit, fit_scale, layout_nodes};
use super::types::*;

// ─── Diagram geometry ────────────────────────────────────────────────────────

/// A node placed on the slide.
pub(super) struct NodeBox {
    pub(super) center: Pos2,
    pub(super) width: f32,
    pub(super) height: f32,
    pub(super) rect: egui::Rect,
}

/// Where everything in a diagram sits on the slide: one box per node (in node
/// order), the same rects by name for the edges, and the grid the router uses.
pub(super) struct Scene {
    pub(super) boxes: Vec<NodeBox>,
    pub(super) rects: HashMap<String, egui::Rect>,
    pub(super) grid: GridInfo,
}

impl Scene {
    /// Lay the nodes out in the area of `size` at `origin` (the diagram rect
    /// less its padding), then shrink or scale them as the diagram's `scale`
    /// directive says.
    pub(super) fn layout(
        nodes: &[DiagramNode],
        directive: DiagramScale,
        origin: Pos2,
        size: egui::Vec2,
        scale: f32,
    ) -> Self {
        let (area_width, area_height) = (size.x, size.y);
        let (mut layouts, mut grid) =
            layout_nodes(nodes, area_width, area_height, origin.x, origin.y, scale);
        let fit = fit_scale(directive, &layouts, area_height);
        apply_fit(&mut layouts, &mut grid, fit, area_width, area_height);

        let boxes: Vec<NodeBox> = layouts
            .iter()
            .map(|layout| node_box(layout, origin))
            .collect();
        let rects = nodes
            .iter()
            .zip(&boxes)
            .map(|(node, b)| (node.name.clone(), b.rect))
            .collect();
        Self { boxes, rects, grid }
    }
}

fn node_box(layout: &NodeLayout, origin: Pos2) -> NodeBox {
    let center = egui::pos2(origin.x + layout.center_x, origin.y + layout.center_y);
    NodeBox {
        center,
        width: layout.width,
        height: layout.height,
        rect: egui::Rect::from_center_size(center, egui::vec2(layout.width, layout.height)),
    }
}

/// Node rects by name, with layout centres taken relative to `origin`. A
/// repeated name keeps the last node's rect.
pub(super) fn node_rects(
    nodes: &[DiagramNode],
    layouts: &[NodeLayout],
    origin: Pos2,
) -> HashMap<String, egui::Rect> {
    nodes
        .iter()
        .zip(layouts)
        .map(|(node, layout)| (node.name.clone(), node_box(layout, origin).rect))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::parsing::parse_diagram;
    use super::*;

    #[test]
    fn scene_offsets_nodes_into_the_area() {
        let (nodes, _, directive) = parse_diagram("- A\n- B");
        let origin = Pos2::new(100.0, 50.0);
        let scene = Scene::layout(&nodes, directive, origin, egui::vec2(800.0, 400.0), 1.0);
        let (layouts, _) = layout_nodes(&nodes, 800.0, 400.0, 100.0, 50.0, 1.0);

        assert_eq!(scene.boxes.len(), 2);
        assert_eq!(scene.boxes[0].center.x, 100.0 + layouts[0].center_x);
        assert_eq!(scene.boxes[0].center.y, 50.0 + layouts[0].center_y);
        assert_eq!(scene.rects["B"], scene.boxes[1].rect);
        assert_eq!(scene.grid.origin_x, 100.0);
        assert_eq!(
            node_rects(&nodes, &layouts, Pos2::new(100.0, 50.0))["A"],
            scene.boxes[0].rect
        );
    }

    #[test]
    fn scene_applies_scale_factor() {
        let (nodes, _, directive) = parse_diagram("scale: 0.5\n- A\n- B");
        let scene = Scene::layout(&nodes, directive, Pos2::ZERO, egui::vec2(800.0, 400.0), 1.0);
        let (layouts, grid) = layout_nodes(&nodes, 800.0, 400.0, 0.0, 0.0, 1.0);
        assert_eq!(scene.boxes[0].width, layouts[0].width * 0.5);
        assert_eq!(scene.grid.cell_w, grid.cell_w * 0.5);
    }
}
