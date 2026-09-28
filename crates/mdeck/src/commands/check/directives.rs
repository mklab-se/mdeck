//! `@name: value` lines that do not do what their author meant.

use crate::check::{CheckCategory, CheckWarning};
use crate::parser::{self, GLOBAL_DIRECTIVES, SLIDE_DIRECTIVES, blocks, splitter::FenceTracker};

/// Directive warnings, per slide: unknown names (with a "did you mean"),
/// known directives that were not applied because of where they stand (in a
/// list item, a quote, indented), global directives inside a slide, and slide
/// directives written twice.
pub fn directive_warnings(presentation: &parser::Presentation) -> Vec<CheckWarning> {
    let mut out = Vec::new();
    for (i, slide) in presentation.slides.iter().enumerate() {
        let mut warn = |line: usize, message: String| {
            out.push(CheckWarning {
                slide: i + 1,
                line,
                category: CheckCategory::Directive,
                message,
            })
        };
        let mut seen: Vec<&str> = Vec::new();
        for d in &slide.directives {
            let name = d.name.as_str();
            if GLOBAL_DIRECTIVES.contains(&name) {
                warn(
                    d.line,
                    format!("@{name} is ignored inside a slide; it belongs in the frontmatter"),
                );
            } else if !SLIDE_DIRECTIVES.contains(&name) {
                warn(d.line, unknown(name, "is ignored"));
            } else if seen.contains(&name) {
                warn(
                    d.line,
                    format!("@{name} is written twice; the last one wins"),
                );
            }
            seen.push(name);
        }
        for (offset, line) in body_lines(&slide.raw_source) {
            let at = slide.line_at(offset);
            let (nested, text) = strip_container(line);
            let Some(d) = blocks::parse_directive_line(text.trim()) else {
                continue;
            };
            let known = parser::is_known_directive(&d.name);
            if nested && known {
                warn(
                    at,
                    format!(
                        "`@{}: {}` is not applied inside a list, quote or indented block; \
                     put it on its own line under the heading",
                        d.name, d.value
                    ),
                );
            } else if !nested && !known && suggestion(&d.name).is_some() {
                // A typo in the body stays text on the slide. Leading unknown
                // names were removed and are reported above.
                let removed = slide
                    .directives
                    .iter()
                    .any(|s| s.name == d.name && s.value == d.value);
                if !removed {
                    warn(at, unknown(&d.name, "shows as text"));
                }
            }
        }
    }
    out
}

/// "`@x` is not a directive and `what`", with a suggestion when a known
/// name is close.
fn unknown(name: &str, what: &str) -> String {
    match suggestion(name) {
        Some(s) => format!("@{name} is not a directive and {what}; did you mean @{s}?"),
        None => format!("@{name} is not a directive and {what}"),
    }
}

/// The known directive closest to `name`, if it is a likely typo.
fn suggestion(name: &str) -> Option<&'static str> {
    SLIDE_DIRECTIVES
        .iter()
        .chain(GLOBAL_DIRECTIVES)
        .map(|k| (edit_distance(name, k), *k))
        .filter(|&(d, k)| d > 0 && d <= 2 && d < k.len() / 2 + 1)
        .min_by_key(|&(d, _)| d)
        .map(|(_, k)| k)
}

fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != *cb);
            cur.push((prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1));
        }
        prev = cur;
    }
    prev[b.len()]
}

/// The slide's lines outside code fences, up to its speaker notes, with
/// their 0-based line in `raw`.
fn body_lines(raw: &str) -> Vec<(usize, &str)> {
    let mut fences = FenceTracker::new();
    raw.lines()
        .enumerate()
        .take_while(|(_, l)| l.trim() != "???")
        .filter(|(_, l)| !fences.observe(l))
        .collect()
}

/// Strip list markers, quote markers and indentation. Returns whether any
/// were there (a directive there is not applied) and the rest of the line.
fn strip_container(line: &str) -> (bool, &str) {
    let mut rest = line.trim_start();
    let mut nested = rest.len() != line.len();
    loop {
        let before = rest;
        for marker in ["- ", "* ", "+ ", "> "] {
            if let Some(r) = rest.strip_prefix(marker) {
                rest = r.trim_start();
            }
        }
        if let Some(pos) = rest.find(". ")
            && pos > 0
            && rest[..pos].chars().all(|c| c.is_ascii_digit())
        {
            rest = rest[pos + 2..].trim_start();
        }
        if rest.len() == before.len() {
            break;
        }
        nested = true;
    }
    (nested, rest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn warnings(md: &str) -> Vec<String> {
        directive_warnings(&parser::parse(md))
            .into_iter()
            .map(|w| format!("{}: {}", w.slide, w.message))
            .collect()
    }

    fn lines(md: &str) -> Vec<(usize, usize)> {
        directive_warnings(&parser::parse(md))
            .into_iter()
            .map(|w| (w.slide, w.line))
            .collect()
    }

    #[test]
    fn warnings_name_the_directive_line() {
        // Frontmatter, a leading typo, a nested directive and a duplicate.
        let md = "---\ntitle: T\n---\n\n@ilustration: a\n# A\n\n- @layout: code\n\n---\n\n# B\n@layout: code\n\n@layout: quote\n";
        assert_eq!(lines(md), [(1, 5), (1, 8), (2, 15)]);
        // A directive moved to the next heading's slide keeps its line.
        let md = "# A\n\n- one\n\n@theme: dark\n\n# B\n\n- two\n";
        assert_eq!(lines(md), [(2, 5)]);
    }

    #[test]
    fn clean_directives_are_silent() {
        let md =
            "# A\n@illustration: account\n\n- one\n\n# B\n\n@layout: two-column\n\nx\n\n+++\n\ny\n";
        assert!(warnings(md).is_empty(), "{:?}", warnings(md));
        // Prose that happens to start with @ is not a directive.
        assert!(warnings("# A\n\n@team: see you at five\n").is_empty());
    }

    #[test]
    fn typo_is_reported_with_suggestion() {
        let w = warnings("@ilustration: account\n# A\n\n- one\n");
        assert_eq!(
            w,
            ["1: @ilustration is not a directive and is ignored; did you mean @illustration?"]
        );
        let w = warnings("# A\n\n- one\n\n@ilustration: account\n\n# B\n\nx\n");
        assert!(
            w.iter().any(|m| m.contains("did you mean @illustration")),
            "{w:?}"
        );
    }

    #[test]
    fn nested_directive_is_not_applied() {
        let w = warnings("# A\n\n- @illustration: account\n- two\n");
        assert_eq!(w.len(), 1);
        assert!(w[0].contains("not applied inside a list"), "{w:?}");
    }

    #[test]
    fn global_directive_in_slide_and_duplicates() {
        let w = warnings("# A\n@theme: dark\n@layout: bullets\n\n- one\n\n@layout: content\n");
        assert!(
            w.iter()
                .any(|m| m.contains("@theme is ignored inside a slide")),
            "{w:?}"
        );
        assert!(
            w.iter().any(|m| m.contains("@layout is written twice")),
            "{w:?}"
        );
    }

    #[test]
    fn directives_in_code_and_notes_are_ignored() {
        let md = "# A\n\n```text\n- @layout: two-column\n```\n\n???\n\n- @illustration: x\n";
        assert!(warnings(md).is_empty(), "{:?}", warnings(md));
    }
}
