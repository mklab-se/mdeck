//! Writing a slide's blocks as lines of flap cells: text capitalised
//! and wrapped, lists, tables, figures and progress bars, with what the
//! flaps cannot show collected on the way.

use std::collections::BTreeSet;

use super::layout::{Cell, Style};
use super::wheel::{SOLID, flap_chars};
use mdeck_sdk::content::{Block, Inline, ListItem, ListMarker};

/// One styled line of text, before placement.
#[derive(Clone, Debug, Default)]
pub(super) struct Line {
    pub(super) cells: Vec<Cell>,
    /// Reveal step that shows this line.
    pub(super) step: usize,
}

/// Collects lines, missing characters and unsupported content while a
/// slide is laid out.
pub(super) struct Writer {
    pub(super) lines: Vec<Line>,
    pub(super) missing: BTreeSet<char>,
    pub(super) unsupported: BTreeSet<String>,
}

impl Writer {
    pub(super) fn new() -> Self {
        Writer {
            lines: Vec::new(),
            missing: BTreeSet::new(),
            unsupported: BTreeSet::new(),
        }
    }

    /// Styled characters of `inlines`, capitalised; formulas are reported.
    pub(super) fn styled(&mut self, inlines: &[Inline], base: Style, out: &mut Vec<Cell>) {
        for inline in inlines {
            match inline {
                Inline::Text(t) => self.push_text(t, base, out),
                Inline::Bold(c) => {
                    let s = if base == Style::Normal {
                        Style::Strong
                    } else {
                        base
                    };
                    self.styled(c, s, out)
                }
                Inline::Italic(c) | Inline::Strikethrough(c) => self.styled(c, base, out),
                Inline::Code(t) => self.push_text(t, Style::Code, out),
                Inline::Link { text, .. } => self.styled(text, base, out),
                Inline::Math { .. } => {
                    self.unsupported.insert("formulas".into());
                }
            }
        }
    }

    fn push_text(&mut self, text: &str, style: Style, out: &mut Vec<Cell>) {
        for c in text.chars() {
            for f in flap_chars(c) {
                match f {
                    Some(ch) => out.push(Cell { ch, style }),
                    None => {
                        if !c.is_control() {
                            self.missing.insert(c);
                        }
                        out.push(Cell { ch: ' ', style });
                    }
                }
            }
        }
    }

    /// Word-wrap `cells` to `width`, the first line after `first` cells of
    /// prefix and the rest indented by `indent`.
    fn wrap(
        &mut self,
        prefix: Vec<Cell>,
        cells: Vec<Cell>,
        indent: usize,
        width: usize,
        step: usize,
    ) {
        let width = width.max(indent + 4);
        let mut words: Vec<Vec<Cell>> = Vec::new();
        let mut cur = Vec::new();
        for c in cells {
            if c.ch == ' ' {
                if !cur.is_empty() {
                    words.push(std::mem::take(&mut cur));
                }
            } else {
                cur.push(c);
            }
        }
        if !cur.is_empty() {
            words.push(cur);
        }
        let mut line = prefix;
        let mut fresh = true;
        for mut word in words {
            loop {
                let room = width.saturating_sub(line.len());
                let need = word.len() + usize::from(!fresh);
                if need <= room {
                    if !fresh {
                        line.push(Cell::BLANK);
                    }
                    line.extend(word);
                    fresh = false;
                    break;
                }
                if fresh {
                    // a word longer than the line: break it
                    let take = room.max(1);
                    let rest = word.split_off(take.min(word.len()));
                    line.extend(word);
                    self.lines.push(Line {
                        cells: std::mem::take(&mut line),
                        step,
                    });
                    line = vec![Cell::BLANK; indent];
                    word = rest;
                    if word.is_empty() {
                        break;
                    }
                    continue;
                }
                self.lines.push(Line {
                    cells: std::mem::take(&mut line),
                    step,
                });
                line = vec![Cell::BLANK; indent];
                fresh = true;
            }
        }
        if line.iter().any(|c| c.ch != ' ') || self.lines.is_empty() {
            self.lines.push(Line { cells: line, step });
        }
    }

    fn blank(&mut self, step: usize) {
        if self
            .lines
            .last()
            .is_some_and(|l| l.cells.iter().any(|c| c.ch != ' '))
        {
            self.lines.push(Line {
                cells: Vec::new(),
                step,
            });
        }
    }

    fn list(&mut self, items: &[ListItem], ordered: bool, depth: usize, start: u32, width: usize) {
        for (k, item) in items.iter().enumerate() {
            let step = item.step;
            let indent = depth * 2;
            let mut prefix = vec![Cell::BLANK; indent];
            if ordered {
                for ch in format!("{}.", start as usize + k).chars() {
                    prefix.push(Cell {
                        ch,
                        style: Style::Accent,
                    });
                }
            } else {
                prefix.push(Cell {
                    ch: '•',
                    style: Style::Accent,
                });
            }
            prefix.push(Cell::BLANK);
            let hang = prefix.len();
            let mut cells = Vec::new();
            self.styled(&item.inlines, Style::Normal, &mut cells);
            self.wrap(prefix, cells, hang, width, step);
            let child_ordered = item
                .children
                .first()
                .is_some_and(|c| c.marker == ListMarker::Ordered);
            self.list(&item.children, child_ordered, depth + 1, 1, width);
        }
    }

    /// A table as a timetable: columns on cell boundaries, numbers right
    /// aligned, the header dim. Columns that do not fit are cut with `…`.
    fn table(&mut self, headers: &[Vec<Inline>], rows: &[Vec<Vec<Inline>>], width: usize) {
        let mut grid: Vec<Vec<Vec<Cell>>> = Vec::new();
        let mut head = Vec::new();
        for h in headers {
            let mut cells = Vec::new();
            self.styled(h, Style::Dim, &mut cells);
            head.push(cells);
        }
        grid.push(head);
        for row in rows {
            let mut line = Vec::new();
            for cell in row {
                let mut cells = Vec::new();
                self.styled(cell, Style::Normal, &mut cells);
                line.push(cells);
            }
            grid.push(line);
        }
        let ncols = grid.iter().map(Vec::len).max().unwrap_or(0);
        if ncols == 0 {
            return;
        }
        let mut widths: Vec<usize> = (0..ncols)
            .map(|c| {
                grid.iter()
                    .filter_map(|r| r.get(c))
                    .map(Vec::len)
                    .max()
                    .unwrap_or(0)
            })
            .collect();
        let gap = 2;
        // shrink the widest column until the table fits
        while widths.iter().sum::<usize>() + gap * (ncols - 1) > width {
            let (i, w) = widths
                .iter()
                .copied()
                .enumerate()
                .max_by_key(|(_, w)| *w)
                .expect("columns");
            if w <= 3 {
                break;
            }
            widths[i] = w - 1;
        }
        let numeric: Vec<bool> = (0..ncols)
            .map(|c| {
                grid.iter().skip(1).filter_map(|r| r.get(c)).all(|cells| {
                    let s: String = cells.iter().map(|c| c.ch).collect();
                    let s = s.trim();
                    !s.is_empty()
                        && s.chars()
                            .all(|ch| ch.is_ascii_digit() || ".,:%+-$€£ ".contains(ch))
                })
            })
            .collect();
        for (r, row) in grid.iter().enumerate() {
            let mut line = Vec::new();
            for (c, w) in widths.iter().copied().enumerate() {
                if c > 0 {
                    line.extend(std::iter::repeat_n(Cell::BLANK, gap));
                }
                let mut cells = row.get(c).cloned().unwrap_or_default();
                if cells.len() > w {
                    cells.truncate(w.saturating_sub(1));
                    cells.push(Cell {
                        ch: '…',
                        style: Style::Dim,
                    });
                    self.unsupported.insert("table cells cut to fit".into());
                }
                let pad = w - cells.len();
                if numeric[c] && r > 0 {
                    line.extend(std::iter::repeat_n(Cell::BLANK, pad));
                    line.extend(cells);
                } else {
                    line.extend(cells);
                    line.extend(std::iter::repeat_n(Cell::BLANK, pad));
                }
            }
            self.lines.push(Line {
                cells: line,
                step: 0,
            });
        }
    }

    /// `- Label: value` lines (KPI cards) as label left, value right.
    fn figures(&mut self, content: &str, width: usize) {
        for (label, value) in label_values(content) {
            let value = value
                .split("(trend")
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            let mut l = Vec::new();
            self.push_text(&label, Style::Normal, &mut l);
            let mut v = Vec::new();
            self.push_text(&value, Style::Heading, &mut v);
            let room = width.saturating_sub(v.len() + 1);
            l.truncate(room);
            let pad = width - l.len() - v.len();
            l.extend(std::iter::repeat_n(Cell::BLANK, pad));
            l.extend(v);
            self.lines.push(Line { cells: l, step: 0 });
        }
    }

    /// `- Label: 85%` lines (progress bars) as label, a bar of solid flaps
    /// and the value.
    fn progress(&mut self, content: &str, width: usize) {
        let entries = label_values(content);
        let label_w = entries
            .iter()
            .map(|(l, _)| l.chars().count())
            .max()
            .unwrap_or(0)
            .min(width / 2);
        for (label, value) in entries {
            let pct: f32 = value
                .trim()
                .trim_end_matches('%')
                .trim()
                .parse()
                .unwrap_or(0.0);
            let mut l = Vec::new();
            self.push_text(&label, Style::Normal, &mut l);
            l.truncate(label_w);
            l.extend(std::iter::repeat_n(Cell::BLANK, label_w - l.len() + 1));
            let mut v = Vec::new();
            self.push_text(&format!("{:.0}%", pct), Style::Heading, &mut v);
            let bar = width.saturating_sub(l.len() + 5);
            let filled = ((pct.clamp(0.0, 100.0) / 100.0) * bar as f32).round() as usize;
            for k in 0..bar {
                l.push(Cell {
                    ch: SOLID,
                    style: if k < filled {
                        Style::Heading
                    } else {
                        Style::Dim
                    },
                });
            }
            let pad = width.saturating_sub(l.len() + v.len());
            l.extend(std::iter::repeat_n(Cell::BLANK, pad));
            l.extend(v);
            self.lines.push(Line { cells: l, step: 0 });
        }
    }

    pub(super) fn blocks(&mut self, blocks: &[Block], width: usize, title: bool) {
        let mut first_heading = true;
        for (i, block) in blocks.iter().enumerate() {
            match block {
                Block::Heading { level, inlines } => {
                    let style = if title && !first_heading && *level > 1 {
                        Style::Normal
                    } else {
                        Style::Heading
                    };
                    let mut cells = Vec::new();
                    self.styled(inlines, style, &mut cells);
                    self.blank(0);
                    self.wrap(Vec::new(), cells, 0, width, 0);
                    if first_heading && i + 1 < blocks.len() {
                        self.lines.push(Line::default());
                    }
                    first_heading = false;
                }
                Block::Paragraph { inlines } => {
                    let style = if title { Style::Dim } else { Style::Normal };
                    let mut cells = Vec::new();
                    self.styled(inlines, style, &mut cells);
                    self.blank(0);
                    self.wrap(Vec::new(), cells, 0, width, 0);
                }
                Block::List { ordered, items } => {
                    self.blank(0);
                    self.list(items, *ordered, 0, 1, width);
                }
                Block::BlockQuote { inlines } => {
                    let mut cells = vec![Cell {
                        ch: '"',
                        style: Style::Accent,
                    }];
                    // the quote's paragraphs (joined by line breaks) run on
                    self.styled(inlines, Style::Normal, &mut cells);
                    cells.push(Cell {
                        ch: '"',
                        style: Style::Accent,
                    });
                    self.blank(0);
                    self.wrap(Vec::new(), cells, 1, width, 0);
                }
                Block::Table { headers, rows } => {
                    self.blank(0);
                    self.table(headers, rows, width);
                }
                Block::Visual { tag, content, .. } if tag == "kpi" => {
                    self.blank(0);
                    self.figures(content, width);
                }
                Block::Visual { tag, content, .. } if tag == "progress" => {
                    self.blank(0);
                    self.progress(content, width);
                }
                Block::HorizontalRule => self.blank(0),
                Block::Image { .. } | Block::ColumnSeparator => {}
                // in the image panel (see `layout::lay_out`)
                Block::Visual { tag, .. } if tag == THERMAL => {}
                Block::CodeBlock { .. } => {
                    self.unsupported.insert("code blocks".into());
                }
                Block::Visual { tag, .. } => {
                    self.unsupported.insert(visual_name(tag));
                }
            }
        }
    }
}

/// `- Label: value` pairs of a visualization block.
fn label_values(content: &str) -> Vec<(String, String)> {
    content
        .lines()
        .filter_map(|l| {
            let t = l.trim().trim_start_matches(['-', '+', '*']).trim();
            let (label, value) = t.split_once(": ")?;
            Some((label.trim().to_string(), value.trim().to_string()))
        })
        .collect()
}

/// The fence tag of a thermal image, which the board shows in its panel.
pub(super) const THERMAL: &str = "thermal";

/// A visual kind, by its fence tag, the way `--check` names it.
fn visual_name(tag: &str) -> String {
    let known = match tag {
        "architecture" => "diagrams",
        "wordcloud" => "word clouds",
        "timeline" => "timelines",
        "pie" => "pie charts",
        "bar" => "bar charts",
        "line" => "line charts",
        "donut" => "donut charts",
        "funnel" => "funnel charts",
        "radar" => "radar charts",
        "stackedbar" => "stacked bar charts",
        "venn" => "Venn diagrams",
        "scatter" => "scatter plots",
        "orgchart" => "org charts",
        "gantt" => "Gantt charts",
        "gitgraph" => "git graphs",
        "flower" => "flowers",
        "artifactflow" => "artifact flows",
        THERMAL => "thermal images",
        // a visual from an extension: named by its tag
        other => return format!("@{other} visuals"),
    };
    known.to_string()
}
