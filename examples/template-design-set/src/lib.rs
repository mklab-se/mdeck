//! The `template-design-set` design set: every slide as a poster, a large title over
//! a few lines of text.
//!
//! Made with `mdeck sdk new design-set template-design-set`. A design set arranges a
//! slide's content in code. Use one when the data design sets (`standard`,
//! `editorial`) cannot express the look, or as the design set of a board
//! engine (see `docs/sdk/design-sets.md`).

use mdeck_sdk::content::{Block, ListItem, ListMarker, Slide, plain_text};
use mdeck_sdk::design::{DesignCx, DesignSet};
use mdeck_sdk::geometry::Hint;
use mdeck_sdk::paint::{Align2, Font, Pos2, Rect};
use mdeck_sdk::problem::Problem;
use mdeck_sdk::registry::{Registry, RegistryError};

/// The entry point `mdeck build --with` calls: register what this crate brings.
pub fn register(r: &mut Registry) -> Result<(), RegistryError> {
    r.design_set(Box::new(Poster))?;
    r.theme("template-design-set", include_str!("../theme.yaml"))
}

/// The design set.
pub struct Poster;

/// One line of body text and the reveal step that shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    /// The text.
    pub text: String,
    /// 0: shown with the slide; n: shown from step n.
    pub step: usize,
}

/// The body lines of `slide`: paragraphs, quotes and list items (with `+`
/// items revealed one step at a time), in reading order.
pub fn lines(slide: &Slide) -> Vec<Line> {
    let mut out = Vec::new();
    let mut step = 0;
    let mut items = |list: &[ListItem], first: Option<u32>, out: &mut Vec<Line>| {
        for (k, item) in list.iter().enumerate() {
            if item.marker == ListMarker::NextStep {
                step += 1;
            }
            // a numbered list counts from its first number (`3.`)
            let marker = match first {
                Some(n) => format!("{}.", n as usize + k),
                None => "·".to_string(),
            };
            out.push(Line {
                text: format!("{marker} {}", plain_text(&item.inlines)),
                step,
            });
        }
    };
    let mut first_heading = true;
    for block in &slide.blocks {
        match block {
            Block::Heading { .. } if first_heading => first_heading = false,
            Block::Heading { inlines, .. } | Block::Paragraph { inlines, .. } => out.push(Line {
                text: plain_text(inlines),
                step: 0,
            }),
            Block::BlockQuote { blocks, .. } => out.push(Line {
                text: format!("“{}”", plain_text(&Block::quote_text(blocks))),
                step: 0,
            }),
            Block::Callout { kind, blocks, .. } => out.push(Line {
                text: format!(
                    "{}: {}",
                    kind.label(),
                    plain_text(&Block::quote_text(blocks))
                ),
                step: 0,
            }),
            Block::List {
                ordered,
                start,
                items: list,
                ..
            } => items(list, ordered.then_some(*start), &mut out),
            // Blocks this set does not show (see `unsupported`), and kinds
            // added in a later SDK.
            _ => {}
        }
    }
    out
}

impl DesignSet for Poster {
    /// The name a theme selects it by (`designs: template-design-set`).
    fn name(&self) -> &str {
        "template-design-set"
    }

    /// Draw `slide` into `rect` at the context's reveal step.
    fn render(&self, cx: &mut DesignCx, slide: &Slide, rect: Rect) {
        let s = cx.scale();
        let tokens = cx.tokens().clone();
        let area = rect.shrink(120.0 * s);
        let mut y = area.top();
        if let Some(title) = slide.title() {
            let r = cx.painter().text(
                Pos2::new(area.left(), y),
                Align2::LEFT_TOP,
                &title,
                Font::display(120.0 * s),
                tokens.heading,
            );
            y = r.bottom() + 48.0 * s;
            cx.painter().rect_filled(
                Rect::from_min_max(
                    Pos2::new(area.left(), y - 24.0 * s),
                    Pos2::new(area.left() + 160.0 * s, y - 18.0 * s),
                ),
                0.0,
                tokens.accent,
            );
        }
        let font = Font::body(44.0 * s);
        for line in lines(slide).iter().filter(|l| l.step <= cx.step()) {
            if y > area.bottom() {
                break;
            }
            let r = cx.painter().text(
                Pos2::new(area.left(), y),
                Align2::LEFT_TOP,
                &line.text,
                font,
                tokens.text,
            );
            y = r.bottom() + 18.0 * s;
        }
        // Tell the engine where the copy is, so it stays calm there.
        cx.publish(Hint::Frame(Rect::from_min_max(
            area.min,
            Pos2::new(area.right(), y.min(area.bottom())),
        )));
    }

    /// What this set does not show, one problem per block, for `mdeck --check`.
    fn unsupported(&self, slide: &Slide) -> Vec<Problem> {
        slide
            .blocks
            .iter()
            .filter_map(|b| match b {
                Block::Image { .. } => Some("images"),
                Block::CodeBlock { .. } => Some("code"),
                Block::Table { .. } => Some("tables"),
                Block::Visual { .. } => Some("visuals"),
                _ => None,
            })
            .map(|what| {
                Problem::new(
                    "design",
                    format!("the template-design-set design set does not show {what}"),
                )
                .at(slide.line)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mdeck_sdk::content::Inline;

    fn text(s: &str) -> Vec<Inline> {
        vec![Inline::Text(s.into())]
    }

    #[test]
    fn lists_reveal_step_by_step() {
        let item = |marker, s: &str| {
            let mut item = ListItem::new(marker, text(s));
            item.step = usize::from(marker == ListMarker::NextStep);
            item
        };
        let mut slide = Slide::new("bullet");
        slide.blocks = vec![
            Block::heading(2, text("Title")),
            Block::list(vec![
                item(ListMarker::Static, "a"),
                item(ListMarker::NextStep, "b"),
            ]),
        ];
        let l = lines(&slide);
        assert_eq!((l[0].step, l[1].step), (0, 1));
    }

    #[test]
    fn numbered_lists_count_from_their_start() {
        let mut slide = Slide::new("bullet");
        slide.blocks = vec![Block::ordered_list(
            3,
            vec![ListItem::new(ListMarker::Ordered, text("c"))],
        )];
        assert_eq!(lines(&slide)[0].text, "3. c");
    }

    #[test]
    fn reports_what_it_cannot_show() {
        let mut slide = Slide::new("visual");
        slide.line = 12;
        slide.blocks = vec![Block::visual("bar", "")];
        let p = Poster.unsupported(&slide);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].line, Some(12));
    }
}
