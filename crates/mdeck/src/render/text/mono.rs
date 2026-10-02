//! Code as written: no ligatures, and inline code on its line's baseline.
//!
//! Monospace fonts like JetBrains Mono join `---`, `->` or `<!--` into
//! ligatures, which hides the characters a slide is showing. epaint shapes
//! each section of a layout job on its own, so code is appended with a
//! section boundary between neighbouring punctuation: nothing can join.
//!
//! Inline code is set smaller than its prose, and epaint aligns a smaller
//! font to the bottom of the row, which puts it below the prose baseline
//! when the prose has a tall line height. [`settle`] moves each chip (the
//! code's background and its glyphs) up onto the baseline of the prose
//! around it. Only inline code has a section background, so the chips are
//! found from the background quads at the start of each row's mesh.

use std::sync::Arc;

use eframe::egui::{self, Color32, Rect};

/// Append `text` to `job` in `format`, with no two neighbouring punctuation
/// characters in the same section (so no ligature can form).
pub fn append_code(job: &mut egui::text::LayoutJob, text: &str, format: egui::text::TextFormat) {
    let mut start = 0;
    let mut prev: Option<char> = None;
    for (i, c) in text.char_indices() {
        if prev.is_some_and(|p| joins(p) && joins(c)) {
            push(job, &text[start..i], &format);
            start = i;
        }
        prev = Some(c);
    }
    push(job, &text[start..], &format);
}

/// A section of its own (`LayoutJob::append` merges equal neighbours).
fn push(job: &mut egui::text::LayoutJob, text: &str, format: &egui::text::TextFormat) {
    use egui::text::{ByteIndex, LayoutSection};
    let start = job.text.len();
    job.text.push_str(text);
    job.sections.push(LayoutSection {
        leading_space: 0.0,
        byte_range: ByteIndex(start)..ByteIndex(job.text.len()),
        format: format.clone(),
    });
}

/// Characters a code font may join into a ligature.
fn joins(c: char) -> bool {
    c.is_ascii_punctuation()
}

/// `galley` with every inline code chip moved onto its row's prose
/// baseline. Heights do not change, so measuring is unaffected.
pub fn settle(galley: Arc<egui::Galley>) -> Arc<egui::Galley> {
    let any_chip = galley
        .job
        .sections
        .iter()
        .any(|s| s.format.background != Color32::TRANSPARENT);
    if !any_chip {
        return galley;
    }
    let mut out: egui::Galley = (*galley).clone();
    let ppp = out.pixels_per_point.max(f32::EPSILON);
    let mut moved = false;
    for placed in &mut out.rows {
        let row = &placed.row;
        let chips = chip_rects(row);
        if chips.is_empty() {
            continue;
        }
        let inside = |g: &egui::epaint::text::Glyph| {
            let x = g.pos.x + g.advance_width / 2.0;
            chips.iter().any(|c| x >= c.left() && x <= c.right())
        };
        let Some(baseline) = row
            .glyphs
            .iter()
            .filter(|g| !inside(g) && !g.chr.is_whitespace() && visible(g))
            .map(|g| g.pos.y)
            .next()
        else {
            continue;
        };
        let Some(code_y) = row.glyphs.iter().filter(|g| inside(g)).map(|g| g.pos.y).next() else {
            continue;
        };
        let dy = ((baseline - code_y) * ppp).round() / ppp;
        if dy.abs() < 0.5 / ppp {
            continue;
        }
        let row = Arc::make_mut(&mut placed.row);
        let chip_end = row.visuals.glyph_vertex_range.start;
        let mut shift: Vec<std::ops::Range<usize>> = vec![0..chip_end];
        for g in row.glyphs.iter_mut().filter(|g| inside(g)) {
            g.pos.y += dy;
            if !g.uv_rect.is_nothing() {
                let v = g.first_vertex as usize;
                shift.push(v..v + 4);
            }
        }
        let vertices = &mut row.visuals.mesh.vertices;
        for r in shift {
            for v in vertices.iter_mut().take(r.end).skip(r.start) {
                v.pos.y += dy;
            }
        }
        row.visuals.mesh_bounds = row.visuals.mesh.calc_bounds();
        moved = true;
    }
    if !moved {
        return galley;
    }
    out.mesh_bounds = out
        .rows
        .iter()
        .map(|r| r.row.visuals.mesh_bounds.translate(r.pos.to_vec2()))
        .fold(Rect::NOTHING, |a, b| a.union(b));
    Arc::new(out)
}

/// The background quads at the start of `row`'s mesh (inline code chips).
fn chip_rects(row: &egui::epaint::text::Row) -> Vec<Rect> {
    let v = &row.visuals;
    v.mesh.vertices[..v.glyph_vertex_range.start.min(v.mesh.vertices.len())]
        .chunks_exact(4)
        .map(|q| Rect::from_points(&q.iter().map(|v| v.pos).collect::<Vec<_>>()))
        .collect()
}

/// A glyph that is not an invisible formula placeholder.
fn visible(g: &egui::epaint::text::Glyph) -> bool {
    !crate::render::math::is_placeholder_char(g.chr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::test_support::with_ui;
    use eframe::egui::text::{LayoutJob, TextFormat};
    use eframe::egui::{FontFamily, FontId};

    fn sections(job: &LayoutJob) -> Vec<&str> {
        job.sections
            .iter()
            .map(|s| &job.text[s.byte_range.start.0..s.byte_range.end.0])
            .collect()
    }

    #[test]
    fn punctuation_never_shares_a_section() {
        let mut job = LayoutJob::default();
        append_code(&mut job, "a --- b <!-- c -->", TextFormat::default());
        assert_eq!(job.text, "a --- b <!-- c -->");
        for s in sections(&job) {
            let p: Vec<char> = s.chars().filter(|c| joins(*c)).collect();
            assert!(
                s.chars()
                    .zip(s.chars().skip(1))
                    .all(|(a, b)| !(joins(a) && joins(b))),
                "{s:?} can join {p:?}"
            );
        }
        let mut word = LayoutJob::default();
        append_code(&mut word, "design", TextFormat::default());
        assert_eq!(sections(&word), ["design"]);
        let mut empty = LayoutJob::default();
        append_code(&mut empty, "", TextFormat::default());
        assert_eq!(empty.sections.len(), 1);
    }

    #[test]
    fn inline_code_sits_on_the_prose_baseline() {
        with_ui(|ui| {
            let size = 34.0;
            let prose = TextFormat {
                font_id: FontId::new(size, FontFamily::Proportional),
                line_height: Some(size * 1.5),
                ..Default::default()
            };
            let code = TextFormat {
                font_id: FontId::new(size * 0.85, FontFamily::Monospace),
                background: Color32::from_gray(40),
                ..Default::default()
            };
            let mut job = LayoutJob::default();
            job.append("A list is ", 0.0, prose.clone());
            append_code(&mut job, "points", code);
            job.append(" today", 0.0, prose);
            let raw = ui.painter().layout_job(job);
            let row = &raw.rows[0].row;
            let (p, c) = (row.glyphs[0].pos.y, row.glyphs[10].pos.y);
            assert!(c > p + 1.0, "egui puts the smaller code low: {c} vs {p}");
            let settled = settle(raw.clone());
            let row = &settled.rows[0].row;
            assert_eq!(row.glyphs[10].pos.y, row.glyphs[0].pos.y);
            assert_eq!(settled.rect, raw.rect, "the height does not change");
            // the chip moved with its text: it ends above the row's bottom
            let chip = chip_rects(row)[0];
            assert!(chip.bottom() < row.size.y - 1.0, "{chip:?}");
            assert!(chip.top() >= 0.0);
        });
    }
}
