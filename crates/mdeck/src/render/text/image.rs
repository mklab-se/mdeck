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
        ImageState::Missing if crate::assets::placeholders::is_image(path) => {
            // a 3:2 card, as large as the area allows, centred in it
            let w = available.width();
            let h = (w * 2.0 / 3.0).min(available.height()).max(160.0 * scale);
            let rect = egui::Rect::from_center_size(available.center(), egui::vec2(w, h));
            let rect = rect.translate(egui::vec2(0.0, (available.top() - rect.top()).max(0.0)));
            draw_pending_image(&cx.text(), alt, rect);
            rect
        }
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
    } else if directives.width.is_some() || directives.height.is_some() {
        // Explicit width and/or height, still bounded by the available area
        let by_w = directives
            .width
            .as_deref()
            .map(|w| parse_size(w, avail_w, scale) / tex_size.x);
        let by_h = directives
            .height
            .as_deref()
            .map(|h| parse_size(h, avail_h, scale) / tex_size.y);
        let wanted = match (by_w, by_h) {
            (Some(w), Some(h)) => w.min(h),
            (Some(k), None) | (None, Some(k)) => k,
            (None, None) => unreachable!("one of them is set"),
        };
        wanted.min(avail_w / tex_size.x).min(avail_h / tex_size.y)
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

/// A `generate:` image that has not been generated: the prompt in italics
/// on a soft card in `rect`, and a quiet note on what makes it.
fn draw_pending_image(cx: &TextCx, prompt: &str, rect: egui::Rect) {
    let (ui, theme, opacity, scale) = (cx.ui, cx.theme, cx.opacity, cx.scale);
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        12.0 * scale,
        Theme::with_opacity(theme.code_background, opacity * 0.7),
    );
    painter.rect_stroke(
        rect.shrink(8.0 * scale),
        8.0 * scale,
        Stroke::new(
            1.0 * scale,
            Theme::with_opacity(theme.accent, opacity * 0.3),
        ),
        egui::StrokeKind::Inside,
    );
    let wrap = (rect.width() - 96.0 * scale).max(40.0 * scale);
    let label = if prompt.trim().is_empty() {
        "A picture for this slide"
    } else {
        prompt.trim()
    };
    let centred = |text: &str, size: f32, italics: bool, color: Color32| {
        let mut job = egui::text::LayoutJob::single_section(
            text.to_string(),
            egui::TextFormat {
                font_id: FontId::new(size, theme.body_family()),
                color,
                italics,
                ..Default::default()
            },
        );
        job.wrap.max_width = wrap;
        job.halign = egui::Align::Center;
        painter.layout_job(job)
    };
    let main = centred(
        label,
        theme.body_size * 0.75 * scale,
        true,
        Theme::with_opacity(theme.foreground, opacity * 0.75),
    );
    let note = centred(
        "not generated yet \u{00b7} mdeck ai images",
        theme.body_size * 0.5 * scale,
        false,
        Theme::with_opacity(theme.foreground, opacity * 0.4),
    );
    let gap = 18.0 * scale;
    let total = main.rect.height() + gap + note.rect.height();
    let top = rect.center().y - total / 2.0;
    let x = rect.center().x;
    painter.galley(Pos2::new(x, top), main, Color32::WHITE);
    painter.galley(
        Pos2::new(x, top + total - note.rect.height()),
        note,
        Color32::WHITE,
    );
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

    #[test]
    fn height_directive_sizes_the_image_and_both_keep_the_aspect() {
        let tex = egui::vec2(600.0, 300.0);
        let area = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(2000.0, 1000.0));
        let h = ImageDirectives {
            height: Some("50%".into()),
            ..Default::default()
        };
        let r = compute_image_rect(&h, tex, area, 1.0);
        assert_eq!((r.width(), r.height()), (1000.0, 500.0));
        // both: the smaller wins, so the image keeps its aspect
        let both = ImageDirectives {
            width: Some("300px".into()),
            height: Some("50%".into()),
            ..Default::default()
        };
        assert_eq!(compute_image_rect(&both, tex, area, 1.0).width(), 300.0);
        // never beyond the available area
        let big = ImageDirectives {
            height: Some("5000px".into()),
            ..Default::default()
        };
        assert_eq!(compute_image_rect(&big, tex, area, 1.0).height(), 1000.0);
    }
}
