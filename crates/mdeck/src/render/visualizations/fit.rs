//! Fitting labels into the space a chart has for them.

use eframe::egui::{self, Color32, FontId};

/// The largest font size (at most `font.size`, at least `min_font_size`) at
/// which every one of `texts` fits within `max_width`. Use it to give a group
/// of labels one shared size instead of shrinking each label independently.
pub fn fit_font_size(
    painter: &egui::Painter,
    texts: &[&str],
    font: &FontId,
    max_width: f32,
    min_font_size: f32,
) -> f32 {
    let min_font_size = min_font_size.min(font.size);
    if max_width <= 0.0 {
        return min_font_size;
    }
    let mut size = font.size;
    for text in texts {
        let probe = FontId::new(size, font.family.clone());
        let width = painter
            .layout_no_wrap(text.to_string(), probe, Color32::WHITE)
            .rect
            .width();
        if width > max_width {
            size = (size * max_width / width).max(min_font_size);
        }
    }
    size
}

/// Lay out `text` so it fits within `max_width`: first shrink the font down to
/// `min_font_size`, then truncate with an ellipsis if it still does not fit.
pub fn fit_text(
    painter: &egui::Painter,
    text: &str,
    font: FontId,
    color: Color32,
    max_width: f32,
    min_font_size: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut font = font;
    let mut galley = painter.layout_no_wrap(text.to_string(), font.clone(), color);
    if max_width <= 0.0 || galley.rect.width() <= max_width {
        return galley;
    }

    // Shrink proportionally in one go, then nudge down until it fits.
    let min_font_size = min_font_size.min(font.size);
    font.size = (font.size * max_width / galley.rect.width()).max(min_font_size);
    galley = painter.layout_no_wrap(text.to_string(), font.clone(), color);
    while galley.rect.width() > max_width && font.size > min_font_size {
        font.size = (font.size - 0.5).max(min_font_size);
        galley = painter.layout_no_wrap(text.to_string(), font.clone(), color);
    }
    if galley.rect.width() <= max_width {
        return galley;
    }

    // Truncate: binary search the longest prefix that fits with an ellipsis.
    let chars: Vec<char> = text.chars().collect();
    let candidate = |n: usize| -> String {
        let prefix: String = chars[..n].iter().collect();
        format!("{}…", prefix.trim_end())
    };
    let (mut lo, mut hi) = (0usize, chars.len());
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let g = painter.layout_no_wrap(candidate(mid), font.clone(), color);
        if g.rect.width() <= max_width {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    painter.layout_no_wrap(candidate(lo), font.clone(), color)
}

/// How many labels to skip between drawn labels so that labels `label_width`
/// wide do not overlap when their slots are `slot_width` apart. Returns 1 when
/// every label fits.
pub fn label_stride(label_width: f32, slot_width: f32) -> usize {
    if slot_width <= 0.0 || label_width <= slot_width {
        return 1;
    }
    (label_width / slot_width).ceil().max(1.0) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::visualizations::tests::with_test_painter;

    #[test]
    fn test_label_stride() {
        assert_eq!(label_stride(40.0, 100.0), 1);
        assert_eq!(label_stride(100.0, 100.0), 1);
        assert_eq!(label_stride(101.0, 100.0), 2);
        assert_eq!(label_stride(250.0, 100.0), 3);
        assert_eq!(label_stride(50.0, 0.0), 1);
    }

    #[test]
    fn test_fit_text_keeps_short_text_unchanged() {
        with_test_painter(|painter| {
            let font = FontId::proportional(20.0);
            let g = fit_text(painter, "Short", font, Color32::WHITE, 500.0, 10.0);
            assert_eq!(g.text(), "Short");
            assert_eq!(g.job.sections[0].format.font_id.size, 20.0);
        });
    }

    #[test]
    fn test_fit_text_shrinks_then_truncates() {
        with_test_painter(|painter| {
            let font = FontId::proportional(20.0);
            let text = "A fairly long category label";
            let full = painter.layout_no_wrap(text.to_string(), font.clone(), Color32::WHITE);
            let full_w = full.rect.width();

            // Mild overflow: shrinks the font, keeps the full text
            let g = fit_text(
                painter,
                text,
                font.clone(),
                Color32::WHITE,
                full_w * 0.8,
                10.0,
            );
            assert_eq!(g.text(), text);
            assert!(g.rect.width() <= full_w * 0.8 + 0.01);
            let size = g.job.sections[0].format.font_id.size;
            assert!((10.0..20.0).contains(&size));

            // Severe overflow: hits the floor and truncates with an ellipsis
            let g = fit_text(painter, text, font, Color32::WHITE, full_w * 0.3, 16.0);
            assert!(g.text().ends_with('…'));
            assert!(g.text().len() < text.len());
            assert!(g.rect.width() <= full_w * 0.3 + 0.01);
            assert_eq!(g.job.sections[0].format.font_id.size, 16.0);
        });
    }
}
