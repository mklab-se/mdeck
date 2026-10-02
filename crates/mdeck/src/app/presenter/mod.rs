//! The presenter's cockpit: the current slide, what comes next, the speaker
//! notes rendered as markdown, and the elapsed time. The window shows it in
//! a second OS window (`V`, `--presenter`); with one display `V` shows the
//! notes overlay instead. Export draws the same layout with the hidden
//! `--presenter-view` flag, so it can be looked at without a window.

use std::time::{Duration, Instant};

use eframe::egui::{self, FontId, Pos2, Rect, Stroke, pos2, vec2};

use crate::deck::{Deck, SlideFrame};
use crate::render;
use crate::theme::Theme;

mod notes;
pub use notes::{draw_notes, draw_overlay, notes_blocks};

/// The cockpit's own colours: a quiet dark desk, whatever the deck's theme.
mod ink {
    use eframe::egui::Color32;
    pub const DESK: Color32 = Color32::from_rgb(0x0E, 0x0F, 0x12);
    pub const PANEL: Color32 = Color32::from_rgb(0x17, 0x19, 0x1E);
    pub const HAIRLINE: Color32 = Color32::from_rgb(0x2A, 0x2D, 0x35);
    pub const TEXT: Color32 = Color32::from_rgb(0xE8, 0xE9, 0xED);
    pub const MUTED: Color32 = Color32::from_rgb(0x8B, 0x8F, 0x9A);
    pub const ACCENT: Color32 = Color32::from_rgb(0xFF, 0xB4, 0x54);
}

/// Where everything goes in a presenter window of a given size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layout {
    /// One unit: the window's size against a 1600x1000 reference.
    pub unit: f32,
    /// Deck title and progress.
    pub header: Rect,
    /// The current slide, large.
    pub current: Rect,
    /// Timer and position, under the current slide.
    pub status: Rect,
    /// The next slide or step, smaller.
    pub next: Rect,
    /// The notes, filling the rest of the right column.
    pub notes: Rect,
}

/// Lay the cockpit out in `rect`: the current slide takes about three
/// fifths of the width, the next slide and the notes share the rest.
pub fn layout(rect: Rect) -> Layout {
    let unit = (rect.width() / 1600.0).min(rect.height() / 1000.0).max(0.1);
    let m = 32.0 * unit;
    let label = 30.0 * unit;
    let header = Rect::from_min_size(
        rect.min + vec2(m, m * 0.6),
        vec2(rect.width() - 2.0 * m, 40.0 * unit),
    );
    let top = header.bottom() + m * 0.8;
    let inner_w = rect.width() - 3.0 * m;
    let status_h = 150.0 * unit;
    // the current slide as wide as three fifths allow, but never so tall
    // that the timer under it falls off the window
    let max_h = (rect.bottom() - m - status_h - label - top).max(10.0);
    let cur_w = (inner_w * 0.6).min(max_h * 16.0 / 9.0);
    let current = Rect::from_min_size(
        pos2(rect.left() + m, top + label),
        vec2(cur_w, cur_w * 9.0 / 16.0),
    );
    let status = Rect::from_min_max(
        pos2(current.left(), current.bottom() + m * 0.6),
        pos2(current.right(), rect.bottom() - m),
    );
    let right_x = current.right() + m;
    let right_w = (rect.right() - m - right_x).max(10.0);
    let next = Rect::from_min_size(
        pos2(right_x, top + label),
        vec2(right_w, right_w * 9.0 / 16.0),
    );
    let notes = Rect::from_min_max(
        pos2(right_x, next.bottom() + m * 0.6 + label),
        pos2(right_x + right_w, rect.bottom() - m),
    );
    Layout {
        unit,
        header,
        current,
        status,
        next,
        notes,
    }
}

/// What comes after `reveal` on slide `index`: the next step of the same
/// slide, the next slide from its start, or the end of the deck.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Upcoming {
    Step { index: usize, reveal: usize },
    Slide { index: usize },
    End,
}

pub fn upcoming(index: usize, reveal: usize, max_steps: &[usize]) -> Upcoming {
    let max = max_steps.get(index).copied().unwrap_or(0);
    if reveal < max {
        Upcoming::Step {
            index,
            reveal: reveal + 1,
        }
    } else if index + 1 < max_steps.len() {
        Upcoming::Slide { index: index + 1 }
    } else {
        Upcoming::End
    }
}

/// Elapsed time as `m:ss`, or `h:mm:ss` from the first hour.
pub fn format_elapsed(d: Duration) -> String {
    let s = d.as_secs();
    let (h, m, s) = (s / 3600, (s / 60) % 60, s % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

/// Everything the cockpit shows for one moment of the talk.
pub struct View<'a> {
    pub deck: &'a Deck,
    pub theme: &'a Theme,
    pub index: usize,
    pub reveal: usize,
    /// On the end slide, past the last one.
    pub end: bool,
    pub elapsed: Duration,
}

/// Paint the presenter view over `rect`.
pub fn draw(ui: &mut egui::Ui, rect: Rect, view: &View) {
    // Slides drawn here must not feed the engine's geometry hints.
    let hints = render::hints::enabled(ui.ctx());
    render::hints::set_enabled(ui.ctx(), false);
    paint(ui, rect, view);
    render::hints::set_enabled(ui.ctx(), hints);
}

fn paint(ui: &mut egui::Ui, rect: Rect, view: &View) {
    let l = layout(rect);
    let u = l.unit;
    let painter = &ui.painter().clone();
    painter.rect_filled(rect, 0.0, ink::DESK);
    let count = view.deck.slide_count();

    // Header: the deck's title and a hairline of progress through it.
    let title = view
        .deck
        .presentation
        .meta
        .title
        .clone()
        .unwrap_or_else(|| {
            view.deck
                .file
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        });
    painter.text(
        l.header.left_center(),
        egui::Align2::LEFT_CENTER,
        title,
        FontId::proportional(22.0 * u),
        ink::MUTED,
    );
    painter.text(
        l.header.right_center(),
        egui::Align2::RIGHT_CENTER,
        "PRESENTER",
        FontId::monospace(14.0 * u),
        ink::HAIRLINE.gamma_multiply(2.2),
    );
    let bar_y = l.header.bottom() + 6.0 * u;
    painter.hline(
        l.header.x_range(),
        bar_y,
        Stroke::new(2.0 * u, ink::HAIRLINE),
    );
    let done = if view.end {
        1.0
    } else {
        (view.index + 1) as f32 / count.max(1) as f32
    };
    painter.hline(
        l.header.left()..=l.header.left() + l.header.width() * done,
        bar_y,
        Stroke::new(2.0 * u, ink::ACCENT),
    );

    // Now
    label(painter, l.current, u, "NOW", None);
    if view.end {
        end_card(painter, l.current, u);
    } else {
        slide(ui, view, view.index, view.reveal, l.current);
    }

    // Next
    match (
        view.end,
        upcoming(view.index, view.reveal, &view.deck.max_steps),
    ) {
        (true, _) | (false, Upcoming::End) => {
            label(painter, l.next, u, "NEXT", Some("end of deck".into()));
            end_card(painter, l.next, u);
        }
        (false, Upcoming::Step { index, reveal }) => {
            let max = view.deck.max_steps.get(index).copied().unwrap_or(0);
            label(
                painter,
                l.next,
                u,
                "NEXT STEP",
                Some(format!("{reveal} of {max}")),
            );
            slide(ui, view, index, reveal, l.next);
        }
        (false, Upcoming::Slide { index }) => {
            label(
                painter,
                l.next,
                u,
                "NEXT SLIDE",
                Some(format!("{} of {count}", index + 1)),
            );
            slide(ui, view, index, 0, l.next);
        }
    }

    // Status: the elapsed time, large, and where we are.
    let timer = format_elapsed(view.elapsed);
    let t = painter.text(
        l.status.left_top() + vec2(0.0, 8.0 * u),
        egui::Align2::LEFT_TOP,
        timer,
        FontId::monospace(76.0 * u),
        ink::TEXT,
    );
    painter.text(
        pos2(t.right() + 18.0 * u, t.bottom() - 12.0 * u),
        egui::Align2::LEFT_BOTTOM,
        "elapsed  ·  Shift+V resets",
        FontId::proportional(16.0 * u),
        ink::MUTED,
    );
    let max = view.deck.max_steps.get(view.index).copied().unwrap_or(0);
    let position = if view.end {
        format!("End  ·  {count} slides")
    } else if max > 0 {
        format!(
            "Slide {} of {count}  ·  step {} of {max}",
            view.index + 1,
            view.reveal
        )
    } else {
        format!("Slide {} of {count}", view.index + 1)
    };
    painter.text(
        l.status.right_top() + vec2(0.0, 8.0 * u),
        egui::Align2::RIGHT_TOP,
        position,
        FontId::proportional(24.0 * u),
        ink::TEXT,
    );

    // Notes
    label(painter, l.notes, u, "NOTES", None);
    let notes = if view.end {
        Vec::new()
    } else {
        notes_blocks(
            view.deck
                .presentation
                .slides
                .get(view.index)
                .and_then(|s| s.notes.as_deref()),
        )
    };
    painter.rect_filled(l.notes, 10.0 * u, ink::PANEL);
    draw_notes(ui, l.notes.shrink(22.0 * u), &notes, 26.0 * u);
}

/// A small spaced label above a panel, with an optional detail at its right.
fn label(painter: &egui::Painter, panel: Rect, u: f32, text: &str, detail: Option<String>) {
    let y = panel.top() - 12.0 * u;
    painter.text(
        pos2(panel.left(), y),
        egui::Align2::LEFT_BOTTOM,
        spaced(text),
        FontId::monospace(14.0 * u),
        ink::ACCENT,
    );
    if let Some(d) = detail {
        painter.text(
            pos2(panel.right(), y),
            egui::Align2::RIGHT_BOTTOM,
            d,
            FontId::proportional(16.0 * u),
            ink::MUTED,
        );
    }
}

/// Letter-spaced capitals for labels.
fn spaced(s: &str) -> String {
    s.chars()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join("\u{2009}")
}

/// The end of the deck, as a quiet card.
fn end_card(painter: &egui::Painter, r: Rect, u: f32) {
    painter.rect_filled(r, 8.0 * u, ink::PANEL);
    painter.rect_stroke(
        r,
        8.0 * u,
        Stroke::new(1.0, ink::HAIRLINE),
        egui::StrokeKind::Inside,
    );
    painter.text(
        r.center(),
        egui::Align2::CENTER_CENTER,
        "The End",
        FontId::proportional(r.height() * 0.14),
        ink::MUTED,
    );
}

/// Slide `index` at step `reveal`, settled, in `r` (clipped to it).
fn slide(ui: &mut egui::Ui, view: &View, index: usize, reveal: usize, r: Rect) {
    let deck = view.deck;
    let theme = view.theme;
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r).id_salt((
        "presenter-slide",
        index,
        reveal,
        r.min.x as i32,
    )));
    child.shrink_clip_rect(r);
    let painter = child.painter();
    painter.rect_filled(r, 0.0, theme.background);
    let scale = (r.width() / 1920.0).min(r.height() / 1080.0);
    let sheet = render::page::draw(painter, r, theme, scale);
    let scale = (sheet.width() / 1920.0).min(sheet.height() / 1080.0);
    deck.draw_background(painter, sheet, index, 1.0, 0.0, false);
    let cx = render::SlideContext {
        index,
        count: deck.slide_count(),
        deck_title: deck.presentation.meta.title.clone(),
        author: deck.presentation.meta.author.clone(),
        hold_copy: false,
        animate: false,
        engine_drew: false,
    };
    let frame = SlideFrame {
        rect: sheet,
        opacity: 1.0,
        reveal,
        reveal_timestamp: None,
        scale,
    };
    deck.draw_slide(&child, theme, index, frame, &cx);
    deck.draw_logo(painter, sheet, index, scale);
    deck.draw_chrome(painter, theme, sheet, &cx, scale);
    // a hairline frame keeps a dark slide apart from the desk
    ui.painter().rect_stroke(
        r,
        0.0,
        Stroke::new(1.0, ink::HAIRLINE),
        egui::StrokeKind::Outside,
    );
}

/// The presenter window and the notes overlay, as the window holds them.
pub(super) struct Presenter {
    /// The second window is open (or opening).
    pub(super) window: bool,
    /// The notes overlay is shown over the slides.
    pub(super) overlay: bool,
    /// The elapsed time counts from here (`Shift+V` resets it).
    pub(super) since: Instant,
    /// Opening: where the presenter window was asked to go and when, and
    /// the slide window's monitor (left edge and width) to tell displays
    /// apart once it has settled.
    pub(super) placing: Option<Placing>,
    /// `--presenter`: open on the first frame.
    pub(super) open_at_start: bool,
    /// Frames waited for the display to be known (`--presenter`).
    pub(super) start_frames: u32,
    /// Where the presenter window opens.
    pub(super) target: Pos2,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Placing {
    pub(super) since: Instant,
    pub(super) main_x: f32,
    pub(super) monitor_width: f32,
}

impl Presenter {
    pub(super) fn new(open_at_start: bool) -> Self {
        Self {
            window: false,
            overlay: false,
            since: Instant::now(),
            placing: None,
            open_at_start,
            start_frames: 0,
            target: Pos2::ZERO,
        }
    }

    pub(super) fn elapsed(&self) -> Duration {
        self.since.elapsed()
    }

    /// How long a new presenter window gets to land before we look where.
    pub(super) const SETTLE: Duration = Duration::from_millis(900);
}

/// Where to open the presenter window: on the display beside the slides.
/// When the slides run on a display right of the origin, the presenter
/// goes to the primary display; otherwise one display to the right.
pub fn presenter_target(main_pos: Pos2, monitor_width: f32) -> Pos2 {
    if main_pos.x.abs() >= monitor_width / 2.0 {
        pos2(40.0, 40.0)
    } else {
        pos2(main_pos.x + monitor_width + 40.0, main_pos.y + 40.0)
    }
}

/// Whether a window at `x` sits on a different display than the slide
/// window's monitor (left edge `main_x`, `width` wide). A window the OS
/// pulled back onto the slides' display means there is only one.
pub fn on_other_display(main_x: f32, width: f32, x: f32) -> bool {
    x < main_x - 1.0 || x >= main_x + width - 1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_gives_the_current_slide_the_most_room() {
        for (w, h) in [
            (1600.0, 1000.0),
            (1280.0, 800.0),
            (1920.0, 1080.0),
            (1024.0, 768.0),
        ] {
            let r = Rect::from_min_size(Pos2::ZERO, vec2(w, h));
            let l = layout(r);
            assert!(l.current.width() > l.next.width(), "{w}x{h}");
            assert!((l.current.aspect_ratio() - 16.0 / 9.0).abs() < 0.01);
            assert!((l.next.aspect_ratio() - 16.0 / 9.0).abs() < 0.01);
            // nothing overlaps and everything stays inside the window
            assert!(!l.current.intersects(l.next));
            assert!(!l.current.intersects(l.notes));
            assert!(!l.status.intersects(l.next));
            assert!(l.notes.top() > l.next.bottom());
            for p in [l.current, l.next, l.notes, l.status, l.header] {
                assert!(r.contains_rect(p), "{p:?} outside {w}x{h}");
            }
            assert!(l.notes.height() > h * 0.3, "notes too small at {w}x{h}");
            assert!(l.status.height() > 100.0 * l.unit);
        }
    }

    #[test]
    fn upcoming_is_the_next_step_then_the_next_slide_then_the_end() {
        let steps = [2, 0, 1];
        assert_eq!(
            upcoming(0, 0, &steps),
            Upcoming::Step {
                index: 0,
                reveal: 1
            }
        );
        assert_eq!(upcoming(0, 2, &steps), Upcoming::Slide { index: 1 });
        assert_eq!(upcoming(1, 0, &steps), Upcoming::Slide { index: 2 });
        assert_eq!(
            upcoming(2, 0, &steps),
            Upcoming::Step {
                index: 2,
                reveal: 1
            }
        );
        assert_eq!(upcoming(2, 1, &steps), Upcoming::End);
    }

    #[test]
    fn elapsed_reads_as_a_clock() {
        assert_eq!(format_elapsed(Duration::from_secs(0)), "0:00");
        assert_eq!(format_elapsed(Duration::from_secs(754)), "12:34");
        assert_eq!(format_elapsed(Duration::from_secs(3723)), "1:02:03");
    }

    #[test]
    fn the_presenter_goes_to_the_other_display() {
        // slides on the primary display: presenter one display to the right
        assert_eq!(presenter_target(pos2(0.0, 0.0), 1920.0), pos2(1960.0, 40.0));
        // slides on a display to the right: presenter on the primary one
        assert_eq!(
            presenter_target(pos2(1920.0, 0.0), 1920.0),
            pos2(40.0, 40.0)
        );
        // where it landed decides whether there was a second display
        assert!(on_other_display(0.0, 1920.0, 1960.0));
        assert!(on_other_display(1920.0, 1920.0, 40.0));
        assert!(!on_other_display(0.0, 1920.0, 600.0));
        assert!(!on_other_display(0.0, 1920.0, 0.0));
    }
}
