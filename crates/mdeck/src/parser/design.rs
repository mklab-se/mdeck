//! Which design a slide gets (DES-01..DES-06): the `design:` setting, or the
//! first rule of [`RULES`] its content matches. Both design sets recognise
//! with this one table; a set changes how a design looks, never which
//! design a slide is (DES-10b).

use super::Block;
use super::text::inline_text_len;

/// The 13 slide designs (DES-02).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Design {
    Title,
    Section,
    Statement,
    Points,
    Split,
    Media,
    Gallery,
    Quote,
    Code,
    Visual,
    Columns,
    Table,
    #[default]
    Content,
}

impl Design {
    /// Every design, in catalogue order.
    pub const ALL: [Design; 13] = [
        Design::Title,
        Design::Section,
        Design::Statement,
        Design::Points,
        Design::Split,
        Design::Media,
        Design::Gallery,
        Design::Quote,
        Design::Code,
        Design::Visual,
        Design::Columns,
        Design::Table,
        Design::Content,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Design::Title => "title",
            Design::Section => "section",
            Design::Statement => "statement",
            Design::Points => "points",
            Design::Split => "split",
            Design::Media => "media",
            Design::Gallery => "gallery",
            Design::Quote => "quote",
            Design::Code => "code",
            Design::Visual => "visual",
            Design::Columns => "columns",
            Design::Table => "table",
            Design::Content => "content",
        }
    }

    pub fn from_name(name: &str) -> Option<Design> {
        Design::ALL.into_iter().find(|d| d.name() == name.trim())
    }

    /// What the design is for, one line (the catalogue's purpose column).
    pub fn purpose(self) -> &'static str {
        match self {
            Design::Title => "opens the deck or a part",
            Design::Section => "a divider",
            Design::Statement => "one idea, said big",
            Design::Points => "a heading and a list",
            Design::Split => "text beside a picture",
            Design::Media => "one image, large",
            Design::Gallery => "several images",
            Design::Quote => "a quotation",
            Design::Code => "code with context",
            Design::Visual => "a chart or diagram",
            Design::Columns => "side by side, a comparison",
            Design::Table => "tabular data",
            Design::Content => "anything else, in reading order",
        }
    }

    /// Whether the design has a role for every kind of block on the slide
    /// (headings and paragraphs always have one).
    fn holds(self, s: &Shape) -> bool {
        let (lists, images, code, quotes, visuals, tables) =
            (s.lists, s.images, s.code, s.quotes, s.visuals, s.tables);
        let others = s.callouts;
        let only = |allowed: usize| others == 0 && s.others() == allowed;
        match self {
            Design::Content | Design::Columns => true,
            Design::Title | Design::Section | Design::Statement => only(0),
            Design::Points => only(lists),
            Design::Split => images <= 1 && only(images + lists),
            Design::Media | Design::Gallery => only(images),
            Design::Quote => only(quotes),
            Design::Code => only(code),
            Design::Visual => only(visuals),
            Design::Table => only(tables),
        }
    }

    /// The block the design is built around: without one the design cannot
    /// hold the slide and an explicit `design:` falls back to `content`.
    fn has_core(self, s: &Shape) -> bool {
        match self {
            Design::Title | Design::Section => !s.headings.is_empty(),
            Design::Statement => s.paragraphs > 0,
            Design::Points => s.lists > 0,
            Design::Split | Design::Media | Design::Gallery => s.images > 0,
            Design::Quote => s.quotes > 0,
            Design::Code => s.code > 0,
            Design::Visual => s.visuals > 0,
            Design::Columns => s.separators > 0,
            Design::Table => s.tables > 0,
            Design::Content => true,
        }
    }
}

/// A title's subtitle, a section's kicker: a line of at most this many
/// characters.
pub const SHORT_LINE_CHARS: usize = 120;
/// A statement paragraph: at most this many characters each.
pub const STATEMENT_MAX_CHARS: usize = 240;
/// A statement: at most this many paragraphs.
pub const STATEMENT_MAX_PARAGRAPHS: usize = 2;
/// The lead paragraph of a code, visual or table slide: at most this many
/// characters.
pub const LEAD_MAX_CHARS: usize = 240;

/// The content of a slide, counted the way the rules read it. Horizontal
/// rules are ignored.
#[derive(Debug, Default, Clone)]
pub struct Shape {
    /// The slide is the deck's first.
    pub first: bool,
    /// Heading levels in order.
    pub headings: Vec<u8>,
    pub paragraphs: usize,
    /// Characters of the longest paragraph.
    pub longest_paragraph: usize,
    pub lists: usize,
    pub images: usize,
    pub code: usize,
    pub quotes: usize,
    pub callouts: usize,
    /// Charts, diagrams and thermal images.
    pub visuals: usize,
    pub tables: usize,
    pub separators: usize,
    /// A paragraph comes before the slide's first list.
    pub paragraph_after_list: bool,
    /// A paragraph comes before the slide's quote.
    pub paragraph_before_quote: bool,
    /// Paragraphs before and after the slide's first image.
    pub paragraphs_before_image: usize,
    pub paragraphs_after_image: usize,
    /// The second heading is deeper than the first.
    pub kicker_heading: bool,
}

impl Shape {
    pub fn of(blocks: &[Block], first: bool) -> Self {
        let mut s = Shape {
            first,
            ..Default::default()
        };
        for block in blocks {
            match block {
                Block::Heading { level, .. } => s.headings.push(*level),
                Block::Paragraph { inlines } => {
                    s.paragraphs += 1;
                    let len = inlines.iter().map(inline_text_len).sum();
                    s.longest_paragraph = s.longest_paragraph.max(len);
                    if s.lists > 0 {
                        s.paragraph_after_list = true;
                    }
                    if s.quotes == 0 {
                        s.paragraph_before_quote = true;
                    }
                    if s.images == 0 {
                        s.paragraphs_before_image += 1;
                    } else {
                        s.paragraphs_after_image += 1;
                    }
                }
                Block::List { .. } => s.lists += 1,
                Block::Image { .. } => s.images += 1,
                Block::CodeBlock { .. } => s.code += 1,
                Block::BlockQuote { .. } => s.quotes += 1,
                Block::Callout { .. } => s.callouts += 1,
                Block::Diagram { .. } | Block::Chart { .. } => s.visuals += 1,
                Block::Table { .. } => s.tables += 1,
                Block::ColumnSeparator => s.separators += 1,
                Block::HorizontalRule => {}
            }
        }
        s.kicker_heading = matches!(s.headings.as_slice(), [a, b] if b > a);
        s
    }

    /// Blocks other than headings and paragraphs.
    fn others(&self) -> usize {
        self.lists
            + self.images
            + self.code
            + self.quotes
            + self.callouts
            + self.visuals
            + self.tables
            + self.separators
    }

    fn total(&self) -> usize {
        self.headings.len() + self.paragraphs + self.others()
    }

    fn h1_first(&self) -> bool {
        self.headings.first() == Some(&1)
    }
}

/// One row of the recognition table.
pub struct Rule {
    pub design: Design,
    /// What the slide has, as `--check -v` and the docs print it.
    pub when: &'static str,
    test: fn(&Shape) -> bool,
}

/// The recognition table (DES-03): the first rule a slide's content matches
/// gives its design. `content` holds anything the rules above it do not.
pub const RULES: &[Rule] = &[
    Rule {
        design: Design::Columns,
        when: "a column separator, +++",
        test: |s| s.separators > 0,
    },
    Rule {
        design: Design::Title,
        when: "the first slide, a lone H1",
        test: |s| s.first && s.h1_first() && s.total() == 1,
    },
    Rule {
        design: Design::Title,
        when: "an H1 + one short line, an H2 or a paragraph",
        test: |s| {
            s.h1_first()
                && s.others() == 0
                && match (s.headings.len(), s.paragraphs) {
                    (2, 0) => s.headings[1] == 2,
                    (1, 1) => s.longest_paragraph <= SHORT_LINE_CHARS,
                    _ => false,
                }
        },
    },
    Rule {
        design: Design::Section,
        when: "a lone heading",
        test: |s| s.headings.len() == 1 && s.total() == 1,
    },
    Rule {
        design: Design::Section,
        when: "a heading + a deeper heading, its kicker",
        test: |s| s.kicker_heading && s.total() == 2,
    },
    Rule {
        design: Design::Statement,
        when: "at most a heading + 1 or 2 short paragraphs",
        test: |s| {
            s.headings.len() <= 1
                && (1..=STATEMENT_MAX_PARAGRAPHS).contains(&s.paragraphs)
                && s.longest_paragraph <= STATEMENT_MAX_CHARS
                && s.others() == 0
        },
    },
    Rule {
        design: Design::Points,
        when: "a heading + one list, optionally a lead paragraph before it",
        test: |s| {
            s.headings.len() == 1
                && s.lists == 1
                && s.paragraphs <= 1
                && !s.paragraph_after_list
                && s.others() == 1
        },
    },
    Rule {
        design: Design::Quote,
        when: "one quote, optionally a heading and an attribution after it",
        test: |s| {
            s.quotes == 1
                && s.headings.len() <= 1
                && s.paragraphs <= 1
                && !s.paragraph_before_quote
                && s.others() == 1
        },
    },
    Rule {
        design: Design::Media,
        when: "one image, optionally a heading, a lead before it and a caption after it",
        test: |s| {
            s.images == 1
                && s.headings.len() <= 1
                && s.paragraphs_before_image <= 1
                && s.paragraphs_after_image <= 1
                && s.others() == 1
        },
    },
    Rule {
        design: Design::Gallery,
        when: "two or more images, optionally a heading",
        test: |s| {
            s.images >= 2 && s.headings.len() <= 1 && s.paragraphs == 0 && s.others() == s.images
        },
    },
    Rule {
        design: Design::Split,
        when: "one image + text: paragraphs or one list",
        test: |s| {
            s.images == 1
                && s.headings.len() <= 1
                && s.lists <= 1
                && s.paragraphs + s.lists > 0
                && s.others() == 1 + s.lists
        },
    },
    Rule {
        design: Design::Code,
        when: "one code block, optionally a heading and a short paragraph",
        test: |s| lead_and(s, s.code),
    },
    Rule {
        design: Design::Visual,
        when: "one chart or diagram, optionally a heading and a short paragraph",
        test: |s| lead_and(s, s.visuals),
    },
    Rule {
        design: Design::Table,
        when: "one table, optionally a heading and a short paragraph",
        test: |s| lead_and(s, s.tables),
    },
    Rule {
        design: Design::Content,
        when: "anything else",
        test: |_| true,
    },
];

/// Exactly one block of the kind counted by `n`, at most one heading and
/// one short paragraph, nothing else.
fn lead_and(s: &Shape, n: usize) -> bool {
    n == 1
        && s.others() == 1
        && s.headings.len() <= 1
        && s.paragraphs <= 1
        && s.longest_paragraph <= LEAD_MAX_CHARS
}

/// How a slide got its design, for `--check -v` and `--check`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Reason {
    /// The rule of [`RULES`] the content matched.
    #[default]
    Recognised,
    /// The slide's `design:` setting, and the content fits it.
    Chosen,
    /// The slide's `design:` setting; content its roles do not take shows
    /// in its body.
    ChosenWithRest,
    /// The slide's `design:` setting named a design its content has no
    /// core for (a quote design without a quote): it is a `content` slide.
    FellBack(Design),
}

/// The design a slide gets, and why (the rule's text).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Recognition {
    pub design: Design,
    pub reason: Reason,
    /// The matching rule's text (`when`), for a recognised slide.
    pub rule: &'static str,
}

impl Recognition {
    /// `statement (heading + ...)` as `--check -v` prints it.
    pub fn describe(&self) -> String {
        match &self.reason {
            Reason::Recognised => format!("{} ({})", self.design.name(), self.rule),
            Reason::Chosen => format!("{} (design setting)", self.design.name()),
            Reason::ChosenWithRest => format!(
                "{} (design setting; what its roles do not take shows in the body)",
                self.design.name()
            ),
            Reason::FellBack(wanted) => format!(
                "content (design: {} has no {} here)",
                wanted.name(),
                core_name(*wanted)
            ),
        }
    }
}

fn core_name(d: Design) -> &'static str {
    match d {
        Design::Title | Design::Section => "heading",
        Design::Statement => "paragraph",
        Design::Points => "list",
        Design::Split | Design::Media | Design::Gallery => "image",
        Design::Quote => "quote",
        Design::Code => "code block",
        Design::Visual => "chart or diagram",
        Design::Columns => "column separator (+++)",
        Design::Table => "table",
        Design::Content => "content",
    }
}

/// The design of a slide whose blocks are `blocks`: its `design:` setting
/// when it names one, else the first matching rule. `first` says whether
/// it is the deck's first slide.
pub fn recognise(design: Option<&str>, blocks: &[Block], first: bool) -> Recognition {
    let shape = Shape::of(blocks, first);
    let matched = RULES
        .iter()
        .find(|r| (r.test)(&shape))
        .expect("the last rule matches everything");
    let Some(wanted) = design.and_then(Design::from_name) else {
        return Recognition {
            design: matched.design,
            reason: Reason::Recognised,
            rule: matched.when,
        };
    };
    if !wanted.has_core(&shape) {
        return Recognition {
            design: Design::Content,
            reason: Reason::FellBack(wanted),
            rule: "",
        };
    }
    // the content fits the chosen design when it has a role for every block
    let fits = wanted.holds(&shape);
    Recognition {
        design: wanted,
        reason: if fits {
            Reason::Chosen
        } else {
            Reason::ChosenWithRest
        },
        rule: "",
    }
}

/// The recognition table as markdown, for the format reference and the docs
/// (`<!-- generated: designs -->`).
pub fn rules_reference() -> String {
    let mut out = String::from("| Design | For |\n|---|---|\n");
    for d in Design::ALL {
        out.push_str(&format!("| `{}` | {} |\n", d.name(), d.purpose()));
    }
    out.push_str(
        "\nThe recognition table, in order: the first rule a slide matches gives its design.\n\n",
    );
    out.push_str("| # | Design | The slide has |\n|---|---|---|\n");
    for (i, r) in RULES.iter().enumerate() {
        out.push_str(&format!(
            "| {} | `{}` | {} |\n",
            i + 1,
            r.design.name(),
            r.when
        ));
    }
    out.push_str(&format!(
        "\nA short line is at most {SHORT_LINE_CHARS} characters; a statement has at most \
         {STATEMENT_MAX_PARAGRAPHS} paragraphs of at most {STATEMENT_MAX_CHARS} characters; the \
         lead paragraph of a code, visual or table slide is at most {LEAD_MAX_CHARS} characters. \
         Horizontal rules do not count.\n"
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::blocks;

    fn of(md: &str) -> Recognition {
        recognise(None, &blocks::parse(md), false)
    }

    fn design(md: &str) -> Design {
        of(md).design
    }

    #[test]
    fn every_rule_has_a_slide_that_matches_it_first() {
        // one example per row of the table, in order: the example must be
        // recognised by exactly that row
        let examples: &[(&str, bool)] = &[
            ("# A\n\nleft\n\n+++\n\nright", false),
            ("# Welcome", true),
            ("# Welcome\n\n## A subtitle", false),
            ("## Part two", false),
            ("## Part two\n\n### Where it goes", false),
            ("## Heading\n\nOne idea, said in a sentence.", false),
            ("## Points\n\nLead.\n\n- a\n- b", false),
            ("## Wise\n\n> Stay hungry.\n\n-- Steve", false),
            ("## Photo\n\n![a](a.png)\n\nA caption", false),
            ("## Photos\n\n![a](a.png)\n\n![b](b.png)", false),
            ("## Split\n\n![a](a.png)\n\n- one\n- two", false),
            ("## Code\n\nRun it:\n\n```rust\nfn main() {}\n```", false),
            ("## Chart\n\n```@pie\n- A: 1\n- B: 2\n```", false),
            ("## Table\n\n| a | b |\n|---|---|\n| 1 | 2 |", false),
            ("## Mixed\n\n- a\n\n```rust\nx\n```", false),
        ];
        assert_eq!(examples.len(), RULES.len());
        for (i, (md, first)) in examples.iter().enumerate() {
            let r = recognise(None, &blocks::parse(md), *first);
            assert_eq!(r.rule, RULES[i].when, "example {i}: {md}");
            assert_eq!(r.design, RULES[i].design, "example {i}");
        }
    }

    #[test]
    fn the_common_slide_is_a_statement() {
        assert_eq!(
            design("## Why\n\nBecause slides should be easy."),
            Design::Statement
        );
        assert_eq!(design("Just one sentence."), Design::Statement);
        let long = "word ".repeat(60);
        assert_eq!(design(&format!("## Why\n\n{long}")), Design::Content);
        assert_eq!(design("## A\n\none\n\ntwo\n\nthree"), Design::Content);
    }

    #[test]
    fn title_and_section_borders() {
        // a lone H1 opens the deck as a title, later it is a divider
        assert_eq!(
            recognise(None, &blocks::parse("# Hi"), true).design,
            Design::Title
        );
        assert_eq!(design("# Hi"), Design::Section);
        assert_eq!(design("# Hi\n\nShort subtitle"), Design::Title);
        // an H1 with an H3 is a section with a kicker, not a title
        assert_eq!(design("# Hi\n\n### deeper"), Design::Section);
        // 40 CJK characters are a short line (characters, not bytes)
        let sub = "漢".repeat(40);
        assert_eq!(design(&format!("# タイトル\n\n{sub}")), Design::Title);
    }

    #[test]
    fn visuals_no_longer_take_the_whole_slide() {
        // DES-07: a diagram with bullets is a content slide that shows both
        let md = "## A\n\n- one\n\n```@architecture\nA -> B\n```";
        assert_eq!(design(md), Design::Content);
        let md = "## Two\n\n```@pie\n- A: 1\n```\n\n```@pie\n- B: 1\n```";
        assert_eq!(design(md), Design::Content);
    }

    #[test]
    fn a_chosen_design_is_honoured_and_reported() {
        let b = blocks::parse("## A\n\n- one\n\n> q");
        let r = recognise(Some("quote"), &b, false);
        assert_eq!(
            (r.design, r.reason.clone()),
            (Design::Quote, Reason::ChosenWithRest)
        );
        let r = recognise(Some("points"), &blocks::parse("## A\n\n- one"), false);
        assert_eq!(r.reason, Reason::Chosen);
        // no quote: the slide falls back to content
        let r = recognise(Some("quote"), &blocks::parse("## A\n\n- one"), false);
        assert_eq!(r.design, Design::Content);
        assert_eq!(r.reason, Reason::FellBack(Design::Quote));
        assert!(r.describe().contains("no quote"), "{}", r.describe());
        // an unknown name is recognised (and reported by the settings check)
        let r = recognise(Some("sideways"), &blocks::parse("## A\n\n- one"), false);
        assert_eq!(r.design, Design::Points);
    }

    #[test]
    fn names_round_trip_and_match_the_language_table() {
        for d in Design::ALL {
            assert_eq!(Design::from_name(d.name()), Some(d));
        }
        let names: Vec<&str> = Design::ALL.iter().map(|d| d.name()).collect();
        assert_eq!(names, crate::language::DESIGNS);
    }

    #[test]
    fn the_user_docs_list_every_rule() {
        let docs = include_str!("../../../../docs/writing-slides.md");
        for (i, rule) in RULES.iter().enumerate() {
            let row = format!("| {} | `{}` | ", i + 1, rule.design.name());
            assert!(docs.contains(&row), "docs/writing-slides.md lacks {row}");
            let when = rule
                .when
                .replace("+++", "`+++`")
                .replace("separator, `+++`", "separator, `+++`");
            assert!(
                docs.contains(&when) || docs.contains(rule.when),
                "docs/writing-slides.md lacks `{}`",
                rule.when
            );
        }
        assert!(docs.contains(&format!("at most {SHORT_LINE_CHARS} characters")));
    }

    #[test]
    fn the_reference_lists_every_rule_and_threshold() {
        let r = rules_reference();
        for rule in RULES {
            assert!(r.contains(rule.when));
        }
        assert!(r.contains(&SHORT_LINE_CHARS.to_string()));
        assert!(r.contains(&STATEMENT_MAX_CHARS.to_string()));
    }
}
