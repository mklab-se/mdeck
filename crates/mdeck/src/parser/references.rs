//! Reference-style link definitions (`[label]: url`) and footnotes
//! (`[^id]: text`). Definitions may stand anywhere in the deck, so they are
//! collected from the whole body before it is split, and their lines are
//! blanked: they never show on a slide. While a deck parses, the inline
//! parser resolves `[text][label]` against them and hides footnote markers,
//! remembering which footnotes the current slide uses so their text can be
//! appended to its notes.

use std::cell::RefCell;
use std::collections::HashMap;

use super::splitter::FenceTracker;

/// The deck's definitions.
#[derive(Debug, Default, Clone)]
pub struct Definitions {
    /// Link labels (lowercase, whitespace collapsed) and their urls.
    links: HashMap<String, String>,
    /// Footnote ids and their text, in the order they are defined.
    footnotes: Vec<(String, String)>,
}

impl Definitions {
    pub fn link(&self, label: &str) -> Option<&str> {
        self.links.get(&normalize(label)).map(String::as_str)
    }

    fn footnote(&self, id: &str) -> Option<&str> {
        self.footnotes
            .iter()
            .find(|(i, _)| i == id)
            .map(|(_, t)| t.as_str())
    }
}

/// Collapse a label as CommonMark matches them: case and runs of spaces
/// do not matter.
fn normalize(label: &str) -> String {
    label
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Take the definitions out of `body`: their lines become blank, so the
/// lines of everything else stay where they were.
pub fn collect(body: &str) -> (String, Definitions) {
    let mut defs = Definitions::default();
    let lines: Vec<&str> = body.split('\n').collect();
    let mut out: Vec<&str> = Vec::with_capacity(lines.len());
    let mut fences = FenceTracker::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if fences.observe(line) || line.len() - line.trim_start().len() > 3 {
            out.push(line);
            i += 1;
            continue;
        }
        if let Some((id, text)) = footnote_definition(line) {
            // Indented lines after it continue the footnote.
            let mut text = text.to_string();
            out.push("");
            i += 1;
            while i < lines.len()
                && lines[i].starts_with(['\t', ' '])
                && !lines[i].trim().is_empty()
            {
                text.push(' ');
                text.push_str(lines[i].trim());
                out.push("");
                i += 1;
            }
            defs.footnotes.push((id.to_string(), text));
            continue;
        }
        if let Some((label, url)) = link_definition(line) {
            defs.links
                .entry(normalize(label))
                .or_insert_with(|| url.to_string());
            out.push("");
            i += 1;
            continue;
        }
        out.push(line);
        i += 1;
    }
    (out.join("\n"), defs)
}

/// `[^id]: text`.
fn footnote_definition(line: &str) -> Option<(&str, &str)> {
    let rest = line.trim_start().strip_prefix("[^")?;
    let (id, rest) = rest.split_once("]:")?;
    (!id.is_empty() && !id.contains(char::is_whitespace)).then(|| (id, rest.trim()))
}

/// `[label]: url "optional title"`.
fn link_definition(line: &str) -> Option<(&str, &str)> {
    let rest = line.trim_start().strip_prefix('[')?;
    if rest.starts_with('^') {
        return None;
    }
    let (label, rest) = rest.split_once("]:")?;
    let url = rest.split_whitespace().next()?;
    let url = url.trim_start_matches('<').trim_end_matches('>');
    (!label.trim().is_empty() && !url.is_empty()).then_some((label, url))
}

thread_local! {
    static CURRENT: RefCell<Option<Definitions>> = const { RefCell::new(None) };
    static USED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// Run `f` with `defs` as the definitions the inline parser resolves against.
pub fn with<T>(defs: Definitions, f: impl FnOnce() -> T) -> T {
    let previous = CURRENT.with(|c| c.replace(Some(defs)));
    let out = f();
    CURRENT.with(|c| *c.borrow_mut() = previous);
    out
}

/// The url of the link label `label`, while a deck parses.
pub fn resolve(label: &str) -> Option<String> {
    CURRENT.with(|c| c.borrow().as_ref()?.link(label).map(str::to_string))
}

/// Note that the current slide shows footnote `id`'s marker.
pub fn use_footnote(id: &str) {
    USED.with(|u| {
        let mut u = u.borrow_mut();
        if !u.iter().any(|x| x == id) {
            u.push(id.to_string());
        }
    });
}

/// The footnotes the slide used since the last call, as notes text
/// (`[^1]: text` per line), and the ids that have no definition.
pub fn take_footnotes() -> (Option<String>, Vec<String>) {
    let used = USED.with(|u| std::mem::take(&mut *u.borrow_mut()));
    CURRENT.with(|c| {
        let c = c.borrow();
        let mut lines = Vec::new();
        let mut missing = Vec::new();
        for id in used {
            match c.as_ref().and_then(|d| d.footnote(&id)) {
                Some(text) => lines.push(format!("[^{id}]: {text}")),
                None => missing.push(id),
            }
        }
        ((!lines.is_empty()).then(|| lines.join("\n\n")), missing)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definitions_are_collected_and_blanked() {
        let body = "# A\n\nSee [docs][d].[^1]\n\n[d]: https://x.dev \"Docs\"\n[^1]: A footnote\n    that wraps.\n\n```\n[e]: kept\n```";
        let (out, defs) = collect(body);
        assert_eq!(out.lines().count(), body.lines().count());
        assert!(!out.contains("https://x.dev"));
        assert!(out.contains("[e]: kept"), "inside a fence stays");
        assert_eq!(defs.link("D"), Some("https://x.dev"));
        assert_eq!(defs.footnote("1"), Some("A footnote that wraps."));
    }

    #[test]
    fn footnotes_used_by_a_slide_become_notes() {
        let (_, defs) = collect("[^a]: Alpha\n[^b]: Beta");
        with(defs, || {
            use_footnote("b");
            use_footnote("zzz");
            let (notes, missing) = take_footnotes();
            assert_eq!(notes.as_deref(), Some("[^b]: Beta"));
            assert_eq!(missing, ["zzz"]);
            assert_eq!(take_footnotes(), (None, vec![]));
        });
        assert_eq!(resolve("a"), None, "outside a parse nothing resolves");
    }
}
