//! Ember layouts: the MKLab site's composition for text slides.
//!
//! Copy sits in a narrow column on the left, over a soft dark pillow, with a
//! tracked monospace eyebrow, an editorial serif heading and a light-weight
//! lead. Elements fade up in a stagger when the slide is entered, and list
//! items fade up as they reveal. Slides whose content fills the frame (code,
//! charts, diagrams, images, tables) keep their regular layouts and only
//! inherit the palette and fonts.

mod chrome;
mod jobs;
mod pieces;
mod pillow;
mod quote;
mod slides;

use std::time::Instant;

use eframe::egui::{self, Color32, Pos2, Rect};

use crate::parser::{Block, Layout, Slide};
use crate::render::{BlockCx, SlideContext};
use crate::theme::Theme;

pub use chrome::{draw_chrome, draw_say_line};
pub use jobs::display_job;

/// The first slide of a deck reads as its title page when it opens with an
/// H1, whatever the generic layout inference decided about its paragraph.
pub fn is_title(slide: &Slide, index: usize) -> bool {
    slide.layout == Layout::Title
        || (index == 0
            && matches!(slide.blocks.first(), Some(Block::Heading { level: 1, .. }))
            && !slide.blocks.iter().any(|b| {
                matches!(
                    b,
                    Block::List { .. } | Block::CodeBlock { .. } | Block::Image { .. }
                )
            }))
}

/// Whether Ember draws this slide itself (otherwise the regular layout runs).
pub fn handles(slide: &Slide) -> bool {
    match slide.layout {
        Layout::Title | Layout::Section | Layout::Quote => true,
        Layout::Bullet | Layout::Content => !slide.blocks.iter().any(|b| {
            matches!(
                b,
                Block::Image { .. }
                    | Block::CodeBlock { .. }
                    | Block::Table { .. }
                    | Block::Diagram { .. }
            )
        }),
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Palette (Ember tokens)
// ---------------------------------------------------------------------------

fn fade(c: Color32, a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (a.clamp(0.0, 1.0) * 255.0) as u8)
}

fn ease_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

// ---------------------------------------------------------------------------
// Entry animation state (kept in egui's temp data, keyed by slide index)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Entry {
    entered_at: f64,
    /// The frame the slide was last drawn in.
    last_frame: u64,
}

/// Seconds since this slide was (re)entered. A slide that was not drawn in
/// the previous frame counts as re-entered, so navigating back replays the
/// stagger just like the site does. Frames, not seconds: an idle window
/// can go seconds between frames without the slide ever leaving the screen.
fn entry_age(ui: &egui::Ui, index: usize, animate: bool, hold: bool) -> f32 {
    if !animate {
        return 10.0;
    }
    let now = ui.input(|i| i.time);
    let frame = ui.ctx().cumulative_frame_nr();
    let id = egui::Id::new(("ember-entry", index));
    let age = ui.ctx().data_mut(|d| {
        let e = d.get_temp_mut_or_insert_with(id, || Entry {
            entered_at: now,
            last_frame: frame,
        });
        // While the intro holds the copy back, the clock keeps restarting so
        // the stagger begins the moment the logo dissolves.
        if hold || frame > e.last_frame + 1 {
            e.entered_at = now;
        }
        e.last_frame = frame;
        (now - e.entered_at) as f32
    });
    if age < 2.5 {
        ui.ctx().request_repaint();
    }
    age
}

/// Fade/rise progress of the `nth` copy element, staggered 120 ms apart
/// over a 650 ms rise, as on the site.
fn stagger(age: f32, nth: usize) -> f32 {
    ease_out((age - 0.12 * nth as f32) / 0.65)
}

// ---------------------------------------------------------------------------
// Geometry
// ---------------------------------------------------------------------------

/// Left copy column: 7% in from the edge, 44% of the width.
fn copy_column(rect: Rect) -> Rect {
    let left = rect.left() + rect.width() * 0.07;
    let width = rect.width() * 0.44;
    Rect::from_min_size(
        Pos2::new(left, rect.top()),
        egui::vec2(width, rect.height()),
    )
}

fn roman(n: usize) -> String {
    const TABLE: [(usize, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut n = n;
    let mut out = String::new();
    for (v, s) in TABLE {
        while n >= v {
            out.push_str(s);
            n -= v;
        }
    }
    out
}

/// Eyebrow text for content slides: "II · Deck title".
fn slide_eyebrow(cx: &SlideContext) -> (String, String) {
    let numeral = roman(cx.index.max(1));
    let rest = cx
        .deck_title
        .clone()
        .or_else(|| cx.author.clone())
        .unwrap_or_default();
    if rest.is_empty() {
        (numeral, String::new())
    } else {
        (numeral, format!(" · {rest}"))
    }
}

struct Sizes {
    eyebrow: f32,
    h1: f32,
    h2: f32,
    lead: f32,
    quote: f32,
}

fn sizes(theme: &Theme, scale: f32) -> Sizes {
    Sizes {
        eyebrow: 17.0 * scale,
        h1: theme.h1_size * 1.08 * scale,
        h2: theme.h2_size * scale,
        lead: (theme.body_size - 4.0) * scale,
        quote: theme.h2_size * 0.82 * scale,
    }
}

// ---------------------------------------------------------------------------
// Slide renderers
// ---------------------------------------------------------------------------

/// What an Ember slide renderer draws with.
struct Frame<'a> {
    ui: &'a egui::Ui,
    theme: &'a Theme,
    rect: Rect,
    opacity: f32,
    scale: f32,
    /// Seconds since the slide was entered; negative while the intro holds
    /// the copy back.
    age: f32,
    sz: Sizes,
    deck: &'a SlideContext,
    reveal_step: usize,
    reveal_timestamp: Option<Instant>,
}

impl Frame<'_> {
    /// Opacity of the pillow as the slide is entered.
    fn pillow_alpha(&self) -> f32 {
        self.opacity * ease_out(self.age / 0.9)
    }

    /// Draw `galley` at `pos` as the `nth` element of the entry stagger:
    /// faded in and risen into place. Returns its progress.
    fn place(&self, galley: std::sync::Arc<egui::Galley>, pos: Pos2, nth: usize) -> f32 {
        let p = stagger(self.age, nth);
        crate::render::math::galley_faded(
            self.ui.painter(),
            Pos2::new(pos.x, pos.y + (1.0 - p) * 14.0 * self.scale),
            galley,
            self.opacity * p,
        );
        p
    }
}

/// Render an Ember-handled slide. Caller guarantees [`handles`] is true.
pub fn render(cx: &BlockCx, slide: &Slide, rect: Rect, deck: &SlideContext) {
    let age = if deck.hold_copy {
        entry_age(cx.ui, deck.index, deck.animate, true);
        -1.0
    } else {
        entry_age(cx.ui, deck.index, deck.animate, false)
    };
    let f = Frame {
        ui: cx.ui,
        theme: cx.theme,
        rect,
        opacity: cx.opacity,
        scale: cx.scale,
        age,
        sz: sizes(cx.theme, cx.scale),
        deck,
        reveal_step: cx.reveal_step,
        reveal_timestamp: cx.reveal_timestamp,
    };
    if is_title(slide, deck.index) {
        slides::render_title(&f, slide);
        return;
    }
    match slide.layout {
        Layout::Title => slides::render_title(&f, slide),
        Layout::Section => slides::render_section(&f, slide),
        Layout::Quote => quote::render_quote(&f, slide),
        _ => slides::render_copy(&f, slide),
    }
}

/// Height of the copy stack, for the app's overflow measurement.
pub fn measure_content_height(
    ui: &egui::Ui,
    slide: &Slide,
    theme: &Theme,
    rect: Rect,
    scale: f32,
) -> f32 {
    if !matches!(slide.layout, Layout::Bullet | Layout::Content) {
        return 0.0;
    }
    let column = copy_column(rect);
    let sz = sizes(theme, scale);
    let pieces = pieces::content_pieces(ui, slide, theme, &sz, column.width(), scale);
    pieces::full_height(&pieces) + 60.0 * scale
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roman_numerals() {
        assert_eq!(roman(1), "I");
        assert_eq!(roman(4), "IV");
        assert_eq!(roman(9), "IX");
        assert_eq!(roman(14), "XIV");
        assert_eq!(roman(40), "XL");
    }

    #[test]
    fn slide_eyebrow_prefers_the_deck_title_over_the_author() {
        let mut deck = SlideContext {
            index: 2,
            author: Some("Ada".into()),
            ..Default::default()
        };
        assert_eq!(slide_eyebrow(&deck), ("II".into(), " · Ada".into()));
        deck.deck_title = Some("Engines".into());
        assert_eq!(slide_eyebrow(&deck), ("II".into(), " · Engines".into()));
        let first = SlideContext::default();
        assert_eq!(slide_eyebrow(&first), ("I".into(), String::new()));
    }

    /// One frame at `time` seconds; `draw` says whether slide 3 is drawn.
    fn frame(ctx: &egui::Context, time: f64, draw: bool) -> Option<f32> {
        let mut age = None;
        let input = egui::RawInput {
            time: Some(time),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| {
            if draw {
                age = Some(entry_age(ui, 3, true, false));
            }
        });
        out.textures_delta.clear();
        age
    }

    #[test]
    fn an_idle_window_does_not_replay_the_entrance() {
        // Regression: when nothing asks for repaints (an art engine has
        // finished drawing), the next frame can come seconds later. That gap
        // must not count as leaving the slide, or every line of copy
        // animates in again on the next key press.
        let ctx = egui::Context::default();
        assert_eq!(frame(&ctx, 0.0, true), Some(0.0));
        let later = frame(&ctx, 5.0, true).unwrap();
        assert!(later > 4.9, "an idle gap replayed the entrance: {later}");
        // Leaving the slide for a frame and coming back does replay it.
        frame(&ctx, 6.0, false);
        assert_eq!(frame(&ctx, 7.0, true), Some(0.0));
    }

    #[test]
    fn stagger_orders_elements_and_completes() {
        assert!(stagger(0.0, 0) < 0.01);
        assert!(stagger(0.3, 0) > stagger(0.3, 1));
        assert!((stagger(5.0, 3) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn handles_text_slides_only() {
        let mk = |layout, blocks| Slide {
            directives: vec![],
            blocks,
            layout,
            raw_source: String::new(),
            line: 0,
            source_lines: Vec::new(),
            notes: None,
            story_hint: None,
            scene_script: None,
            illustration: None,
            logo: None,
            art: None,
        };
        assert!(handles(&mk(Layout::Title, vec![])));
        assert!(handles(&mk(Layout::Bullet, vec![])));
        assert!(!handles(&mk(Layout::Code, vec![])));
        assert!(!handles(&mk(Layout::Visualization, vec![])));
        let with_table = mk(
            Layout::Content,
            vec![Block::Table {
                headers: vec![],
                rows: vec![],
            }],
        );
        assert!(!handles(&with_table));
    }
}
