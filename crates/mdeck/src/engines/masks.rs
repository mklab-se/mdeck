//! Glyph and text masks sampled out of egui's font atlas: the countdown
//! digits and the end words, in the theme's display face.

use eframe::egui;

use super::Mask;
use crate::render::particles;
use crate::theme::Theme;

/// Spectral's 1 wears a long flag. Keep only the half of it nearest the stem,
/// then renormalise the mask to its new width.
pub(super) fn trim_flag((pts, aspect): Mask) -> Mask {
    // The flag is the part left of the stem in the top third of the glyph;
    // the stem starts around 45% of the width in this face.
    let flag_cut = 0.24;
    let kept: Vec<[f32; 2]> = pts
        .iter()
        .copied()
        .filter(|p| !(p[1] < 0.34 && p[0] < flag_cut))
        .collect();
    let min_x = kept.iter().map(|p| p[0]).fold(1.0, f32::min);
    let width = (1.0 - min_x).max(1e-3);
    let renormalised = kept
        .into_iter()
        .map(|p| [(p[0] - min_x) / width, p[1]])
        .collect();
    (std::sync::Arc::new(renormalised), aspect * width)
}

/// Sample a glyph's coverage out of egui's font atlas into mask points in the
/// unit square, returning them with the glyph's width / height.
pub(super) fn glyph_mask(ui: &egui::Ui, theme: &Theme, ch: char) -> Mask {
    text_mask(ui, theme, &ch.to_string())
}

/// Sample a whole string's coverage out of egui's font atlas into mask points
/// in the unit square, with the text's width / height. Glyphs keep their
/// layout positions, so spacing and kerning come from the face.
pub(super) fn text_mask(ui: &egui::Ui, theme: &Theme, text: &str) -> Mask {
    let font = egui::FontId::new(220.0, theme.display_family());
    let (glyphs, image) = ui.fonts_mut(|f| {
        let galley = f.layout_no_wrap(text.to_string(), font, egui::Color32::WHITE);
        let glyphs: Vec<(egui::Pos2, egui::Vec2, [u16; 2], [u16; 2])> = galley
            .rows
            .iter()
            .flat_map(|r| r.glyphs.iter())
            .filter(|g| g.uv_rect.max[0] > g.uv_rect.min[0])
            .map(|g| {
                (
                    g.pos + g.uv_rect.offset,
                    g.uv_rect.size,
                    g.uv_rect.min,
                    g.uv_rect.max,
                )
            })
            .collect();
        (glyphs, f.image())
    });
    if glyphs.is_empty() {
        return (std::sync::Arc::new(Vec::new()), 0.6);
    }
    // bounding box of the ink in layout points
    let (mut bx0, mut by0, mut bx1, mut by1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for (pos, size, _, _) in &glyphs {
        bx0 = bx0.min(pos.x);
        by0 = by0.min(pos.y);
        bx1 = bx1.max(pos.x + size.x);
        by1 = by1.max(pos.y + size.y);
    }
    let (bw, bh) = ((bx1 - bx0).max(1.0), (by1 - by0).max(1.0));
    let mut pts = Vec::new();
    for (pos, size, min, max) in &glyphs {
        let (x0, y0, x1, y1) = (
            min[0] as usize,
            min[1] as usize,
            max[0] as usize,
            max[1] as usize,
        );
        let (gw, gh) = ((x1 - x0).max(1), (y1 - y0).max(1));
        let step = (bh as usize / 90).max(1);
        for y in (y0..y1).step_by(step) {
            for x in (x0..x1).step_by(step) {
                if image[(x, y)].a() > 110 {
                    let px = pos.x + (x - x0) as f32 / gw as f32 * size.x;
                    let py = pos.y + (y - y0) as f32 / gh as f32 * size.y;
                    pts.push([(px - bx0) / bw, (py - by0) / bh]);
                }
            }
        }
    }
    // Masks are consumed in order (a group of n particles takes the first n
    // points), and scanline order would light the top of the glyph first.
    shuffle(&mut pts, 0x6C7F);
    (std::sync::Arc::new(pts), bw / bh)
}

/// Deterministic Fisher-Yates.
fn shuffle(pts: &mut [[f32; 2]], seed: u64) {
    let mut rng = particles::Rng::new(seed);
    for i in (1..pts.len()).rev() {
        let j = (rng.unit() * (i + 1) as f32) as usize;
        pts.swap(i, j.min(i));
    }
}

// The masks are drawn in ember's display face.
#[cfg(all(test, feature = "particles"))]
mod tests {
    use super::*;

    #[test]
    fn end_words_mask_is_wide_and_dense() {
        let ctx = egui::Context::default();
        crate::render::fonts::install(&ctx);
        let theme = Theme::ember();
        let mut output = ctx.run_ui(Default::default(), |ui| {
            let (pts, aspect) = text_mask(ui, &theme, "THE END");
            assert!(pts.len() > 1500, "only {} points", pts.len());
            assert!(aspect > 4.0 && aspect < 9.0, "aspect {aspect}");
            let (w, h) = (((14.0 * aspect) * 2.0) as usize, 14usize);
            let mut grid = vec![vec![' '; w + 1]; h + 1];
            for p in pts.iter() {
                let x = (p[0] * w as f32) as usize;
                let y = (p[1] * h as f32) as usize;
                grid[y.min(h)][x.min(w)] = '#';
            }
            eprintln!("--- THE END (aspect {aspect:.2}, {} points)", pts.len());
            for row in grid {
                eprintln!("{}", row.into_iter().collect::<String>());
            }
        });
        output.textures_delta.clear();
    }

    /// The digit masks come out of egui's font atlas; make sure the sampled
    /// coverage is a real glyph (dense, right aspect) and print it so a
    /// reviewer can eyeball the shape in the test output.
    #[test]
    fn digit_masks_are_glyph_shaped() {
        let ctx = egui::Context::default();
        crate::render::fonts::install(&ctx);
        let theme = Theme::ember();
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1920.0, 1080.0),
                )),
                ..Default::default()
            },
            |ui| {
                for ch in ['3', '2', '1'] {
                    let mut mask = glyph_mask(ui, &theme, ch);
                    if ch == '1' {
                        mask = trim_flag(mask);
                    }
                    let (pts, aspect) = mask;
                    assert!(pts.len() > 300, "{ch}: only {} points", pts.len());
                    assert!(aspect > 0.35 && aspect < 0.9, "{ch}: aspect {aspect}");
                    // ASCII dump, 24 rows
                    let (w, h) = (((24.0 * aspect) * 2.0) as usize, 24usize);
                    let mut grid = vec![vec![' '; w + 1]; h + 1];
                    for p in pts.iter() {
                        let x = (p[0] * w as f32) as usize;
                        let y = (p[1] * h as f32) as usize;
                        grid[y.min(h)][x.min(w)] = '#';
                    }
                    eprintln!("--- {ch} (aspect {aspect:.2}, {} points)", pts.len());
                    for row in grid {
                        eprintln!("{}", row.into_iter().collect::<String>());
                    }
                }
            },
        );
        output.textures_delta.clear();
    }
}
