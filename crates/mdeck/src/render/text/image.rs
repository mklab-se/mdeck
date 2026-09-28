//! Images: drawn from the cache inside an area, or a placeholder card.

use eframe::egui::{self, Color32, FontId, Pos2, Stroke};

use crate::parser::ImageDirectives;
use crate::render::image_cache::ImageState;
use crate::render::{BlockCx, TextCx};
use crate::theme::Theme;

/// Maximum height (at reference resolution) of an image drawn inline in a block flow.
pub(super) const IMAGE_MAX_HEIGHT: f32 = 400.0;

/// Draw an image, loading from cache. Falls back to placeholder if unavailable.
pub fn draw_image(
    cx: &BlockCx,
    path: &str,
    alt: &str,
    directives: &ImageDirectives,
    pos: Pos2,
    max_width: f32,
) -> f32 {
    let max_height = IMAGE_MAX_HEIGHT * cx.scale;
    let available = egui::Rect::from_min_size(pos, egui::vec2(max_width, max_height));
    draw_image_in_area(cx, path, alt, directives, available).height()
}

/// Draw an image with full control over the available area (used by image_slide layout).
/// Returns the actual drawn rect.
pub fn draw_image_in_area(
    cx: &BlockCx,
    path: &str,
    alt: &str,
    directives: &ImageDirectives,
    available: egui::Rect,
) -> egui::Rect {
    let (ui, opacity, scale) = (cx.ui, cx.opacity, cx.scale);
    match cx.image_cache.state(ui.ctx(), path) {
        ImageState::Ready(texture) => {
            let tex_size = texture.size_vec2();
            let draw_rect = compute_image_rect(directives, tex_size, available, scale);
            crate::render::hints::push(ui.ctx(), crate::render::hints::Hint::Frame(draw_rect));
            let alpha = (opacity * 255.0) as u8;
            let tint = Color32::from_rgba_unmultiplied(255, 255, 255, alpha);
            let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            ui.painter().image(texture.id(), draw_rect, uv, tint);
            draw_rect
        }
        // Reserve the space quietly; the decode thread repaints when done.
        ImageState::Loading => available,
        ImageState::Missing => {
            let height =
                draw_image_placeholder(&cx.text(), alt, available.left_top(), available.width());
            egui::Rect::from_min_size(available.left_top(), egui::vec2(available.width(), height))
        }
    }
}

/// Compute where an image of `tex_size` pixels is drawn inside `available`.
///
/// `scale` is the resolution scale factor: images are never upscaled beyond
/// their reference-resolution size (`tex_size * scale`), so a picture looks the
/// same at 1080p and in a 4K export.
fn compute_image_rect(
    directives: &ImageDirectives,
    tex_size: egui::Vec2,
    available: egui::Rect,
    scale: f32,
) -> egui::Rect {
    let avail_w = available.width();
    let avail_h = available.height();

    let factor = if directives.fill {
        // Cover: scale to fill, center, may crop
        (avail_w / tex_size.x).max(avail_h / tex_size.y)
    } else if let Some(ref width_str) = directives.width {
        // Explicit width, still bounded by the available height
        let target_w = parse_size(width_str, avail_w, scale);
        (target_w / tex_size.x).min(avail_h / tex_size.y)
    } else {
        // Contain: fit within available area, preserve aspect ratio
        (avail_w / tex_size.x).min(avail_h / tex_size.y).min(scale)
    };

    let draw_w = tex_size.x * factor;
    let draw_h = tex_size.y * factor;
    let offset_x = (avail_w - draw_w) / 2.0;
    let offset_y = (avail_h - draw_h) / 2.0;
    egui::Rect::from_min_size(
        egui::pos2(available.left() + offset_x, available.top() + offset_y),
        egui::vec2(draw_w, draw_h),
    )
}

/// Parse a size directive. Percentages are relative to `reference`; pixel
/// values (with or without a `px` suffix) are in reference-resolution pixels
/// and are multiplied by `scale`.
fn parse_size(s: &str, reference: f32, scale: f32) -> f32 {
    let s = s.trim();
    if let Some(pct) = s.strip_suffix('%')
        && let Ok(v) = pct.trim().parse::<f32>()
    {
        return reference * v / 100.0;
    }
    let px = s.strip_suffix("px").unwrap_or(s).trim();
    match px.parse::<f32>() {
        Ok(v) => v * scale,
        Err(_) => reference * 0.8,
    }
}

/// A framed "[Image: alt]" card where an image could not be loaded. Returns
/// the height used.
pub fn draw_image_placeholder(cx: &TextCx, alt: &str, pos: Pos2, max_width: f32) -> f32 {
    let (ui, theme, opacity, scale) = (cx.ui, cx.theme, cx.opacity, cx.scale);
    let height = 200.0 * scale;
    let bg = Theme::with_opacity(theme.code_background, opacity);
    let color = Theme::with_opacity(theme.foreground, opacity * 0.6);

    let rect = egui::Rect::from_min_size(pos, egui::vec2(max_width, height));
    ui.painter().rect_filled(rect, 8.0 * scale, bg);
    ui.painter().rect_stroke(
        rect,
        8.0 * scale,
        Stroke::new(1.0 * scale, color),
        egui::StrokeKind::Outside,
    );

    let label = if alt.is_empty() {
        "[Image]".to_string()
    } else {
        format!("[Image: {alt}]")
    };
    let galley = ui.painter().layout(
        label,
        FontId::new(theme.body_size * 0.8 * scale, theme.body_family()),
        color,
        max_width,
    );
    let text_pos = Pos2::new(
        pos.x + (max_width - galley.rect.width()) / 2.0,
        pos.y + (height - galley.rect.height()) / 2.0,
    );
    crate::render::math::galley(ui.painter(), text_pos, galley, color);

    height
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contain_mode_upscales_only_to_reference_size() {
        let d = ImageDirectives::default();
        let tex = egui::vec2(400.0, 200.0);
        let area = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(2000.0, 2000.0));
        // 1080p: never larger than native pixels
        assert_eq!(compute_image_rect(&d, tex, area, 1.0).width(), 400.0);
        // 4K export: twice the pixels, same apparent size
        assert_eq!(compute_image_rect(&d, tex, area, 2.0).width(), 800.0);
        // Still bounded by the available area
        let small = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(100.0, 2000.0));
        assert_eq!(compute_image_rect(&d, tex, small, 2.0).width(), 100.0);
    }

    #[test]
    fn width_directive_scales_with_resolution() {
        assert_eq!(parse_size("300px", 1000.0, 2.0), 600.0);
        assert_eq!(parse_size("300", 1000.0, 2.0), 600.0);
        assert_eq!(parse_size("50%", 1000.0, 2.0), 500.0);
        assert_eq!(parse_size("garbage", 1000.0, 2.0), 800.0);

        let d = ImageDirectives {
            width: Some("300px".into()),
            ..Default::default()
        };
        let tex = egui::vec2(600.0, 300.0);
        let area = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(2000.0, 2000.0));
        assert_eq!(compute_image_rect(&d, tex, area, 2.0).width(), 600.0);
    }
}
