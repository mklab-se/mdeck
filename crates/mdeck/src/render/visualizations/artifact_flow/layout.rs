//! Geometry of an artifact flow: three columns with room for the edge
//! labels between them, cards stacked in each, and where edges meet a card.

use eframe::egui::{Pos2, Rect, Vec2};

/// Width of the producer and consumer columns, the service column and the
/// gaps the edges cross, relative to each other.
const SIDE: f32 = 0.26;
const MIDDLE: f32 = 0.22;
const GAP: f32 = 0.13;
/// A flow with fewer columns may grow this much before it centres instead.
const MAX_GROW: f32 = 1.25;

/// The rects of the columns that have nodes (by column index), spread over
/// `rect` with a gap between neighbours.
pub fn columns(rect: Rect, present: [bool; 3]) -> [Option<Rect>; 3] {
    let shown: Vec<usize> = (0..3).filter(|&c| present[c]).collect();
    let mut out = [None; 3];
    if shown.is_empty() {
        return out;
    }
    let width = |c: usize| if c == 1 { MIDDLE } else { SIDE };
    let total: f32 = shown.iter().map(|&c| width(c)).sum::<f32>() + GAP * (shown.len() - 1) as f32;
    let k = (1.0 / total).min(MAX_GROW) * rect.width();
    let mut x = rect.center().x - total * k / 2.0;
    for (i, &c) in shown.iter().enumerate() {
        if i > 0 {
            x += GAP * k;
        }
        let w = width(c) * k;
        out[c] = Some(Rect::from_min_size(
            Pos2::new(x, rect.top()),
            Vec2::new(w, rect.height()),
        ));
        x += w;
    }
    out
}

/// Cards of `heights` stacked `gap` apart, centred vertically in `area`,
/// as wide as it.
pub fn stack(area: Rect, heights: &[f32], gap: f32) -> Vec<Rect> {
    let total: f32 = heights.iter().sum::<f32>() + gap * heights.len().saturating_sub(1) as f32;
    let mut y = area.center().y - total / 2.0;
    heights
        .iter()
        .map(|&h| {
            let r = Rect::from_min_size(Pos2::new(area.left(), y), Vec2::new(area.width(), h));
            y += h + gap;
            r
        })
        .collect()
}

/// Where `n` edges meet one side of a card at `x`, spread over the middle
/// of `top..bottom` (one edge meets the middle).
pub fn ports(x: f32, top: f32, bottom: f32, n: usize) -> Vec<Pos2> {
    let h = bottom - top;
    let (from, span) = if n <= 1 {
        (0.5, 0.0)
    } else {
        // spread wider the more there are, up to the middle 70%
        let span = (0.2 + 0.2 * (n - 1) as f32).min(0.7);
        (0.5 - span / 2.0, span)
    };
    (0..n)
        .map(|i| {
            let t = if n <= 1 {
                from
            } else {
                from + span * i as f32 / (n - 1) as f32
            };
            Pos2::new(x, top + h * t)
        })
        .collect()
}

/// Push `rects` (sorted top to bottom) apart by moving each one up until
/// it clears the one below it by `gap` (labels sit above their own line, so
/// up keeps them clear of it), then keep the group inside `within`.
pub fn separate_up(rects: &mut [Rect], gap: f32, within: Rect) {
    for i in (0..rects.len().saturating_sub(1)).rev() {
        let max_bottom = rects[i + 1].top() - gap;
        if rects[i].bottom() > max_bottom {
            let dy = rects[i].bottom() - max_bottom;
            rects[i] = rects[i].translate(Vec2::new(0.0, -dy));
        }
    }
    if let Some(first) = rects.first()
        && first.top() < within.top()
    {
        let dy = within.top() - first.top();
        for r in rects.iter_mut() {
            *r = r.translate(Vec2::new(0.0, dy));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect() -> Rect {
        Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 500.0))
    }

    #[test]
    fn three_columns_fill_the_width_left_to_right() {
        let [p, s, c] = columns(rect(), [true; 3]);
        let (p, s, c) = (p.unwrap(), s.unwrap(), c.unwrap());
        assert!((p.left() - 0.0).abs() < 1e-3 && (c.right() - 1000.0).abs() < 1e-3);
        assert!((p.width() - 260.0).abs() < 1e-3 && (s.width() - 220.0).abs() < 1e-3);
        assert!((s.left() - p.right() - 130.0).abs() < 1e-3);
        assert!(s.center().x == 500.0);
    }

    #[test]
    fn two_columns_grow_a_little_and_centre() {
        let [p, s, c] = columns(rect(), [true, false, true]);
        assert!(s.is_none());
        let (p, c) = (p.unwrap(), c.unwrap());
        assert!((p.width() - 260.0 * 1.25).abs() < 1e-3);
        assert!(((p.left() + c.right()) / 2.0 - 500.0).abs() < 1e-3);
        assert_eq!(columns(rect(), [false; 3]), [None; 3]);
    }

    #[test]
    fn cards_stack_centred() {
        let area = Rect::from_min_size(Pos2::new(0.0, 100.0), Vec2::new(200.0, 400.0));
        let cards = stack(area, &[100.0, 50.0], 20.0);
        assert_eq!(cards[0].top(), 100.0 + (400.0 - 170.0) / 2.0);
        assert_eq!(cards[1].top(), cards[0].bottom() + 20.0);
    }

    #[test]
    fn ports_spread_from_the_middle() {
        assert_eq!(ports(5.0, 0.0, 100.0, 1), vec![Pos2::new(5.0, 50.0)]);
        let two = ports(5.0, 0.0, 100.0, 2);
        assert!((two[0].y - 30.0).abs() < 1e-3 && (two[1].y - 70.0).abs() < 1e-3);
        let many = ports(5.0, 0.0, 100.0, 9);
        assert!((many[0].y - 15.0).abs() < 1e-3 && (many[8].y - 85.0).abs() < 1e-3);
        assert!(ports(5.0, 0.0, 100.0, 0).is_empty());
    }

    #[test]
    fn labels_are_pushed_up_apart_and_kept_inside() {
        let r = |y: f32| Rect::from_min_size(Pos2::new(0.0, y), Vec2::new(10.0, 20.0));
        let mut rects = [r(100.0), r(110.0), r(300.0)];
        separate_up(&mut rects, 4.0, rect());
        assert_eq!(rects[1].top(), 110.0, "the lowest of a pair stays");
        assert_eq!(rects[0].bottom(), 106.0);
        assert_eq!(rects[2].top(), 300.0);
        let mut high = [r(0.0), r(5.0)];
        separate_up(&mut high, 4.0, rect());
        assert_eq!(high[0].top(), 0.0);
        assert_eq!(high[1].top(), 24.0);
    }
}
