//! Slide settings: `key: value` lines in an HTML comment.
//!
//! A comment whose first line is `key: value` with a key of the language
//! ([`crate::language::SETTINGS`]) is a settings comment, and every line in
//! it is a setting of the slide it stands in, wherever in the slide that is.
//! Any other comment is an ordinary comment.

use super::splitter::FenceTracker;
use super::{Problem, ProblemKind, Setting};

/// The value of the setting `name`, trimmed; when it is written twice, the
/// last wins. Blank values count as not written.
pub fn setting<'a>(settings: &'a [Setting], name: &str) -> Option<&'a str> {
    settings
        .iter()
        .rev()
        .find(|d| d.name == name)
        .map(|d| d.value.trim())
        .filter(|v| !v.is_empty())
}

/// A `key: value` line: a word of letters, digits and `-`, a colon, a value.
pub fn parse_setting_line(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.trim().split_once(':')?;
    let key = key.trim_end();
    let word = !key.is_empty()
        && key.starts_with(|c: char| c.is_ascii_alphabetic())
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    word.then(|| (key, value.trim()))
}

/// Take a slide's settings comments out of `raw`. Returns the settings (each
/// `line` the 0-based line in `raw`), the problems found reading them, and
/// the content with those comments' lines blanked, so the blocks around them
/// stay apart and every other line keeps its place.
pub fn extract_settings(raw: &str) -> (Vec<Setting>, Vec<Problem>, String) {
    let lines: Vec<&str> = raw.lines().collect();
    let mut settings = Vec::new();
    let mut problems = Vec::new();
    let mut out: Vec<&str> = Vec::with_capacity(lines.len());
    let mut fences = FenceTracker::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        if fences.observe(line) || !trimmed.starts_with("<!--") {
            out.push(line);
            i += 1;
            continue;
        }
        // The comment's lines, and its text between `<!--` and `-->`.
        let Some((end, body)) = comment_at(&lines, i) else {
            out.push(line);
            i += 1;
            continue;
        };
        let body_lines: Vec<(usize, &str)> = body
            .iter()
            .copied()
            .filter(|(_, l)| !l.trim().is_empty())
            .collect();
        let first_key = body_lines
            .first()
            .and_then(|(_, l)| parse_setting_line(l))
            .map(|(k, _)| k);
        match first_key {
            Some(key) if crate::language::setting(key).is_some() => {
                for (at, l) in body_lines {
                    match parse_setting_line(l) {
                        Some((name, value)) => settings.push(Setting {
                            name: name.to_string(),
                            value: value.to_string(),
                            line: at,
                        }),
                        None => problems.push(Problem {
                            kind: ProblemKind::Setting,
                            line: at,
                            message: format!(
                                "`{}` in a settings comment is not a `key: value` setting",
                                l.trim()
                            ),
                        }),
                    }
                }
                out.extend(std::iter::repeat_n("", end - i));
            }
            Some(key) => {
                let names = crate::language::SETTINGS.iter().map(|s| s.name);
                if let Some(s) = crate::language::suggestion(key, names) {
                    problems.push(Problem {
                        kind: ProblemKind::Setting,
                        line: body_lines[0].0,
                        message: format!(
                            "`{key}` is not a setting, so this comment is ignored; did you mean `{s}`?"
                        ),
                    });
                }
                out.extend(&lines[i..end]);
            }
            None => out.extend(&lines[i..end]),
        }
        i = end;
    }
    (settings, problems, out.join("\n"))
}

/// The comment opening at `lines[start]` when it fills whole lines: the
/// index after its last line, and the text inside it per line.
fn comment_at<'a>(lines: &[&'a str], start: usize) -> Option<(usize, Vec<(usize, &'a str)>)> {
    let mut body = Vec::new();
    let first = lines[start].trim().strip_prefix("<!--")?;
    let mut i = start;
    let mut text = first;
    loop {
        if let Some((inside, after)) = text.split_once("-->") {
            if !after.trim().is_empty() {
                return None;
            }
            body.push((i, inside));
            return Some((i + 1, body));
        }
        body.push((i, text));
        i += 1;
        text = lines.get(i)?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(raw: &str) -> Vec<(String, String, usize)> {
        extract_settings(raw)
            .0
            .into_iter()
            .map(|s| (s.name, s.value, s.line))
            .collect()
    }

    #[test]
    fn one_line_and_block_comments_hold_settings() {
        let raw = "## Why\n<!-- design: statement -->\n\ntext\n<!--\ndesign: split\npicture: rocket\n-->\n";
        let (settings, problems, content) = extract_settings(raw);
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(settings.len(), 3);
        assert_eq!(
            names(raw)[1..],
            [
                ("design".into(), "split".into(), 5),
                ("picture".into(), "rocket".into(), 6)
            ]
        );
        assert_eq!(content.split('\n').count(), raw.lines().count());
        assert!(!content.contains("design"));
        assert!(content.contains("text"));
        assert_eq!(setting(&settings, "design"), Some("split"), "the last wins");
    }

    #[test]
    fn ordinary_comments_stay_and_typos_are_reported() {
        let raw = "<!-- just a note -->\n<!-- desgin: quote -->\n<!-- team: five -->\n```\n<!-- design: code -->\n```";
        let (settings, problems, content) = extract_settings(raw);
        assert!(settings.is_empty(), "{settings:?}");
        assert_eq!(content, raw);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].message.contains("did you mean `design`"));
        assert_eq!(problems[0].line, 1);
    }

    #[test]
    fn every_line_of_a_settings_comment_is_a_setting() {
        let raw = "<!--\ndesign: quote\nnot a setting\n-->";
        let (settings, problems, _) = extract_settings(raw);
        assert_eq!(settings.len(), 1);
        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0].line, 2);
        // Unknown keys after a known first key are kept for --check.
        assert_eq!(names("<!-- design: quote\nlayoot: x -->").len(), 2);
    }

    #[test]
    fn setting_lines_need_a_word_key() {
        assert_eq!(
            parse_setting_line("logo-height: 72 "),
            Some(("logo-height", "72"))
        );
        for line in ["@layout: x", ": x", "two words: x", "layout"] {
            assert!(parse_setting_line(line).is_none(), "{line:?}");
        }
    }
}
