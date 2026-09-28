//! Cast labels: the collision check `mdeck --check` runs and the painter
//! that draws them over the field.

use eframe::egui::{Pos2, Rect};

use super::{Label, Staged};

/// Pairs of labels whose boxes would overlap on a slide of the given aspect.
/// Label boxes are estimated from text length (tracked mono at 13 px on a
/// 1080 px tall slide) so scripts can be checked without a font.
pub fn label_collisions(staged: &Staged, aspect: f32) -> Vec<(String, String)> {
    let char_w = 0.0066; // fraction of slide width per character
    let h = 0.028; // fraction of slide height
    let boxes: Vec<(String, Rect)> = staged
        .labels
        .iter()
        .map(|l| {
            let w = l.text.chars().count() as f32 * char_w;
            (
                l.text.clone(),
                Rect::from_min_size(Pos2::new(l.u - w / 2.0, l.v), eframe::egui::vec2(w, h)),
            )
        })
        .collect();
    let _ = aspect;
    let mut out = Vec::new();
    for i in 0..boxes.len() {
        for j in i + 1..boxes.len() {
            if boxes[i].1.intersects(boxes[j].1) {
                out.push((boxes[i].0.clone(), boxes[j].0.clone()));
            }
        }
    }
    out
}

/// Draw the cast labels over the field: tracked mono, fading with each
/// member's group and warming when it runs hot.
pub fn draw_labels(
    painter: &eframe::egui::Painter,
    labels: &[Label],
    field: &crate::render::particles::Field,
    rect: Rect,
    theme: &crate::theme::Theme,
    scale: f32,
    opacity: f32,
) {
    use eframe::egui::{Color32, FontId, text::LayoutJob, text::TextFormat};
    let size = 13.0 * scale;
    for l in labels {
        let life = field.group_life(l.group);
        if life < 0.02 {
            continue;
        }
        let heat = field.group_heat(l.group);
        let base = if l.figure {
            theme.code_foreground
        } else {
            theme.muted
        };
        let ember = theme.accent_soft;
        let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * heat) as u8;
        let color = Color32::from_rgba_unmultiplied(
            mix(base.r(), ember.r()),
            mix(base.g(), ember.g()),
            mix(base.b(), ember.b()),
            (life * opacity * 255.0) as u8,
        );
        let mut job = LayoutJob::default();
        job.append(
            &l.text.to_uppercase(),
            0.0,
            TextFormat {
                font_id: FontId::new(size, theme.mono_family()),
                color,
                extra_letter_spacing: size * 0.18,
                ..Default::default()
            },
        );
        let galley = painter.layout_job(job);
        let x = rect.left() + l.u * rect.width() - galley.rect.width() / 2.0;
        let y = rect.top() + l.v * rect.height();
        painter.galley(Pos2::new(x, y), galley, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Layout;
    use crate::render::story::fixtures::{SAMPLE, test_library};
    use crate::render::story::{Script, stage};

    #[test]
    fn labels_in_neighbouring_cells_do_not_collide() {
        let s = Script::parse(SAMPLE).unwrap();
        let mut lib = test_library();
        let staged = stage(&s, Layout::Bullet, 16.0 / 9.0, &mut lib);
        assert!(label_collisions(&staged, 16.0 / 9.0).is_empty());
        // two long labels in adjacent cells on the same row do collide
        let tight = Script::parse(
            "cast:\n  - { id: a, kind: box, label: 'A rather long label here', cell: left }\n  - { id: b, kind: box, label: 'Another rather long label', cell: center }\n",
        )
        .unwrap();
        let staged = stage(&tight, Layout::Bullet, 16.0 / 9.0, &mut lib);
        assert!(!label_collisions(&staged, 16.0 / 9.0).is_empty());
    }
}
