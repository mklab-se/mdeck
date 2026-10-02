//! Visual fences (`@bar`, `@architecture`, ...): lines that are neither a
//! setting nor an item, unknown settings and attributes, and values that do
//! not parse, each on its line in the file (VIZ-04). `@thermal` blocks are
//! checked with their sources in the `thermal` category.

use crate::check::{CheckCategory, CheckWarning};
use crate::parser;
use crate::render::visualizations;

pub fn visual_warnings(presentation: &parser::Presentation) -> Vec<CheckWarning> {
    let mut out = Vec::new();
    for (i, slide) in presentation.slides.iter().enumerate() {
        for (tag, open, content) in visual_fences(slide) {
            let Some(problems) = visualizations::check_tag(&tag, &content) else {
                continue;
            };
            for p in problems {
                out.push(CheckWarning {
                    slide: i + 1,
                    line: slide.line_at(open + 1 + p.offset),
                    category: CheckCategory::Visual,
                    message: format!("{tag}: {}", p.message),
                    place: None,
                });
            }
        }
    }
    out
}

/// Each visual fence on the slide (except `@thermal`): its tag, the offset
/// of its opening line in the slide's source, and its content.
fn visual_fences(slide: &parser::Slide) -> Vec<(String, usize, String)> {
    let mut fences = parser::splitter::FenceTracker::new();
    let mut out = Vec::new();
    let mut current: Option<(String, usize, String)> = None;
    for (offset, line) in slide.raw_source.lines().enumerate() {
        let was_open = fences.is_open();
        fences.observe(line);
        match (was_open, fences.is_open()) {
            (false, true) => {
                let info = line.trim().trim_start_matches(['`', '~']).trim_start();
                let tag = info.split_whitespace().next().unwrap_or("");
                current = (tag != "@thermal" && visualizations::is_visual_tag(tag))
                    .then(|| (tag.to_string(), offset, String::new()));
            }
            (true, false) => out.extend(current.take()),
            (true, true) => {
                if let Some((_, _, content)) = current.as_mut() {
                    content.push_str(line);
                    content.push('\n');
                }
            }
            (false, false) => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn problems_name_their_line_in_the_file() {
        let md = "# A\n\n```@bar\nx-label: Year\ncolour: red\n- 2024: 10\n- 2025\n```\n\n```@architecture\n- A -> B\n- C (pos: x)\n```\n\n```@thermal\nnonsense\n```\n";
        let p = parser::parse(md);
        let w = visual_warnings(&p);
        let lines: Vec<usize> = w.iter().map(|w| w.line).collect();
        assert_eq!(lines, [5, 7, 12], "{w:?}");
        assert!(w[0].message.starts_with("@bar: unknown setting 'colour'"));
        assert!(w.iter().all(|w| w.category == CheckCategory::Visual));
    }
}
