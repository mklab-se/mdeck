//! The slide content model: what the parser makes of a slide, for design
//! sets and board engines that lay slides out themselves.
//!
//! **First draft.** These types are copied from mdeck's parser model
//! (`crates/mdeck/src/parser/model.rs`) as of the SDK's first commit, with
//! three changes for the registries (D11): the layout is a design name
//! instead of a closed enum, charts and diagrams are one
//! [`Block::Visual`] keyed by the fence tag instead of a closed `Chart`
//! enum, and the story fields are gone (stories are removed in v2). The
//! parser is switched to produce these types in a later step; until then
//! the host converts.

/// One slide.
///
/// ```
/// use mdeck_sdk::content::{Block, Inline, Slide};
/// let mut s = Slide::default();
/// s.blocks.push(Block::Heading { level: 1, inlines: vec![Inline::Text("Hello".into())] });
/// assert_eq!(s.title().as_deref(), Some("Hello"));
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Slide {
    /// The slide's directives (`@name: value` lines), in order.
    pub directives: Vec<Directive>,
    /// The content.
    pub blocks: Vec<Block>,
    /// The design (layout) the slide uses, by name (`title`, `bullet`, ...).
    pub design: String,
    /// The original markdown of the slide.
    pub raw_source: String,
    /// 1-based line in the deck file where the slide starts; 0 when not parsed from a file.
    pub line: usize,
    /// 1-based deck file line of each `raw_source` line.
    pub source_lines: Vec<usize>,
    /// Speaker notes.
    pub notes: Option<String>,
    /// The point cloud illustration the slide names (`@illustration`).
    pub illustration: Option<String>,
    /// The slide's logo (a path, or `none`).
    pub logo: Option<String>,
    /// The slide's generated-art scene (`@art`), or `none`.
    pub art: Option<String>,
}

impl Slide {
    /// The deck file line of directive `name` (the last one when written
    /// twice), else the slide's line.
    ///
    /// ```
    /// use mdeck_sdk::content::{Directive, Slide};
    /// let mut s = Slide { line: 3, ..Default::default() };
    /// s.directives.push(Directive { name: "engine".into(), value: "led".into(), line: 5 });
    /// assert_eq!(s.directive_line("engine"), 5);
    /// assert_eq!(s.directive_line("logo"), 3);
    /// ```
    pub fn directive_line(&self, name: &str) -> usize {
        self.directives
            .iter()
            .rev()
            .find(|d| d.name == name)
            .map_or(self.line, |d| d.line)
    }

    /// The value of directive `name` (the last one when written twice).
    ///
    /// ```
    /// use mdeck_sdk::content::{Directive, Slide};
    /// let mut s = Slide::default();
    /// s.directives.push(Directive { name: "x".into(), value: "1".into(), line: 0 });
    /// assert_eq!(s.directive("x"), Some("1"));
    /// ```
    pub fn directive(&self, name: &str) -> Option<&str> {
        self.directives
            .iter()
            .rev()
            .find(|d| d.name == name)
            .map(|d| d.value.as_str())
    }

    /// The deck file line of `raw_source` line `offset` (0-based).
    ///
    /// ```
    /// let s = mdeck_sdk::content::Slide { line: 10, ..Default::default() };
    /// assert_eq!(s.line_at(2), 12);
    /// ```
    pub fn line_at(&self, offset: usize) -> usize {
        self.source_lines
            .get(offset)
            .copied()
            .unwrap_or(self.line + offset)
    }

    /// The plain text of the slide's first heading.
    ///
    /// See [`Slide`] for an example.
    pub fn title(&self) -> Option<String> {
        self.blocks.iter().find_map(|b| match b {
            Block::Heading { inlines, .. } => Some(plain_text(inlines)),
            _ => None,
        })
    }
}

/// The text of inlines without their formatting.
///
/// ```
/// use mdeck_sdk::content::{plain_text, Inline};
/// let t = plain_text(&[Inline::Text("a ".into()), Inline::Bold(vec![Inline::Text("b".into())])]);
/// assert_eq!(t, "a b");
/// ```
pub fn plain_text(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for i in inlines {
        match i {
            Inline::Text(s) | Inline::Code(s) => out.push_str(s),
            Inline::Bold(v) | Inline::Italic(v) | Inline::Strikethrough(v) => {
                out.push_str(&plain_text(v))
            }
            Inline::Link { text, .. } => out.push_str(&plain_text(text)),
            Inline::Math { tex, .. } => out.push_str(tex),
        }
    }
    out
}

/// A directive: `@name: value`.
///
/// See [`Slide::directive_line`] for an example.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Directive {
    /// The name, without `@`.
    pub name: String,
    /// The value, trimmed.
    pub value: String,
    /// 1-based line in the deck file.
    pub line: usize,
}

/// A block of slide content.
///
/// ```
/// use mdeck_sdk::content::Block;
/// let b = Block::Visual { tag: "barchart".into(), content: "A: 1".into(), step_base: 0 };
/// assert!(matches!(b, Block::Visual { .. }));
/// ```
#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::enum_variant_names)]
pub enum Block {
    /// A heading.
    Heading {
        /// 1 to 6.
        level: u8,
        /// The heading text.
        inlines: Vec<Inline>,
    },
    /// A paragraph.
    Paragraph {
        /// The paragraph text.
        inlines: Vec<Inline>,
    },
    /// A bulleted or numbered list.
    List {
        /// Numbered.
        ordered: bool,
        /// The items.
        items: Vec<ListItem>,
    },
    /// An image.
    Image {
        /// Alt text.
        alt: String,
        /// Path or URL as written.
        path: String,
        /// Size and placement options.
        directives: ImageDirectives,
    },
    /// A fenced code block.
    CodeBlock {
        /// The fence's language.
        language: Option<String>,
        /// The code.
        code: String,
        /// 1-based lines to highlight.
        highlight_lines: Vec<usize>,
    },
    /// A block quote.
    BlockQuote {
        /// The quote text.
        inlines: Vec<Inline>,
    },
    /// A table.
    Table {
        /// Header cells.
        headers: Vec<Vec<Inline>>,
        /// Rows of cells.
        rows: Vec<Vec<Vec<Inline>>>,
    },
    /// A horizontal rule.
    HorizontalRule,
    /// A visual (chart, diagram, any registered visual kind): a fence whose
    /// info string is `@tag`.
    Visual {
        /// The fence tag without `@` (`barchart`, `diagram`, ...).
        tag: String,
        /// The fence's body, for the visual to parse.
        content: String,
        /// Reveal steps on the slide before this visual's own (its steps
        /// follow on from them).
        step_base: usize,
    },
    /// A column break (`+++`).
    ColumnSeparator,
}

/// Size and placement options of an image.
///
/// ```
/// let d = mdeck_sdk::content::ImageDirectives::default();
/// assert!(!d.fill);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ImageDirectives {
    /// Width as written (`50%`, `400px`).
    pub width: Option<String>,
    /// Height as written.
    pub height: Option<String>,
    /// Fill the slide, cropping.
    pub fill: bool,
    /// Fit inside the content area.
    pub fit: bool,
    /// Alignment as written (`left`, `center`, `right`).
    pub align: Option<String>,
}

/// Inline text and formatting.
///
/// See [`plain_text`] for an example.
#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::enum_variant_names)]
pub enum Inline {
    /// Plain text.
    Text(String),
    /// Bold.
    Bold(Vec<Inline>),
    /// Italic.
    Italic(Vec<Inline>),
    /// Struck through.
    Strikethrough(Vec<Inline>),
    /// Inline code.
    Code(String),
    /// LaTeX math: `$...$` inline, `$$...$$` display.
    Math {
        /// The TeX source.
        tex: String,
        /// Display (block) math.
        display: bool,
    },
    /// A link.
    Link {
        /// The link text.
        text: Vec<Inline>,
        /// The target.
        url: String,
    },
}

/// A list item, with nested items.
///
/// ```
/// use mdeck_sdk::content::{ListItem, ListMarker};
/// let item = ListItem { marker: ListMarker::Static, inlines: vec![], children: vec![], step: 0 };
/// assert_eq!(item.marker, ListMarker::Static);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct ListItem {
    /// How the item is revealed.
    pub marker: ListMarker,
    /// The item text.
    pub inlines: Vec<Inline>,
    /// Nested items.
    pub children: Vec<ListItem>,
    /// The reveal step the item appears at (0: with the slide). The parser
    /// numbers `+` items across the slide in reading order, visuals
    /// included, and children appear with their parent.
    pub step: usize,
}

/// How a list item is revealed.
///
/// See [`ListItem`] for an example.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ListMarker {
    /// Shown with the slide.
    Static,
    /// Revealed on the next step.
    NextStep,
    /// Revealed together with the previous item.
    WithPrev,
    /// A numbered item.
    Ordered,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_ignores_later_headings_and_formatting() {
        let s = Slide {
            blocks: vec![
                Block::Paragraph { inlines: vec![] },
                Block::Heading {
                    level: 2,
                    inlines: vec![
                        Inline::Text("A ".into()),
                        Inline::Italic(vec![Inline::Text("b".into())]),
                    ],
                },
                Block::Heading {
                    level: 1,
                    inlines: vec![Inline::Text("later".into())],
                },
            ],
            ..Default::default()
        };
        assert_eq!(s.title().as_deref(), Some("A b"));
    }
}
