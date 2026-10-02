//! Which role each block of a slide fills in its design (DES-01, DES-04):
//! the copy before the plate, the plate itself, the copy after it, and the
//! columns of a `columns` slide. Every block lands somewhere: whatever a
//! design's roles do not name goes into the copy as body, in reading order.

use crate::parser::{Block, Design, Inline};

/// A text role (see `theme::arrangement::Roles`), or a block drawn by the
/// generic block renderer at the copy width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Title,
    Subtitle,
    Kicker,
    Heading,
    Statement,
    Lead,
    Body,
    List,
    Quote,
    Attribution,
    Caption,
    /// Drawn whole by the block renderer (code, tables, callouts, inline
    /// images and visuals).
    Block,
}

/// A block and the role it fills. Quotes split into their quotation and
/// attribution carry the runs directly.
#[derive(Debug, Clone)]
pub struct Item<'s> {
    pub role: Role,
    pub block: &'s Block,
    /// Inline runs replacing the block's own (a quote's quotation or
    /// attribution).
    pub inlines: Option<Vec<Inline>>,
    /// A quote's own blocks (without an attribution taken from them), laid
    /// out with their structure when they hold a list or a nested quote.
    pub quote: Option<&'s [Block]>,
}

impl<'s> Item<'s> {
    fn new(role: Role, block: &'s Block) -> Self {
        Item {
            role,
            block,
            inlines: None,
            quote: None,
        }
    }
}

/// A slide's content sorted into its design's places.
#[derive(Debug, Default)]
pub struct Parts<'s> {
    pub copy: Vec<Item<'s>>,
    pub plate: Vec<&'s Block>,
    pub footer: Vec<Item<'s>>,
    pub columns: Vec<Vec<Item<'s>>>,
}

/// The kind of block a design puts on its plate.
fn plate_kind(design: Design, block: &Block) -> bool {
    match design {
        Design::Split | Design::Media | Design::Gallery => matches!(block, Block::Image { .. }),
        Design::Code => matches!(block, Block::CodeBlock { .. }),
        Design::Visual => matches!(block, Block::Chart { .. } | Block::Diagram { .. }),
        Design::Table => matches!(block, Block::Table { .. }),
        _ => false,
    }
}

/// The role a block takes in the copy of `design`. `title_taken` says
/// whether the slide's title heading has been placed.
fn role_of(design: Design, block: &Block, title_taken: bool, after_plate: bool) -> Role {
    match block {
        Block::Heading { level, .. } => {
            if !title_taken {
                Role::Title
            } else if design == Design::Title && *level >= 2 {
                Role::Subtitle
            } else if design == Design::Section {
                Role::Kicker
            } else {
                Role::Heading
            }
        }
        Block::Paragraph { .. } => match design {
            Design::Title => Role::Subtitle,
            Design::Section => Role::Kicker,
            Design::Statement => Role::Statement,
            Design::Media | Design::Gallery if after_plate => Role::Caption,
            Design::Points | Design::Code | Design::Visual | Design::Table | Design::Media
                if !after_plate =>
            {
                Role::Lead
            }
            _ => Role::Body,
        },
        Block::List { .. } => Role::List,
        Block::BlockQuote { .. } => Role::Quote,
        Block::HorizontalRule | Block::ColumnSeparator => Role::Block,
        _ => Role::Block,
    }
}

/// Sort `blocks` into the places of `design`.
pub fn of(design: Design, blocks: &[Block]) -> Parts<'_> {
    let mut parts = Parts::default();
    if design == Design::Columns {
        columns(blocks, &mut parts);
        return parts;
    }
    let first_plate = blocks.iter().position(|b| plate_kind(design, b));
    let last_plate = blocks.iter().rposition(|b| plate_kind(design, b));
    // split shows one image beside the text; more go inline
    let single = design == Design::Split;
    let mut title_taken = false;
    let mut quote_seen = false;
    for (i, block) in blocks.iter().enumerate() {
        if matches!(block, Block::HorizontalRule | Block::ColumnSeparator) {
            continue;
        }
        if plate_kind(design, block) && (!single || Some(i) == first_plate) {
            parts.plate.push(block);
            continue;
        }
        let after = last_plate.is_some_and(|l| i > l) && design != Design::Split;
        let role = role_of(design, block, title_taken, after);
        if role == Role::Title {
            title_taken = true;
        }
        let target = if after {
            &mut parts.footer
        } else {
            &mut parts.copy
        };
        match block {
            Block::BlockQuote { blocks: inner } => {
                // the paragraph after a quote on a quote slide is its
                // attribution; otherwise the quote's own last line may be
                let next_is_attr = design == Design::Quote
                    && matches!(blocks.get(i + 1), Some(Block::Paragraph { .. }));
                let (own_blocks, own) = Block::quote_split(inner, next_is_attr);
                target.push(Item {
                    role: Role::Quote,
                    block,
                    inlines: Some(Block::quote_inlines(own_blocks)),
                    quote: Some(own_blocks),
                });
                if let Some(own) = own {
                    target.push(Item {
                        role: Role::Attribution,
                        block,
                        inlines: Some(own.clone()),
                        quote: None,
                    });
                }
                quote_seen = true;
            }
            Block::Paragraph { .. } if design == Design::Quote && quote_seen => {
                target.push(Item::new(Role::Attribution, block));
                quote_seen = false;
            }
            _ => target.push(Item::new(role, block)),
        }
    }
    parts
}

/// A `columns` slide: a leading H1 or H2 spans the columns; every
/// separator starts the next column.
fn columns<'s>(blocks: &'s [Block], parts: &mut Parts<'s>) {
    let mut current: Vec<Item<'s>> = Vec::new();
    let mut started = false;
    let mut title_taken = false;
    for block in blocks {
        match block {
            Block::ColumnSeparator => {
                parts.columns.push(std::mem::take(&mut current));
                started = true;
            }
            Block::Heading { level, .. }
                if !started && current.is_empty() && *level <= 2 && !title_taken =>
            {
                parts.copy.push(Item::new(Role::Title, block));
                title_taken = true;
            }
            Block::HorizontalRule => {}
            _ => {
                let role = match block {
                    Block::Heading { .. } => Role::Heading,
                    Block::Paragraph { .. } => Role::Body,
                    Block::List { .. } => Role::List,
                    Block::BlockQuote { .. } => Role::Quote,
                    _ => Role::Block,
                };
                let item = match block {
                    Block::BlockQuote { blocks } => Item {
                        role,
                        block,
                        inlines: Some(Block::quote_inlines(blocks)),
                        quote: Some(blocks),
                    },
                    _ => Item::new(role, block),
                };
                current.push(item);
            }
        }
    }
    parts.columns.push(current);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::blocks;

    fn roles(items: &[Item]) -> Vec<Role> {
        items.iter().map(|i| i.role).collect()
    }

    /// Every block of `md` lands in some place of `design`.
    fn count(design: Design, md: &str) -> (usize, usize) {
        let b = blocks::parse(md);
        let p = of(design, &b);
        let quotes = b
            .iter()
            .filter(|x| matches!(x, Block::BlockQuote { .. }))
            .count();
        let shown = p.copy.len()
            + p.plate.len()
            + p.footer.len()
            + p.columns.iter().map(Vec::len).sum::<usize>();
        let expected = b
            .iter()
            .filter(|x| !matches!(x, Block::HorizontalRule | Block::ColumnSeparator))
            .count();
        // a quote may add its attribution as a second item
        (shown.min(expected + quotes), expected)
    }

    #[test]
    fn no_design_drops_a_block() {
        let md = "# T\n\nIntro\n\n![a](a.png)\n\n- x\n\n> q\n\n```rust\nx\n```\n\n| a |\n|---|\n| 1 |\n\n```@pie\n- A: 1\n```\n\nAfter";
        for d in Design::ALL {
            let (shown, expected) = count(d, md);
            assert!(shown >= expected, "{d:?}: {shown} < {expected}");
        }
    }

    #[test]
    fn media_keeps_the_paragraph_before_the_image() {
        // D2: a paragraph before the image is the lead, after it the caption
        let b = blocks::parse("## H\n\nBefore\n\n![a](a.png)\n\nAfter");
        let p = of(Design::Media, &b);
        assert_eq!(roles(&p.copy), [Role::Title, Role::Lead]);
        assert_eq!(p.plate.len(), 1);
        assert_eq!(roles(&p.footer), [Role::Caption]);
    }

    #[test]
    fn quote_keeps_every_quote_and_heading() {
        // D3
        let b = blocks::parse("## A\n\n> one\n\n## B\n\n> two\n\n-- Who");
        let p = of(Design::Quote, &b);
        assert_eq!(
            roles(&p.copy),
            [
                Role::Title,
                Role::Quote,
                Role::Heading,
                Role::Quote,
                Role::Attribution
            ]
        );
    }

    #[test]
    fn visual_keeps_text_and_every_chart_in_order() {
        // D4: text after a chart stays after it; two charts share the plate
        let b =
            blocks::parse("## V\n\nLead\n\n```@pie\n- A: 1\n```\n\n```@bar\n- B: 1\n```\n\nAfter");
        let p = of(Design::Visual, &b);
        assert_eq!(roles(&p.copy), [Role::Title, Role::Lead]);
        assert_eq!(p.plate.len(), 2);
        assert_eq!(roles(&p.footer), [Role::Body]);
    }

    #[test]
    fn columns_split_at_separators() {
        let b = blocks::parse("## Compare\n\nLeft\n\n+++\n\n- right\n\n+++\n\nThird");
        let p = of(Design::Columns, &b);
        assert_eq!(roles(&p.copy), [Role::Title]);
        assert_eq!(p.columns.len(), 3);
        assert_eq!(roles(&p.columns[1]), [Role::List]);
    }
}
