//! The `{{name}}` design set: every slide as a poster, a large title over
//! a few lines of text.
//!
//! Made with `mdeck sdk new design-set {{name}}`. A design set arranges a
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
    r.theme("{{name}}", include_str!("../theme.yaml"))
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

/// The body lines of `slide`: paragraphs, then list items (with `+` items
/// revealed one step at a time).
pub fn lines(slide: &Slide) -> Vec<Line> {
    let mut out = Vec::new();
    let mut step = 0;
    let mut items = |list: &[ListItem], out: &mut Vec<Line>| {
        for item in list {
            if item.marker == ListMarker::NextStep {
                step += 1;
            }
            out.push(Line {
                text: format!("· {}", plain_text(&item.inlines)),
                step,
            });
        }
    };
    let mut first_heading = true;
    for block in &slide.blocks {
        match block {
            Block::Heading { .. } if first_heading => first_heading = false,
            Block::Heading { inlines, .. }
            | Block::Paragraph { inlines }
            | Block::BlockQuote { inlines } => out.push(Line {
                text: plain_text(inlines),
                step: 0,
            }),
            Block::List { items: list, .. } => items(list, &mut out),
            _ => {}
        }
    }
    out
}

impl DesignSet for Poster {
    /// The name a theme selects it by (`designs: {{name}}`).
    fn name(&self) -> &str {
        "{{name}}"
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
                    format!("the {{name}} design set does not show {what}"),
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
        let item = |marker, s: &str| ListItem {
            marker,
            inlines: text(s),
            children: vec![],
        };
        let slide = Slide {
            blocks: vec![
                Block::Heading {
                    level: 2,
                    inlines: text("Title"),
                },
                Block::List {
                    ordered: false,
                    items: vec![
                        item(ListMarker::Static, "a"),
                        item(ListMarker::NextStep, "b"),
                    ],
                },
            ],
            ..Default::default()
        };
        let l = lines(&slide);
        assert_eq!((l[0].step, l[1].step), (0, 1));
    }

    #[test]
    fn reports_what_it_cannot_show() {
        let slide = Slide {
            line: 12,
            blocks: vec![Block::Visual {
                tag: "bar".into(),
                content: String::new(),
            }],
            ..Default::default()
        };
        let p = Poster.unsupported(&slide);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].line, Some(12));
    }
}
