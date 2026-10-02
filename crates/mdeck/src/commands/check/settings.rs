//! Settings that do not do what their author meant: unknown names, invalid
//! values, deck settings in a slide (and slide settings in the
//! frontmatter), duplicates, and v1 syntax, each with the v2 form to write.

use crate::check::{CheckCategory, CheckWarning};
use crate::language::{self, Scope};
use crate::parser::{self, ProblemKind, splitter::FenceTracker};

/// Every settings warning: the frontmatter's (slide 0), then each slide's.
pub fn settings_warnings(presentation: &parser::Presentation) -> Vec<CheckWarning> {
    let mut out = Vec::new();
    let mut warn = |slide: usize, line: usize, message: String| {
        out.push(CheckWarning {
            slide,
            line,
            category: CheckCategory::Settings,
            message,
            place: None,
        })
    };
    for s in &presentation.meta.settings {
        if let Some(message) = deck_problem(&s.name, &s.value) {
            warn(0, s.line, message);
        }
    }
    for (i, slide) in presentation.slides.iter().enumerate() {
        let n = i + 1;
        let mut seen: Vec<&str> = Vec::new();
        for s in &slide.settings {
            let name = s.name.as_str();
            if let Some(message) = slide_problem(name, &s.value, &slide.settings) {
                warn(n, s.line, message);
            } else if seen.contains(&name) {
                warn(
                    n,
                    s.line,
                    format!("`{name}` is written twice; the last one wins"),
                );
            }
            seen.push(name);
        }
        for p in slide
            .problems
            .iter()
            .filter(|p| p.kind == ProblemKind::Setting)
        {
            warn(n, p.line, p.message.clone());
        }
        for (offset, line) in body_lines(&slide.raw_source) {
            if let Some(message) = v1_line(line) {
                warn(n, slide.line_at(offset), message);
            }
        }
        if n < presentation.slides.len()
            && let Some(line) = trailing_settings(slide)
        {
            warn(
                n,
                line,
                format!(
                    "these settings end slide {n}, just above slide {}'s heading; \
                     they apply to slide {n}. Move them under the heading if they were meant for slide {}",
                    n + 1,
                    n + 1
                ),
            );
        }
    }
    out
}

/// The line of a settings comment that closes a slide with other content
/// (only blank lines after it), which reads as if it belonged to the next
/// slide's heading (MD-08).
fn trailing_settings(slide: &parser::Slide) -> Option<usize> {
    let lines: Vec<&str> = slide.raw_source.lines().collect();
    let last = lines.iter().rposition(|l| !l.trim().is_empty())?;
    if !lines[last].trim_end().ends_with("-->") {
        return None;
    }
    let start = (0..=last).rev().find(|&i| lines[i].contains("<!--"))?;
    let from = slide.line_at(start);
    let in_comment = slide.settings.iter().any(|s| s.line >= from);
    // Directly under the slide's own heading it is plainly meant for it.
    let before = lines[..start].iter().rev().find(|l| !l.trim().is_empty())?;
    let under_heading = before.trim_start().starts_with('#');
    (in_comment && !under_heading).then_some(from)
}

/// What is wrong with frontmatter key `name: value`, if anything.
fn deck_problem(name: &str, value: &str) -> Option<String> {
    if let Some(v1) = name.strip_prefix('@') {
        return Some(match language::v1_replacement(v1, false) {
            Some(v2) if v2.starts_with('`') => {
                let key = v2.trim_start_matches('`').split(':').next().unwrap_or(v1);
                format!("`{name}: {value}` is v1 syntax; write `{key}: {value}`")
            }
            Some(v2) if v2.starts_with("nothing") => {
                format!("`{name}` is v1 syntax and was removed in v2")
            }
            Some(v2) => format!("`{name}` is v1 syntax; it maps to {v2}"),
            None => format!("`{name}` is v1 syntax and not a setting"),
        });
    }
    let Some(def) = language::setting(name) else {
        let deck_names = language::SETTINGS
            .iter()
            .filter(|s| s.scope.in_deck())
            .map(|s| s.name);
        return Some(unknown(name, "deck", deck_names));
    };
    if def.scope == Scope::Slide {
        return Some(format!(
            "`{name}` is a slide setting; write it in a slide as <!-- {name}: {value} -->"
        ));
    }
    if name == "transition" && value.trim() == "zoom" {
        return Some(
            "`transition: zoom` works on a slide with `zoom-to`, not for the whole deck".into(),
        );
    }
    language::invalid_value(def, value)
}

/// What is wrong with slide setting `name: value`, if anything.
fn slide_problem(name: &str, value: &str, all: &[parser::Setting]) -> Option<String> {
    let Some(def) = language::setting(name) else {
        let slide_names = language::SETTINGS
            .iter()
            .filter(|s| s.scope.in_slide())
            .map(|s| s.name);
        return Some(unknown(name, "slide", slide_names));
    };
    if def.scope == Scope::Deck {
        return Some(format!(
            "`{name}` is a deck setting; it belongs in the frontmatter, not in a slide"
        ));
    }
    if name == "transition" && value.trim() == "zoom" && parser::setting(all, "zoom-to").is_none() {
        return Some("`transition: zoom` needs `zoom-to: <spot>` on the same slide".into());
    }
    language::invalid_value(def, value)
}

/// "`x` is not a <scope> setting", with a suggestion when a name is close.
fn unknown<'a>(name: &str, scope: &str, names: impl IntoIterator<Item = &'a str>) -> String {
    match language::suggestion(name, names) {
        Some(s) => format!("`{name}` is not a {scope} setting; did you mean `{s}`?"),
        None => format!("`{name}` is not a {scope} setting"),
    }
}

/// A v1 construct on a slide line, with its v2 form: `@key: value` slide
/// directives and the `???` notes separator.
fn v1_line(line: &str) -> Option<String> {
    let text = strip_container(line).trim();
    if text.len() >= 3 && text.chars().all(|c| c == '?') {
        return Some(format!(
            "`{text}` is v1 syntax; put speaker notes in a ```@notes block"
        ));
    }
    let rest = text.strip_prefix('@')?;
    let (key, value) = parser::parse_setting_line(rest)?;
    let v2 = v1_slide_form(key, value)?;
    Some(format!("\"@{key}: {value}\" is v1 syntax; write {v2}"))
}

/// The v2 form of v1 slide directive `@key: value`, if `key` was one.
fn v1_slide_form(key: &str, value: &str) -> Option<String> {
    let comment = |k: &str, v: &str| format!("<!-- {k}: {v} -->");
    Some(match key {
        "layout" => comment("design", v1_design(value)),
        "illustration" => comment("picture", value),
        "art" if value == "none" => comment("picture", "none"),
        "art" => comment("picture-prompt", value),
        "zoom" => comment("zoom-to", value),
        "class" => "nothing: it was removed in v2".into(),
        _ => {
            let def = language::setting(key)?;
            if def.scope.in_slide() {
                comment(key, value)
            } else {
                format!("`{key}: {value}` in the frontmatter")
            }
        }
    })
}

/// The design a v1 layout name became.
fn v1_design(layout: &str) -> &str {
    match layout {
        "bullet" | "bullets" => "points",
        "image" => "media",
        "two-column" => "columns",
        "diagram" | "architecture" | "visualization" => "visual",
        other => other,
    }
}

/// The slide's lines outside code fences, with their 0-based line in `raw`.
pub(super) fn body_lines(raw: &str) -> Vec<(usize, &str)> {
    let mut fences = FenceTracker::new();
    raw.lines()
        .enumerate()
        .filter(|(_, l)| !fences.observe(l))
        .collect()
}

/// Strip list markers, quote markers and indentation.
fn strip_container(line: &str) -> &str {
    let mut rest = line.trim_start();
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
            return rest;
        }
    }
}

/// Fences whose `@` tag names no mdeck fence: a v1 name (with its v2 tag) or
/// a typo (with a suggestion). Without a known tag they show as code.
pub fn fence_warnings(presentation: &parser::Presentation) -> Vec<CheckWarning> {
    let mut out = Vec::new();
    for (i, slide) in presentation.slides.iter().enumerate() {
        let mut fences = FenceTracker::new();
        for (offset, line) in slide.raw_source.lines().enumerate() {
            let opening = !fences.is_open();
            if !(fences.observe(line) && opening) {
                continue;
            }
            let info = line.trim().trim_start_matches(['`', '~']).trim();
            let Some(tag) = info
                .split_whitespace()
                .next()
                .filter(|t| t.starts_with('@'))
            else {
                continue;
            };
            if language::fence(tag).is_some() || crate::render::visualizations::is_visual_tag(tag) {
                continue;
            }
            let message = match language::v1_fence(tag) {
                Some(v2) => format!("```{tag} is v1 syntax; write ```{v2}"),
                None => {
                    let tags = language::FENCES.iter().map(|f| f.tag);
                    match language::suggestion(tag, tags) {
                        Some(s) => format!(
                            "```{tag} is not an mdeck fence and shows as code; did you mean ```{s}?"
                        ),
                        None => format!("```{tag} is not an mdeck fence and shows as code"),
                    }
                }
            };
            out.push(CheckWarning {
                slide: i + 1,
                line: slide.line_at(offset),
                category: CheckCategory::Visual,
                message,
                place: None,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn warnings(md: &str) -> Vec<String> {
        let p = parser::parse(md);
        settings_warnings(&p)
            .into_iter()
            .chain(fence_warnings(&p))
            .map(|w| format!("{}:{}: {}", w.slide, w.line, w.message))
            .collect()
    }

    #[test]
    fn clean_settings_are_silent() {
        let md = "---\ntitle: T\ntheme: dark\ntransition: fade\nreveal: none\n---\n# A\n<!-- picture: rocket -->\n\n- one\n\n# B\n<!--\ndesign: columns\ntransition: zoom\nzoom-to: Pump\n-->\n\nx\n\n+++\n\ny\n\n```@bar\n- a: 1\n```\n";
        assert!(warnings(md).is_empty(), "{:?}", warnings(md));
        // Prose that happens to start with @ is not v1 syntax.
        assert!(warnings("# A\n\n@team: see you at five\n").is_empty());
    }

    #[test]
    fn v1_frontmatter_names_its_v2_key() {
        let w = warnings("---\n@theme: dark\n@art: a harbour\n@story: x\n---\n# A\n");
        assert_eq!(
            w,
            [
                "0:2: `@theme: dark` is v1 syntax; write `theme: dark`",
                "0:3: `@art: a harbour` is v1 syntax; write `art-world: a harbour`",
                "0:4: `@story` is v1 syntax and was removed in v2",
            ]
        );
    }

    #[test]
    fn v1_slide_lines_name_their_v2_form() {
        let w = warnings("# A\n@layout: quote\n\n- @illustration: rocket\n\n???\n\nnotes");
        assert_eq!(
            w,
            [
                "1:2: \"@layout: quote\" is v1 syntax; write <!-- design: quote -->",
                "1:4: \"@illustration: rocket\" is v1 syntax; write <!-- picture: rocket -->",
                "1:6: `???` is v1 syntax; put speaker notes in a ```@notes block",
            ]
        );
        let w = warnings("# A\n@layout: two-column\n@theme: dark\n");
        assert!(w[0].ends_with("<!-- design: columns -->"), "{w:?}");
        assert!(w[1].ends_with("`theme: dark` in the frontmatter"), "{w:?}");
    }

    #[test]
    fn unknown_names_and_values_are_reported() {
        let w = warnings(
            "---\nthme: dark\ntransition: wipe\ndesign: quote\n---\n# A\n<!-- design: quote\nlogo-position: top-left\npictur: x\ndesign: sideways -->\n",
        );
        assert_eq!(w.len(), 6, "{w:?}");
        assert!(w[0].contains("`thme` is not a deck setting; did you mean `theme`?"));
        assert!(w[1].contains("`transition: wipe`: expected"));
        assert!(w[2].contains("`design` is a slide setting"));
        assert!(w[3].contains("`logo-position` is a deck setting"));
        assert!(w[4].contains("`pictur` is not a slide setting; did you mean `picture`?"));
        assert!(w[5].contains("`design: sideways`: expected"));
    }

    #[test]
    fn duplicates_typos_and_zoom_without_target() {
        let w = warnings(
            "# A\n<!-- design: quote -->\n<!-- design: code -->\n<!-- desgin: x -->\n<!-- transition: zoom -->\n",
        );
        assert_eq!(w.len(), 3, "{w:?}");
        assert!(w[0].contains("written twice"), "{w:?}");
        assert!(w[1].contains("needs `zoom-to"), "{w:?}");
        assert!(w[2].contains("did you mean `design`"), "{w:?}");
    }

    #[test]
    fn settings_above_the_next_heading_are_flagged() {
        // MD-08: they stay on their slide, and --check says so.
        let w = warnings("# A\n\n- one\n\n<!-- design: quote -->\n# B\n\n> q\n");
        assert_eq!(w.len(), 1, "{w:?}");
        assert!(w[0].starts_with("1:5: these settings end slide 1"), "{w:?}");
        // Under the heading, or on the last slide, is fine.
        assert!(warnings("# A\n<!-- design: quote -->\n\n> q\n").is_empty());
        assert!(warnings("# A\n<!-- design: quote -->\n\n---\n\nx\n").is_empty());
        assert!(warnings("# A\n\n- one\n\n# B\n\nx\n<!-- logo: none -->\n").is_empty());
    }

    #[test]
    fn unknown_and_renamed_fences() {
        let w = warnings(
            "# A\n\n```@barchart\n- a: 1\n```\n\n```@donutt\n- a: 1\n```\n\n```@zzz\n```\n\n```rust\n```\n",
        );
        assert_eq!(
            w,
            [
                "1:3: ```@barchart is v1 syntax; write ```@bar",
                "1:7: ```@donutt is not an mdeck fence and shows as code; did you mean ```@donut?",
                "1:11: ```@zzz is not an mdeck fence and shows as code",
            ]
        );
    }
}
