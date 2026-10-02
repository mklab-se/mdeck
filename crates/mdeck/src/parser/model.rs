//! The document model the parser produces.

use super::Layout;

#[derive(Debug, Clone)]
pub struct Presentation {
    pub meta: PresentationMeta,
    pub slides: Vec<Slide>,
}

#[derive(Debug, Clone, Default)]
pub struct PresentationMeta {
    pub title: Option<String>,
    pub author: Option<String>,
    pub date: Option<String>,
    pub theme: Option<String>,
    pub transition: Option<String>,
    pub aspect: Option<String>,
    pub code_theme: Option<String>,
    pub footer: Option<String>,
    pub image_style: Option<String>,
    pub icon_style: Option<String>,
    pub slide_level: Option<u8>,
    /// `@countdown: false` turns off the opening countdown (Ember and Nord).
    pub countdown: Option<bool>,
    /// `@engine`: run the deck on this engine instead of the theme's.
    pub engine: Option<String>,
    /// `@logo`: a PNG or SVG shown on every slide (`none` hides a theme's logo).
    pub logo: Option<String>,
    /// `@logo-position`: top-left, top-right, bottom-left or bottom-right.
    pub logo_position: Option<String>,
    /// `@logo-opacity`: 0 to 1, or a percentage.
    pub logo_opacity: Option<String>,
    /// `@logo-height`: height in px on a 1920x1080 slide.
    pub logo_height: Option<String>,
    /// `@background`: an image behind every slide (`none` for no image).
    pub background: Option<String>,
    /// `@background-opacity`: 0 to 1, or a percentage.
    pub background_opacity: Option<String>,
    /// `@art` in the frontmatter: the deck's world for generated art
    /// (setting, era, recurring characters).
    pub art: Option<String>,
    /// `@palette`: the palette of `@thermal` blocks that name none.
    pub palette: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Slide {
    pub directives: Vec<Directive>,
    pub blocks: Vec<Block>,
    pub layout: Layout,
    /// The original raw markdown source text for this slide.
    pub raw_source: String,
    /// 1-based line in the deck file where the slide starts (its first
    /// `raw_source` line), frontmatter included; 0 when not parsed from a file.
    pub line: usize,
    /// 1-based line in the deck file of each `raw_source` line. Not always
    /// `line` plus the offset: blank lines between directives moved to the
    /// next heading's slide are dropped from `raw_source`.
    pub source_lines: Vec<usize>,
    /// Speaker notes for this slide (content after `???` separator).
    pub notes: Option<String>,
    /// Name of the point cloud illustration for this slide (`@illustration`).
    pub illustration: Option<String>,
    /// This slide's `@logo`: a PNG or SVG path, or `none` to hide the logo here.
    pub logo: Option<String>,
    /// This slide's `@art`: the scene to draw, or `none` for no art here.
    pub art: Option<String>,
}

impl Slide {
    /// The deck file line of the slide directive `name` (the last one when
    /// it is written twice, as in [`super::directive`]), else the slide's.
    pub fn directive_line(&self, name: &str) -> usize {
        self.directives
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

#[derive(Debug, Clone)]
pub struct Directive {
    pub name: String,
    pub value: String,
    /// 1-based line in the deck file ([`super::parse`] maps it; from
    /// `extract_directives` alone it is the 0-based line within the slide).
    pub line: usize,
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
    BlockQuote {
        inlines: Vec<Inline>,
    },
    Table {
        headers: Vec<Vec<Inline>>,
        rows: Vec<Vec<Vec<Inline>>>,
    },
    HorizontalRule,
    Diagram {
        content: String,
    },
    /// A ```@chart fence: one of the [`Chart`] visualizations.
    Chart {
        kind: Chart,
        content: String,
    },
    ColumnSeparator,
}

#[derive(Debug, Clone, Default)]
pub struct ImageDirectives {
    pub width: Option<String>,
    pub height: Option<String>,
    pub fill: bool,
    pub fit: bool,
    pub align: Option<String>,
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
        #[cfg_attr(
            not(test),
            expect(
                dead_code,
                reason = "slides draw link text only; the target is kept in the document model"
            )
        )]
        url: String,
    },
}

/// The visualizations a fenced block can hold, named by the fence's tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chart {
    WordCloud,
    Timeline,
    Pie,
    Bar,
    Line,
    Donut,
    KpiCards,
    Funnel,
    Radar,
    StackedBar,
    VennDiagram,
    ProgressBars,
    ScatterPlot,
    Org,
    Gantt,
    GitGraph,
    /// A platform in the middle and the teams around it (`@flower`).
    Flower,
    /// Artifacts from producers through services to consumers (`@artifactflow`).
    ArtifactFlow,
    /// A thermal image with its lens, reveals and spots (`@thermal`).
    Thermal,
}

impl Chart {
    /// Fence tags and their charts. A fence matches the first tag its info
    /// string starts with (`@donut` also covers `@donutchart`).
    pub const TAGS: &[(&str, Chart)] = &[
        ("@wordcloud", Chart::WordCloud),
        ("@timeline", Chart::Timeline),
        ("@piechart", Chart::Pie),
        ("@barchart", Chart::Bar),
        ("@linechart", Chart::Line),
        ("@donut", Chart::Donut),
        ("@kpi", Chart::KpiCards),
        ("@funnel", Chart::Funnel),
        ("@radar", Chart::Radar),
        ("@stackedbar", Chart::StackedBar),
        ("@venn", Chart::VennDiagram),
        ("@progress", Chart::ProgressBars),
        ("@scatter", Chart::ScatterPlot),
        ("@orgchart", Chart::Org),
        ("@gantt", Chart::Gantt),
        ("@gitgraph", Chart::GitGraph),
        ("@flower", Chart::Flower),
        ("@artifactflow", Chart::ArtifactFlow),
        ("@thermal", Chart::Thermal),
    ];

    /// The chart a fence info string (```` ```@barchart ````) names.
    pub fn from_info(info: &str) -> Option<Chart> {
        Self::TAGS
            .iter()
            .find(|(tag, _)| info.starts_with(tag))
            .map(|&(_, chart)| chart)
    }
}

#[derive(Debug, Clone)]
pub struct ListItem {
    pub marker: ListMarker,
    pub inlines: Vec<Inline>,
    pub children: Vec<ListItem>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ListMarker {
    Static,
    NextStep,
    WithPrev,
    Ordered,
}
