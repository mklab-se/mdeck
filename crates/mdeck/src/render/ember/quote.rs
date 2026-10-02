//! Quote slides: an optional heading, the quotation in the display face
//! with an ember hairline, and the attribution as an eyebrow.

use std::sync::Arc;

use eframe::egui::{self, Pos2, Rect};

use super::jobs::{display_job, eyebrow_job};
use super::pillow::pillow;
use super::{Frame, copy_column, fade, slide_eyebrow};
use crate::parser::{Block, Inline, Slide};

/// The attribution as plain text, with a leading `--`, `---` or em dash
/// dropped (the eyebrow reads without one).
fn attribution_text(inlines: &[Inline]) -> String {
    inlines
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
        .map(|i| match i {
            Inline::Text(s) | Inline::Code(s) => s,
            Inline::Bold(c) | Inline::Italic(c) | Inline::Strikethrough(c) => plain_text(&c),
            Inline::Link { text, .. } => plain_text(&text),
            Inline::Math { tex, .. } => tex,
        })
        .collect()
}

/// The text runs directly inside `inlines` (one level, as the eyebrow shows).
fn plain_text(inlines: &[Inline]) -> String {
    inlines
        .iter()
        .map(|x| {
            if let Inline::Text(s) = x {
                s.clone()
            } else {
                String::new()
            }
        })
        .collect()
}

pub(super) fn render_quote(f: &Frame, slide: &Slide) {
    let (ui, theme, rect, sz, scale) = (f.ui, f.theme, f.rect, &f.sz, f.scale);
    let mut quote_blocks = None;
    let mut attribution = None;
    let mut heading = None;
    for block in &slide.blocks {
        match block {
            Block::Heading { inlines, .. } => heading = Some(inlines),
            Block::BlockQuote { blocks } => quote_blocks = Some(blocks),
            Block::Paragraph { inlines } if quote_blocks.is_some() => {
                attribution = Some(inlines.clone())
            }
            _ => {}
        }
    }
    let quote = quote_blocks.map(|b| {
        let (quote, own) = Block::quote_parts(b, attribution.is_some());
        attribution = attribution.take().or(own);
        quote
    });
    let quote = quote.as_ref();
    let attribution = attribution.as_ref();
    let column = copy_column(rect);
    let width = rect.width() * 0.58;
    let (num, rest) = slide_eyebrow(f.deck);
    let eyebrow = ui
        .painter()
        .layout_job(eyebrow_job(theme, &num, &rest, sz.eyebrow, 1.0));
    let head = heading.map(|h| {
        ui.painter().layout_job(display_job(
            h,
            sz.h2 * 0.8,
            theme.heading_color,
            width,
            theme,
        ))
    });
    let q = quote.map(|q| {
        let mut job = display_job(q, sz.quote, theme.heading_color, width, theme);
        for s in &mut job.sections {
            s.format.italics = true;
            s.format.line_height = Some(sz.quote * 1.22);
        }
        ui.painter().layout_job(job)
    });
    let attr = attribution.map(|a| {
        let text = attribution_text(a);
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
    pillow(ui.painter(), copy, f.pillow_alpha(), theme);

    let x = column.left();
    let mut y = top;
    let mut nth = 0;
    let mut place = |g: Arc<egui::Galley>, y: &mut f32, extra: f32| {
        let h = g.rect.height();
        let p = f.place(g, Pos2::new(x, *y), nth);
        nth += 1;
        *y += h + extra;
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
        ui.painter().line_segment(
            [
                Pos2::new(x - 28.0 * scale, y - h + 6.0 * scale),
                Pos2::new(x - 28.0 * scale, y - 6.0 * scale),
            ],
            egui::Stroke::new(1.5, fade(theme.accent, f.opacity * p)),
        );
    }
    if let Some(g) = attr {
        y += gap;
        place(g, &mut y, 0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attribution_drops_a_leading_dash_and_flattens_runs() {
        let text = |s: &str| Inline::Text(s.into());
        assert_eq!(attribution_text(&[text("-- Alan Kay")]), "Alan Kay");
        assert_eq!(attribution_text(&[text("  --- Ada")]), "Ada");
        assert_eq!(attribution_text(&[text("\u{2014}Grace")]), "Grace");
        // only the first run loses its dash
        assert_eq!(
            attribution_text(&[
                text("Ada, "),
                Inline::Italic(vec![text("-- Notes")]),
                Inline::Code("x".into()),
            ]),
            "Ada, -- Notesx"
        );
        assert_eq!(
            attribution_text(&[Inline::Link {
                text: vec![text("site")],
                url: "https://example.com".into(),
            }]),
            "site"
        );
        // A run that is not the first keeps its text as is
        assert_eq!(attribution_text(&[text("a"), text("-- b")]), "a-- b");
    }
}
