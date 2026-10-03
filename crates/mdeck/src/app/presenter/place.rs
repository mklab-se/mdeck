//! Where the windows go: the displays as the system lists them, the one
//! the slides are on, the display the presenter window takes, the next one
//! for `M`, and the side-by-side arrangement on a single display.

use eframe::egui::{Pos2, Rect, Vec2, pos2, vec2};

/// A connected display, in the window system's points (egui's coordinates).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Display {
    pub rect: Rect,
    /// The computer's own screen: where a presenter sits.
    pub builtin: bool,
}

/// The connected displays, left to right then top to bottom; empty when
/// the system cannot say.
pub fn displays() -> Vec<Display> {
    let Ok(all) = display_info::DisplayInfo::all() else {
        return Vec::new();
    };
    let mut out: Vec<Display> = all
        .iter()
        .map(|d| {
            // macOS lists points already; elsewhere the list is in pixels
            let k = if cfg!(target_os = "macos") || d.scale_factor <= 0.0 {
                1.0
            } else {
                d.scale_factor
            };
            Display {
                rect: Rect::from_min_size(
                    pos2(d.x as f32 / k, d.y as f32 / k),
                    vec2(d.width as f32 / k, d.height as f32 / k),
                ),
                builtin: d.is_builtin,
            }
        })
        .collect();
    out.sort_by(|a, b| {
        (a.rect.left(), a.rect.top())
            .partial_cmp(&(b.rect.left(), b.rect.top()))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

/// The display a window at `at` (its top-left corner) is on: the one
/// holding the point just inside the corner, else the nearest.
pub fn display_of(displays: &[Display], at: Pos2) -> Option<usize> {
    let p = at + vec2(8.0, 8.0);
    displays
        .iter()
        .position(|d| d.rect.contains(p))
        .or_else(|| {
            (0..displays.len()).min_by(|&a, &b| {
                let da = displays[a].rect.distance_sq_to_pos(p);
                let db = displays[b].rect.distance_sq_to_pos(p);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
        })
}

/// The display for the presenter window when the slides are on `own`: the
/// computer's own screen if the slides are elsewhere, else the first other
/// one; `None` with a single display.
pub fn presenter_display(displays: &[Display], own: usize) -> Option<usize> {
    let others = (0..displays.len()).filter(|&i| i != own);
    others
        .clone()
        .find(|&i| displays[i].builtin)
        .or_else(|| others.clone().next())
}

/// The display after `own` for `M`, wrapping around; `None` with one.
pub fn next_display(displays: &[Display], own: usize) -> Option<usize> {
    (displays.len() > 1).then(|| (own + 1) % displays.len())
}

/// Two windows on one display: the slides (16:9, the larger) on the left
/// and the presenter window filling the rest, both clear of the menu bar
/// and the dock. Returns each window's outer top-left and inner size.
pub fn side_by_side(display: Rect) -> SideBySide {
    // menu bar above, dock below, a margin around and between
    const TOP: f32 = 40.0;
    const BOTTOM: f32 = 90.0;
    const MARGIN: f32 = 24.0;
    const GAP: f32 = 16.0;
    // a window's title bar, above its inner size
    const TITLE: f32 = 28.0;
    let area = Rect::from_min_max(
        pos2(display.left() + MARGIN, display.top() + TOP),
        pos2(display.right() - MARGIN, display.bottom() - BOTTOM),
    );
    let mut slides_w = (area.width() - GAP) * 0.6;
    let mut slides_h = slides_w * 9.0 / 16.0;
    if slides_h + TITLE > area.height() {
        slides_h = area.height() - TITLE;
        slides_w = slides_h * 16.0 / 9.0;
    }
    let presenter_x = area.left() + slides_w + GAP;
    SideBySide {
        slides: (area.left_top(), vec2(slides_w, slides_h)),
        presenter: (
            pos2(presenter_x, area.top()),
            vec2(area.right() - presenter_x, area.height() - TITLE),
        ),
    }
}

/// [`side_by_side`]'s result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SideBySide {
    pub slides: (Pos2, Vec2),
    pub presenter: (Pos2, Vec2),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn display(x: f32, y: f32, w: f32, h: f32, builtin: bool) -> Display {
        Display {
            rect: Rect::from_min_size(pos2(x, y), vec2(w, h)),
            builtin,
        }
    }

    /// A laptop with a projector to its left, of another size: the case a
    /// guess of "one screen to the right" got wrong.
    fn laptop_and_projector() -> Vec<Display> {
        vec![
            display(-1920.0, 0.0, 1920.0, 1080.0, false),
            display(0.0, 0.0, 1512.0, 982.0, true),
        ]
    }

    #[test]
    fn the_presenter_takes_the_laptop_screen_when_the_slides_are_elsewhere() {
        let d = laptop_and_projector();
        let own = display_of(&d, pos2(-1920.0, 0.0)).unwrap();
        assert_eq!(own, 0);
        assert_eq!(presenter_display(&d, own), Some(1));
        // slides on the laptop: the presenter goes to the other display
        let own = display_of(&d, pos2(0.0, 0.0)).unwrap();
        assert_eq!(presenter_display(&d, own), Some(0));
    }

    #[test]
    fn displays_above_and_below_count_too() {
        let d = vec![
            display(0.0, -1080.0, 1920.0, 1080.0, false),
            display(0.0, 0.0, 1728.0, 1117.0, true),
        ];
        assert_eq!(display_of(&d, pos2(100.0, -1000.0)), Some(0));
        assert_eq!(presenter_display(&d, 0), Some(1));
        assert_eq!(next_display(&d, 1), Some(0));
    }

    #[test]
    fn one_display_has_nowhere_else() {
        let d = vec![display(0.0, 0.0, 2560.0, 1440.0, false)];
        assert_eq!(presenter_display(&d, 0), None);
        assert_eq!(next_display(&d, 0), None);
        assert_eq!(display_of(&[], pos2(0.0, 0.0)), None);
    }

    #[test]
    fn a_window_off_every_display_belongs_to_the_nearest() {
        let d = laptop_and_projector();
        assert_eq!(display_of(&d, pos2(3000.0, 10.0)), Some(1));
    }

    #[test]
    fn m_steps_through_every_display_and_wraps() {
        let d = vec![
            display(0.0, 0.0, 100.0, 100.0, true),
            display(100.0, 0.0, 100.0, 100.0, false),
            display(200.0, 0.0, 100.0, 100.0, false),
        ];
        assert_eq!(next_display(&d, 0), Some(1));
        assert_eq!(next_display(&d, 2), Some(0));
    }

    #[test]
    fn side_by_side_fits_both_windows_on_the_display() {
        for (w, h) in [
            (2560.0, 1440.0),
            (1512.0, 982.0),
            (1920.0, 1080.0),
            (1280.0, 1024.0),
        ] {
            let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(w, h));
            let s = side_by_side(screen);
            let (sp, ss) = s.slides;
            let (pp, ps) = s.presenter;
            assert!((ss.x / ss.y - 16.0 / 9.0).abs() < 0.01, "{w}x{h}");
            assert!(ss.x > ps.x, "the slides are the larger window: {w}x{h}");
            assert!(pp.x > sp.x + ss.x, "side by side: {w}x{h}");
            assert!(pp.x + ps.x <= w && sp.y + ss.y < h, "{w}x{h}");
            assert!(ps.x > 300.0 && ps.y > 300.0, "{w}x{h}");
        }
    }
}
