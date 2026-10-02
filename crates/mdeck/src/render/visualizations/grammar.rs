//! The one grammar every visual fence uses (VIZ-03):
//!
//! - **settings** are `key: value` lines before the first item;
//! - **items** are list lines (`- `, `+ `) with optional trailing
//!   `(key: value, ...)` attributes; a kind's verbs (`petal`, `lens`,
//!   `commit`) are the first word of an item;
//! - **relations** are items of the form `A -> B: label`;
//! - `#` starts a comment: a whole line, or the rest of a line after
//!   whitespace (`image: a.png  # white-hot`).
//!
//! [`Source::parse`] splits a block into these parts and records the lines
//! that fit none of them as [`Problem`]s; each visual then reads the parts
//! and adds its own problems (unknown settings and attributes, bad values),
//! which `--check` reports with their line (VIZ-04).

use std::cell::RefCell;

use super::VizReveal;

/// Something in a block that does not parse, with its 0-based line within
/// the block (the line after the opening fence is 0).
#[derive(Debug, Clone, PartialEq)]
pub struct Problem {
    pub offset: usize,
    pub message: String,
}

/// A `key: value` setting line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Setting<'a> {
    pub offset: usize,
    pub key: &'a str,
    pub value: &'a str,
}

/// A list line: its reveal marker, its text without attributes, and the
/// attributes.
#[derive(Debug, Clone, PartialEq)]
pub struct Item<'a> {
    pub offset: usize,
    pub reveal: VizReveal,
    /// Leading whitespace before the marker (0 for a top-level item).
    pub indent: usize,
    /// The text after the marker, without the attributes, trimmed.
    pub text: &'a str,
    pub attrs: Vec<(&'a str, &'a str)>,
}

/// The arrows a relation can use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrow {
    /// `->`
    Forward,
    /// `<-`
    Reverse,
    /// `<->`
    Both,
    /// `--`
    Line,
    /// `-->`
    Dashed,
}

impl Arrow {
    /// Every arrow, longest token first so `-->` is not read as `--`.
    pub const ALL: [Arrow; 5] = [
        Arrow::Both,
        Arrow::Dashed,
        Arrow::Forward,
        Arrow::Reverse,
        Arrow::Line,
    ];
    /// Only `->`.
    pub const FORWARD: [Arrow; 1] = [Arrow::Forward];

    pub fn token(self) -> &'static str {
        match self {
            Arrow::Forward => "->",
            Arrow::Reverse => "<-",
            Arrow::Both => "<->",
            Arrow::Line => "--",
            Arrow::Dashed => "-->",
        }
    }
}

/// `A -> B: label`.
#[derive(Debug, Clone, PartialEq)]
pub struct Relation<'a> {
    pub from: &'a str,
    pub arrow: Arrow,
    pub to: &'a str,
    pub label: Option<&'a str>,
}

/// A visual block split into settings and items.
#[derive(Debug)]
pub struct Source<'a> {
    pub settings: Vec<Setting<'a>>,
    pub items: Vec<Item<'a>>,
    /// v1-style `# key: value` comment lines, so a kind can say that a
    /// setting it knows is now written without `#`.
    commented: Vec<Setting<'a>>,
    problems: RefCell<Vec<Problem>>,
}

impl<'a> Source<'a> {
    pub fn parse(content: &'a str) -> Self {
        let mut src = Source {
            settings: Vec::new(),
            items: Vec::new(),
            commented: Vec::new(),
            problems: RefCell::new(Vec::new()),
        };
        for (offset, raw) in content.lines().enumerate() {
            let trimmed = raw.trim();
            if trimmed.starts_with('#') {
                if let Some((key, value)) = setting_line(trimmed.trim_start_matches('#').trim()) {
                    src.commented.push(Setting { offset, key, value });
                }
                continue;
            }
            let line = strip_comment(raw);
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some((reveal, body)) = marker(trimmed) {
                let (text, attrs) = split_attrs(body);
                src.items.push(Item {
                    offset,
                    reveal,
                    indent: line.len() - line.trim_start().len(),
                    text,
                    attrs,
                });
            } else if let Some((key, value)) = setting_line(trimmed) {
                if !src.items.is_empty() {
                    src.problem(
                        offset,
                        format!("setting '{key}' must come before the first item"),
                    );
                }
                src.settings.push(Setting { offset, key, value });
            } else {
                src.problem(
                    offset,
                    format!("'{trimmed}' is not a setting (key: value) or an item (- ...)"),
                );
            }
        }
        src
    }

    /// The value of setting `key` (any case); the last one wins.
    pub fn setting(&self, key: &str) -> Option<&'a str> {
        self.settings
            .iter()
            .rev()
            .find(|s| s.key.eq_ignore_ascii_case(key))
            .map(|s| s.value)
    }

    /// The setting line for `key` (any case); the last one wins.
    pub fn setting_line(&self, key: &str) -> Option<&Setting<'a>> {
        self.settings
            .iter()
            .rev()
            .find(|s| s.key.eq_ignore_ascii_case(key))
    }

    /// Setting `key` as one of `options` (any case); another value is
    /// reported and read as unset.
    pub fn choice(&self, key: &str, options: &[&'static str]) -> Option<&'static str> {
        let s = self.setting_line(key)?;
        let found = options.iter().find(|o| o.eq_ignore_ascii_case(s.value));
        if found.is_none() {
            self.problem(
                s.offset,
                format!("{key}: '{}' is not one of {}", s.value, options.join(", ")),
            );
        }
        found.copied()
    }

    /// Report settings not in `known`, and `# key:` comments for a known
    /// key (the v1 spelling).
    pub fn check_settings(&self, known: &[&str]) {
        for s in &self.settings {
            if !known.iter().any(|k| k.eq_ignore_ascii_case(s.key)) {
                let hint = if known.is_empty() {
                    "this visual has no settings".to_string()
                } else {
                    format!("known settings: {}", known.join(", "))
                };
                self.problem(s.offset, format!("unknown setting '{}' ({hint})", s.key));
            }
        }
        for s in &self.commented {
            if known.iter().any(|k| k.eq_ignore_ascii_case(s.key)) {
                self.problem(
                    s.offset,
                    format!(
                        "'#' starts a comment; write the setting as '{}: {}'",
                        s.key, s.value
                    ),
                );
            }
        }
    }

    /// Record a problem on block line `offset`.
    pub fn problem(&self, offset: usize, message: impl Into<String>) {
        self.problems.borrow_mut().push(Problem {
            offset,
            message: message.into(),
        });
    }

    /// Every problem found, in line order.
    pub fn into_problems(self) -> Vec<Problem> {
        let mut p = self.problems.into_inner();
        p.sort_by_key(|p| p.offset);
        p
    }
}

impl<'a> Item<'a> {
    /// The value of attribute `key` (any case); the last one wins.
    pub fn attr(&self, key: &str) -> Option<&'a str> {
        self.attrs
            .iter()
            .rev()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| *v)
    }

    /// Report attributes not in `known`.
    pub fn check_attrs(&self, src: &Source, known: &[&str]) {
        for (key, _) in &self.attrs {
            if !known.iter().any(|k| k.eq_ignore_ascii_case(key)) {
                let hint = if known.is_empty() {
                    "these items take no attributes".to_string()
                } else {
                    format!("known attributes: {}", known.join(", "))
                };
                src.problem(self.offset, format!("unknown attribute '{key}' ({hint})"));
            }
        }
    }

    /// The text as `label: value`, split at the first `": "`.
    pub fn label_value(&self) -> Option<(&'a str, &'a str)> {
        let (l, v) = self.text.split_once(": ")?;
        Some((l.trim(), v.trim()))
    }

    /// The first word of the text when it is one of `words` (any case), and
    /// the rest. `words` are a kind's verbs (`petal`, `lens`).
    pub fn keyword(&self, words: &[&'static str]) -> Option<(&'static str, &'a str)> {
        let (head, rest) = self
            .text
            .split_once(char::is_whitespace)
            .unwrap_or((self.text, ""));
        let word = words.iter().find(|w| head.eq_ignore_ascii_case(w))?;
        Some((word, rest.trim()))
    }

    /// The text as a relation using one of `arrows`.
    pub fn relation(&self, arrows: &[Arrow]) -> Option<Relation<'a>> {
        relation(self.text, arrows)
    }
}

/// `text` as `A -> B: label`, with the arrow surrounded by spaces.
pub fn relation<'a>(text: &'a str, arrows: &[Arrow]) -> Option<Relation<'a>> {
    let (pos, arrow) = Arrow::ALL
        .iter()
        .filter(|a| arrows.contains(a))
        .filter_map(|&a| text.find(&format!(" {} ", a.token())).map(|p| (p, a)))
        .min_by_key(|&(p, a)| (p, std::cmp::Reverse(a.token().len())))?;
    let from = text[..pos].trim();
    let rest = &text[pos + arrow.token().len() + 2..];
    let (to, label) = match rest.split_once(':') {
        Some((t, l)) => (t.trim(), Some(l.trim()).filter(|l| !l.is_empty())),
        None => (rest.trim(), None),
    };
    if from.is_empty() || to.is_empty() {
        return None;
    }
    Some(Relation {
        from,
        arrow,
        to,
        label,
    })
}

/// `"text"` or `'text'` without its quotes; other text as it is.
pub fn unquote(s: &str) -> &str {
    let s = s.trim();
    for q in ['"', '\''] {
        if let Some(inner) = s.strip_prefix(q).and_then(|r| r.strip_suffix(q)) {
            return inner;
        }
    }
    s
}

/// The reveal marker and the rest of an item line.
fn marker(trimmed: &str) -> Option<(VizReveal, &str)> {
    let reveal = match trimmed.chars().next()? {
        '-' | '*' => VizReveal::Static,
        '+' => VizReveal::NextStep,
        _ => return None,
    };
    let rest = &trimmed[1..];
    if rest.is_empty() {
        return Some((reveal, ""));
    }
    rest.starts_with(char::is_whitespace)
        .then(|| (reveal, rest.trim()))
}

/// `key: value` where the key is a word of letters, digits, `-` and `_`.
fn setting_line(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.split_once(':')?;
    let key_ok = key.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    key_ok.then(|| (key, value.trim()))
}

/// The line up to a `#` that follows whitespace and is followed by
/// whitespace or the end (outside quotes): `PR #42` keeps its `#`.
fn strip_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut quote: Option<u8> = None;
    for (i, &b) in bytes.iter().enumerate() {
        match quote {
            Some(q) if b == q => quote = None,
            Some(_) => {}
            None if b == b'"' => quote = Some(b),
            None if b == b'#'
                && i > 0
                && bytes[i - 1].is_ascii_whitespace()
                && bytes.get(i + 1).is_none_or(|c| c.is_ascii_whitespace()) =>
            {
                return &line[..i];
            }
            None => {}
        }
    }
    line
}

/// Split trailing `(key: value, ...)` attributes off an item's text. A
/// trailing parenthesis that is not a list of `key: value` pairs (`Revenue
/// (USD)`) is part of the text.
fn split_attrs(body: &str) -> (&str, Vec<(&str, &str)>) {
    let body = body.trim_end();
    if !body.ends_with(')') {
        return (body, Vec::new());
    }
    let Some(open) = matching_open(body) else {
        return (body, Vec::new());
    };
    if open > 0 && !body[..open].ends_with(char::is_whitespace) {
        return (body, Vec::new());
    }
    let inner = &body[open + 1..body.len() - 1];
    let mut attrs = Vec::new();
    for part in split_attr_list(inner) {
        let Some((key, value)) = setting_line(part.trim()) else {
            return (body, Vec::new());
        };
        attrs.push((key, unquote(value)));
    }
    if attrs.is_empty() {
        return (body, Vec::new());
    }
    (body[..open].trim_end(), attrs)
}

/// The index of the `(` that the final `)` closes, outside quotes.
fn matching_open(body: &str) -> Option<usize> {
    let bytes = body.as_bytes();
    let mut depth = 0i32;
    let mut quote: Option<u8> = None;
    for i in (0..bytes.len()).rev() {
        let b = bytes[i];
        match quote {
            Some(q) if b == q => quote = None,
            Some(_) => {}
            None if b == b'"' => quote = Some(b),
            None if b == b')' => depth += 1,
            None if b == b'(' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            None => {}
        }
    }
    None
}

/// Split `icon: user, pos: 1,2, prompt: "a, b"` at the commas that start a
/// new `key:` (outside quotes), so values may hold commas.
fn split_attr_list(inner: &str) -> Vec<&str> {
    let bytes = inner.as_bytes();
    let mut parts = Vec::new();
    let mut start = 0;
    let mut quote: Option<u8> = None;
    for (i, &b) in bytes.iter().enumerate() {
        match quote {
            Some(q) if b == q => quote = None,
            Some(_) => {}
            None if b == b'"' => quote = Some(b),
            None if b == b',' && setting_line(inner[i + 1..].trim_start()).is_some() => {
                let next = inner[i + 1..].trim_start();
                // only when the next part starts with `key:` itself
                let key_end = next.find(':').unwrap_or(0);
                if !next[..key_end].contains(char::is_whitespace) {
                    parts.push(&inner[start..i]);
                    start = i + 1;
                }
            }
            None => {}
        }
    }
    parts.push(&inner[start..]);
    parts
}

// ─── Shared readers ─────────────────────────────────────────────────────────

/// A `- Label: value` item read as a number.
#[derive(Debug, Clone, PartialEq)]
pub struct LabelValue {
    pub label: String,
    pub value: f32,
    pub reveal: VizReveal,
}

/// A `- Label: v1, v2, ...` item read as numbers.
#[derive(Debug, Clone, PartialEq)]
pub struct LabelValues {
    pub label: String,
    pub values: Vec<f32>,
    pub reveal: VizReveal,
}

/// Every item as `Label: value`; items that are not are reported with
/// `example` (`- Sales: 40`) and left out.
pub fn label_value_items(src: &Source, example: &str) -> Vec<LabelValue> {
    src.items
        .iter()
        .filter_map(|item| {
            item.check_attrs(src, &[]);
            match super::parse_label_value(item.text) {
                Some((label, value)) => Some(LabelValue {
                    label,
                    value,
                    reveal: item.reveal,
                }),
                None => {
                    src.problem(
                        item.offset,
                        format!(
                            "'{}' is not a label and a number, e.g. '{example}'",
                            item.text
                        ),
                    );
                    None
                }
            }
        })
        .collect()
}

/// Every item as `Label: v1, v2, ...`; values that do not parse are
/// reported, and an item without any is left out.
pub fn label_values_items(src: &Source, example: &str) -> Vec<LabelValues> {
    src.items
        .iter()
        .filter_map(|item| {
            item.check_attrs(src, &[]);
            let Some((label, raw)) = item.label_value() else {
                src.problem(
                    item.offset,
                    format!(
                        "'{}' is not a label and numbers, e.g. '{example}'",
                        item.text
                    ),
                );
                return None;
            };
            let raw = super::strip_thousands_separators(raw);
            let mut values = Vec::new();
            for v in raw.split(',') {
                match super::parse_value(v) {
                    Some(n) => values.push(n),
                    None => src.problem(item.offset, format!("'{}' is not a number", v.trim())),
                }
            }
            (!values.is_empty()).then(|| LabelValues {
                label: label.to_string(),
                values,
                reveal: item.reveal,
            })
        })
        .collect()
}

/// A comma-separated setting value as trimmed names.
pub fn name_list(value: &str) -> Vec<String> {
    value.split(',').map(|s| s.trim().to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_items_comments_and_problems() {
        let src = Source::parse(
            "# a comment\n\
             title: Plan  # trailing comment\n\
             \n\
             - A: 1\n\
             + B: 2 (size: 3)\n\
             * C\n\
             stray line\n\
             late: 1\n",
        );
        assert_eq!(src.setting("title"), Some("Plan"));
        assert_eq!(src.items.len(), 3);
        assert_eq!(src.items[1].text, "B: 2");
        assert_eq!(src.items[1].attr("size"), Some("3"));
        assert_eq!(src.items[1].offset, 4);
        let p = src.into_problems();
        assert_eq!(p.iter().map(|p| p.offset).collect::<Vec<_>>(), [6, 7]);
    }

    #[test]
    fn attributes_keep_commas_quotes_and_plain_parentheses() {
        let src = Source::parse(
            "- Gateway (icon: api, pos: 1,2, prompt: \"A router, with: colons\")\n\
             - Revenue (USD): 4\n\
             - Spend (2024)\n\
             - Merge: \"PR #42\"\n",
        );
        let gw = &src.items[0];
        assert_eq!(gw.text, "Gateway");
        assert_eq!(gw.attr("pos"), Some("1,2"));
        assert_eq!(gw.attr("prompt"), Some("A router, with: colons"));
        assert_eq!(src.items[1].text, "Revenue (USD): 4");
        assert_eq!(src.items[2].text, "Spend (2024)");
        assert!(src.items[2].attrs.is_empty());
        assert_eq!(src.items[3].text, "Merge: \"PR #42\"");
    }

    #[test]
    fn relations_keywords_and_label_values() {
        let src = Source::parse(
            "- A --> B: calls\n- petal Payments: takes money\n- x-y -> z\n- A <-> B\n",
        );
        let r = src.items[0].relation(&Arrow::ALL).unwrap();
        assert_eq!(
            (r.from, r.arrow, r.to, r.label),
            ("A", Arrow::Dashed, "B", Some("calls"))
        );
        assert_eq!(src.items[0].relation(&Arrow::FORWARD), None);
        assert_eq!(
            src.items[1].keyword(&["petal"]),
            Some(("petal", "Payments: takes money"))
        );
        assert_eq!(src.items[1].keyword(&["center"]), None);
        let r = src.items[2].relation(&Arrow::FORWARD).unwrap();
        assert_eq!((r.from, r.to), ("x-y", "z"));
        assert_eq!(
            src.items[3].relation(&Arrow::ALL).unwrap().arrow,
            Arrow::Both
        );
        assert_eq!(
            src.items[1].label_value(),
            Some(("petal Payments", "takes money"))
        );
    }

    #[test]
    fn unknown_settings_attributes_and_v1_comments_are_reported() {
        let src = Source::parse("# x-label: Old\ncolour: red\n- A: 1 (shade: 2)\n");
        src.check_settings(&["x-label"]);
        for item in &src.items {
            item.check_attrs(&src, &["size"]);
        }
        let p = src.into_problems();
        assert_eq!(p.len(), 3, "{p:?}");
        assert!(p[0].message.contains("x-label: Old"));
        assert!(p[1].message.contains("colour"));
        assert!(p[2].message.contains("shade"));
    }

    #[test]
    fn a_bare_marker_is_an_empty_item() {
        let src = Source::parse("-\n+\n-not an item\n");
        assert_eq!(src.items.len(), 2);
        assert_eq!(src.into_problems().len(), 1);
    }
}
