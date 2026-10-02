//! Which layout a slide gets: the `design:` setting, or inferred from its
//! blocks. Phase 1 maps the v2 design names onto the v1 layouts; phase 3
//! replaces layouts with designs.

use super::Block;
use super::text::inline_text_len;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
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
    #[default]
    Content,
}

/// The layout a slide gets: the one its `design` names, else the inferred one.
pub(super) fn classify_layout(design: Option<&str>, blocks: &[Block]) -> Layout {
    design
        .and_then(|d| design_layout(d, blocks))
        .unwrap_or_else(|| infer_layout(blocks))
}

/// The layout that draws the design `name` until designs exist (phase 3).
/// An unknown name gives `None` (the slide's layout is inferred, and
/// `--check` reports the name).
fn design_layout(name: &str, blocks: &[Block]) -> Option<Layout> {
    let has = |f: fn(&Block) -> bool| blocks.iter().any(f);
    Some(match name.trim() {
        "title" => Layout::Title,
        "section" => Layout::Section,
        "statement" | "table" | "content" => Layout::Content,
        "points" => Layout::Bullet,
        "split" => {
            if has(|b| matches!(b, Block::List { .. })) {
                Layout::Bullet
            } else {
                Layout::Content
            }
        }
        "media" => Layout::Image,
        "gallery" => Layout::Gallery,
        "quote" => Layout::Quote,
        "code" => Layout::Code,
        "visual" => {
            if has(|b| matches!(b, Block::Diagram { .. })) {
                Layout::Diagram
            } else {
                Layout::Visualization
            }
        }
        "columns" => Layout::TwoColumn,
        _ => return None,
    })
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
    /// GitHub alerts; they also count as paragraphs.
    callouts: usize,
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
                Block::Callout { .. } => {
                    c.paragraphs += 1;
                    c.callouts += 1;
                }
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

    // A callout is drawn in the generic flow, which no layout below has.
    if c.callouts > 0 {
        return Layout::Content;
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

    #[test]
    fn every_design_name_maps_to_a_layout() {
        for &name in crate::language::DESIGNS {
            assert!(design_layout(name, &[]).is_some(), "{name}");
        }
        assert_eq!(design_layout("sideways", &[]), None);
        let list = blocks::parse("- a");
        assert_eq!(design_layout("split", &list), Some(Layout::Bullet));
        assert_eq!(design_layout("split", &[]), Some(Layout::Content));
        let md = "# A\n<!-- design: quote -->\n\n- one\n";
        assert_eq!(parse(md).slides[0].layout, Layout::Quote);
        // An unknown design is inferred (and reported by --check).
        let md = "# A\n<!-- design: sideways -->\n\n- one\n";
        assert_eq!(parse(md).slides[0].layout, Layout::Bullet);
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
        let content = "# Compare\n<!-- design: columns -->\n\nLeft side\n\n+++\n\nRight side";
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
        assert!(matches!(classify_layout(None, &blocks), Layout::Title));
        // H1 + H3, or H1 + H2 + more content, are not title slides
        let blocks = blocks::parse("# Title\n\n### Deep");
        assert!(!matches!(classify_layout(None, &blocks), Layout::Title));
        let blocks = blocks::parse("# Title\n\n## Subtitle\n\nA paragraph");
        assert!(!matches!(classify_layout(None, &blocks), Layout::Title));
        // Lone H1 remains a section
        let blocks = blocks::parse("# Title");
        assert!(matches!(classify_layout(None, &blocks), Layout::Section));
    }
}
