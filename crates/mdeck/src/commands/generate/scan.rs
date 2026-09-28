//! Finding image and icon markers in the raw markdown, and rewriting the
//! lines once their images exist.

use anyhow::Result;

use crate::parser::{self, Layout};
use crate::prompt::Orientation;

/// An image marker found in the markdown file.
pub(super) struct ImageMarker {
    /// 0-indexed line number in the raw file.
    pub line_index: usize,
    /// The alt text / prompt (may be empty for auto-prompt).
    pub alt_text: String,
    /// 1-indexed slide number.
    pub slide_number: usize,
    /// Image orientation based on layout context.
    pub orientation: Orientation,
}

/// A diagram icon marker found in the markdown file.
pub(super) struct IconMarker {
    /// 0-indexed line number in the raw file.
    pub line_index: usize,
    /// The prompt for the icon.
    pub prompt_text: String,
    /// 1-indexed slide number.
    pub slide_number: usize,
}

pub(super) fn scan_markers(
    lines: &[&str],
    presentation: &parser::Presentation,
) -> Result<(Vec<ImageMarker>, Vec<IconMarker>)> {
    let mut image_markers = Vec::new();
    let mut icon_markers = Vec::new();

    // Build a map from line ranges to slide info
    // We'll scan lines directly for the markers
    let mut current_slide = 0usize;
    let mut in_diagram = false;

    for (line_idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Check for image-generation markers: ![...](image-generation)
        if let Some(captures) = parse_image_gen_line(trimmed) {
            let slide_num = find_slide_for_line(line_idx, lines, presentation);
            let layout = if slide_num > 0 && slide_num <= presentation.slides.len() {
                presentation.slides[slide_num - 1].layout
            } else {
                Layout::Content
            };

            let orientation = orientation_for_layout(layout);

            image_markers.push(ImageMarker {
                line_index: line_idx,
                alt_text: captures,
                slide_number: slide_num,
                orientation,
            });
        }

        // Track diagram blocks
        if trimmed.starts_with("```") && trimmed.contains("@architecture") {
            in_diagram = true;
            current_slide = find_slide_for_line(line_idx, lines, presentation);
            continue;
        }
        if in_diagram && trimmed == "```" {
            in_diagram = false;
            continue;
        }

        // Check for icon markers inside diagrams
        if in_diagram
            && trimmed.contains("icon: generate-image")
            && let Some(prompt_text) = extract_icon_prompt(trimmed)
        {
            icon_markers.push(IconMarker {
                line_index: line_idx,
                prompt_text,
                slide_number: current_slide,
            });
        }
    }

    Ok((image_markers, icon_markers))
}

/// Parse a line like `![alt text](image-generation)` and return the alt text.
fn parse_image_gen_line(line: &str) -> Option<String> {
    let line = line.trim();
    if !line.contains("](image-generation)") {
        return None;
    }
    // Extract alt text from ![alt](image-generation) possibly with directives after
    let start = line.find("![")?;
    let alt_start = start + 2;
    let alt_end = line[alt_start..].find(']')? + alt_start;
    let after_bracket = &line[alt_end + 1..];
    if after_bracket.starts_with("(image-generation)") {
        let alt = line[alt_start..alt_end].to_string();
        // Strip inline directives from alt text (e.g. "@fill")
        let alt = alt
            .split_whitespace()
            .filter(|w| !w.starts_with('@'))
            .collect::<Vec<_>>()
            .join(" ");
        Some(alt)
    } else {
        None
    }
}

/// Extract the prompt from a diagram line like `Gateway (icon: generate-image, prompt: "An API gateway", pos: 1,1)`.
fn extract_icon_prompt(line: &str) -> Option<String> {
    // Look for prompt: "..." or prompt: '...'
    let prompt_start = line.find("prompt:")?;
    let after = &line[prompt_start + "prompt:".len()..];
    let after = after.trim_start();

    let (quote, rest) = if let Some(stripped) = after.strip_prefix('"') {
        ('"', stripped)
    } else {
        let stripped = after.strip_prefix('\'')?;
        ('\'', stripped)
    };

    let end = rest.find(quote)?;
    Some(rest[..end].to_string())
}

/// Find which slide (1-indexed) a line belongs to by matching against raw source.
fn find_slide_for_line(
    line_idx: usize,
    lines: &[&str],
    presentation: &parser::Presentation,
) -> usize {
    // Build cumulative line count per slide using raw_source
    let mut offset = 0;
    // Skip frontmatter
    if !lines.is_empty() && lines[0].trim() == "---" {
        // Find closing ---
        for (i, line) in lines.iter().enumerate().skip(1) {
            if line.trim() == "---" {
                offset = i + 1;
                break;
            }
        }
    }

    let mut slide_start = offset;
    for (slide_idx, slide) in presentation.slides.iter().enumerate() {
        let slide_lines = slide.raw_source.lines().count();
        // Account for separators between slides (blank lines, ---)
        let slide_end = slide_start + slide_lines;
        if line_idx >= slide_start && line_idx < slide_end + 3 {
            return slide_idx + 1;
        }
        slide_start = slide_end;
        // Skip separator lines
        while slide_start < lines.len()
            && (lines[slide_start].trim().is_empty() || lines[slide_start].trim() == "---")
        {
            slide_start += 1;
        }
    }
    // Fallback
    presentation.slides.len().max(1)
}

fn orientation_for_layout(layout: Layout) -> Orientation {
    match layout {
        Layout::Image => Orientation::Horizontal,
        Layout::Bullet | Layout::Code | Layout::Quote | Layout::Content => {
            // These could be side-panel layouts if they have an image
            Orientation::Vertical
        }
        Layout::TwoColumn => Orientation::Vertical,
        _ => Orientation::Horizontal,
    }
}

// ── Line replacement ─────────────────────────────────────────────────────────

/// Point an `![...](image-generation)` line at the generated file.
pub(super) fn replace_image_marker(line: &str, path: &str) -> String {
    line.replace("image-generation", path)
}

/// Replace `icon: generate-image, prompt: "..."` with `icon: <name>` in a diagram line.
pub(super) fn replace_icon_marker(line: &str, icon_name: &str) -> String {
    // Strategy: replace `icon: generate-image` with `icon: <name>` and remove `prompt: "..."`
    let mut result = line.to_string();

    // Replace the icon value
    result = result.replace("icon: generate-image", &format!("icon: {icon_name}"));

    // Remove the prompt: "..." portion (with surrounding commas)
    if let Some(prompt_start) = result.find("prompt:") {
        // Find the extent of the prompt value including quotes
        let after = &result[prompt_start..];
        let colon_end = "prompt:".len();
        let after_colon = after[colon_end..].trim_start();
        let quote = after_colon.chars().next().unwrap_or(' ');
        if quote == '"' || quote == '\'' {
            let rest = &after_colon[1..];
            if let Some(end) = rest.find(quote) {
                let prompt_byte_end =
                    prompt_start + colon_end + (after_colon.len() - rest.len()) + end + 1;

                // Remove the prompt portion and any surrounding comma
                let before = &result[..prompt_start];
                let after = &result[prompt_byte_end..];

                // Clean up commas: ", prompt: ..." or "prompt: ..., "
                let before = before.trim_end_matches([',', ' ']);
                let after = after.trim_start_matches([',', ' ']);

                result = if before.is_empty() {
                    after.to_string()
                } else if after.is_empty() {
                    before.to_string()
                } else {
                    format!("{before}, {after}")
                };
            }
        }
    }

    result
}

/// `content` with the given lines replaced, keeping its line endings and
/// trailing newline.
pub(super) fn rewrite_lines(content: &str, replacements: &[(usize, String)]) -> String {
    let mut new_lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    for (line_idx, new) in replacements {
        new_lines[*line_idx] = new.clone();
    }

    // Preserve original line ending style
    let line_ending = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut new_content = new_lines.join(line_ending);
    if content.ends_with('\n') || content.ends_with("\r\n") {
        new_content.push_str(line_ending);
    }
    new_content
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_image_gen_line() {
        assert_eq!(
            parse_image_gen_line("![a sunset over mountains](image-generation)"),
            Some("a sunset over mountains".to_string())
        );
        assert_eq!(
            parse_image_gen_line("![](image-generation)"),
            Some(String::new())
        );
        assert_eq!(parse_image_gen_line("![photo](photo.jpg)"), None);
        assert_eq!(
            parse_image_gen_line("![my prompt @fill](image-generation)"),
            Some("my prompt".to_string())
        );
    }

    #[test]
    fn test_extract_icon_prompt() {
        assert_eq!(
            extract_icon_prompt(
                "Gateway (icon: generate-image, prompt: \"An API gateway\", pos: 1,1)"
            ),
            Some("An API gateway".to_string())
        );
        assert_eq!(
            extract_icon_prompt("DB (icon: generate-image, prompt: 'A database icon')"),
            Some("A database icon".to_string())
        );
        assert_eq!(extract_icon_prompt("Server (icon: server, pos: 1,1)"), None);
    }

    #[test]
    fn test_replace_icon_marker() {
        let line = "- Gateway (icon: generate-image, prompt: \"An API gateway\", pos: 1,1)";
        let result = replace_icon_marker(line, "api-gateway");
        assert!(result.contains("icon: api-gateway"));
        assert!(!result.contains("prompt:"));
        assert!(!result.contains("generate-image"));
        assert!(result.contains("pos: 1,1"));
    }

    #[test]
    fn test_replace_icon_marker_no_prompt() {
        let line = "- Server (icon: generate-image, pos: 1,1)";
        let result = replace_icon_marker(line, "server-icon");
        assert_eq!(result, "- Server (icon: server-icon, pos: 1,1)");
    }

    #[test]
    fn rewrite_lines_keeps_crlf_and_trailing_newline() {
        let content = "a\r\n![x](image-generation)\r\nc\r\n";
        let out = rewrite_lines(content, &[(1, "![x](images/x.png)".into())]);
        assert_eq!(out, "a\r\n![x](images/x.png)\r\nc\r\n");
        assert_eq!(rewrite_lines("a\nb", &[(0, "z".into())]), "z\nb");
    }
}
