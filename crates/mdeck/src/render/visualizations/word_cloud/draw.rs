//! Painting: words in the theme's edge palette, rotated ones turned 90° CCW,
//! revealed by step.

use eframe::egui::{FontId, Pos2, Vec2};
use eframe::epaint::TextShape;

use super::cache::{LAYOUT_CACHE_CAP, cache_key, layout_cache};
use super::layout::{WordLayout, compute_layout};
use super::parse::parse_word_cloud;
use crate::render::visualizations::{VizCtx, VizReveal, assign_steps};
use crate::theme::Theme;

pub fn draw_word_cloud(
    cx: &VizCtx,
    content: &str,
    pos: Pos2,
    max_width: f32,
    max_height: f32,
) -> f32 {
    let VizCtx {
        ui,
        theme,
        opacity,
        scale,
        reveal_step,
        reveal_timestamp: _,
    } = *cx;
    let entries = parse_word_cloud(content);
    if entries.is_empty() {
        return 0.0;
    }

    let height = if max_height > 0.0 {
        max_height
    } else {
        500.0 * scale
    };

    // Get or compute layout
    // the layout depends on the face, so the theme is part of the key
    let key = cache_key(
        &format!("{}\u{0}{}", theme.name, content),
        max_width as u32,
        height as u32,
    );
    let layouts = {
        let mut cache = layout_cache();
        if let Some(cached) = cache.get(&key) {
            cached.clone()
        } else {
            let layout = compute_layout(ui, theme, &entries, max_width, height, scale);
            if cache.len() >= LAYOUT_CACHE_CAP {
                cache.clear();
            }
            cache.insert(key, layout.clone());
            layout
        }
    };

    // Compute reveal steps
    let reveals: Vec<VizReveal> = entries.iter().map(|e| e.reveal).collect();
    let steps = assign_steps(&reveals);
    let palette = theme.edge_palette();

    let painter = ui.painter();

    for (i, entry) in entries.iter().enumerate() {
        let step = steps.get(i).copied().unwrap_or(0);
        if step > reveal_step {
            continue;
        }

        if let Some(wl) = layouts.get(i) {
            // Skip words that couldn't be placed (font_size 0)
            if wl.font_size < 1.0 {
                continue;
            }
            let color_idx = i % palette.len();
            let color = Theme::with_opacity(palette[color_idx], opacity);
            let font_id = FontId::new(wl.font_size, theme.body_family());

            let galley = painter.layout_no_wrap(entry.text.clone(), font_id, color);

            if wl.rotated {
                let anchor_pos = pos + rotated_anchor(wl);
                let text_shape = TextShape::new(anchor_pos, galley, color)
                    .with_angle(-std::f32::consts::FRAC_PI_2)
                    .with_opacity_factor(opacity);
                painter.add(text_shape);
            } else {
                let text_pos = Pos2::new(pos.x + wl.x, pos.y + wl.y);
                painter.galley(text_pos, galley, color);
            }
        }
    }

    height
}

/// Where a word turned 90° CCW is anchored, relative to the cloud's origin.
/// The galley turns about its top-left corner: the text then runs upward
/// from the anchor and its lines extend to the right, so the anchor is the
/// bottom-left corner of the word's reserved box.
fn rotated_anchor(wl: &WordLayout) -> Vec2 {
    Vec2::new(wl.x, wl.y + wl.height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{Rect, emath::Rot2};

    #[test]
    fn a_rotated_word_is_drawn_inside_its_box() {
        // A word 120 wide and 30 tall, turned: a 30x120 box at (200, 100).
        let (text_w, text_h) = (120.0, 30.0);
        let wl = WordLayout {
            x: 200.0,
            y: 100.0,
            width: text_h,
            height: text_w,
            font_size: 24.0,
            rotated: true,
        };
        let anchor = Pos2::ZERO + rotated_anchor(&wl);
        // The galley's corners, turned about the anchor the way TextShape does.
        let rot = Rot2::from_angle(-std::f32::consts::FRAC_PI_2);
        let corners = [
            Vec2::ZERO,
            Vec2::new(text_w, 0.0),
            Vec2::new(0.0, text_h),
            Vec2::new(text_w, text_h),
        ]
        .map(|c| anchor + rot * c);
        let drawn = Rect::from_points(&corners);
        let reserved = Rect::from_min_size(Pos2::new(wl.x, wl.y), Vec2::new(wl.width, wl.height));
        assert!(
            (drawn.min - reserved.min).length() < 0.01
                && (drawn.max - reserved.max).length() < 0.01,
            "drawn {drawn:?}, reserved {reserved:?}"
        );
    }
}
