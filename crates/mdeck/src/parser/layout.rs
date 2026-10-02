//! The v1 layout kinds, kept only as a coarse view of a slide's design for
//! the engines that pick their scenery by it (the particle scenes, the
//! figure stage, the split-flap board). Renderers, `--check` and themes use
//! [`super::Design`]; engines move to designs with the SDK (phase 2b).

use super::{Block, Design};

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
    #[allow(
        dead_code,
        reason = "the particle scenes still name it; no design maps to it"
    )]
    TwoColumn,
    #[default]
    Content,
}

impl Layout {
    /// The layout kind closest to `design` for a slide holding `blocks`.
    pub fn of(design: Design, blocks: &[Block]) -> Layout {
        match design {
            Design::Title => Layout::Title,
            Design::Section => Layout::Section,
            Design::Statement => Layout::Content,
            // a slide that gives its stage up to wide content is quiet
            // behind it, like a chart
            Design::Content if blocks.iter().any(wide) => Layout::Visualization,
            Design::Content => Layout::Content,
            Design::Table | Design::Columns => Layout::Visualization,
            Design::Points => Layout::Bullet,
            Design::Split => Layout::Image,
            Design::Media => Layout::Image,
            Design::Gallery => Layout::Gallery,
            Design::Quote => Layout::Quote,
            Design::Code => Layout::Code,
            Design::Visual => {
                if blocks.iter().any(|b| matches!(b, Block::Diagram { .. })) {
                    Layout::Diagram
                } else {
                    Layout::Visualization
                }
            }
        }
    }
}

/// A block that needs more width than a copy column gives it.
fn wide(b: &Block) -> bool {
    matches!(
        b,
        Block::Image { .. }
            | Block::CodeBlock { .. }
            | Block::Table { .. }
            | Block::Chart { .. }
            | Block::Diagram { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn every_design_has_a_layout_view() {
        let pres = parse(
            "# Title\n\n## Sub\n\n---\n\n## Points\n\n- a\n\n---\n\n## D\n\n```@architecture\nA -> B\n```\n",
        );
        let layouts: Vec<Layout> = pres.slides.iter().map(|s| s.layout).collect();
        assert_eq!(layouts, [Layout::Title, Layout::Bullet, Layout::Diagram]);
        for d in Design::ALL {
            // total: every design maps
            let _ = Layout::of(d, &[]);
        }
    }
}
