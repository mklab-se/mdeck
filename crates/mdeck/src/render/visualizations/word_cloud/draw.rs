//! Painting: words in the theme's edge palette, rotated ones turned 90° CCW,
//! revealed by step.

use eframe::egui::{FontId, Pos2};
use eframe::epaint::TextShape;

use super::cache::{LAYOUT_CACHE_CAP, cache_key, layout_cache};
use super::layout::compute_layout;
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
                // Rotate -90° (CCW). Pivot is at pos (top-left of unrotated text).
                // For visual bbox at (vx, vy) with visual size (orig_h, orig_w):
                //   anchor pos.x = vx + visual_width (= vx + orig_h)
                //   anchor pos.y = vy
                let anchor_pos = Pos2::new(pos.x + wl.x + wl.width, pos.y + wl.y);
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
