//! Ember layouts: the MKLab site's composition for text slides.
//!
//! Copy sits in a narrow column on the left, over a soft dark pillow, with a
//! tracked monospace eyebrow, an editorial serif heading and a light-weight
//! lead. Elements fade up in a stagger when the slide is entered, and list
//! items fade up as they reveal. Slides whose content fills the frame (code,
//! charts, diagrams, images, tables) keep their regular layouts and only
//! inherit the palette and fonts.

use std::time::Instant;

use eframe::egui::{self, Color32, Pos2, Rect};

use crate::parser::{Block, Inline, Layout, ListItem, ListMarker, Slide};
use crate::theme::{FONT_BODY_LIGHT, FONT_BODY_MEDIUM, Theme};

/// Facts about the deck that the eyebrow and chrome show.
#[derive(Clone, Debug, Default)]
pub struct SlideContext {
    pub index: usize,
    pub count: usize,
    pub deck_title: Option<String>,
    pub author: Option<String>,
    /// While the logo intro runs, the title copy holds back.
    pub hold_copy: bool,
    /// Play entry and reveal animations (false for export and thumbnails).
    pub animate: bool,
    /// Story beats on this slide: (current step, beat count).
    pub beats: Option<(usize, usize)>,
    /// The presenter's line for the current beat (shown with the HUD).
    pub say: Option<String>,
}

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

const INK_050: Color32 = Color32::from_rgb(0xEC, 0xEC, 0xEF);
const INK_100: Color32 = Color32::from_rgb(0xD6, 0xD6, 0xDB);
const INK_200: Color32 = Color32::from_rgb(0xB4, 0xB4, 0xBC);
const INK_300: Color32 = Color32::from_rgb(0x8F, 0x8F, 0x98);
const INK_700: Color32 = Color32::from_rgb(0x24, 0x24, 0x29);
const EMBER: Color32 = Color32::from_rgb(0xFF, 0x4D, 0x1C);
const EMBER_SOFT: Color32 = Color32::from_rgb(0xFF, 0x8A, 0x66);
const CANDLE: Color32 = Color32::from_rgb(0xF5, 0xA6, 0x23);

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
    last_seen: f64,
}

/// Seconds since this slide was (re)entered. A slide that has not been drawn
/// for a quarter second counts as re-entered, so navigating back replays the
/// stagger just like the site does.
fn entry_age(ui: &egui::Ui, index: usize, animate: bool, hold: bool) -> f32 {
    if !animate {
        return 10.0;
    }
    let now = ui.input(|i| i.time);
    let id = egui::Id::new(("ember-entry", index));
    let age = ui.ctx().data_mut(|d| {
        let e = d.get_temp_mut_or_insert_with(id, || Entry {
            entered_at: now,
            last_seen: now,
        });
        // While the intro holds the copy back, the clock keeps restarting so
        // the stagger begins the moment the logo dissolves.
        if hold || now - e.last_seen > 0.25 {
            e.entered_at = now;
        }
        e.last_seen = now;
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
// Text helpers
// ---------------------------------------------------------------------------

/// Tracked uppercase monospace label. `accent_prefix` is drawn in ember.
fn eyebrow_job(
    theme: &Theme,
    accent_prefix: &str,
    rest: &str,
    size: f32,
    alpha: f32,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let font = egui::FontId::new(size, theme.mono_family());
    let format = |color: Color32| egui::text::TextFormat {
        font_id: font.clone(),
        color,
        extra_letter_spacing: size * 0.22,
        ..Default::default()
    };
    if !accent_prefix.is_empty() {
        job.append(
            &accent_prefix.to_uppercase(),
            0.0,
            format(fade(EMBER, alpha)),
        );
    }
    job.append(&rest.to_uppercase(), 0.0, format(fade(INK_300, alpha)));
    job
}

/// Display heading in the serif face; `*emphasis*` turns ember instead of italic.
pub fn display_job(
    inlines: &[Inline],
    size: f32,
    color: Color32,
    width: f32,
    theme: &Theme,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = width;
    let base = egui::text::TextFormat {
        font_id: egui::FontId::new(size, theme.display_family()),
        color,
        extra_letter_spacing: -size * 0.018,
        line_height: Some(size * 1.06),
        ..Default::default()
    };
    append_display(&mut job, inlines, &base, color, theme);
    job
}

fn append_display(
    job: &mut egui::text::LayoutJob,
    inlines: &[Inline],
    base: &egui::text::TextFormat,
    color: Color32,
    theme: &Theme,
) {
    for inline in inlines {
        match inline {
            Inline::Text(s) => job.append(s, 0.0, base.clone()),
            Inline::Bold(children) | Inline::Strikethrough(children) => {
                append_display(job, children, base, color, theme)
            }
            Inline::Italic(children) => {
                let mut f = base.clone();
                f.color = fade(EMBER_SOFT, color.a() as f32 / 255.0);
                append_display(job, children, &f, color, theme);
            }
            Inline::Code(s) => {
                let mut f = base.clone();
                f.font_id = egui::FontId::new(base.font_id.size * 0.8, theme.mono_family());
                job.append(s, 0.0, f);
            }
            Inline::Link { text, .. } => append_display(job, text, base, color, theme),
        }
    }
}

/// Body copy in the grotesque: light weight, relaxed leading, `**strong**`
/// brighter and medium, `*em*` in soft ember (never italic, as on the site).
fn lead_job(
    inlines: &[Inline],
    size: f32,
    alpha: f32,
    width: f32,
    theme: &Theme,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = width;
    let base = egui::text::TextFormat {
        font_id: egui::FontId::new(size, egui::FontFamily::Name(FONT_BODY_LIGHT.into())),
        color: fade(INK_200, alpha),
        line_height: Some(size * 1.5),
        ..Default::default()
    };
    append_lead(&mut job, inlines, &base, alpha, theme);
    job
}

fn append_lead(
    job: &mut egui::text::LayoutJob,
    inlines: &[Inline],
    base: &egui::text::TextFormat,
    alpha: f32,
    theme: &Theme,
) {
    for inline in inlines {
        match inline {
            Inline::Text(s) => job.append(s, 0.0, base.clone()),
            Inline::Bold(children) => {
                let mut f = base.clone();
                f.font_id = egui::FontId::new(
                    base.font_id.size,
                    egui::FontFamily::Name(FONT_BODY_MEDIUM.into()),
                );
                f.color = fade(INK_050, alpha);
                append_lead(job, children, &f, alpha, theme);
            }
            Inline::Italic(children) => {
                let mut f = base.clone();
                f.color = fade(EMBER_SOFT, alpha);
                append_lead(job, children, &f, alpha, theme);
            }
            Inline::Strikethrough(children) => {
                let mut f = base.clone();
                f.strikethrough = egui::Stroke::new(1.0, base.color);
                append_lead(job, children, &f, alpha, theme);
            }
            Inline::Code(s) => {
                let mut f = base.clone();
                f.font_id = egui::FontId::new(base.font_id.size * 0.86, theme.mono_family());
                f.color = fade(INK_100, alpha);
                f.background = fade(EMBER, alpha * 0.10);
                job.append(s, 0.0, f);
            }
            Inline::Link { text, .. } => {
                let mut f = base.clone();
                f.color = fade(EMBER_SOFT, alpha);
                f.underline = egui::Stroke::new(1.0, fade(EMBER_SOFT, alpha * 0.5));
                append_lead(job, text, &f, alpha, theme);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Pillow
// ---------------------------------------------------------------------------

/// The soft dark ellipse that keeps copy readable over the lights: a radial
/// gradient from 72% black at the centre to transparent, drawn as a fan of
/// concentric rings so egui's linear vertex interpolation follows the curve.
pub fn pillow(painter: &egui::Painter, copy: Rect, alpha: f32) {
    let center = copy.center();
    let rx = copy.width() * 0.5 + copy.width() * 0.34;
    let ry = copy.height() * 0.5 + copy.height() * 0.42 + 40.0;
    let stops: [(f32, f32); 5] = [
        (0.0, 0.72),
        (0.30, 0.62),
        (0.55, 0.38),
        (0.78, 0.0),
        (1.0, 0.0),
    ];
    let segments = 48;
    let mut mesh = egui::Mesh::default();
    let color_at = |a: f32| fade(Color32::from_rgb(5, 5, 5), a * alpha);
    // centre vertex
    mesh.colored_vertex(center, color_at(stops[0].1));
    for (ri, (r, a)) in stops.iter().enumerate().skip(1) {
        for s in 0..segments {
            let ang = s as f32 / segments as f32 * std::f32::consts::TAU;
            let p = Pos2::new(center.x + ang.cos() * rx * r, center.y + ang.sin() * ry * r);
            mesh.colored_vertex(p, color_at(*a));
        }
        let ring_start = 1 + (ri - 1) * segments;
        if ri == 1 {
            for s in 0..segments {
                let a = ring_start + s;
                let b = ring_start + (s + 1) % segments;
                mesh.add_triangle(0, a as u32, b as u32);
            }
        } else {
            let prev = ring_start - segments;
            for s in 0..segments {
                let a0 = prev + s;
                let a1 = prev + (s + 1) % segments;
                let b0 = ring_start + s;
                let b1 = ring_start + (s + 1) % segments;
                mesh.add_triangle(a0 as u32, b0 as u32, b1 as u32);
                mesh.add_triangle(a0 as u32, b1 as u32, a1 as u32);
            }
        }
    }
    painter.add(egui::Shape::mesh(mesh));
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
// Blocks
// ---------------------------------------------------------------------------

/// One element of the copy stack, laid out and ready to draw.
struct Piece {
    galley: std::sync::Arc<egui::Galley>,
    /// Space below this piece.
    gap: f32,
    /// Reveal step (0 = always) for list items.
    step: usize,
    /// Draw an ember dot to the left (list items).
    dot: bool,
    /// Draw an ember hairline to the left spanning the piece (quotes).
    bar: bool,
}

fn item_pieces(
    ui: &egui::Ui,
    items: &[ListItem],
    sz: &Sizes,
    width: f32,
    theme: &Theme,
    scale: f32,
    pieces: &mut Vec<Piece>,
) {
    let mut counter = 0usize;
    for item in items {
        let step = match item.marker {
            ListMarker::Static | ListMarker::Ordered => 0,
            ListMarker::NextStep => {
                counter += 1;
                counter
            }
            ListMarker::WithPrev => counter,
        };
        let job = lead_job(&item.inlines, sz.lead, 1.0, width - 34.0 * scale, theme);
        let galley = ui.painter().layout_job(job);
        pieces.push(Piece {
            galley,
            gap: 16.0 * scale,
            step,
            dot: true,
            bar: false,
        });
        // One level of nesting: same treatment, indented by the caller via dot spacing
        if !item.children.is_empty() {
            for child in &item.children {
                let job = lead_job(
                    &child.inlines,
                    sz.lead * 0.9,
                    0.85,
                    width - 60.0 * scale,
                    theme,
                );
                let galley = ui.painter().layout_job(job);
                pieces.push(Piece {
                    galley,
                    gap: 12.0 * scale,
                    step,
                    dot: true,
                    bar: false,
                });
            }
        }
    }
}

/// Lay out the whole copy stack for a bullet/content slide.
fn content_pieces(
    ui: &egui::Ui,
    slide: &Slide,
    theme: &Theme,
    sz: &Sizes,
    width: f32,
    scale: f32,
) -> Vec<Piece> {
    let mut pieces = Vec::new();
    for block in &slide.blocks {
        match block {
            Block::Heading { level, inlines } => {
                let size = if *level <= 1 {
                    sz.h2 * 1.1
                } else if *level == 2 {
                    sz.h2
                } else {
                    sz.h2 * 0.7
                };
                let job = display_job(inlines, size, INK_050, width, theme);
                pieces.push(Piece {
                    galley: ui.painter().layout_job(job),
                    gap: 28.0 * scale,
                    step: 0,
                    dot: false,
                    bar: false,
                });
            }
            Block::Paragraph { inlines } => {
                let job = lead_job(inlines, sz.lead, 1.0, width, theme);
                pieces.push(Piece {
                    galley: ui.painter().layout_job(job),
                    gap: 22.0 * scale,
                    step: 0,
                    dot: false,
                    bar: false,
                });
            }
            Block::List { items, .. } => {
                item_pieces(ui, items, sz, width, theme, scale, &mut pieces);
                if let Some(last) = pieces.last_mut() {
                    last.gap = 26.0 * scale;
                }
            }
            Block::BlockQuote { inlines } => {
                let job = lead_job(inlines, sz.lead, 1.0, width - 30.0 * scale, theme);
                pieces.push(Piece {
                    galley: ui.painter().layout_job(job),
                    gap: 22.0 * scale,
                    step: 0,
                    dot: false,
                    bar: true,
                });
            }
            Block::HorizontalRule => {}
            _ => {}
        }
    }
    pieces
}

/// Draw a stack of pieces from `top`, applying entry stagger and reveal fades.
#[allow(clippy::too_many_arguments)]
fn draw_pieces(
    ui: &egui::Ui,
    pieces: &[Piece],
    left: f32,
    top: f32,
    opacity: f32,
    age: f32,
    reveal_step: usize,
    reveal_timestamp: Option<Instant>,
    scale: f32,
    start_nth: usize,
) {
    let painter = ui.painter();
    let mut y = top;
    let mut nth = start_nth;
    let reveal_age = reveal_timestamp
        .filter(|_| age < 9.0)
        .map(|t| t.elapsed().as_secs_f32())
        .unwrap_or(10.0);
    for piece in pieces {
        if piece.step > reveal_step {
            continue;
        }
        // Items in the step that just revealed rise like an entering element.
        let progress = if piece.step > 0 && piece.step == reveal_step && reveal_age < 1.0 {
            ease_out(reveal_age / 0.55)
        } else {
            stagger(age, nth)
        };
        if progress < 1.0 {
            ui.ctx().request_repaint();
        }
        let a = opacity * progress;
        let rise = (1.0 - progress) * 14.0 * scale;
        let pos = Pos2::new(left, y + rise);
        let color = piece
            .galley
            .job
            .sections
            .first()
            .map(|s| s.format.color)
            .unwrap_or(INK_200);
        let tint = fade(Color32::WHITE, a);
        painter.galley_with_override_text_color(pos, piece.galley.clone(), tint);
        let _ = color;
        if piece.dot {
            let first_line_h = piece
                .galley
                .rows
                .first()
                .map(|r| r.rect().height())
                .unwrap_or(20.0);
            let cy = pos.y + first_line_h * 0.55;
            painter.circle_filled(
                Pos2::new(left - 18.0 * scale, cy),
                3.2 * scale,
                fade(EMBER, a),
            );
        }
        if piece.bar {
            let h = piece.galley.rect.height();
            painter.line_segment(
                [
                    Pos2::new(left - 16.0 * scale, pos.y + 2.0),
                    Pos2::new(left - 16.0 * scale, pos.y + h - 2.0),
                ],
                egui::Stroke::new(1.0, fade(CANDLE, a)),
            );
        }
        y += piece.galley.rect.height() + piece.gap;
        nth += 1;
    }
}

fn stack_height(pieces: &[Piece], reveal_step: usize) -> f32 {
    let mut h = 0.0;
    let mut last_gap = 0.0;
    for p in pieces.iter().filter(|p| p.step <= reveal_step) {
        h += p.galley.rect.height() + p.gap;
        last_gap = p.gap;
    }
    (h - last_gap).max(0.0)
}

fn full_height(pieces: &[Piece]) -> f32 {
    stack_height(pieces, usize::MAX)
}

// ---------------------------------------------------------------------------
// Slide renderers
// ---------------------------------------------------------------------------

/// Render an Ember-handled slide. Caller guarantees [`handles`] is true.
#[allow(clippy::too_many_arguments)]
pub fn render(
    ui: &egui::Ui,
    slide: &Slide,
    theme: &Theme,
    rect: Rect,
    opacity: f32,
    reveal_step: usize,
    reveal_timestamp: Option<Instant>,
    scale: f32,
    cx: &SlideContext,
) {
    let age = if cx.hold_copy {
        entry_age(ui, cx.index, cx.animate, true);
        -1.0
    } else {
        entry_age(ui, cx.index, cx.animate, false)
    };
    let sz = sizes(theme, scale);
    if is_title(slide, cx.index) {
        render_title(ui, slide, theme, rect, opacity, age, &sz, scale, cx);
        return;
    }
    match slide.layout {
        Layout::Title => render_title(ui, slide, theme, rect, opacity, age, &sz, scale, cx),
        Layout::Section => render_section(ui, slide, theme, rect, opacity, age, &sz, scale, cx),
        Layout::Quote => render_quote(ui, slide, theme, rect, opacity, age, &sz, scale, cx),
        _ => render_copy(
            ui,
            slide,
            theme,
            rect,
            opacity,
            age,
            &sz,
            reveal_step,
            reveal_timestamp,
            scale,
            cx,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn render_title(
    ui: &egui::Ui,
    slide: &Slide,
    theme: &Theme,
    rect: Rect,
    opacity: f32,
    age: f32,
    sz: &Sizes,
    scale: f32,
    cx: &SlideContext,
) {
    let mut heading = None;
    let mut subtitle = None;
    for block in &slide.blocks {
        match block {
            Block::Heading { level: 1, inlines } => heading = Some(inlines),
            Block::Heading { inlines, .. } if subtitle.is_none() => subtitle = Some(inlines),
            Block::Paragraph { inlines } if subtitle.is_none() => subtitle = Some(inlines),
            _ => {}
        }
    }
    let width = rect.width() * 0.62;
    let eyebrow_text = match (&cx.author, &cx.deck_title) {
        (Some(a), Some(t)) if a != t => format!("{a} · {t}"),
        (Some(a), _) => a.clone(),
        (None, Some(t)) => t.clone(),
        (None, None) => "mdeck".to_string(),
    };
    let eyebrow = ui
        .painter()
        .layout_job(eyebrow_job(theme, "", &eyebrow_text, sz.eyebrow, 1.0));
    let title = heading.map(|h| {
        ui.painter()
            .layout_job(display_job(h, sz.h1, INK_050, width, theme))
    });
    let sub = subtitle.map(|s| {
        ui.painter()
            .layout_job(lead_job(s, sz.lead * 1.05, 1.0, width * 0.8, theme))
    });

    let gap1 = 30.0 * scale;
    let gap2 = 40.0 * scale;
    let total = eyebrow.rect.height()
        + gap1
        + title.as_ref().map(|g| g.rect.height()).unwrap_or(0.0)
        + sub.as_ref().map(|g| gap2 + g.rect.height()).unwrap_or(0.0);
    let copy_w = title
        .as_ref()
        .map(|g| g.rect.width())
        .unwrap_or(width)
        .max(sub.as_ref().map(|g| g.rect.width()).unwrap_or(0.0));
    let top = rect.center().y - total / 2.0 - rect.height() * 0.02;
    let copy = Rect::from_center_size(
        Pos2::new(rect.center().x, top + total / 2.0),
        egui::vec2(copy_w, total),
    );
    let pillow_a = if age < 0.0 { 0.0 } else { ease_out(age / 0.9) } * opacity;
    pillow(ui.painter(), copy, pillow_a);
    if age < 0.0 {
        ui.ctx().request_repaint();
        return;
    }

    let painter = ui.painter();
    let mut y = top;
    let centered = |g: &egui::Galley| rect.center().x - g.rect.width() / 2.0;

    let p0 = stagger(age, 0);
    painter.galley_with_override_text_color(
        Pos2::new(centered(&eyebrow), y + (1.0 - p0) * 14.0 * scale),
        eyebrow.clone(),
        fade(Color32::WHITE, opacity * p0),
    );
    y += eyebrow.rect.height() + gap1;
    if let Some(g) = title {
        let p = stagger(age, 1);
        painter.galley_with_override_text_color(
            Pos2::new(centered(&g), y + (1.0 - p) * 14.0 * scale),
            g.clone(),
            fade(Color32::WHITE, opacity * p),
        );
        y += g.rect.height() + gap2;
    }
    if let Some(g) = sub {
        let p = stagger(age, 2);
        painter.galley_with_override_text_color(
            Pos2::new(centered(&g), y + (1.0 - p) * 14.0 * scale),
            g.clone(),
            fade(Color32::WHITE, opacity * p),
        );
    }

    // "Space to begin" hint, as on the site, on the very first slide only.
    if cx.index == 0 {
        let p = stagger(age, 6);
        let hint = painter.layout_job(eyebrow_job(
            theme,
            "",
            "Space to begin",
            sz.eyebrow * 0.85,
            1.0,
        ));
        painter.galley_with_override_text_color(
            Pos2::new(
                rect.center().x - hint.rect.width() / 2.0,
                rect.bottom() - 62.0 * scale,
            ),
            hint,
            fade(Color32::WHITE, opacity * p * 0.9),
        );
    }
    if stagger(age, 6) < 1.0 {
        ui.ctx().request_repaint();
    }
}

#[allow(clippy::too_many_arguments)]
fn render_section(
    ui: &egui::Ui,
    slide: &Slide,
    theme: &Theme,
    rect: Rect,
    opacity: f32,
    age: f32,
    sz: &Sizes,
    scale: f32,
    cx: &SlideContext,
) {
    let heading = slide.blocks.iter().find_map(|b| match b {
        Block::Heading { inlines, .. } => Some(inlines),
        _ => None,
    });
    let column = copy_column(rect);
    let width = rect.width() * 0.56;
    let (num, rest) = slide_eyebrow(cx);
    let eyebrow = ui
        .painter()
        .layout_job(eyebrow_job(theme, &num, &rest, sz.eyebrow, 1.0));
    let title = heading.map(|h| {
        ui.painter()
            .layout_job(display_job(h, sz.h1, INK_050, width, theme))
    });
    let gap = 30.0 * scale;
    let total =
        eyebrow.rect.height() + gap + title.as_ref().map(|g| g.rect.height()).unwrap_or(0.0);
    // Sits low, like the site's `slide--bottom`.
    let top = rect.bottom() - rect.height() * 0.14 - total;
    let copy_w = title.as_ref().map(|g| g.rect.width()).unwrap_or(width);
    let copy = Rect::from_min_size(Pos2::new(column.left(), top), egui::vec2(copy_w, total));
    pillow(ui.painter(), copy, opacity * ease_out(age / 0.9));

    let painter = ui.painter();
    let p0 = stagger(age, 0);
    painter.galley_with_override_text_color(
        Pos2::new(column.left(), top + (1.0 - p0) * 14.0 * scale),
        eyebrow.clone(),
        fade(Color32::WHITE, opacity * p0),
    );
    if let Some(g) = title {
        let p = stagger(age, 1);
        painter.galley_with_override_text_color(
            Pos2::new(
                column.left(),
                top + eyebrow.rect.height() + gap + (1.0 - p) * 14.0 * scale,
            ),
            g,
            fade(Color32::WHITE, opacity * p),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn render_quote(
    ui: &egui::Ui,
    slide: &Slide,
    theme: &Theme,
    rect: Rect,
    opacity: f32,
    age: f32,
    sz: &Sizes,
    scale: f32,
    cx: &SlideContext,
) {
    let mut quote = None;
    let mut attribution = None;
    let mut heading = None;
    for block in &slide.blocks {
        match block {
            Block::Heading { inlines, .. } => heading = Some(inlines),
            Block::BlockQuote { inlines } => quote = Some(inlines),
            Block::Paragraph { inlines } if quote.is_some() => attribution = Some(inlines),
            _ => {}
        }
    }
    let column = copy_column(rect);
    let width = rect.width() * 0.58;
    let (num, rest) = slide_eyebrow(cx);
    let eyebrow = ui
        .painter()
        .layout_job(eyebrow_job(theme, &num, &rest, sz.eyebrow, 1.0));
    let head = heading.map(|h| {
        ui.painter()
            .layout_job(display_job(h, sz.h2 * 0.8, INK_050, width, theme))
    });
    let q = quote.map(|q| {
        let mut job = display_job(q, sz.quote, INK_050, width, theme);
        for s in &mut job.sections {
            s.format.italics = true;
            s.format.line_height = Some(sz.quote * 1.22);
        }
        ui.painter().layout_job(job)
    });
    let attr = attribution.map(|a| {
        let cleaned: Vec<Inline> = a
            .iter()
            .cloned()
            .enumerate()
            .map(|(i, inl)| match inl {
                Inline::Text(s) if i == 0 => {
                    let t = s.trim_start();
                    Inline::Text(
                        t.strip_prefix("---")
                            .or_else(|| t.strip_prefix("--"))
                            .or_else(|| t.strip_prefix('\u{2014}'))
                            .map(|r| r.trim_start().to_string())
                            .unwrap_or(s),
                    )
                }
                other => other,
            })
            .collect();
        let text: String = cleaned
            .iter()
            .map(|i| match i {
                Inline::Text(s) | Inline::Code(s) => s.clone(),
                Inline::Bold(c) | Inline::Italic(c) | Inline::Strikethrough(c) => c
                    .iter()
                    .map(|x| {
                        if let Inline::Text(s) = x {
                            s.clone()
                        } else {
                            String::new()
                        }
                    })
                    .collect(),
                Inline::Link { text, .. } => text
                    .iter()
                    .map(|x| {
                        if let Inline::Text(s) = x {
                            s.clone()
                        } else {
                            String::new()
                        }
                    })
                    .collect(),
            })
            .collect();
        ui.painter()
            .layout_job(eyebrow_job(theme, "", &text, sz.eyebrow, 1.0))
    });

    let gap = 26.0 * scale;
    let mut total = eyebrow.rect.height() + gap;
    if let Some(g) = &head {
        total += g.rect.height() + gap;
    }
    if let Some(g) = &q {
        total += g.rect.height();
    }
    if let Some(g) = &attr {
        total += gap + g.rect.height();
    }
    let top = rect.center().y - total / 2.0;
    let copy_w = q.as_ref().map(|g| g.rect.width()).unwrap_or(width);
    let copy = Rect::from_min_size(Pos2::new(column.left(), top), egui::vec2(copy_w, total));
    pillow(ui.painter(), copy, opacity * ease_out(age / 0.9));

    let painter = ui.painter();
    let x = column.left();
    let mut y = top;
    let mut nth = 0;
    let mut place = |g: std::sync::Arc<egui::Galley>, y: &mut f32, extra: f32| {
        let p = stagger(age, nth);
        nth += 1;
        painter.galley_with_override_text_color(
            Pos2::new(x, *y + (1.0 - p) * 14.0 * scale),
            g.clone(),
            fade(Color32::WHITE, opacity * p),
        );
        *y += g.rect.height() + extra;
        p
    };
    place(eyebrow, &mut y, gap);
    if let Some(g) = head {
        place(g, &mut y, gap);
    }
    if let Some(g) = q {
        let h = g.rect.height();
        let p = place(g, &mut y, 0.0);
        // ember hairline down the left of the quotation
        painter.line_segment(
            [
                Pos2::new(x - 28.0 * scale, y - h + 6.0 * scale),
                Pos2::new(x - 28.0 * scale, y - 6.0 * scale),
            ],
            egui::Stroke::new(1.5, fade(EMBER, opacity * p)),
        );
    }
    if let Some(g) = attr {
        y += gap;
        place(g, &mut y, 0.0);
    }
}

#[allow(clippy::too_many_arguments)]
fn render_copy(
    ui: &egui::Ui,
    slide: &Slide,
    theme: &Theme,
    rect: Rect,
    opacity: f32,
    age: f32,
    sz: &Sizes,
    reveal_step: usize,
    reveal_timestamp: Option<Instant>,
    scale: f32,
    cx: &SlideContext,
) {
    let column = copy_column(rect);
    let (num, rest) = slide_eyebrow(cx);
    let eyebrow = ui
        .painter()
        .layout_job(eyebrow_job(theme, &num, &rest, sz.eyebrow, 1.0));
    let pieces = content_pieces(ui, slide, theme, sz, column.width(), scale);

    let eyebrow_gap = 22.0 * scale;
    let total = eyebrow.rect.height() + eyebrow_gap + full_height(&pieces);
    let available = rect.height() * 0.80;
    let top = if total < available {
        rect.center().y - total / 2.0
    } else {
        rect.top() + rect.height() * 0.10
    };
    let copy_w = pieces
        .iter()
        .map(|p| p.galley.rect.width())
        .fold(eyebrow.rect.width(), f32::max);
    let copy = Rect::from_min_size(
        Pos2::new(column.left(), top),
        egui::vec2(copy_w.min(column.width()), total.min(rect.height() * 0.9)),
    );
    pillow(ui.painter(), copy, opacity * ease_out(age / 0.9));

    let p0 = stagger(age, 0);
    ui.painter().galley_with_override_text_color(
        Pos2::new(column.left(), top + (1.0 - p0) * 14.0 * scale),
        eyebrow.clone(),
        fade(Color32::WHITE, opacity * p0),
    );
    draw_pieces(
        ui,
        &pieces,
        column.left(),
        top + eyebrow.rect.height() + eyebrow_gap,
        opacity,
        age,
        reveal_step,
        reveal_timestamp,
        scale,
        1,
    );
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
    let pieces = content_pieces(ui, slide, theme, &sz, column.width(), scale);
    full_height(&pieces) + 60.0 * scale
}

// ---------------------------------------------------------------------------
// Chrome
// ---------------------------------------------------------------------------

/// Counter and progress hairline, as on the site's talk decks.
pub fn draw_chrome(
    painter: &egui::Painter,
    theme: &Theme,
    rect: Rect,
    cx: &SlideContext,
    scale: f32,
) {
    let (index, count) = (cx.index, cx.count);
    let size = 15.0 * scale;
    let font = egui::FontId::new(size, theme.mono_family());
    let mut job = egui::text::LayoutJob::default();
    let fmt = |c: Color32| egui::text::TextFormat {
        font_id: font.clone(),
        color: c,
        extra_letter_spacing: size * 0.18,
        ..Default::default()
    };
    job.append(&format!("{:02}", index + 1), 0.0, fmt(INK_100));
    job.append(&format!(" · {:02}", count), 0.0, fmt(INK_300));
    let galley = painter.layout_job(job);
    painter.galley(
        Pos2::new(
            rect.right() - 32.0 * scale - galley.rect.width(),
            rect.bottom() - 26.0 * scale - galley.rect.height(),
        ),
        galley,
        INK_300,
    );

    // beat ticks: how many steps this slide still has in it
    if let Some((cur, total)) = cx.beats
        && total > 1
    {
        let w = 10.0 * scale;
        let gap = 5.0 * scale;
        let right = rect.right() - 32.0 * scale;
        let y = rect.bottom() - 26.0 * scale - galley_h(painter, theme, size) - 12.0 * scale;
        for k in 0..total {
            let x1 = right - (total - 1 - k) as f32 * (w + gap);
            let color = if k <= cur { EMBER } else { INK_700 };
            painter.line_segment(
                [Pos2::new(x1 - w, y), Pos2::new(x1, y)],
                egui::Stroke::new(1.0, color),
            );
        }
    }

    // progress hairline
    let y = rect.bottom() - 1.0;
    painter.line_segment(
        [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
        egui::Stroke::new(1.0, fade(INK_700, 0.9)),
    );
    let frac = if count > 1 {
        index as f32 / (count - 1) as f32
    } else {
        1.0
    };
    painter.line_segment(
        [
            Pos2::new(rect.left(), y),
            Pos2::new(rect.left() + rect.width() * frac, y),
        ],
        egui::Stroke::new(1.0, EMBER),
    );
}

fn galley_h(painter: &egui::Painter, theme: &Theme, size: f32) -> f32 {
    painter
        .layout_no_wrap(
            "00".into(),
            egui::FontId::new(size, theme.mono_family()),
            INK_300,
        )
        .rect
        .height()
}

/// The presenter's line for the current beat, bottom-left, presenter-only.
pub fn draw_say_line(painter: &egui::Painter, theme: &Theme, rect: Rect, line: &str, scale: f32) {
    let size = 18.0 * scale;
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = rect.width() * 0.5;
    job.append(
        line,
        0.0,
        egui::text::TextFormat {
            font_id: egui::FontId::new(size, theme.body_family()),
            color: CANDLE,
            line_height: Some(size * 1.4),
            ..Default::default()
        },
    );
    let galley = painter.layout_job(job);
    let pos = Pos2::new(
        rect.left() + rect.width() * 0.07,
        rect.bottom() - 30.0 * scale - galley.rect.height(),
    );
    let bg = Rect::from_min_size(pos, galley.rect.size()).expand(10.0 * scale);
    painter.rect_filled(bg, 4.0 * scale, fade(Color32::from_rgb(5, 5, 5), 0.7));
    painter.galley(pos, galley, CANDLE);
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
            notes: None,
            story_hint: None,
            scene_script: None,
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
