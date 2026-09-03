/// Split a document body (after frontmatter extraction) into raw slide strings.
///
/// Three mechanisms create slide breaks (all coexist and combine):
/// 1. `---` with blank lines on both sides
/// 2. Three or more consecutive blank lines (4+ newlines)
/// 3. Heading-level splits: a heading at or above the slide level starts a new slide
///
/// The `slide_level` parameter controls which heading level triggers splits:
/// - `Some(n)` — explicitly set via `@slide-level: n` in frontmatter; headings at
///   level 1..=n all split slides.
/// - `None` — inferred: if there is exactly one H1, both H1 and H2 split (level 2);
///   if there are multiple H1s, only H1 splits (level 1).
pub fn split(body: &str, slide_level: Option<u8>) -> Vec<String> {
    // Phase 1: Replace explicit --- separators and blank-line gaps with a sentinel
    let sentinel = "\x00SLIDE_BREAK\x00";

    // Normalize line endings
    let body = body.replace("\r\n", "\n");

    // Split into lines first
    let lines: Vec<String> = body.split('\n').map(String::from).collect();

    // Determine effective slide level. When it is inferred (not set via
    // `@slide-level`), an H2 directly under an H1 is treated as a subtitle.
    let merge_subtitle = slide_level.is_none();
    let level = slide_level.unwrap_or_else(|| infer_slide_level(&lines));

    // Process lines to detect separators (never inside fenced code blocks)
    let mut i = 0;
    let mut output_lines: Vec<String> = Vec::new();
    let mut fences = FenceTracker::new();
    while i < lines.len() {
        let line = &lines[i];
        let trimmed = line.trim();
        let in_fence = fences.observe(line);

        // Check for --- separator with blank lines around it
        if !in_fence && is_dash_separator(trimmed) {
            // Check if previous line is blank and next line is blank
            let prev_blank = i == 0
                || output_lines
                    .last()
                    .is_some_and(|l: &String| l.trim().is_empty())
                || (!output_lines.is_empty() && output_lines.last().is_some_and(|l| l == sentinel));
            let next_blank =
                i + 1 >= lines.len() || lines.get(i + 1).is_some_and(|l| l.trim().is_empty());

            if prev_blank && next_blank {
                // Remove trailing blank line from output if present
                if output_lines.last().is_some_and(|l| l.trim().is_empty()) {
                    output_lines.pop();
                }
                output_lines.push(sentinel.to_string());
                // Skip next blank line
                if i + 1 < lines.len() && lines[i + 1].trim().is_empty() {
                    i += 1;
                }
                i += 1;
                continue;
            }
        }

        output_lines.push(line.clone());
        i += 1;
    }

    // Phase 2: Replace 3+ consecutive blank lines with sentinel
    // (blank lines inside fenced code blocks never split)
    let mut final_lines: Vec<String> = Vec::new();
    let mut blank_count = 0;
    let mut fences = FenceTracker::new();
    for line in &output_lines {
        if line == sentinel {
            blank_count = 0;
            final_lines.push(line.clone());
            continue;
        }
        if fences.observe(line) {
            blank_count = 0;
            final_lines.push(line.clone());
            continue;
        }
        if line.trim().is_empty() {
            blank_count += 1;
            if blank_count < 3 {
                final_lines.push(line.clone());
            } else if blank_count == 3 {
                // Remove the 2 blank lines we already added
                final_lines.pop();
                final_lines.pop();
                final_lines.push(sentinel.to_string());
            }
            // else: more blank lines, skip them
        } else {
            blank_count = 0;
            final_lines.push(line.clone());
        }
    }

    // Rejoin into a string
    let result = final_lines.join("\n");

    // Phase 3: Split by sentinel
    let chunks: Vec<String> = result
        .split(sentinel)
        .map(|s| s.trim().to_string())
        .collect();

    // Phase 4: Apply heading-level splits within each chunk
    let mut slides: Vec<String> = Vec::new();
    for chunk in chunks {
        if chunk.is_empty() {
            continue;
        }
        split_by_heading_level(&chunk, level, merge_subtitle, &mut slides);
    }

    slides
}

/// Infer the slide level from the document content.
/// If there is exactly one H1 heading, infer level 2 (H1 + H2 both split).
/// If there are multiple H1 headings, infer level 1 (only H1 splits).
/// If there are no H1 headings, infer level 2 so H2 headings can split.
fn infer_slide_level(lines: &[String]) -> u8 {
    let mut h1_count = 0u32;
    let mut fences = FenceTracker::new();

    for line in lines {
        if !fences.observe(line) && line.starts_with("# ") {
            h1_count += 1;
        }
    }

    if h1_count <= 1 { 2 } else { 1 }
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

    /// Feed the next line. Returns `true` if the line belongs to a fenced code
    /// block — including the opening and closing fence lines themselves.
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

/// Split a chunk by heading level: when a heading at or above the given `level`
/// appears and the current slide already has content, insert a break.
/// Lines inside fenced code blocks are never treated as headings.
///
/// With `merge_subtitle`, an H2 that directly follows an H1 (with nothing but
/// blank lines or directives between them) never splits: `# Title` +
/// `## Subtitle` is the canonical title slide.
fn split_by_heading_level(chunk: &str, level: u8, merge_subtitle: bool, slides: &mut Vec<String>) {
    let mut current = String::new();
    let mut has_content = false;
    // True while the only content line in `current` is a single H1.
    let mut only_h1 = false;
    let mut fences = FenceTracker::new();

    for line in chunk.lines() {
        let trimmed = line.trim();
        let in_fence = fences.observe(line);

        let is_subtitle_of_h1 = merge_subtitle && only_h1 && heading_level(line) == Some(2);

        if !in_fence && is_heading_at_level(line, level) && has_content && !is_subtitle_of_h1 {
            // This heading starts a new slide.
            // Move any trailing directives from the old slide to the new one,
            // since `@layout: X` placed just before a heading belongs to
            // the heading's slide.
            let slide_text = current.trim().to_string();
            let (content_part, trailing_directives) = strip_trailing_directives(&slide_text);
            if !content_part.is_empty() {
                slides.push(content_part);
            }
            current = String::new();
            if !trailing_directives.is_empty() {
                current.push_str(&trailing_directives);
                current.push('\n');
            }
            has_content = false;
        }

        if !current.is_empty() {
            current.push('\n');
        }
        current.push_str(line);

        // Directives (@key: value) don't count as content for heading inference
        if !trimmed.is_empty() && !is_directive(trimmed) {
            only_h1 = !has_content && !in_fence && heading_level(line) == Some(1);
            has_content = true;
        }
    }

    let slide_text = current.trim().to_string();
    if !slide_text.is_empty() {
        slides.push(slide_text);
    }
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

/// Check if a line is a markdown heading at or above the given level.
/// E.g., level=2 matches `# ` (H1) and `## ` (H2) but not `### ` (H3).
fn is_heading_at_level(line: &str, level: u8) -> bool {
    heading_level(line).is_some_and(|h| h <= level)
}

/// Split trailing directive lines (and blank lines before them) from a slide's raw text.
/// Returns `(content, directives)` where `directives` contains only `@key: value` lines.
fn strip_trailing_directives(text: &str) -> (String, String) {
    let lines: Vec<&str> = text.lines().collect();

    // Walk backwards from the end, collecting contiguous directive / blank lines
    let mut split_at = lines.len();
    for i in (0..lines.len()).rev() {
        let trimmed = lines[i].trim();
        if trimmed.is_empty() || is_directive(trimmed) {
            split_at = i;
        } else {
            break;
        }
    }

    if split_at == lines.len() {
        // Nothing to strip
        return (text.to_string(), String::new());
    }

    let content = lines[..split_at].join("\n").trim().to_string();
    let directives: String = lines[split_at..]
        .iter()
        .filter(|l| !l.trim().is_empty())
        .copied()
        .collect::<Vec<&str>>()
        .join("\n");

    (content, directives)
}

fn is_dash_separator(line: &str) -> bool {
    line.len() >= 3 && line.chars().all(|c| c == '-')
}

fn is_directive(line: &str) -> bool {
    line.starts_with('@')
        && line.contains(':')
        && line[1..line.find(':').unwrap()]
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blank_line_split() {
        let body = "Slide one\n\n\n\nSlide two";
        let slides = split(body, None);
        assert_eq!(slides.len(), 2);
        assert_eq!(slides[0], "Slide one");
        assert_eq!(slides[1], "Slide two");
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
        // Explicit @slide-level: 3 — H1, H2, and H3 all split
        let body = "# Title\n\n## Part\n\n### Detail\n\nContent";
        let slides = split(body, Some(3));
        assert_eq!(slides.len(), 3, "Expected 3 slides, got {:?}", slides);
    }

    #[test]
    fn test_explicit_slide_level_1() {
        // Explicit @slide-level: 1 — only H1 splits, even with single H1
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
    fn test_combined_separators() {
        let body = "Slide one\n\n\n\n---\n\n\n\nSlide two";
        let slides = split(body, None);
        // Should produce 2 slides, not 3 (overlapping separators = single break)
        assert_eq!(slides.len(), 2);
    }

    #[test]
    fn test_directive_before_heading_moves_to_next_slide() {
        let body = "# Title\n\nSubtitle\n\n@layout: two-column\n# Second Slide\n\nContent";
        let slides = split(body, Some(1));
        assert_eq!(slides.len(), 2, "Expected 2 slides, got {}", slides.len());
        // Directive should NOT be on the first slide
        assert!(
            !slides[0].contains("@layout"),
            "First slide should not contain @layout directive: {}",
            slides[0]
        );
        // Directive should be on the second slide (before the heading)
        assert!(
            slides[1].contains("@layout: two-column"),
            "Second slide should start with @layout directive: {}",
            slides[1]
        );
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
        let (meta, body) = super::super::frontmatter::extract(content);
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
        let body = "~~~\n---\n\n\n\n\n~~~\n\n\n\n# Next";
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
