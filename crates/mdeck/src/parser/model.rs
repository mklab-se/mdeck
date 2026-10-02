//! The document model the parser produces.

use super::{Design, Layout, Recognition};

#[derive(Debug, Clone)]
pub struct Presentation {
    pub meta: PresentationMeta,
    pub slides: Vec<Slide>,
}

#[derive(Debug, Clone, Default)]
pub struct PresentationMeta {
    pub title: Option<String>,
    pub author: Option<String>,
    pub theme: Option<String>,
    pub transition: Option<String>,
    pub footer: Option<String>,
    pub image_style: Option<String>,
    pub icon_style: Option<String>,
    pub slide_level: Option<u8>,
    /// `countdown: on|off`: the engine's opening countdown.
    pub countdown: Option<bool>,
    /// `reveal: none` shows every `+` item at once.
    pub reveal: Option<bool>,
    /// `engine`: run the deck on this engine instead of the theme's.
    pub engine: Option<String>,
    /// `logo`: a PNG or SVG shown on every slide (`none` hides a theme's logo).
    pub logo: Option<String>,
    /// `logo-position`: top-left, top-right, bottom-left or bottom-right.
    pub logo_position: Option<String>,
    /// `logo-opacity`: 0 to 1, or a percentage.
    pub logo_opacity: Option<String>,
    /// `logo-height`: height in px on a 1920x1080 slide.
    pub logo_height: Option<String>,
    /// `background`: an image behind every slide (`none` for no image).
    pub background: Option<String>,
    /// `background-opacity`: 0 to 1, or a percentage.
    pub background_opacity: Option<String>,
    /// `art-world`: the deck's world for generated art (setting, era,
    /// recurring characters).
    pub art_world: Option<String>,
    /// `palette`: the palette of `@thermal` blocks that name none.
    pub palette: Option<String>,
    /// Every frontmatter key as written, known or not, with its line (for
    /// `--check`).
    pub settings: Vec<Setting>,
}

#[derive(Debug, Clone, Default)]
pub struct Slide {
    /// The slide's settings as written in its settings comments, known or
    /// not, in order.
    pub settings: Vec<Setting>,
    pub blocks: Vec<Block>,
    /// The slide's design: its `design:` setting or the recognised one.
    pub design: Design,
    /// How the design was decided (for `--check`).
    pub recognition: Recognition,
    /// The v1 layout kind closest to the design, for the engines that key
    /// their scenery on it (see [`Layout`]).
    pub layout: Layout,
    /// The original raw markdown source text for this slide.
    pub raw_source: String,
    /// 1-based line in the deck file where the slide starts (its first
    /// `raw_source` line), frontmatter included; 0 when not parsed from a file.
    pub line: usize,
    /// 1-based line in the deck file of each `raw_source` line.
    pub source_lines: Vec<usize>,
    /// Speaker notes for this slide: the markdown of its ```` ```@notes ````
    /// blocks, joined in order, then the text of its footnotes.
    pub notes: Option<String>,
    /// Whether `+` items are steps on this slide (`reveal: none` turns them off).
    pub reveal: bool,
    /// The slide's steps: the last step any item or visual on it is shown at.
    pub steps: usize,
    /// Name of the point cloud picture for this slide (`picture:`).
    pub illustration: Option<String>,
    /// This slide's `logo`: a PNG or SVG path, or `none` to hide the logo here.
    pub logo: Option<String>,
    /// What generated art draws for this slide (`picture-prompt`), or `none`
    /// when `picture: none` keeps the slide empty.
    pub art: Option<String>,
    /// What the parser could not take as written: malformed settings lines,
    /// content that will not show as meant. `--check` reports them.
    pub problems: Vec<Problem>,
}

impl Slide {
    /// The deck file line of the slide setting `name` (the last one when
    /// it is written twice, as in [`super::setting`]), else the slide's.
    pub fn setting_line(&self, name: &str) -> usize {
        self.settings
            .iter()
            .rev()
            .find(|d| d.name == name)
            .map_or(self.line, |d| d.line)
    }

    /// The deck file line of `raw_source` line `offset` (0-based).
    pub fn line_at(&self, offset: usize) -> usize {
        self.source_lines
            .get(offset)
            .copied()
            .unwrap_or(self.line + offset)
    }

    /// The plain text of the slide's first heading.
    pub fn title(&self) -> Option<String> {
        self.blocks.iter().find_map(|b| match b {
            Block::Heading { inlines, .. } => Some(
                inlines
                    .iter()
                    .filter_map(|i| match i {
                        Inline::Text(s) => Some(s.as_str()),
                        _ => None,
                    })
                    .collect::<String>(),
            ),
            _ => None,
        })
    }
}

/// One `key: value` setting as written.
#[derive(Debug, Clone)]
pub struct Setting {
    pub name: String,
    pub value: String,
    /// 1-based line in the deck file ([`super::parse`] maps it; from the
    /// settings reader alone it is the 0-based line within the slide).
    pub line: usize,
}

/// Something in a slide the parser could not take as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub kind: ProblemKind,
    /// 1-based deck file line (0-based within the slide before mapping).
    pub line: usize,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProblemKind {
    /// A settings comment line that is not `key: value`.
    Setting,
    /// Markdown that will not show as meant.
    Content,
}

#[derive(Debug, Clone)]
#[allow(clippy::enum_variant_names)]
pub enum Block {
    Heading {
        level: u8,
        inlines: Vec<Inline>,
    },
    Paragraph {
        inlines: Vec<Inline>,
    },
    List {
        ordered: bool,
        /// The number of an ordered list's first item.
        start: u32,
        items: Vec<ListItem>,
    },
    Image {
        alt: String,
        path: String,
        directives: ImageDirectives,
    },
    CodeBlock {
        language: Option<String>,
        code: String,
        highlight_lines: Vec<usize>,
    },
    /// A quote with its own blocks: paragraphs, lists, nested quotes.
    BlockQuote {
        blocks: Vec<Block>,
    },
    /// A GitHub alert (`> [!NOTE]`), drawn as a callout.
    Callout {
        kind: Alert,
        blocks: Vec<Block>,
    },
    Table {
        headers: Vec<Vec<Inline>>,
        /// Each column's alignment from the separator row.
        align: Vec<Align>,
        rows: Vec<Vec<Vec<Inline>>>,
    },
    HorizontalRule,
    Diagram {
        content: String,
        /// Steps on the slide before this diagram's own (they follow on).
        step_base: usize,
    },
    /// A ```@chart fence: one of the [`Chart`] visualizations.
    Chart {
        kind: Chart,
        content: String,
        /// Steps on the slide before this chart's own (they follow on).
        step_base: usize,
    },
    ColumnSeparator,
}

impl Block {
    /// The paragraphs of a quote, as inline runs; lists and nested quotes
    /// inside it give their text.
    pub fn quote_paragraphs(blocks: &[Block]) -> Vec<Vec<Inline>> {
        let mut out = Vec::new();
        for b in blocks {
            match b {
                Block::Paragraph { inlines } | Block::Heading { inlines, .. } => {
                    out.push(inlines.clone())
                }
                Block::BlockQuote { blocks } | Block::Callout { blocks, .. } => {
                    out.extend(Block::quote_paragraphs(blocks))
                }
                Block::List { items, .. } => {
                    for item in items {
                        out.push(item.inlines.clone());
                    }
                }
                _ => {}
            }
        }
        out
    }

    /// A quote slide's quotation and attribution. With no paragraph after
    /// the quote (`after` false), a quote of several paragraphs ends in its
    /// attribution (`> text`, `>`, `> Who`), which never runs into the
    /// quotation (D25).
    pub fn quote_parts(blocks: &[Block], after: bool) -> (Vec<Inline>, Option<Vec<Inline>>) {
        // The attribution is a short last paragraph of the quote's own.
        let own = match blocks {
            [rest @ .., Block::Paragraph { inlines }]
                if !after
                    && !rest.is_empty()
                    && super::text::inlines_to_text(inlines).chars().count() <= 80 =>
            {
                Some((rest, inlines.clone()))
            }
            _ => None,
        };
        let (blocks, attribution) = match own {
            Some((rest, a)) => (rest, Some(a)),
            None => (blocks, None),
        };
        let paragraphs = Block::quote_paragraphs(blocks);
        let mut quote = Vec::new();
        for (i, p) in paragraphs.into_iter().enumerate() {
            if i > 0 {
                quote.push(Inline::Text("\n".into()));
            }
            quote.extend(p);
        }
        (quote, attribution)
    }

    /// A quote's text as one run, paragraphs separated by line breaks.
    pub fn quote_inlines(blocks: &[Block]) -> Vec<Inline> {
        let mut out = Vec::new();
        for (i, p) in Block::quote_paragraphs(blocks).into_iter().enumerate() {
            if i > 0 {
                out.push(Inline::Text("\n".into()));
            }
            out.extend(p);
        }
        out
    }
}

/// The kind of a GitHub alert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alert {
    Note,
    Tip,
    Important,
    Warning,
    Caution,
}

impl Alert {
    /// The alert a `[!NOTE]` marker names (case-insensitive).
    pub fn from_marker(marker: &str) -> Option<Alert> {
        match marker.to_ascii_uppercase().as_str() {
            "NOTE" => Some(Alert::Note),
            "TIP" => Some(Alert::Tip),
            "IMPORTANT" => Some(Alert::Important),
            "WARNING" => Some(Alert::Warning),
            "CAUTION" => Some(Alert::Caution),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Alert::Note => "Note",
            Alert::Tip => "Tip",
            Alert::Important => "Important",
            Alert::Warning => "Warning",
            Alert::Caution => "Caution",
        }
    }
}

/// A table column's alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Default)]
pub struct ImageDirectives {
    /// `@width: 60%`: a share of the space, or pixels at 1920x1080.
    pub width: Option<String>,
    /// `@height: 400px`: likewise.
    pub height: Option<String>,
    /// `@fill`: cover the space, cropping.
    pub fill: bool,
    /// Options that were not understood, for `--check`.
    pub problems: Vec<String>,
}

#[derive(Debug, Clone)]
#[allow(clippy::enum_variant_names)]
pub enum Inline {
    Text(String),
    Bold(Vec<Inline>),
    Italic(Vec<Inline>),
    Strikethrough(Vec<Inline>),
    Code(String),
    /// LaTeX math: `$...$` inline, `$$...$$` display.
    Math {
        tex: String,
        display: bool,
    },
    Link {
        text: Vec<Inline>,
        url: String,
    },
}

/// The visual a fenced block holds, named by its fence's tag (without
/// `@`). Visual kinds are looked up in the registry (D7, D11): a tag no
/// registered visual has is not a visual, and its fence stays a code block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Chart(&'static str);

#[allow(non_upper_case_globals, reason = "built-in kinds read like variants")]
impl Chart {
    /// A thermal image with its lens, reveals and spots (`@thermal`).
    pub const Thermal: Chart = Chart("thermal");

    /// The visual a fence info string (```` ```@bar ````) names, when one is
    /// registered under that tag.
    pub fn from_info(info: &str) -> Option<Chart> {
        let tag = info.split_whitespace().next()?.strip_prefix('@')?;
        if let Some(visual) = crate::registry::get().visual_for(tag) {
            return Some(Chart(visual.tag()));
        }
        crate::extensions::external::configured_tag(tag).map(Chart)
    }

    /// Whether this is an external visual program's tag (EXT-18) rather
    /// than a registered visual.
    pub fn is_external(self) -> bool {
        crate::registry::get().visual_for(self.0).is_none()
    }

    /// The fence tag without `@` (`bar`).
    pub fn tag(self) -> &'static str {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct ListItem {
    pub marker: ListMarker,
    pub inlines: Vec<Inline>,
    pub children: Vec<ListItem>,
    /// The slide step the item appears at (0: from the start). Set by
    /// [`super::steps::number`].
    pub step: usize,
    /// A task list item's box: `Some(true)` for `[x]`, `Some(false)` for `[ ]`.
    pub checked: Option<bool>,
}

impl ListItem {
    pub fn new(marker: ListMarker, inlines: Vec<Inline>, children: Vec<ListItem>) -> Self {
        ListItem {
            marker,
            inlines,
            children,
            step: 0,
            checked: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ListMarker {
    /// `-` or `*`: always shown.
    Static,
    /// `+`: shown at its own step.
    NextStep,
    Ordered,
}
