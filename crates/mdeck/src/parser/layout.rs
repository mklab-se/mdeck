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
            Design::Statement | Design::Table | Design::Content => Layout::Content,
            Design::Points | Design::Split => Layout::Bullet,
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
            Design::Columns => Layout::TwoColumn,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn every_design_has_a_layout_view() {
        let pres = parse("# Title\n\n## Sub\n\n---\n\n## Points\n\n- a\n\n---\n\n## D\n\n```@architecture\nA -> B\n```\n");
        let layouts: Vec<Layout> = pres.slides.iter().map(|s| s.layout).collect();
        assert_eq!(layouts, [Layout::Title, Layout::Bullet, Layout::Diagram]);
        for d in Design::ALL {
            // total: every design maps
            let _ = Layout::of(d, &[]);
        }
    }
}
