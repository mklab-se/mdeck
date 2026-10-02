/// Split a document body (after frontmatter extraction) into raw slide strings.
///
/// Two things start a slide:
/// 1. a heading (ATX `#` or setext `===` / `---`) at or above the slide
///    level, when the current slide already has content;
/// 2. `---` with blank lines (or the start or end) on both sides.
///
/// Lines in fenced code (including ```` ```@notes ````) and in HTML comments
/// never split. Nothing moves between slides: every line stays in the slide
/// it was written in.
///
/// The `slide_level` parameter controls which heading level triggers splits:
/// - `Some(n)`: set with `slide-level: n` in the frontmatter; headings at
///   level 1..=n all split slides.
/// - `None`: inferred: with zero or one H1, H1 and H2 split (level 2), and an
///   H2 directly under a lone H1 with no body is its subtitle; with several
///   H1s only H1 splits (level 1).
pub fn split(body: &str, slide_level: Option<u8>) -> Vec<String> {
    let body = body.replace("\r\n", "\n");
    let lines: Vec<&str> = body.split('\n').collect();
    let kinds = classify(&lines);

    let merge_subtitle = slide_level.is_none();
    let level = slide_level.unwrap_or_else(|| infer_slide_level(&kinds));

    let mut slides: Vec<String> = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    let mut has_content = false;
    // True while the only content in `current` is a single H1.
    let mut only_h1 = false;
    let flush = |current: &mut Vec<&str>, slides: &mut Vec<String>| {
        let text = current.join("\n").trim().to_string();
        if !text.is_empty() {
            slides.push(text);
        }
        current.clear();
    };

    for (i, &line) in lines.iter().enumerate() {
        let kind = kinds[i];
        if kind == Line::Break {
            flush(&mut current, &mut slides);
            has_content = false;
            only_h1 = false;
            continue;
        }
        if let Line::Heading(h) = kind
            && h <= level
            && has_content
        {
            let subtitle = merge_subtitle && only_h1 && h == 2 && !has_body(&kinds[i + 1..], level);
            if !subtitle {
                flush(&mut current, &mut slides);
                has_content = false;
            }
        }
        current.push(line);
        match kind {
            Line::Blank | Line::Comment | Line::Underline | Line::Break => {}
            Line::Heading(h) => {
                only_h1 = !has_content && h == 1;
                has_content = true;
            }
            Line::Fenced | Line::Text => {
                only_h1 = false;
                has_content = true;
            }
        }
    }
    flush(&mut current, &mut slides);
    slides
}

/// What a body line is, for splitting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Line {
    Blank,
    /// An ATX heading, or the text line of a setext heading.
    Heading(u8),
    /// A setext heading's `===` / `---` line.
    Underline,
    /// In (or opening or closing) a fenced block.
    Fenced,
    /// In an HTML comment.
    Comment,
    /// A `---` slide break.
    Break,
    Text,
}

/// Classify every line: fences and comments first, then headings (a
/// one-line paragraph followed by `===` or `---` is a setext heading, as
/// the block parser reads it), then `---` breaks.
fn classify(lines: &[&str]) -> Vec<Line> {
    let mut kinds = Vec::with_capacity(lines.len());
    let mut fences = FenceTracker::new();
    let mut in_comment = false;
    for &line in lines {
        let trimmed = line.trim();
        let kind = if in_comment {
            in_comment = !trimmed.contains("-->");
            Line::Comment
        } else if fences.observe(line) {
            Line::Fenced
        } else if let Some(rest) = trimmed.strip_prefix("<!--") {
            in_comment = !rest.contains("-->");
            Line::Comment
        } else if trimmed.is_empty() {
            Line::Blank
        } else if let Some(h) = heading_level(line) {
            Line::Heading(h)
        } else {
            Line::Text
        };
        kinds.push(kind);
    }
    // Setext headings: a paragraph's first and only line, then an underline.
    for i in 0..lines.len().saturating_sub(1) {
        let starts_paragraph = i == 0 || kinds[i - 1] != Line::Text;
        if kinds[i] == Line::Text
            && kinds[i + 1] == Line::Text
            && starts_paragraph
            && is_paragraph_line(lines[i].trim())
            && let Some(h) = setext_level(lines[i + 1].trim())
        {
            kinds[i] = Line::Heading(h);
            kinds[i + 1] = Line::Underline;
        }
    }
    // `---` breaks: blank (or the start, or another break) before, blank
    // (or the end) after.
    for i in 0..lines.len() {
        let before = i == 0 || matches!(kinds[i - 1], Line::Blank | Line::Break);
        let after = kinds.get(i + 1).is_none_or(|k| *k == Line::Blank);
        if kinds[i] == Line::Text && is_dash_separator(lines[i].trim()) && before && after {
            kinds[i] = Line::Break;
        }
    }
    kinds
}

/// Whether a line could be the text of a setext heading: not the start of
/// another block the block parser reads first.
fn is_paragraph_line(trimmed: &str) -> bool {
    let list = |t: &str| {
        let mut c = t.chars();
        matches!((c.next(), c.next()), (Some('-' | '+' | '*'), Some(' ')))
            || t.split_once(". ")
                .is_some_and(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
    };
    !(trimmed.starts_with('>')
        || trimmed.starts_with('|')
        || trimmed.starts_with('<')
        || trimmed.starts_with("![")
        || trimmed == "+++"
        || list(trimmed)
        || is_dash_separator(trimmed))
}

/// `===` (H1) or `---` (H2), at least three characters.
fn setext_level(trimmed: &str) -> Option<u8> {
    if trimmed.len() < 3 {
        None
    } else if trimmed.chars().all(|c| c == '=') {
        Some(1)
    } else if trimmed.chars().all(|c| c == '-') {
        Some(2)
    } else {
        None
    }
}

/// Infer the slide level from the headings: level 2 with zero or one H1,
/// level 1 with several.
fn infer_slide_level(kinds: &[Line]) -> u8 {
    let h1_count = kinds.iter().filter(|k| **k == Line::Heading(1)).count();
    if h1_count <= 1 { 2 } else { 1 }
}

/// Whether the lines from here hold content before the next slide-splitting
/// heading or break. Blank lines and comments do not count.
fn has_body(kinds: &[Line], level: u8) -> bool {
    for kind in kinds {
        match *kind {
            Line::Blank | Line::Comment | Line::Underline => {}
            Line::Break => return false,
            Line::Heading(h) if h <= level => return false,
            _ => return true,
        }
    }
    false
}

/// Return the ATX heading level of a line (`# ` → 1, `## ` → 2, ...), if any.
fn heading_level(line: &str) -> Option<u8> {
    let hash_count = line.chars().take_while(|&c| c == '#').count();
    let is_heading = (1..=6).contains(&hash_count)
        && line
            .get(hash_count..)
            .is_some_and(|rest| rest.starts_with(' '));
    is_heading.then_some(hash_count as u8)
}

fn is_dash_separator(line: &str) -> bool {
    line.len() >= 3 && line.chars().all(|c| c == '-')
}

/// Where each line of each slide from [`split`] stands in `body`, as a
/// 0-based line index. The splitter keeps a slide's lines in order and only
/// drops breaks (and trims the slide's ends), so each non-blank line is the
/// next body line with the same text, and a blank line follows the line
/// before it.
pub fn locate(body: &str, slides: &[String]) -> Vec<Vec<usize>> {
    let body = body.replace("\r\n", "\n");
    let lines: Vec<&str> = body.split('\n').collect();
    let mut next = 0;
    slides
        .iter()
        .map(|slide| {
            let mut prev: Option<usize> = None;
            slide
                .lines()
                .map(|line| {
                    let text = line.trim();
                    let at = if text.is_empty() {
                        prev.map_or(next, |p| p + 1)
                    } else {
                        let at = (next..lines.len())
                            .find(|&i| lines[i].trim() == text)
                            .unwrap_or(next);
                        next = at + 1;
                        at
                    };
                    prev = Some(at);
                    at
                })
                .collect()
        })
        .collect()
}

/// Tracks whether successive lines are inside a fenced code block (``` or ~~~).
///
/// Shared by every pass that must ignore markdown syntax inside code
/// (slide splitting, heading inference, speaker-note extraction).
#[derive(Debug, Default, Clone)]
pub struct FenceTracker {
    /// `(fence_char, fence_len)` of the currently open fence, if any.
    open: Option<(char, usize)>,
}

impl FenceTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether a fence is open after the lines fed so far.
    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// Feed the next line. Returns `true` if the line belongs to a fenced code
    /// block: including the opening and closing fence lines themselves.
    pub fn observe(&mut self, line: &str) -> bool {
        let trimmed = line.trim();
        if let Some((fence_char, fence_len)) = self.open {
            let closing = trimmed.chars().take_while(|&c| c == fence_char).count();
            if closing >= fence_len && trimmed.chars().skip(closing).all(char::is_whitespace) {
                self.open = None;
            }
            true
        } else if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            let fence_char = trimmed.chars().next().unwrap_or('`');
            let fence_len = trimmed.chars().take_while(|&c| c == fence_char).count();
            self.open = Some((fence_char, fence_len));
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn located(body: &str, level: Option<u8>) -> Vec<Vec<usize>> {
        locate(body, &split(body, level))
    }

    #[test]
    fn locate_does_not_match_a_line_twice() {
        // The same text on two slides maps to two different lines.
        let body = "# A\n\nsame\n\n---\n\nsame";
        assert_eq!(located(body, None), [vec![0, 1, 2], vec![6]]);
    }

    #[test]
    fn breaks_need_blank_lines_around_them() {
        let body = "a\n\n---\n\nb\n---\nc\n\n---";
        let slides = split(body, None);
        assert_eq!(slides, ["a", "b\n---\nc"]);
        // Breaks in fences and comments stay put.
        let body = "a\n\n```\n\n---\n\n```\n<!--\n\n---\n\n-->\nb";
        assert_eq!(split(body, None).len(), 1);
    }

    #[test]
    fn blank_lines_never_split() {
        // MD-05: three blank lines are just blank lines.
        assert_eq!(split("one\n\n\n\n\ntwo", None).len(), 1);
    }

    #[test]
    fn setext_headings_split_like_atx() {
        // MD-02, D21
        let body = "Title\n=====\n\nIntro\n\nPart\n----\n\nText\n\nNext\n----\n\nMore";
        let slides = split(body, None);
        assert_eq!(
            slides,
            [
                "Title\n=====\n\nIntro",
                "Part\n----\n\nText",
                "Next\n----\n\nMore"
            ]
        );
        // Several setext H1s give level 1.
        let body = "A\n===\n\nSub\n---\n\nx\n\nB\n===\n\ny";
        assert_eq!(split(body, None).len(), 2);
        // A two-line paragraph over `---` is not a heading, and a list item
        // is never a setext heading.
        assert_eq!(split("# A\n\nx\n\none\ntwo\n---\n", None).len(), 1);
        assert_eq!(split("# A\n\nx\n\n- item\n---\n", None).len(), 1);
    }

    #[test]
    fn settings_never_move_between_slides() {
        // MD-08, D1: a comment above a heading stays on the slide before it,
        // including one indented into a list item.
        let body = "# One\n\n- a\n  <!-- picture: x -->\n\n<!-- design: quote -->\n# Two\n\nb";
        let slides = split(body, Some(1));
        assert_eq!(slides.len(), 2);
        assert!(slides[0].contains("picture") && slides[0].contains("design"));
        assert!(slides[1].starts_with("# Two"));
        // After a break, a comment before the heading opens the slide.
        let slides = split(
            "# One\n\n---\n\n<!-- design: quote -->\n# Two\n\nb",
            Some(1),
        );
        assert_eq!(slides[1], "<!-- design: quote -->\n# Two\n\nb");
    }

    #[test]
    fn locate_follows_breaks_and_headings() {
        let body = "# One\n\n- a\n\n---\n\n# Two\n\n\n\n\n# Three";
        assert_eq!(located(body, Some(1)), [vec![0, 1, 2], vec![6], vec![11]]);
        // Leading blank lines and CRLF do not shift anything.
        let body = "\r\n\r\n# One\r\n\r\n---\r\n\r\n# Two\r\n";
        assert_eq!(located(body, None), [vec![2], vec![6]]);
    }

    #[test]
    fn test_dash_separator() {
        let body = "Slide one\n\n---\n\nSlide two";
        let slides = split(body, None);
        assert_eq!(slides.len(), 2);
        assert_eq!(slides[0], "Slide one");
        assert_eq!(slides[1], "Slide two");
    }

    #[test]
    fn test_heading_inference_multiple_h1() {
        let body = "# First\n\nContent\n\n# Second\n\nMore content";
        let slides = split(body, None);
        assert_eq!(slides.len(), 2);
        assert!(slides[0].starts_with("# First"));
        assert!(slides[1].starts_with("# Second"));
    }

    #[test]
    fn test_h2_section_under_h1_is_its_own_slide() {
        // #13: `# Title` then `## Section` with content is a README, not a subtitle.
        let body = "# Coffee Club\n\n## Why we meet\n\n- Better beans\n\n## When\n\n- Tuesdays";
        let slides = split(body, None);
        assert_eq!(
            slides,
            [
                "# Coffee Club",
                "## Why we meet\n\n- Better beans",
                "## When\n\n- Tuesdays"
            ]
        );
        // Code, and content before a `---` break, count as the section's body too.
        let body = "# T\n## Setup\n\n```sh\nmake\n```";
        assert_eq!(split(body, None).len(), 2);
        let body = "# T\n## Setup\n\nText\n\n---\n\n## Next";
        assert_eq!(split(body, None).len(), 3);
    }

    #[test]
    fn test_h2_subtitle_without_content_stays_on_title_slide() {
        for body in [
            "# Coffee Club\n\n## Better beans, better breaks",
            "# Coffee Club\n\n## Better beans\n\n## Why\n\n- one",
            "# Coffee Club\n## Better beans\n\n---\n\n## Why\n\n- one",
            "# Coffee Club\n\n## Better beans\n\n<!-- picture: cup -->\n\n## Why\n\n- one",
        ] {
            let slides = split(body, None);
            assert!(
                slides[0].contains("## Better beans"),
                "{body:?} gave {slides:?}"
            );
        }
    }

    #[test]
    fn test_single_h1_infers_h2_split() {
        // Single H1 → infer slide level 2, so H2 also splits
        let body = "# Title\n\nSubtitle\n\n## Section One\n\nContent\n\n## Section Two\n\nMore";
        let slides = split(body, None);
        assert_eq!(slides.len(), 3, "Expected 3 slides, got {:?}", slides);
        assert!(slides[0].starts_with("# Title"));
        assert!(slides[1].starts_with("## Section One"));
        assert!(slides[2].starts_with("## Section Two"));
    }

    #[test]
    fn test_no_h1_infers_h2_split() {
        // No H1 at all → infer slide level 2, so H2 splits
        let body = "## First\n\nContent\n\n## Second\n\nMore";
        let slides = split(body, None);
        assert_eq!(slides.len(), 2);
        assert!(slides[0].starts_with("## First"));
        assert!(slides[1].starts_with("## Second"));
    }

    #[test]
    fn test_explicit_slide_level() {
        // Explicit @slide-level: 3: H1, H2, and H3 all split
        let body = "# Title\n\n## Part\n\n### Detail\n\nContent";
        let slides = split(body, Some(3));
        assert_eq!(slides.len(), 3, "Expected 3 slides, got {:?}", slides);
    }

    #[test]
    fn test_explicit_slide_level_1() {
        // Explicit @slide-level: 1: only H1 splits, even with single H1
        let body = "# Title\n\nSubtitle\n\n## Section\n\nContent";
        let slides = split(body, Some(1));
        assert_eq!(slides.len(), 1);
    }

    #[test]
    fn test_heading_inference_first_heading() {
        // First heading shouldn't split (no prior content)
        let body = "# Only Heading\n\nContent here";
        let slides = split(body, None);
        assert_eq!(slides.len(), 1);
    }

    #[test]
    fn test_heading_in_code_block_no_split() {
        let body = "# Title\n\n```python\n# this is a comment\nprint('hi')\n```";
        let slides = split(body, None);
        assert_eq!(
            slides.len(),
            1,
            "Hash comment in code block should not split"
        );
    }

    #[test]
    fn test_poker_night_slide_count() {
        let content = include_str!("../../../../samples/poker-night.md");
        // Strip frontmatter
        let (meta, body, _) = super::super::frontmatter::extract(content);
        let slides = split(&body, meta.slide_level);
        assert!(
            slides.len() >= 14,
            "Expected at least 14 slides, got {}",
            slides.len()
        );
    }

    #[test]
    fn test_dash_separator_inside_code_block_no_split() {
        // A `---` line inside a fenced code block (e.g. YAML frontmatter shown
        // as an example) must not split the slide.
        let body = "# Config\n\n```yaml\n\n---\n\ntitle: x\n---\n```\n\nAfter";
        let slides = split(body, None);
        assert_eq!(slides.len(), 1, "got {:?}", slides);
        assert!(slides[0].contains("title: x"));
        assert!(slides[0].contains("After"));
    }

    #[test]
    fn test_blank_lines_inside_code_block_no_split() {
        let body = "# Code\n\n```python\nx = 1\n\n\n\n\ny = 2\n```\n\nAfter";
        let slides = split(body, None);
        assert_eq!(slides.len(), 1, "got {:?}", slides);
        assert!(slides[0].contains("y = 2"));
    }

    #[test]
    fn test_tilde_fence_inside_code_block_no_split() {
        let body = "~~~\n---\n\n\n\n\n~~~\n\n# Next";
        let slides = split(body, None);
        assert_eq!(slides.len(), 2, "got {:?}", slides);
        assert!(slides[0].contains("---"));
        assert_eq!(slides[1], "# Next");
    }

    #[test]
    fn test_separator_after_code_block_still_splits() {
        let body = "```\ncode\n```\n\n---\n\nSecond";
        let slides = split(body, None);
        assert_eq!(slides.len(), 2, "got {:?}", slides);
    }

    #[test]
    fn test_h1_followed_by_h2_is_one_title_slide() {
        // Single H1 → level 2 inferred, but `## Subtitle` directly under the
        // H1 belongs to the title slide.
        let body = "# Title\n\n## Subtitle\n\n## Section\n\nContent";
        let slides = split(body, None);
        assert_eq!(slides.len(), 2, "got {:?}", slides);
        assert_eq!(slides[0], "# Title\n\n## Subtitle");
        assert!(slides[1].starts_with("## Section"));
    }

    #[test]
    fn test_h1_h2_adjacent_no_blank_line() {
        let body = "# Title\n## Subtitle\n\n## Section";
        let slides = split(body, None);
        assert_eq!(slides.len(), 2, "got {:?}", slides);
        assert_eq!(slides[0], "# Title\n## Subtitle");
    }

    #[test]
    fn test_explicit_slide_level_still_splits_h2_after_h1() {
        // The subtitle merge is an inference heuristic; an explicit
        // `@slide-level` means exactly what it says.
        let body = "# Title\n\n## Subtitle\n\nContent";
        let slides = split(body, Some(2));
        assert_eq!(slides.len(), 2, "got {:?}", slides);
    }

    #[test]
    fn test_h1_content_h2_still_splits() {
        let body = "# Title\n\nIntro text\n\n## Section\n\nContent";
        let slides = split(body, None);
        assert_eq!(slides.len(), 2, "got {:?}", slides);
    }

    #[test]
    fn test_fence_tracker() {
        let mut t = FenceTracker::new();
        assert!(!t.observe("text"));
        assert!(t.observe("```rust"));
        assert!(t.observe("# not a heading"));
        assert!(t.observe("~~~")); // different fence char does not close
        assert!(t.observe("``")); // too short does not close
        assert!(t.observe("````")); // longer run closes
        assert!(!t.observe("# heading"));
        assert!(t.observe("~~~~"));
        assert!(t.observe("~~~")); // shorter run does not close
        assert!(t.observe("~~~~"));
        assert!(!t.observe("done"));
    }

    #[test]
    fn test_h2_no_split_with_multiple_h1() {
        // Multiple H1s → infer level 1, H2 does NOT split
        let body = "# First\n\n## Sub\n\nContent\n\n# Second\n\nMore";
        let slides = split(body, None);
        assert_eq!(slides.len(), 2);
        assert!(slides[0].contains("## Sub"));
    }
}
