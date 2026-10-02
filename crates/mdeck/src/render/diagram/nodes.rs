use eframe::egui::{self, Color32, FontId, Pos2, Stroke};

use super::DiagramCx;
use super::icons::draw_icon_fallback;
use super::scene::{NodeBox, Scene};
use super::types::*;
use crate::render::hints;
use crate::theme::Theme;

// ─── Nodes ───────────────────────────────────────────────────────────────────

/// Node colours at the diagram's opacity.
struct NodeStyle {
    border: Color32,
    fill: Color32,
    shadow: Color32,
    label: Color32,
    icon: Color32,
}

impl NodeStyle {
    fn new(theme: &Theme, opacity: f32) -> Self {
        Self {
            border: Theme::with_opacity(theme.accent, opacity * 0.8),
            fill: Theme::with_opacity(theme.code_background, opacity * 0.95),
            shadow: Theme::with_opacity(Color32::from_rgb(0, 0, 0), opacity * 0.1),
            label: Theme::with_opacity(theme.foreground, opacity),
            icon: Theme::with_opacity(theme.accent, opacity * 0.9),
        }
    }
}

/// Draw every node revealed by the current step.
pub(super) fn draw_nodes(cx: &DiagramCx, nodes: &[DiagramNode], scene: &Scene, steps: &[usize]) {
    let style = NodeStyle::new(cx.theme, cx.opacity);
    for (i, (node, node_box)) in nodes.iter().zip(&scene.boxes).enumerate() {
        if steps.get(i).copied().unwrap_or(0) <= cx.reveal_step {
            draw_node(cx, &style, node, node_box);
        }
    }
}

/// Where an icon's image is: a path as written, a name in
/// `media/diagram-icons/`, nothing for no icon or an unfilled `generate:`.
fn icon_image_path(icon: &str) -> Option<String> {
    let icon = icon.trim();
    if icon.is_empty() || icon.ends_with(':') || icon == "generate" {
        return None;
    }
    if icon.contains('/') {
        return Some(icon.to_string());
    }
    Some(format!("media/diagram-icons/{icon}.png"))
}

/// A card with a drop shadow: icon in the upper half, label below.
fn draw_node(cx: &DiagramCx, style: &NodeStyle, node: &DiagramNode, b: &NodeBox) {
    let painter = cx.painter;
    let scale = cx.scale;
    let corner_radius = 8.0 * scale;

    let shadow_rect = b.rect.translate(egui::vec2(3.0 * scale, 3.0 * scale));
    painter.rect_filled(shadow_rect, corner_radius, style.shadow);
    painter.rect_filled(b.rect, corner_radius, style.fill);
    hints::push(cx.ui.ctx(), hints::Hint::Frame(b.rect));
    painter.rect_stroke(
        b.rect,
        corner_radius,
        Stroke::new(2.5 * scale, style.border),
        egui::StrokeKind::Outside,
    );

    // Icon area (top portion of node)
    let icon_size = b.height * 0.5;
    let icon_center = Pos2::new(b.center.x, b.center.y - b.height * 0.12);
    if !draw_icon_image(cx, &node.icon, icon_center, icon_size) {
        let icon_name = if node.icon.is_empty() {
            "box"
        } else {
            &node.icon
        };
        draw_icon_fallback(
            painter,
            icon_name,
            icon_center,
            icon_size,
            style.icon,
            scale,
        );
    }

    // Label text below icon
    let galley = painter.layout(
        node.label.clone(),
        FontId::new(cx.theme.body_size * 0.8 * scale, cx.theme.body_family()),
        style.label,
        b.width - 8.0 * scale,
    );
    let text_y = b.center.y + b.height * 0.25;
    let text_pos = egui::pos2(b.center.x - galley.rect.width() / 2.0, text_y);
    painter.galley(text_pos, galley, style.label);
}

/// Draw the icon image if it loads, keeping its aspect ratio: a path (a
/// generated icon the deck resolved, `talk.assets/icons/x.png`) as it is, a
/// name as `media/diagram-icons/{icon}.png`. Returns whether an image was
/// drawn.
fn draw_icon_image(cx: &DiagramCx, icon: &str, center: Pos2, icon_size: f32) -> bool {
    let Some(icon_path) = icon_image_path(icon) else {
        return false;
    };
    let Some(texture) = cx.image_cache.get_or_load(cx.ui, &icon_path) else {
        return false;
    };
    let max_size = icon_size * 0.85;
    let tex_size = texture.size_vec2();
    let aspect = tex_size.x / tex_size.y.max(1.0);
    let (w, h) = if aspect >= 1.0 {
        (max_size, max_size / aspect)
    } else {
        (max_size * aspect, max_size)
    };
    let img_rect = egui::Rect::from_center_size(center, egui::vec2(w, h));
    let tint = Theme::with_opacity(Color32::WHITE, cx.opacity);
    cx.painter.image(
        texture.id(),
        img_rect,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        tint,
    );
    true
}

#[cfg(test)]
mod tests {
    use super::icon_image_path;

    #[test]
    fn icons_are_names_or_resolved_paths() {
        assert_eq!(
            icon_image_path("server").as_deref(),
            Some("media/diagram-icons/server.png")
        );
        assert_eq!(
            icon_image_path("talk.assets/icons/gw.png").as_deref(),
            Some("talk.assets/icons/gw.png")
        );
        assert_eq!(icon_image_path(""), None);
        assert_eq!(icon_image_path("generate:"), None, "not generated yet");
        assert_eq!(icon_image_path("generate"), None);
    }
}
