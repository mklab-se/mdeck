//! Placement: font sizes from the size values, then a dense spiral packing
//! inside an ellipse, largest words first, and a final fit to the area.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use eframe::egui::{self, Color32, FontId};

use super::parse::WordEntry;
use crate::theme::Theme;

/// Smallest font (reference pixels) a word may be drawn at: half the body
/// size, the readability floor used by every visualization.
const WORD_CLOUD_MIN_FONT: f32 = 22.0;

#[derive(Debug, Clone)]
pub(super) struct WordLayout {
    /// Visual bounding box position and size (after rotation)
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) width: f32,
    pub(super) height: f32,
    pub(super) font_size: f32,
    /// Whether this word is rotated 90° counter-clockwise
    pub(super) rotated: bool,
}

/// Deterministic "random" check: should this word be rotated?
/// Uses a hash of the word text + index so it's stable across frames.
/// Small, short words rotate more often: they slot into vertical gaps
/// between large horizontal words. Long words stay horizontal since
/// they'd create tall columns if rotated.
fn should_rotate(text: &str, index: usize, rank: usize, total: usize) -> bool {
    // Never rotate if too few words (need density for cloud shape)
    if total < 12 {
        return false;
    }
    // Only the smallest quarter of words can rotate: they're small enough
    // to slot into vertical gaps without creating columns at the edges.
    if rank < (total * 3) / 4 {
        return false;
    }

    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    index.hash(&mut hasher);
    let hash = hasher.finish();

    // Among the smallest quarter: short words rotate ~45%, long words ~15%
    let length_penalty = (text.len() as f32 / 12.0).min(1.0);
    let threshold = (45.0 - length_penalty * 30.0) as u64;
    hash % 100 < threshold
}

/// Compute font sizes that fill the available area densely.
/// The largest word's size value maps to a font that takes up a significant
/// portion of the area; other words scale proportionally.
fn compute_font_sizes(
    entries: &[WordEntry],
    area_width: f32,
    area_height: f32,
    scale: f32,
) -> Vec<f32> {
    if entries.is_empty() {
        return vec![];
    }

    let max_size = entries
        .iter()
        .map(|e| e.size)
        .fold(0.0f32, f32::max)
        .max(1.0);
    let min_size = entries
        .iter()
        .map(|e| e.size)
        .fold(f32::MAX, f32::min)
        .max(1.0);

    // Scale max font based on word count: more words → smaller fonts.
    // Sized to create a dense cloud shape within an elliptical boundary.
    let n = entries.len() as f32;
    // Scale fonts to fill the elliptical cloud area densely.
    let count_factor = (8.0 / n.max(1.0)).sqrt().clamp(0.28, 0.80);
    // Max font sized so the biggest word is prominent but not overwhelming.
    // At 1920x1080 with ~75 words: roughly 8-10% of area height.
    let max_font = (area_height * 0.28 * count_factor).min(area_width * 0.14 * count_factor);
    // Smallest word is ~14% of the largest, but never below the readable floor
    let min_font = (max_font * 0.14).max(WORD_CLOUD_MIN_FONT * scale);

    entries
        .iter()
        .map(|e| {
            if (max_size - min_size).abs() < 0.001 {
                max_font * scale
            } else {
                let t = (e.size - min_size) / (max_size - min_size);
                // Moderate convex curve: big words are clearly bigger, but not
                // so extreme that they dwarf everything else.
                let t_curved = t.powf(1.5);
                (min_font + t_curved * (max_font - min_font)) * scale
            }
        })
        .collect()
}

struct PlaceCtx {
    cx: f32,
    cy: f32,
    area_width: f32,
    area_height: f32,
    /// Ellipse semi-axes for cloud shape constraint
    ellipse_a: f32,
    ellipse_b: f32,
    scale: f32,
}

/// Check whether a rectangle fits inside the cloud ellipse.
/// We check that the rectangle's center is within a shrunk ellipse
/// (shrunk by half the rect dimensions) so the whole rect stays inside.
fn rect_inside_ellipse(x: f32, y: f32, w: f32, h: f32, ctx: &PlaceCtx) -> bool {
    let center_x = x + w / 2.0;
    let center_y = y + h / 2.0;
    // Shrink ellipse by half the word dimensions so edges stay inside
    let a = (ctx.ellipse_a - w / 2.0).max(1.0);
    let b = (ctx.ellipse_b - h / 2.0).max(1.0);
    let dx = center_x - ctx.cx;
    let dy = center_y - ctx.cy;
    (dx * dx) / (a * a) + (dy * dy) / (b * b) <= 1.0
}

/// Try to place a word using spiral search. Returns None if no valid position found.
/// `word_size` is the font size of the word being placed, used to adapt spiral granularity.
fn spiral_place(
    ctx: &PlaceCtx,
    w: f32,
    h: f32,
    placed: &[WordLayout],
    pad: f32,
    word_size: f32,
) -> Option<(f32, f32)> {
    let mut t = 0.0f32;
    // Smaller words need finer spiral steps to find gaps between larger words
    let size_ratio = (word_size / (ctx.area_height * 0.15)).min(1.0);
    let t_step = 0.015 + size_ratio * 0.035; // finer steps for denser packing
    // Slow growth rate keeps words close to center (cloud-like)
    let base_growth = (ctx.area_width + ctx.area_height) * 0.00025;
    let growth = (base_growth * (0.3 + size_ratio * 0.7)) * ctx.scale;
    // Horizontal stretch to match ellipse shape
    let aspect = (ctx.ellipse_a / ctx.ellipse_b).max(1.0);

    let max_iters = if size_ratio < 0.3 { 40000 } else { 25000 };

    for _ in 0..max_iters {
        let angle = t * 2.5;
        let r = t * growth;
        let x = ctx.cx + r * angle.cos() * aspect - w / 2.0;
        let y = ctx.cy + r * angle.sin() - h / 2.0;

        // Check elliptical boundary (cloud shape) instead of rectangular
        if rect_inside_ellipse(x, y, w, h, ctx) {
            let overlaps = placed.iter().any(|p| {
                x < p.x + p.width + pad
                    && x + w + pad > p.x
                    && y < p.y + p.height + pad
                    && y + h + pad > p.y
            });
            if !overlaps {
                return Some((x, y));
            }
        }
        t += t_step;
    }
    None
}

/// Try to place a word, testing both its preferred rotation and the alternative.
/// Returns the placed WordLayout or None if it truly can't fit.
fn try_place_word(
    ui: &egui::Ui,
    theme: &Theme,
    ctx: &PlaceCtx,
    entry: &WordEntry,
    base_fs: f32,
    prefer_rotated: bool,
    placed: &[WordLayout],
) -> Option<WordLayout> {
    // Try preferred rotation first, then the alternative
    for &try_rotated in &[prefer_rotated, !prefer_rotated] {
        // Try progressively smaller sizes
        for shrink in 0..6 {
            let try_fs = base_fs * (1.0 - shrink as f32 * 0.10);
            if try_fs < (WORD_CLOUD_MIN_FONT * ctx.scale).min(base_fs) {
                break; // don't go below the readable floor; drop the word instead
            }
            let font_id = FontId::new(try_fs, theme.body_family());
            let galley = ui
                .painter()
                .layout_no_wrap(entry.text.clone(), font_id, Color32::WHITE);
            let orig_w = galley.rect.width();
            let orig_h = galley.rect.height();

            let (vis_w, vis_h) = if try_rotated {
                (orig_h, orig_w)
            } else {
                (orig_w, orig_h)
            };

            // Padding between words creates gaps that small vertical words
            // can slot into. Scale with font size so big words get bigger gaps.
            let pad = (try_fs * 0.15).max(2.0 * ctx.scale);
            if let Some((x, y)) = spiral_place(ctx, vis_w, vis_h, placed, pad, try_fs) {
                return Some(WordLayout {
                    x,
                    y,
                    width: vis_w,
                    height: vis_h,
                    font_size: try_fs,
                    rotated: try_rotated,
                });
            }
        }
    }
    None
}

/// Dense spiral placement: place largest words first at center, pack tightly.
/// Words that can't fit are dropped (not overlaid on top of others).
/// Some words are rotated 90° CCW for a classic word cloud look.
pub(super) fn compute_layout(
    ui: &egui::Ui,
    theme: &Theme,
    entries: &[WordEntry],
    area_width: f32,
    area_height: f32,
    scale: f32,
) -> Vec<WordLayout> {
    let font_sizes = compute_font_sizes(entries, area_width, area_height, scale);

    // Sort by font size descending (place largest first)
    let mut sorted_indices: Vec<usize> = (0..entries.len()).collect();
    sorted_indices.sort_by(|a, b| {
        font_sizes[*b]
            .partial_cmp(&font_sizes[*a])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut placed: Vec<WordLayout> = Vec::new();
    // Initialize with zero-size layouts (unplaced words won't be drawn)
    let mut result = vec![
        WordLayout {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            font_size: 0.0,
            rotated: false,
        };
        entries.len()
    ];

    // Ellipse creates a cloud shape floating in the center with margins.
    // Words that don't fit get dropped, giving clean cloud edges.
    let ellipse_a = area_width * 0.44; // horizontal semi-axis (~88% of width)
    let ellipse_b = area_height * 0.44; // vertical semi-axis (~88% of height)

    let ctx = PlaceCtx {
        cx: area_width / 2.0,
        cy: area_height / 2.0,
        area_width,
        area_height,
        ellipse_a,
        ellipse_b,
        scale,
    };

    let total = entries.len();
    for (rank, &orig_idx) in sorted_indices.iter().enumerate() {
        let entry = &entries[orig_idx];
        let fs = font_sizes[orig_idx];
        let prefer_rotated = should_rotate(&entry.text, orig_idx, rank, total);

        if let Some(layout) = try_place_word(ui, theme, &ctx, entry, fs, prefer_rotated, &placed) {
            placed.push(layout.clone());
            result[orig_idx] = layout;
        }
        // Words that can't fit are simply not placed (font_size stays 0)
    }

    fit_layout_to_area(&mut result, area_width, area_height);
    result
}

/// Scale a finished layout about the area center so the cloud fills the
/// available space. Text metrics are linear in font size, so scaling
/// positions, sizes and fonts by the same factor keeps words from overlapping.
fn fit_layout_to_area(layouts: &mut [WordLayout], area_width: f32, area_height: f32) {
    let placed: Vec<&WordLayout> = layouts.iter().filter(|l| l.font_size > 0.0).collect();
    if placed.is_empty() {
        return;
    }
    let min_x = placed.iter().map(|l| l.x).fold(f32::MAX, f32::min);
    let min_y = placed.iter().map(|l| l.y).fold(f32::MAX, f32::min);
    let max_x = placed
        .iter()
        .map(|l| l.x + l.width)
        .fold(f32::MIN, f32::max);
    let max_y = placed
        .iter()
        .map(|l| l.y + l.height)
        .fold(f32::MIN, f32::max);
    let bbox_w = (max_x - min_x).max(1.0);
    let bbox_h = (max_y - min_y).max(1.0);

    // Leave a margin so the cloud doesn't touch the edges of its area.
    let target_w = area_width * 0.94;
    let target_h = area_height * 0.94;
    let factor = (target_w / bbox_w).min(target_h / bbox_h).clamp(1.0, 2.5);
    if factor <= 1.0 {
        return;
    }

    let bbox_cx = (min_x + max_x) / 2.0;
    let bbox_cy = (min_y + max_y) / 2.0;
    let area_cx = area_width / 2.0;
    let area_cy = area_height / 2.0;
    for l in layouts.iter_mut().filter(|l| l.font_size > 0.0) {
        let cx = l.x + l.width / 2.0;
        let cy = l.y + l.height / 2.0;
        l.width *= factor;
        l.height *= factor;
        l.font_size *= factor;
        l.x = area_cx + (cx - bbox_cx) * factor - l.width / 2.0;
        l.y = area_cy + (cy - bbox_cy) * factor - l.height / 2.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::visualizations::VizReveal;

    fn layout(x: f32, y: f32, w: f32, h: f32, fs: f32) -> WordLayout {
        WordLayout {
            x,
            y,
            width: w,
            height: h,
            font_size: fs,
            rotated: false,
        }
    }

    #[test]
    fn test_fit_layout_scales_small_cloud_to_fill_area() {
        // Two words clustered in the middle of a 1000x500 area
        let mut layouts = vec![
            layout(450.0, 240.0, 100.0, 20.0, 20.0),
            layout(460.0, 260.0, 60.0, 10.0, 10.0),
            layout(0.0, 0.0, 0.0, 0.0, 0.0), // unplaced word stays untouched
        ];
        fit_layout_to_area(&mut layouts, 1000.0, 500.0);
        // bbox was 100x30 → limited by the 2.5x cap
        assert!((layouts[0].font_size - 50.0).abs() < 1e-3);
        assert!((layouts[0].width - 250.0).abs() < 1e-3);
        assert_eq!(layouts[2].font_size, 0.0);
        // Cloud stays inside the area and centered
        let cx = layouts[0].x + layouts[0].width / 2.0;
        assert!(cx > 400.0 && cx < 600.0);
        assert!(layouts[0].x >= 0.0 && layouts[1].y >= 0.0);
    }

    #[test]
    fn test_fit_layout_never_shrinks() {
        let mut layouts = vec![layout(10.0, 10.0, 980.0, 480.0, 40.0)];
        fit_layout_to_area(&mut layouts, 1000.0, 500.0);
        assert_eq!(layouts[0].font_size, 40.0);
        assert_eq!(layouts[0].x, 10.0);
    }

    #[test]
    fn test_should_rotate_never_for_top_words() {
        // Top 75% of words by rank should never rotate
        for rank in 0..37 {
            assert!(!should_rotate("Big", rank, rank, 50));
        }
    }

    #[test]
    fn test_should_rotate_never_for_few_words() {
        // Fewer than 12 words → no rotation
        for i in 0..11 {
            assert!(!should_rotate("Word", i, i, 11));
        }
    }

    #[test]
    fn test_should_rotate_deterministic() {
        // Same input should always give the same result
        let r1 = should_rotate("Test", 5, 5, 20);
        let r2 = should_rotate("Test", 5, 5, 20);
        assert_eq!(r1, r2);
    }

    #[test]
    fn test_compute_font_sizes_scales_proportionally() {
        let entries = vec![
            WordEntry {
                text: "Big".to_string(),
                size: 50.0,
                reveal: VizReveal::Static,
            },
            WordEntry {
                text: "Small".to_string(),
                size: 10.0,
                reveal: VizReveal::Static,
            },
        ];
        let sizes = compute_font_sizes(&entries, 1800.0, 900.0, 1.0);
        assert!(
            sizes[0] > sizes[1],
            "Bigger size value should produce bigger font"
        );
        assert!(
            sizes[0] > 100.0,
            "Largest font should be substantial: {}",
            sizes[0]
        );
        assert!(
            sizes[1] > 20.0,
            "Smallest font should be legible: {}",
            sizes[1]
        );
    }
}
