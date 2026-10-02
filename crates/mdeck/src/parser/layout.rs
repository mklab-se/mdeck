//! Which layout a slide gets: the `@layout` directive, or inferred from its blocks.

use super::text::inline_text_len;
use super::{Block, Directive, directive};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Layout {
    Title,
    Section,
    Image,
    Gallery,
    Quote,
    Code,
    Bullet,
    Diagram,
    Visualization,
    TwoColumn,
    Content,
}

impl Layout {
    /// `@layout` values and their layouts. Aliases share a layout.
    pub const NAMES: &[(&str, Layout)] = &[
        ("title", Layout::Title),
        ("section", Layout::Section),
        ("image", Layout::Image),
        ("gallery", Layout::Gallery),
        ("quote", Layout::Quote),
        ("code", Layout::Code),
        ("bullets", Layout::Bullet),
        ("bullet", Layout::Bullet),
        ("diagram", Layout::Diagram),
        ("architecture", Layout::Diagram),
        ("visualization", Layout::Visualization),
        ("two-column", Layout::TwoColumn),
        ("content", Layout::Content),
    ];

    /// The layout an `@layout` value names (exact, case-sensitive).
    pub fn from_name(name: &str) -> Option<Layout> {
        Self::NAMES
            .iter()
            .find(|(n, _)| *n == name)
            .map(|&(_, layout)| layout)
    }
}

pub(super) fn classify_layout(directives: &[Directive], blocks: &[Block]) -> Layout {
    explicit_layout(directives).unwrap_or_else(|| infer_layout(blocks))
}

/// The layout the slide's `@layout` asks for (the last one wins). A value
/// that names no layout gives [`Layout::Content`].
fn explicit_layout(directives: &[Directive]) -> Option<Layout> {
    directive(directives, "layout").map(|value| Layout::from_name(value).unwrap_or(Layout::Content))
}

/// How many blocks of each kind a slide holds.
#[derive(Default)]
struct Counts {
    headings: Vec<u8>,
    paragraphs: usize,
    lists: usize,
    images: usize,
    code_blocks: usize,
    quotes: usize,
    diagrams: usize,
    visualizations: usize,
    tables: usize,
    column_separators: usize,
}

impl Counts {
    fn of(blocks: &[Block]) -> Self {
        let mut c = Counts::default();
        for block in blocks {
            match block {
                Block::Heading { level, .. } => c.headings.push(*level),
                Block::Paragraph { .. } => c.paragraphs += 1,
                Block::List { .. } => c.lists += 1,
                Block::Image { .. } => c.images += 1,
                Block::CodeBlock { .. } => c.code_blocks += 1,
                Block::BlockQuote { .. } => c.quotes += 1,
                Block::Diagram { .. } => c.diagrams += 1,
                Block::Chart { .. } => c.visualizations += 1,
                Block::Table { .. } => c.tables += 1,
                Block::ColumnSeparator => c.column_separators += 1,
                Block::HorizontalRule => {}
            }
        }
        c
    }
}

/// The layout a slide without `@layout` gets, from the kinds of blocks it holds.
fn infer_layout(blocks: &[Block]) -> Layout {
    let c = Counts::of(blocks);
    let total = blocks.len();

    // 1. Diagram / Visualization
    if c.diagrams > 0 {
        return Layout::Diagram;
    }
    if c.visualizations > 0 {
        return Layout::Visualization;
    }

    // 2. Two-column (has column separator)
    if c.column_separators > 0 {
        return Layout::TwoColumn;
    }

    // 3. Title: H1 + optional (H2 or short P), nothing else
    if let Some(layout) = title_layout(blocks, &c) {
        return layout;
    }

    // 4. Section divider: single heading, nothing else
    if c.headings.len() == 1
        && c.paragraphs == 0
        && c.lists == 0
        && c.images == 0
        && c.code_blocks == 0
        && c.quotes == 0
        && c.tables == 0
    {
        return Layout::Section;
    }

    // 5. Image slide: single image, optional heading, optional short caption
    if c.images == 1 && c.lists == 0 && c.code_blocks == 0 && c.quotes == 0 && c.tables == 0 {
        let other = total - c.images - c.headings.len();
        if other <= 1 {
            return Layout::Image;
        }
    }

    // 6. Gallery: 2+ images
    if c.images >= 2
        && c.lists == 0
        && c.code_blocks == 0
        && c.quotes == 0
        && c.paragraphs == 0
        && c.tables == 0
    {
        return Layout::Gallery;
    }

    // 7. Quote slide (allow one image for side-panel rendering)
    if c.quotes > 0 && c.lists == 0 && c.code_blocks == 0 && c.images <= 1 && c.tables == 0 {
        return Layout::Quote;
    }

    // 8. Code slide (allow one image for side-panel rendering)
    if c.code_blocks > 0 && c.lists == 0 && c.images <= 1 && c.quotes == 0 && c.tables == 0 {
        return Layout::Code;
    }

    // 9. Bullet slide: heading + list (allow one image for side-panel rendering)
    if !c.headings.is_empty() && c.lists > 0 && c.code_blocks == 0 && c.images <= 1 && c.quotes == 0
    {
        return Layout::Bullet;
    }

    Layout::Content
}

/// A slide with one H1 and at most one other heading: a lone H1 is a section
/// divider, an H1 with an H2 or a short paragraph is a title slide.
fn title_layout(blocks: &[Block], c: &Counts) -> Option<Layout> {
    let h1_count = c.headings.iter().filter(|&&h| h == 1).count();
    if h1_count != 1 || c.headings.len() > 2 {
        return None;
    }
    let others = blocks.len() - 1;
    if others == 0 {
        // Just H1: could be section or title
        // If it's a lone H1, it's a section divider
        return Some(Layout::Section);
    }
    if others == 1 {
        // Check if the other element is H2 or short paragraph
        for block in blocks {
            match block {
                Block::Heading { level: 2, .. } => return Some(Layout::Title),
                Block::Paragraph { inlines } => {
                    let text_len: usize = inlines.iter().map(inline_text_len).sum();
                    if text_len < 120 {
                        return Some(Layout::Title);
                    }
                }
                _ => {}
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{blocks, parse};

    fn layout_directive(value: &str) -> Vec<Directive> {
        vec![Directive {
            name: "layout".into(),
            value: value.into(),
            line: 0,
        }]
    }

    #[test]
    fn every_layout_name_round_trips() {
        let all = [
            Layout::Title,
            Layout::Section,
            Layout::Image,
            Layout::Gallery,
            Layout::Quote,
            Layout::Code,
            Layout::Bullet,
            Layout::Diagram,
            Layout::Visualization,
            Layout::TwoColumn,
            Layout::Content,
        ];
        for layout in all {
            assert!(
                Layout::NAMES.iter().any(|&(_, l)| l == layout),
                "{layout:?} has no @layout name"
            );
        }
        for &(name, layout) in Layout::NAMES {
            assert_eq!(Layout::from_name(name), Some(layout), "{name}");
            assert_eq!(explicit_layout(&layout_directive(name)), Some(layout));
            let md = format!("@layout: {name}\n\n# A\n\n- one\n");
            assert_eq!(parse(&md).slides[0].layout, layout, "{name}");
        }
        assert_eq!(Layout::from_name("bullets"), Some(Layout::Bullet));
        assert_eq!(Layout::from_name("architecture"), Some(Layout::Diagram));
    }

    #[test]
    fn unknown_layout_names_fall_back_to_content() {
        assert_eq!(Layout::from_name("sideways"), None);
        assert_eq!(Layout::from_name("Title"), None, "names are case-sensitive");
        for value in ["sideways", "Title", ""] {
            assert_eq!(
                explicit_layout(&layout_directive(value)),
                Some(Layout::Content)
            );
        }
        assert_eq!(explicit_layout(&[]), None);
        // The last @layout wins
        let mut two = layout_directive("code");
        two.extend(layout_directive("quote"));
        assert_eq!(explicit_layout(&two), Some(Layout::Quote));
    }

    #[test]
    fn counts_tally_block_kinds() {
        let c = Counts::of(&blocks::parse(
            "# T\n\n## S\n\ntext\n\n- a\n\n> q\n\n+++\n\n***",
        ));
        assert_eq!(c.headings, vec![1, 2]);
        assert_eq!(c.paragraphs, 1);
        assert_eq!(c.lists, 1);
        assert_eq!(c.quotes, 1);
        assert_eq!(c.column_separators, 1);
        assert_eq!(c.images + c.code_blocks + c.tables + c.diagrams, 0);
    }

    #[test]
    fn test_title_slide_layout() {
        let content = "# Hello World\n\nA subtitle here";
        let pres = parse(content);
        assert_eq!(pres.slides.len(), 1);
        assert!(matches!(pres.slides[0].layout, Layout::Title));
    }

    #[test]
    fn test_section_layout() {
        let content = "## Part One";
        let pres = parse(content);
        assert_eq!(pres.slides.len(), 1);
        assert!(matches!(pres.slides[0].layout, Layout::Section));
    }

    #[test]
    fn test_bullet_layout() {
        let content = "# Key Points\n\n- First\n- Second\n- Third";
        let pres = parse(content);
        assert_eq!(pres.slides.len(), 1);
        assert!(matches!(pres.slides[0].layout, Layout::Bullet));
    }

    #[test]
    fn test_quote_layout() {
        let content = "> Something wise\n\n-- Author";
        let pres = parse(content);
        assert_eq!(pres.slides.len(), 1);
        assert!(matches!(pres.slides[0].layout, Layout::Quote));
    }

    #[test]
    fn test_code_layout() {
        let content = "# Example\n\n```rust\nfn main() {}\n```";
        let pres = parse(content);
        assert_eq!(pres.slides.len(), 1);
        assert!(matches!(pres.slides[0].layout, Layout::Code));
    }

    #[test]
    fn test_two_column_layout() {
        let content = "@layout: two-column\n\n# Compare\n\nLeft side\n\n+++\n\nRight side";
        let pres = parse(content);
        assert_eq!(pres.slides.len(), 1);
        assert!(matches!(pres.slides[0].layout, Layout::TwoColumn));
    }

    #[test]
    fn test_image_layout() {
        let content = "![Photo @fill](photo.jpg)\n\nA caption";
        let pres = parse(content);
        assert_eq!(pres.slides.len(), 1);
        assert!(matches!(pres.slides[0].layout, Layout::Image));
    }

    #[test]
    fn test_inline_text_len_counts_chars_not_bytes() {
        // 40 CJK characters are 120 bytes but only 40 characters, still a
        // short subtitle, so H1 + short paragraph is a Title slide.
        let subtitle = "漢".repeat(40);
        let content = format!("# タイトル\n\n{subtitle}");
        let pres = parse(&content);
        assert_eq!(pres.slides.len(), 1);
        assert!(
            matches!(pres.slides[0].layout, Layout::Title),
            "got {:?}",
            pres.slides[0].layout
        );
        let len: usize = match &pres.slides[0].blocks[1] {
            Block::Paragraph { inlines } => inlines.iter().map(inline_text_len).sum(),
            other => panic!("expected paragraph, got {other:?}"),
        };
        assert_eq!(len, 40);
    }

    #[test]
    fn test_h1_h2_title_slide_end_to_end() {
        let content = "# Big Title\n\n## Subtitle\n\n## Section\n\n- a\n- b";
        let pres = parse(content);
        assert_eq!(pres.slides.len(), 2);
        assert!(matches!(pres.slides[0].layout, Layout::Title));
        assert!(matches!(pres.slides[1].layout, Layout::Bullet));
    }

    #[test]
    fn test_h1_h2_blocks_classify_as_title() {
        let blocks = blocks::parse("# Title\n\n## Subtitle");
        assert!(matches!(classify_layout(&[], &blocks), Layout::Title));
        // H1 + H3, or H1 + H2 + more content, are not title slides
        let blocks = blocks::parse("# Title\n\n### Deep");
        assert!(!matches!(classify_layout(&[], &blocks), Layout::Title));
        let blocks = blocks::parse("# Title\n\n## Subtitle\n\nA paragraph");
        assert!(!matches!(classify_layout(&[], &blocks), Layout::Title));
        // Lone H1 remains a section
        let blocks = blocks::parse("# Title");
        assert!(matches!(classify_layout(&[], &blocks), Layout::Section));
    }
}
