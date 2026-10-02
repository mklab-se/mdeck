//! The slide content model: what the parser makes of a slide, for design
//! sets and board engines that lay slides out themselves.
//!
//! The model mirrors mdeck's markdown: headings, paragraphs, lists (with
//! their start number and task boxes), quotes and callouts with the blocks
//! inside them, tables with their column alignment, images, code and
//! visuals. A visual is one [`Block::Visual`] keyed by its fence tag, so a
//! registered visual kind needs no change here, and a slide names its
//! design by name.
//!
//! ## Growing without breaking
//!
//! The enums and structs here are `#[non_exhaustive]`: a 2.x release may
//! add a block kind, an inline kind or a field. Match with a wildcard arm
//! (`_ => {}`) and with `..` in struct patterns, and build values with the
//! constructors ([`Block::paragraph`], [`ListItem::new`], ...) or
//! `Default` plus field assignment:
//!
//! ```
//! use mdeck_sdk::content::{Block, Inline, ListItem, ListMarker, Slide};
//! let mut s = Slide::default();
//! s.line = 3;
//! s.blocks.push(Block::heading(1, vec![Inline::text("Plan")]));
//! s.blocks.push(Block::ordered_list(3, vec![ListItem::new(ListMarker::Ordered, vec![Inline::text("Ship")])]));
//! match &s.blocks[1] {
//!     Block::List { start, .. } => assert_eq!(*start, 3),
//!     _ => unreachable!(),
//! }
//! ```

/// One slide.
///
/// ```
/// use mdeck_sdk::content::{Block, Inline, Slide};
/// let mut s = Slide::default();
/// s.blocks.push(Block::heading(1, vec![Inline::text("Hello")]));
/// assert_eq!(s.title().as_deref(), Some("Hello"));
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
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
    /// The picture the slide names (`picture: name`).
    pub picture: Option<String>,
    /// The slide's logo (a path, or `none`).
    pub logo: Option<String>,
    /// The slide's generated-art scene (`@art`), or `none`.
    pub art: Option<String>,
}

impl Slide {
    /// An empty slide in design `design`.
    ///
    /// ```
    /// let s = mdeck_sdk::content::Slide::new("bullet");
    /// assert_eq!(s.design, "bullet");
    /// assert!(s.blocks.is_empty());
    /// ```
    pub fn new(design: impl Into<String>) -> Self {
        Self {
            design: design.into(),
            ..Self::default()
        }
    }

    /// The deck file line of directive `name` (the last one when written
    /// twice), else the slide's line.
    ///
    /// ```
    /// use mdeck_sdk::content::{Directive, Slide};
    /// let mut s = Slide::default();
    /// s.line = 3;
    /// s.directives.push(Directive::new("engine", "led", 5));
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
    /// s.directives.push(Directive::new("x", "1", 0));
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
    /// let mut s = mdeck_sdk::content::Slide::default();
    /// s.line = 10;
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
/// let t = plain_text(&[Inline::text("a "), Inline::Bold(vec![Inline::text("b")])]);
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
#[non_exhaustive]
pub struct Directive {
    /// The name, without `@`.
    pub name: String,
    /// The value, trimmed.
    pub value: String,
    /// 1-based line in the deck file.
    pub line: usize,
}

impl Directive {
    /// `@name: value` at deck file line `line`.
    ///
    /// See [`Slide::directive_line`] for an example.
    pub fn new(name: impl Into<String>, value: impl Into<String>, line: usize) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            line,
        }
    }
}

/// A block of slide content.
///
/// New kinds of block may be added in a 2.x release: match with a wildcard
/// arm. Variants with fields may gain fields: match them with `..` and
/// build them with the constructors ([`Block::paragraph`],
/// [`Block::list`], ...).
///
/// ```
/// use mdeck_sdk::content::Block;
/// let b = Block::visual("barchart", "A: 1");
/// assert!(matches!(b, Block::Visual { .. }));
/// ```
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
#[allow(clippy::enum_variant_names)]
pub enum Block {
    /// A heading.
    #[non_exhaustive]
    Heading {
        /// 1 to 6.
        level: u8,
        /// The heading text.
        inlines: Vec<Inline>,
    },
    /// A paragraph.
    #[non_exhaustive]
    Paragraph {
        /// The paragraph text.
        inlines: Vec<Inline>,
    },
    /// A bulleted or numbered list.
    #[non_exhaustive]
    List {
        /// Numbered.
        ordered: bool,
        /// The number of a numbered list's first item (`3.` starts at 3);
        /// 1 for a bulleted list.
        start: u32,
        /// The items.
        items: Vec<ListItem>,
    },
    /// An image.
    #[non_exhaustive]
    Image {
        /// Alt text.
        alt: String,
        /// Path or URL as written.
        path: String,
        /// Size options.
        directives: ImageDirectives,
    },
    /// A fenced code block.
    #[non_exhaustive]
    CodeBlock {
        /// The fence's language.
        language: Option<String>,
        /// The code.
        code: String,
        /// 1-based lines to highlight.
        highlight_lines: Vec<usize>,
    },
    /// A block quote, with the blocks inside it (paragraphs, lists, nested
    /// quotes). [`Block::quote_text`] gives it as one run of text.
    #[non_exhaustive]
    BlockQuote {
        /// The quoted blocks.
        blocks: Vec<Block>,
    },
    /// A GitHub alert (`> [!NOTE]`), drawn as a callout.
    #[non_exhaustive]
    Callout {
        /// What kind of callout.
        kind: CalloutKind,
        /// The blocks inside it.
        blocks: Vec<Block>,
    },
    /// A table.
    #[non_exhaustive]
    Table {
        /// Header cells.
        headers: Vec<Vec<Inline>>,
        /// Each column's alignment, from the separator row (`:-:`). May be
        /// shorter than a row: missing columns are [`Align::Left`].
        align: Vec<Align>,
        /// Rows of cells.
        rows: Vec<Vec<Vec<Inline>>>,
    },
    /// A horizontal rule.
    HorizontalRule,
    /// A visual (chart, diagram, any registered visual kind): a fence whose
    /// info string is `@tag`.
    #[non_exhaustive]
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

impl Block {
    /// A heading at `level` (1 to 6).
    ///
    /// ```
    /// use mdeck_sdk::content::{Block, Inline};
    /// assert!(matches!(Block::heading(2, vec![Inline::text("A")]), Block::Heading { level: 2, .. }));
    /// ```
    pub fn heading(level: u8, inlines: Vec<Inline>) -> Self {
        Block::Heading { level, inlines }
    }

    /// A paragraph.
    ///
    /// ```
    /// use mdeck_sdk::content::{Block, Inline};
    /// assert!(matches!(Block::paragraph(vec![Inline::text("A")]), Block::Paragraph { .. }));
    /// ```
    pub fn paragraph(inlines: Vec<Inline>) -> Self {
        Block::Paragraph { inlines }
    }

    /// A bulleted list.
    ///
    /// ```
    /// use mdeck_sdk::content::{Block, Inline, ListItem, ListMarker};
    /// let b = Block::list(vec![ListItem::new(ListMarker::Static, vec![Inline::text("a")])]);
    /// assert!(matches!(b, Block::List { ordered: false, start: 1, .. }));
    /// ```
    pub fn list(items: Vec<ListItem>) -> Self {
        Block::List {
            ordered: false,
            start: 1,
            items,
        }
    }

    /// A numbered list whose first item is number `start`.
    ///
    /// See the [module](self) docs for an example.
    pub fn ordered_list(start: u32, items: Vec<ListItem>) -> Self {
        Block::List {
            ordered: true,
            start,
            items,
        }
    }

    /// An image with no size options.
    ///
    /// ```
    /// use mdeck_sdk::content::Block;
    /// assert!(matches!(Block::image("A cat", "cat.png"), Block::Image { .. }));
    /// ```
    pub fn image(alt: impl Into<String>, path: impl Into<String>) -> Self {
        Block::Image {
            alt: alt.into(),
            path: path.into(),
            directives: ImageDirectives::default(),
        }
    }

    /// A code block with no highlighted lines.
    ///
    /// ```
    /// use mdeck_sdk::content::Block;
    /// assert!(matches!(Block::code(Some("rust"), "fn main() {}"), Block::CodeBlock { .. }));
    /// ```
    pub fn code(language: Option<&str>, code: impl Into<String>) -> Self {
        Block::CodeBlock {
            language: language.map(str::to_owned),
            code: code.into(),
            highlight_lines: Vec::new(),
        }
    }

    /// A block quote holding `blocks`.
    ///
    /// See [`Block::quote_text`] for an example.
    pub fn quote(blocks: Vec<Block>) -> Self {
        Block::BlockQuote { blocks }
    }

    /// A callout of `kind` holding `blocks`.
    ///
    /// ```
    /// use mdeck_sdk::content::{Block, CalloutKind, Inline};
    /// let b = Block::callout(CalloutKind::Tip, vec![Block::paragraph(vec![Inline::text("Save often")])]);
    /// assert!(matches!(b, Block::Callout { kind: CalloutKind::Tip, .. }));
    /// ```
    pub fn callout(kind: CalloutKind, blocks: Vec<Block>) -> Self {
        Block::Callout { kind, blocks }
    }

    /// A table with every column left-aligned.
    ///
    /// ```
    /// use mdeck_sdk::content::{Align, Block, Inline};
    /// let b = Block::table(vec![vec![Inline::text("A")]], vec![vec![vec![Inline::text("1")]]]);
    /// assert!(matches!(&b, Block::Table { align, .. } if align == &[Align::Left]));
    /// ```
    pub fn table(headers: Vec<Vec<Inline>>, rows: Vec<Vec<Vec<Inline>>>) -> Self {
        Block::Table {
            align: vec![Align::Left; headers.len()],
            headers,
            rows,
        }
    }

    /// A visual with fence tag `tag` (without `@`) and body `content`,
    /// first on its slide (no reveal steps before it).
    ///
    /// See [`Block`] for an example.
    pub fn visual(tag: impl Into<String>, content: impl Into<String>) -> Self {
        Block::Visual {
            tag: tag.into(),
            content: content.into(),
            step_base: 0,
        }
    }

    /// The text of a quote's or callout's `blocks` as one run, paragraphs
    /// (and list items and nested quotes) separated by line breaks: for
    /// designs that set a quote as plain text.
    ///
    /// ```
    /// use mdeck_sdk::content::{plain_text, Block, Inline};
    /// let q = [
    ///     Block::paragraph(vec![Inline::text("To be")]),
    ///     Block::quote(vec![Block::paragraph(vec![Inline::text("or not")])]),
    /// ];
    /// assert_eq!(plain_text(&Block::quote_text(&q)), "To be\nor not");
    /// ```
    pub fn quote_text(blocks: &[Block]) -> Vec<Inline> {
        let mut out = Vec::new();
        for b in blocks {
            let part: Vec<Inline> = match b {
                Block::Paragraph { inlines } | Block::Heading { inlines, .. } => inlines.clone(),
                Block::BlockQuote { blocks } | Block::Callout { blocks, .. } => {
                    Block::quote_text(blocks)
                }
                Block::List { items, .. } => {
                    let mut v = Vec::new();
                    for (i, item) in items.iter().enumerate() {
                        if i > 0 {
                            v.push(Inline::text("\n"));
                        }
                        v.extend(item.inlines.iter().cloned());
                    }
                    v
                }
                _ => Vec::new(),
            };
            if part.is_empty() {
                continue;
            }
            if !out.is_empty() {
                out.push(Inline::text("\n"));
            }
            out.extend(part);
        }
        out
    }
}

/// The kind of a callout (a GitHub alert, `> [!NOTE]`).
///
/// See [`Block::callout`] for an example.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CalloutKind {
    /// `[!NOTE]`.
    Note,
    /// `[!TIP]`.
    Tip,
    /// `[!IMPORTANT]`.
    Important,
    /// `[!WARNING]`.
    Warning,
    /// `[!CAUTION]`.
    Caution,
}

impl CalloutKind {
    /// The label a callout shows (`Note`, `Tip`, ...).
    ///
    /// ```
    /// assert_eq!(mdeck_sdk::content::CalloutKind::Warning.label(), "Warning");
    /// ```
    pub fn label(self) -> &'static str {
        match self {
            CalloutKind::Note => "Note",
            CalloutKind::Tip => "Tip",
            CalloutKind::Important => "Important",
            CalloutKind::Warning => "Warning",
            CalloutKind::Caution => "Caution",
        }
    }
}

/// A table column's alignment.
///
/// See [`Block::table`] for an example.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Align {
    /// `:--` or no colon.
    #[default]
    Left,
    /// `:-:`.
    Center,
    /// `--:`.
    Right,
}

/// Size options of an image (`@width`, `@height`, `@fill`).
///
/// ```
/// let mut d = mdeck_sdk::content::ImageDirectives::default();
/// d.width = Some("50%".into());
/// assert!(!d.fill);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ImageDirectives {
    /// Width as written (`50%`, `400px`).
    pub width: Option<String>,
    /// Height as written.
    pub height: Option<String>,
    /// Fill the space, cropping.
    pub fill: bool,
}

/// Inline text and formatting.
///
/// New kinds may be added in a 2.x release: match with a wildcard arm.
///
/// See [`plain_text`] for an example.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
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
    #[non_exhaustive]
    Math {
        /// The TeX source.
        tex: String,
        /// Display (block) math.
        display: bool,
    },
    /// A link.
    #[non_exhaustive]
    Link {
        /// The link text.
        text: Vec<Inline>,
        /// The target.
        url: String,
    },
}

impl Inline {
    /// Plain text.
    ///
    /// See [`plain_text`] for an example.
    pub fn text(s: impl Into<String>) -> Self {
        Inline::Text(s.into())
    }

    /// Math from TeX source, inline or `display`.
    ///
    /// ```
    /// use mdeck_sdk::content::{plain_text, Inline};
    /// assert_eq!(plain_text(&[Inline::math("x^2", false)]), "x^2");
    /// ```
    pub fn math(tex: impl Into<String>, display: bool) -> Self {
        Inline::Math {
            tex: tex.into(),
            display,
        }
    }

    /// A link with `text` to `url`.
    ///
    /// ```
    /// use mdeck_sdk::content::{plain_text, Inline};
    /// assert_eq!(plain_text(&[Inline::link(vec![Inline::text("docs")], "https://x")]), "docs");
    /// ```
    pub fn link(text: Vec<Inline>, url: impl Into<String>) -> Self {
        Inline::Link {
            text,
            url: url.into(),
        }
    }
}

/// A list item, with nested items.
///
/// ```
/// use mdeck_sdk::content::{Inline, ListItem, ListMarker};
/// let mut item = ListItem::new(ListMarker::NextStep, vec![Inline::text("Ship it")]);
/// item.step = 1;
/// item.checked = Some(true);
/// assert_eq!(item.marker, ListMarker::NextStep);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ListItem {
    /// How the item is marked and revealed.
    pub marker: ListMarker,
    /// The item text.
    pub inlines: Vec<Inline>,
    /// Nested items.
    pub children: Vec<ListItem>,
    /// The reveal step the item appears at (0: with the slide). The parser
    /// numbers `+` items across the slide in reading order, visuals
    /// included, and children appear with their parent.
    pub step: usize,
    /// A task list item's box: `Some(true)` for `[x]`, `Some(false)` for
    /// `[ ]`, `None` for an ordinary item.
    pub checked: Option<bool>,
}

impl ListItem {
    /// An item with `inlines`, no children, shown with the slide (step 0)
    /// and no task box.
    ///
    /// See [`ListItem`] for an example.
    pub fn new(marker: ListMarker, inlines: Vec<Inline>) -> Self {
        Self {
            marker,
            inlines,
            children: Vec::new(),
            step: 0,
            checked: None,
        }
    }
}

/// How a list item is marked and revealed.
///
/// See [`ListItem`] for an example.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ListMarker {
    /// `-` or `*`: shown with the slide.
    Static,
    /// `+`: revealed at its own step.
    NextStep,
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
                Block::paragraph(vec![]),
                Block::heading(
                    2,
                    vec![Inline::text("A "), Inline::Italic(vec![Inline::text("b")])],
                ),
                Block::heading(1, vec![Inline::text("later")]),
            ],
            ..Default::default()
        };
        assert_eq!(s.title().as_deref(), Some("A b"));
    }
}
