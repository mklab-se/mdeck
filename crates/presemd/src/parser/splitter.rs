/// Split a document body (after frontmatter extraction) into raw slide strings.
///
/// Three mechanisms create slide breaks:
/// 1. `---` with blank lines on both sides
/// 2. Three or more consecutive blank lines (4+ newlines)
/// 3. A `# ` heading when the current slide already has content
pub fn split(body: &str) -> Vec<String> {
    // Phase 1: Replace explicit --- separators and blank-line gaps with a sentinel
    let sentinel = "\x00SLIDE_BREAK\x00";

    // Normalize line endings
    let body = body.replace("\r\n", "\n");

    // Split into lines first
    let lines: Vec<String> = body.split('\n').map(String::from).collect();

    // Process lines to detect separators
    let mut i = 0;
    let mut output_lines: Vec<String> = Vec::new();
    while i < lines.len() {
        let line = &lines[i];
        let trimmed = line.trim();

        // Check for --- separator with blank lines around it
        if is_dash_separator(trimmed) {
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
    let mut final_lines: Vec<String> = Vec::new();
    let mut blank_count = 0;
    for line in &output_lines {
        if line == sentinel {
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

    // Phase 4: Apply heading inference within each chunk
    let mut slides: Vec<String> = Vec::new();
    for chunk in chunks {
        if chunk.is_empty() {
            continue;
        }
        split_by_heading_inference(&chunk, &mut slides);
    }

    slides
}

/// Split a chunk by H1 heading inference: when `# ` appears at the start of a line
/// and the current slide already has content, insert a break.
fn split_by_heading_inference(chunk: &str, slides: &mut Vec<String>) {
    let mut current = String::new();
    let mut has_content = false;

    for line in chunk.lines() {
        if line.starts_with("# ") && has_content {
            // This H1 starts a new slide
            let trimmed = current.trim().to_string();
            if !trimmed.is_empty() {
                slides.push(trimmed);
            }
            current = String::new();
            has_content = false;
        }

        if !current.is_empty() {
            current.push('\n');
        }
        current.push_str(line);

        // Directives (@key: value) don't count as content for heading inference
        let trimmed = line.trim();
        if !trimmed.is_empty() && !is_directive(trimmed) {
            has_content = true;
        }
    }

    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        slides.push(trimmed);
    }
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
        let slides = split(body);
        assert_eq!(slides.len(), 2);
        assert_eq!(slides[0], "Slide one");
        assert_eq!(slides[1], "Slide two");
    }

    #[test]
    fn test_dash_separator() {
        let body = "Slide one\n\n---\n\nSlide two";
        let slides = split(body);
        assert_eq!(slides.len(), 2);
        assert_eq!(slides[0], "Slide one");
        assert_eq!(slides[1], "Slide two");
    }

    #[test]
    fn test_heading_inference() {
        let body = "# First\n\nContent\n\n# Second\n\nMore content";
        let slides = split(body);
        assert_eq!(slides.len(), 2);
        assert!(slides[0].starts_with("# First"));
        assert!(slides[1].starts_with("# Second"));
    }

    #[test]
    fn test_h2_no_split() {
        let body = "# Title\n\n## Subtitle\n\nContent";
        let slides = split(body);
        assert_eq!(slides.len(), 1);
    }

    #[test]
    fn test_heading_inference_first_heading() {
        // First heading shouldn't split (no prior content)
        let body = "# Only Heading\n\nContent here";
        let slides = split(body);
        assert_eq!(slides.len(), 1);
    }

    #[test]
    fn test_combined_separators() {
        let body = "Slide one\n\n\n\n---\n\n\n\nSlide two";
        let slides = split(body);
        // Should produce 2 slides, not 3 (overlapping separators = single break)
        assert_eq!(slides.len(), 2);
    }

    #[test]
    fn test_poker_night_slide_count() {
        let content = include_str!("../../../../sample-presentations/poker-night.md");
        // Strip frontmatter
        let (_, body) = super::super::frontmatter::extract(content);
        let slides = split(&body);
        assert!(
            slides.len() >= 14,
            "Expected at least 14 slides, got {}",
            slides.len()
        );
    }
}
