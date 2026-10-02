pub mod blocks;
mod directives;
pub mod frontmatter;
pub mod inline;
mod layout;
mod math;
mod model;
mod notes;
pub mod splitter;
mod text;

pub use directives::{GLOBAL_DIRECTIVES, SLIDE_DIRECTIVES, directive, is_known_directive};
pub use layout::Layout;
pub use model::{
    Block, Chart, Directive, ImageDirectives, Inline, ListItem, ListMarker, Presentation,
    PresentationMeta, Slide,
};
pub use text::inlines_to_text;

use layout::classify_layout;
use notes::extract_notes;

pub fn parse(content: &str) -> Presentation {
    let (meta, body, first_line) = frontmatter::extract(content);
    let raws = splitter::split(&body, meta.slide_level);
    let located = splitter::locate(&body, &raws);
    let slides: Vec<Slide> = raws
        .into_iter()
        .zip(located)
        .filter(|(raw, _)| !raw.trim().is_empty())
        .map(|(raw, lines)| {
            let lines = lines.into_iter().map(|l| l + first_line).collect();
            parse_slide(raw, lines)
        })
        .collect();
    Presentation { meta, slides }
}

/// Parse one raw slide from the splitter: notes, directives, blocks, layout.
/// `source_lines` holds the deck file line of each line of `raw`.
fn parse_slide(raw: String, source_lines: Vec<usize>) -> Slide {
    let line = source_lines.first().copied().unwrap_or(0);
    let (content_part, notes) = extract_notes(&raw);
    let (mut directives, content) = blocks::extract_directives(&content_part);
    for d in &mut directives {
        d.line = source_lines.get(d.line).copied().unwrap_or(line + d.line);
    }
    let (mut blocks, story_hint, scene_script) = take_story_blocks(blocks::parse(&content));
    // A slide's @thermal-window is the common window of its thermal images.
    if let Some(window) = trimmed_directive(&directives, "thermal-window") {
        for b in &mut blocks {
            if let Block::Chart {
                kind: Chart::Thermal,
                content,
            } = b
            {
                content.push_str(&format!("\nslide-window: {window}\n"));
            }
        }
    }
    let layout = classify_layout(&directives, &blocks);
    let illustration = trimmed_directive(&directives, "illustration").map(|v| v.to_lowercase());
    let logo = trimmed_directive(&directives, "logo").map(str::to_string);
    let art = trimmed_directive(&directives, "art").map(str::to_string);
    Slide {
        directives,
        blocks,
        layout,
        raw_source: raw,
        line,
        source_lines,
        notes,
        story_hint,
        scene_script,
        illustration,
        logo,
        art,
    }
}

/// The trimmed value of the slide directive `name`, unless it is empty.
fn trimmed_directive<'a>(directives: &'a [Directive], name: &str) -> Option<&'a str> {
    directive(directives, name)
        .map(str::trim)
        .filter(|v| !v.is_empty())
}

/// Pull the story authoring fences out of a slide's blocks so they never
/// render and never influence layout inference.
fn take_story_blocks(blocks: Vec<Block>) -> (Vec<Block>, Option<String>, Option<String>) {
    let mut hint: Option<String> = None;
    let mut script: Option<String> = None;
    let rest = blocks
        .into_iter()
        .filter(|b| match b {
            Block::StoryHint { content } => {
                let h = hint.get_or_insert_with(String::new);
                if !h.is_empty() {
                    h.push_str("\n\n");
                }
                h.push_str(content.trim());
                false
            }
            Block::SceneScript { content } => {
                script = Some(content.clone());
                false
            }
            _ => true,
        })
        .collect();
    (rest, hint.filter(|h| !h.is_empty()), script)
}

/// Count the maximum number of reveal steps in a slide's blocks.
/// Each `+` (NextStep) marker in any list or diagram counts as one step.
pub fn compute_max_steps(blocks: &[Block]) -> usize {
    blocks
        .iter()
        .map(|b| match b {
            Block::List { items, .. } => count_next_steps(items),
            Block::Diagram { content } => crate::render::diagram::count_diagram_steps(content),
            Block::Chart {
                kind: Chart::Thermal,
                content,
            } => crate::render::thermal::default_steps(content),
            Block::Chart { content, .. } => crate::render::visualizations::count_viz_steps(content),
            _ => 0,
        })
        .max()
        .unwrap_or(0)
}

fn count_next_steps(items: &[ListItem]) -> usize {
    let mut count = 0;
    for item in items {
        if item.marker == ListMarker::NextStep {
            count += 1;
        }
        count += count_next_steps(&item.children);
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn illustration_directive_names_a_cloud() {
        let p = parse("@illustration: Server-Rack\n\n# Title\n\n- one\n\n---\n\n# Plain\n");
        assert_eq!(p.slides[0].illustration.as_deref(), Some("server-rack"));
        assert!(p.slides[1].illustration.is_none());
        let empty = parse("@illustration:\n\n# T\n");
        assert!(empty.slides[0].illustration.is_none());
    }

    #[test]
    fn slide_directives_work_under_the_heading() {
        // #12: a heading-split slide with the directive where people write it.
        let md = "# First\n\n- one\n\n# Second\n@illustration: account\n\n- two\n";
        let pres = parse(md);
        assert_eq!(pres.slides.len(), 2);
        let s = &pres.slides[1];
        assert_eq!(s.illustration.as_deref(), Some("account"));
        assert!(matches!(s.layout, Layout::Bullet), "{:?}", s.layout);
        assert_eq!(
            s.blocks.len(),
            2,
            "the directive is not a paragraph: {:?}",
            s.blocks
        );

        // Anywhere at the top level, with a blank line or not; the last one wins.
        let md = "# A\n\nIntro\n\n@layout: two-column\n\nLeft\n\n+++\n\nRight\n@logo: none\n@layout: content\n";
        let s = &parse(md).slides[0];
        assert!(matches!(s.layout, Layout::Content), "{:?}", s.layout);
        assert_eq!(s.logo.as_deref(), Some("none"));
        assert!(
            !s.blocks
                .iter()
                .any(|b| matches!(b, Block::Paragraph { inlines }
            if inlines_to_text(inlines).contains('@')))
        );

        // The placements that already worked keep working.
        for md in [
            "# First\n\n- one\n\n@illustration: account\n# Second\n\n- two\n",
            "# First\n\n- one\n\n---\n\n@illustration: account\n\n# Second\n\n- two\n",
        ] {
            let pres = parse(md);
            assert_eq!(
                pres.slides[1].illustration.as_deref(),
                Some("account"),
                "{md:?}"
            );
            assert_eq!(pres.slides[0].illustration, None, "{md:?}");
        }
    }

    #[test]
    fn unknown_and_nested_directives_stay_text() {
        let md =
            "# A\n\n@team: see you at five\n\n- @layout: code\n\n```text\n@layout: code\n```\n";
        let s = &parse(md).slides[0];
        assert!(s.directives.is_empty(), "{:?}", s.directives);
        assert!(matches!(&s.blocks[1], Block::Paragraph { inlines }
            if inlines_to_text(inlines) == "@team: see you at five"));
    }

    #[test]
    fn test_poker_night_parses() {
        let content = include_str!("../../../../samples/poker-night.md");
        let pres = parse(content);
        assert_eq!(pres.meta.theme.as_deref(), Some("dark"));
        assert_eq!(pres.meta.transition.as_deref(), Some("slide"));
        assert!(
            pres.slides.len() >= 14,
            "Expected at least 14 slides, got {}",
            pres.slides.len()
        );
        assert!(matches!(pres.slides[0].layout, Layout::Title));
    }

    #[test]
    fn test_saloon_workshop_parses() {
        let content = include_str!("../../../../samples/saloon-workshop.md");
        let pres = parse(content);
        assert_eq!(pres.meta.theme.as_deref(), Some("light"));
        assert_eq!(pres.meta.transition.as_deref(), Some("fade"));
        assert_eq!(
            pres.meta.footer.as_deref(),
            Some("mdeck sample presentation")
        );
        assert!(
            pres.slides.len() >= 16,
            "Expected at least 16 slides, got {}",
            pres.slides.len()
        );
        assert!(matches!(pres.slides[0].layout, Layout::Title));
    }

    #[test]
    fn test_multiple_slides() {
        let content = "# Slide One\n\nContent\n\n\n\n# Slide Two\n\nMore content";
        let pres = parse(content);
        assert_eq!(pres.slides.len(), 2);
    }

    #[test]
    fn test_slide_separator_dashes() {
        let content = "# Slide One\n\n---\n\n# Slide Two";
        let pres = parse(content);
        assert_eq!(pres.slides.len(), 2);
    }

    #[test]
    fn test_heading_inference() {
        let content = "# First\n\nSome content\n\n# Second\n\nMore content";
        let pres = parse(content);
        assert_eq!(pres.slides.len(), 2);
    }

    #[test]
    fn test_crlf_document_end_to_end() {
        let content = "---\r\ntitle: \"CRLF\"\r\n@theme: dark\r\n---\r\n\r\n# One\r\n\r\n- a\r\n- b\r\n\r\n---\r\n\r\n# Two\r\n\r\nText\r\n";
        let pres = parse(content);
        assert_eq!(pres.meta.title.as_deref(), Some("CRLF"));
        assert_eq!(pres.meta.theme.as_deref(), Some("dark"));
        assert_eq!(pres.slides.len(), 2, "{:?}", pres.slides);
        assert!(matches!(pres.slides[0].layout, Layout::Bullet));
        assert!(matches!(pres.slides[1].layout, Layout::Title));
    }

    #[test]
    fn trimmed_directive_skips_blank_values() {
        let (dirs, _) = blocks::extract_directives("@logo:   brand.svg  \n@art:   \n# A");
        assert_eq!(trimmed_directive(&dirs, "logo"), Some("brand.svg"));
        assert_eq!(trimmed_directive(&dirs, "art"), None);
        assert_eq!(trimmed_directive(&dirs, "illustration"), None);
    }

    #[test]
    fn slides_and_directives_know_their_file_lines() {
        let md = "---\ntitle: T\n---\n\n# One\n\n- a\n\n@illustration: cup\n\n# Two\n@logo: none\n\nx\n\n---\n\n@layout: code\n\n# Three\n";
        let p = parse(md);
        let lines: Vec<usize> = p.slides.iter().map(|s| s.line).collect();
        assert_eq!(lines, [5, 9, 18]);
        // The trailing `@illustration` moved to Two's slide keeps its own line.
        let two = &p.slides[1];
        assert_eq!(two.directive_line("illustration"), 9);
        assert_eq!(two.directive_line("logo"), 12);
        assert_eq!(two.directive_line("layout"), 9, "none: the slide's line");
        assert_eq!(two.line_at(2), 11, "`# Two`");
        assert_eq!(p.slides[2].directive_line("layout"), 18);
        // Without frontmatter the first line is line 1.
        assert_eq!(parse("# A\n\n# B\n").slides[1].line, 3);
    }

    #[test]
    fn max_steps_counts_nested_next_step_markers() {
        let blocks = blocks::parse("+ a\n  + b\n- c\n\n# H\n\n+ d");
        assert_eq!(compute_max_steps(&blocks), 2);
        assert_eq!(compute_max_steps(&[]), 0);
    }
}
