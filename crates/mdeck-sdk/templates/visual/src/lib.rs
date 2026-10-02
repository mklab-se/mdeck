//! The `@{{name}}` visual: labelled values drawn as horizontal bars.
//!
//! Made with `mdeck sdk new visual {{name}}`. The fence uses mdeck's one
//! grammar for visuals:
//!
//! ~~~text
//! ```@{{name}}
//! title: Revenue by region     # settings: `key: value` lines before the first item
//! max: 100
//! - North: 42                  # items: list lines, `label: value`
//! + South: 31 (color: 2)       # `+` reveals the item on the next step
//! ```
//! ~~~
//!
//! `#` starts a comment anywhere.

use mdeck_sdk::geometry::Hint;
use mdeck_sdk::paint::{Align2, Font, Pos2, Rect, Vec2};
use mdeck_sdk::problem::Problem;
use mdeck_sdk::registry::{Registry, RegistryError};
use mdeck_sdk::visual::{Visual, VisualCx};

/// The entry point `mdeck build --with` calls: register what this crate brings.
pub fn register(r: &mut Registry) -> Result<(), RegistryError> {
    r.visual(Box::new(Bars))?;
    r.theme("{{name}}", include_str!("../theme.yaml"))
}

/// The visual kind. It holds no state: everything comes from the fence.
pub struct Bars;

/// One item: `- label: value (color: n)`.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    /// The label before the colon.
    pub label: String,
    /// The value after it.
    pub value: f32,
    /// The series colour index, from `(color: n)`.
    pub color: usize,
    /// The reveal step that shows it (0: with the slide).
    pub step: usize,
}

/// A parsed fence: settings, items and the problems found on the way.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Parsed {
    /// `title:` setting.
    pub title: Option<String>,
    /// `max:` setting (default: the largest value).
    pub max: Option<f32>,
    /// The items in order.
    pub items: Vec<Item>,
    /// Problems, with 1-based line numbers inside the fence.
    pub problems: Vec<Problem>,
}

/// Parse the fence body.
pub fn parse(src: &str) -> Parsed {
    let mut out = Parsed::default();
    let mut step = 0;
    for (n, raw) in src.lines().enumerate() {
        let line_no = n + 1;
        // `#` always starts a comment
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let problem = |msg: String| Problem::new("visual", msg).at(line_no);
        if let Some(rest) = line.strip_prefix("- ").or(line.strip_prefix("+ ")) {
            if line.starts_with('+') {
                step += 1;
            }
            match item(rest, step) {
                Ok(i) => out.items.push(i),
                Err(e) => out.problems.push(problem(e)),
            }
        } else if !out.items.is_empty() {
            out.problems.push(problem(format!(
                "settings go before the first item: `{line}`"
            )));
        } else {
            match line.split_once(':').map(|(k, v)| (k.trim(), v.trim())) {
                Some(("title", v)) => out.title = Some(v.to_string()),
                Some(("max", v)) => match v.parse() {
                    Ok(m) => out.max = Some(m),
                    Err(_) => out
                        .problems
                        .push(problem(format!("`max` should be a number, not `{v}`"))),
                },
                Some((k, _)) => out.problems.push(problem(format!("unknown setting `{k}`"))),
                None => out.problems.push(problem(format!(
                    "expected `key: value` or an item, not `{line}`"
                ))),
            }
        }
    }
    if out.items.is_empty() {
        out.problems.push(Problem::new(
            "visual",
            "@{{name}} needs at least one item",
        ));
    }
    out
}

fn item(text: &str, step: usize) -> Result<Item, String> {
    // optional `(key: value, ...)` attributes at the end
    let (body, attrs) = match text.rsplit_once('(') {
        Some((b, a)) if a.ends_with(')') => (b.trim(), a.trim_end_matches(')')),
        _ => (text, ""),
    };
    let (label, value) = body
        .split_once(':')
        .ok_or_else(|| format!("an item is `label: value`, not `{body}`"))?;
    let value: f32 = value
        .trim()
        .parse()
        .map_err(|_| format!("`{}` is not a number", value.trim()))?;
    let mut color = 0;
    for attr in attrs.split(',').filter(|a| !a.trim().is_empty()) {
        match attr.split_once(':').map(|(k, v)| (k.trim(), v.trim())) {
            Some(("color", v)) => {
                color = v
                    .parse()
                    .map_err(|_| format!("`color` should be a number, not `{v}`"))?
            }
            _ => return Err(format!("unknown attribute `{}`", attr.trim())),
        }
    }
    Ok(Item {
        label: label.trim().to_string(),
        value,
        color,
        step,
    })
}

impl Visual for Bars {
    /// The fence tag without `@`.
    fn tag(&self) -> &str {
        "{{name}}"
    }

    /// One sentence for `mdeck spec` and the docs.
    fn summary(&self) -> &str {
        "Labelled values as horizontal bars."
    }

    /// Problems for `mdeck --check`. Never panic on bad input: report it.
    fn check(&self, src: &str) -> Vec<Problem> {
        parse(src).problems
    }

    /// How many reveal steps the fence adds (one per `+` item).
    fn steps(&self, src: &str) -> usize {
        parse(src).items.iter().map(|i| i.step).max().unwrap_or(0)
    }

    /// Draw into `rect` and return the height used. Publish what you draw,
    /// so engines can react to it.
    fn draw(&self, cx: &mut VisualCx, src: &str, rect: Rect, step: usize) -> f32 {
        let parsed = parse(src);
        let s = cx.scale();
        let tokens = cx.tokens().clone();
        let max = parsed
            .max
            .unwrap_or_else(|| parsed.items.iter().map(|i| i.value).fold(0.0, f32::max))
            .max(f32::EPSILON);
        let mut y = rect.top();
        if let Some(title) = &parsed.title {
            let r = cx.painter().text(
                Pos2::new(rect.left(), y),
                Align2::LEFT_TOP,
                title,
                Font::body(34.0 * s),
                tokens.heading,
            );
            y = r.bottom() + 16.0 * s;
        }
        let row = 64.0 * s;
        let label_w = rect.width() * 0.25;
        for item in &parsed.items {
            if y + row > rect.bottom() {
                break; // out of room: stop rather than overflow
            }
            if item.step <= step {
                let font = Font::body(28.0 * s);
                cx.painter().text(
                    Pos2::new(rect.left(), y + row / 2.0),
                    Align2::LEFT_CENTER,
                    &item.label,
                    font,
                    tokens.text,
                );
                let track = Rect::from_min_size(
                    Pos2::new(rect.left() + label_w, y + row * 0.2),
                    Vec2::new(rect.width() - label_w, row * 0.6),
                );
                let bar = track.sub_rect(0.0, 0.0, (item.value / max).clamp(0.0, 1.0), 1.0);
                cx.painter().rect_filled(track, 6.0 * s, tokens.rule);
                cx.painter()
                    .rect_filled(bar, 6.0 * s, tokens.series_color(item.color));
                cx.publish(Hint::Bar(bar));
            }
            y += row;
        }
        y - rect.top()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_settings_items_and_steps() {
        let p = parse("title: T\nmax: 10\n- A: 1\n+ B: 2 (color: 3) # comment\n");
        assert!(p.problems.is_empty(), "{:?}", p.problems);
        assert_eq!(p.title.as_deref(), Some("T"));
        assert_eq!(
            p.items[1],
            Item {
                label: "B".into(),
                value: 2.0,
                color: 3,
                step: 1
            }
        );
        assert_eq!(Bars.steps("- A: 1\n+ B: 2\n+ C: 3"), 2);
    }

    #[test]
    fn reports_problems_with_lines() {
        let p = Bars.check("- A: 1\n- B: lots\nmax: 3\n- C");
        let lines: Vec<_> = p.iter().map(|p| p.line).collect();
        assert_eq!(lines, [Some(2), Some(3), Some(4)]);
        assert_eq!(Bars.check("").len(), 1);
    }
}
