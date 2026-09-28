//! Laying a slide out on the board: a fixed grid of character cells, the way
//! a departure board shows a timetable. Every piece of text goes on the
//! board in capitals; what cannot (code, charts, diagrams, formulas, a
//! second image, characters the flaps do not carry) is reported, never
//! typeset outside the board.

use std::collections::BTreeSet;

use crate::parser::{Block, Chart, Inline, Layout, ListItem, ListMarker, Slide};

/// The board: columns and rows of flaps, the same on every slide.
pub const COLS: usize = 32;
pub const ROWS: usize = 12;
/// Blank columns left and right of the text.
const MARGIN: usize = 1;
/// Columns the image panel takes on the right, when a slide has an image.
pub const PANEL: usize = 13;

/// Every character a flap carries, in the order the flaps turn. A cell
/// turns forward through this wheel until it shows its character.
pub const WHEEL: &str = " ABCDEFGHIJKLMNOPQRSTUVWXYZÅÄÖÆØÜÉ0123456789.,:;!?'\"-+/()&%#@*=$€£<>_•…█";

/// A full flap: a solid colour instead of a character (countdown digits,
/// progress bars).
pub const SOLID: char = '█';

/// How a cell's character is coloured.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Style {
    Normal,
    /// Headings: the board's highlight colour.
    Heading,
    /// `**bold**`.
    Strong,
    /// `` `code` ``.
    Code,
    /// Quiet text: subtitles on a title slide, table rules, empty bar cells.
    Dim,
    /// List markers and quote marks.
    Accent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub style: Style,
}

impl Cell {
    pub const BLANK: Cell = Cell {
        ch: ' ',
        style: Style::Normal,
    };
}

/// A laid-out slide.
#[derive(Clone, Debug)]
pub struct Board {
    /// `ROWS * COLS` cells, row by row.
    pub cells: Vec<Cell>,
    /// The first image on the slide (its index in the slide's blocks),
    /// shown in the panel on the right.
    pub image: Option<usize>,
    /// Rows the content needs; more than `ROWS` means it overflows.
    pub needed: usize,
    /// Characters the flaps do not carry (shown as blanks).
    pub missing: BTreeSet<char>,
    /// Content the board does not show, one entry per kind.
    pub unsupported: BTreeSet<String>,
}

impl Board {
    pub fn blank() -> Self {
        Board {
            cells: vec![Cell::BLANK; COLS * ROWS],
            image: None,
            needed: 0,
            missing: BTreeSet::new(),
            unsupported: BTreeSet::new(),
        }
    }

    pub fn get(&self, col: usize, row: usize) -> Cell {
        self.cells[row * COLS + col]
    }

    /// The board's text, one line per row, trailing blanks trimmed (tests
    /// and debugging).
    #[cfg(test)]
    pub fn text(&self) -> Vec<String> {
        (0..ROWS)
            .map(|r| {
                (0..COLS)
                    .map(|c| self.get(c, r).ch)
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect()
    }
}

/// Position of a character on the wheel, if the flaps carry it.
pub fn wheel_index(c: char) -> Option<usize> {
    WHEEL.chars().position(|w| w == c)
}

/// The character after `c` on the wheel (wrapping round).
pub fn wheel_next(c: char) -> char {
    let n = WHEEL.chars().count();
    let i = wheel_index(c).map_or(0, |i| (i + 1) % n);
    WHEEL.chars().nth(i).unwrap_or(' ')
}

/// The character a flap turns to next on its way to `target`. A solid
/// flap is a flap of its own, one turn from anything.
pub fn step_toward(c: char, target: char) -> char {
    if target == SOLID {
        SOLID
    } else {
        wheel_next(c)
    }
}

/// How many flips turn `from` into `to`.
pub fn wheel_distance(from: char, to: char) -> usize {
    if to == SOLID {
        return usize::from(from != SOLID);
    }
    let n = WHEEL.chars().count();
    match (wheel_index(from), wheel_index(to)) {
        (Some(a), Some(b)) => (b + n - a) % n,
        _ => 0,
    }
}

/// A character as a flap shows it: capitals, typographic marks folded to
/// the plain ones the wheel carries. `None` when the flaps cannot show it.
fn flap_chars(c: char) -> Vec<Option<char>> {
    let folded = match c {
        '\u{2018}' | '\u{2019}' | '`' => '\'',
        '\u{201C}' | '\u{201D}' | '\u{00AB}' | '\u{00BB}' => '"',
        '\u{2013}' | '\u{2014}' | '\u{2212}' => '-',
        '\u{00D7}' => 'X',
        '\t' | '\u{00A0}' => ' ',
        c => c,
    };
    folded
        .to_uppercase()
        .map(|u| wheel_index(u).map(|_| u))
        .collect()
}

/// One styled line of text, before placement.
#[derive(Clone, Debug, Default)]
struct Line {
    cells: Vec<Cell>,
    /// Reveal step that shows this line.
    step: usize,
}

/// Collects lines, missing characters and unsupported content while a
/// slide is laid out.
struct Writer {
    lines: Vec<Line>,
    missing: BTreeSet<char>,
    unsupported: BTreeSet<String>,
}

impl Writer {
    /// Styled characters of `inlines`, capitalised; formulas are reported.
    fn styled(&mut self, inlines: &[Inline], base: Style, out: &mut Vec<Cell>) {
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

    fn list(
        &mut self,
        items: &[ListItem],
        ordered: bool,
        depth: usize,
        width: usize,
        counter: &mut usize,
    ) {
        for (k, item) in items.iter().enumerate() {
            let step = match item.marker {
                ListMarker::Static | ListMarker::Ordered => 0,
                ListMarker::NextStep => {
                    *counter += 1;
                    *counter
                }
                ListMarker::WithPrev => *counter,
            };
            let indent = depth * 2;
            let mut prefix = vec![Cell::BLANK; indent];
            if ordered {
                for ch in format!("{}.", k + 1).chars() {
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
            self.list(&item.children, ordered, depth + 1, width, counter);
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

    fn blocks(&mut self, blocks: &[Block], width: usize, title: bool, counter: &mut usize) {
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
                    self.list(items, *ordered, 0, width, counter);
                }
                Block::BlockQuote { inlines } => {
                    let mut cells = vec![Cell {
                        ch: '"',
                        style: Style::Accent,
                    }];
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
                Block::Chart {
                    kind: Chart::KpiCards,
                    content,
                } => {
                    self.blank(0);
                    self.figures(content, width);
                }
                Block::Chart {
                    kind: Chart::ProgressBars,
                    content,
                } => {
                    self.blank(0);
                    self.progress(content, width);
                }
                Block::HorizontalRule => self.blank(0),
                Block::Image { .. } | Block::ColumnSeparator => {}
                Block::StoryHint { .. } | Block::SceneScript { .. } => {}
                Block::CodeBlock { .. } => {
                    self.unsupported.insert("code blocks".into());
                }
                Block::Diagram { .. } => {
                    self.unsupported.insert("diagrams".into());
                }
                other => {
                    self.unsupported.insert(viz_name(other).to_string());
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

fn viz_name(block: &Block) -> &'static str {
    let Block::Chart { kind, .. } = block else {
        return "visualizations";
    };
    match kind {
        Chart::WordCloud => "word clouds",
        Chart::Timeline => "timelines",
        Chart::Pie => "pie charts",
        Chart::Bar => "bar charts",
        Chart::Line => "line charts",
        Chart::Donut => "donut charts",
        Chart::Funnel => "funnel charts",
        Chart::Radar => "radar charts",
        Chart::StackedBar => "stacked bar charts",
        Chart::VennDiagram => "Venn diagrams",
        Chart::ScatterPlot => "scatter plots",
        Chart::Org => "org charts",
        Chart::Gantt => "Gantt charts",
        Chart::GitGraph => "git graphs",
        _ => "visualizations",
    }
}

/// Lay `slide` out on the board. `title`: the slide reads as the deck's
/// title page (centred). Lines revealed after `reveal` keep their rows but
/// stay blank.
pub fn lay_out(slide: &Slide, title: bool, reveal: usize) -> Board {
    let images: Vec<usize> = slide
        .blocks
        .iter()
        .enumerate()
        .filter(|(_, b)| matches!(b, Block::Image { .. }))
        .map(|(i, _)| i)
        .collect();
    let image = images.first().copied();
    let text_cols = if image.is_some() { COLS - PANEL } else { COLS };
    let width = text_cols - 2 * MARGIN;
    let mut w = Writer {
        lines: Vec::new(),
        missing: BTreeSet::new(),
        unsupported: BTreeSet::new(),
    };
    if images.len() > 1 {
        w.unsupported.insert("a second image".into());
    }
    let mut counter = 0;
    let split = slide
        .blocks
        .iter()
        .position(|b| matches!(b, Block::ColumnSeparator));
    if let Some(sep) = split {
        // two columns: headings on top across the board, then the halves
        let lead = slide.blocks[..sep]
            .iter()
            .take_while(|b| matches!(b, Block::Heading { .. }))
            .count();
        w.blocks(&slide.blocks[..lead], width, false, &mut counter);
        let half = (width - 2) / 2;
        let mut left = Writer {
            lines: Vec::new(),
            missing: BTreeSet::new(),
            unsupported: BTreeSet::new(),
        };
        left.blocks(&slide.blocks[lead..sep], half, false, &mut counter);
        let mut right = Writer {
            lines: Vec::new(),
            missing: BTreeSet::new(),
            unsupported: BTreeSet::new(),
        };
        right.blocks(&slide.blocks[sep + 1..], half, false, &mut counter);
        let n = left.lines.len().max(right.lines.len());
        for k in 0..n {
            let l = left.lines.get(k).cloned().unwrap_or_default();
            let r = right.lines.get(k).cloned().unwrap_or_default();
            let mut cells = l.cells.clone();
            cells.resize(half + 2, Cell::BLANK);
            cells.extend(r.cells);
            w.lines.push(Line {
                cells,
                step: l.step.max(r.step),
            });
        }
        for sub in [left, right] {
            w.missing.extend(sub.missing);
            w.unsupported.extend(sub.unsupported);
        }
    } else {
        w.blocks(&slide.blocks, width, title, &mut counter);
    }
    while w
        .lines
        .last()
        .is_some_and(|l| l.cells.iter().all(|c| c.ch == ' '))
    {
        w.lines.pop();
    }

    let centred = title || matches!(slide.layout, Layout::Title | Layout::Section);
    let needed = w.lines.len();
    let mut board = Board::blank();
    let top = if centred && needed < ROWS {
        (ROWS - needed) / 2
    } else {
        0
    };
    for (k, line) in w.lines.iter().take(ROWS).enumerate() {
        if line.step > reveal {
            continue;
        }
        let len = line.cells.len().min(width);
        let left = MARGIN + if centred { (width - len) / 2 } else { 0 };
        for (x, cell) in line.cells.iter().take(width).enumerate() {
            board.cells[(top + k) * COLS + left + x] = *cell;
        }
    }
    if needed > ROWS {
        // the last visible row ends in an ellipsis
        let row = ROWS - 1;
        let last = (MARGIN..MARGIN + width)
            .rev()
            .find(|c| board.get(*c, row).ch != ' ')
            .map_or(MARGIN, |c| (c + 2).min(MARGIN + width - 1));
        board.cells[row * COLS + last] = Cell {
            ch: '…',
            style: Style::Dim,
        };
    }
    board.image = image;
    board.needed = needed;
    board.missing = w.missing;
    board.unsupported = w.unsupported;
    board
}

/// A countdown digit in solid flaps, 7 wide and 9 tall, in the middle of
/// the board.
pub fn digit(d: u8) -> Board {
    const THREE: [&str; 9] = [
        " █████ ",
        "██   ██",
        "     ██",
        "    ██ ",
        "  ████ ",
        "     ██",
        "     ██",
        "██   ██",
        " █████ ",
    ];
    const TWO: [&str; 9] = [
        " █████ ",
        "██   ██",
        "     ██",
        "    ██ ",
        "   ██  ",
        "  ██   ",
        " ██    ",
        "██     ",
        "███████",
    ];
    const ONE: [&str; 9] = [
        "   ██  ",
        "  ███  ",
        " ████  ",
        "   ██  ",
        "   ██  ",
        "   ██  ",
        "   ██  ",
        "   ██  ",
        " ██████",
    ];
    let rows = match d {
        3 => THREE,
        2 => TWO,
        _ => ONE,
    };
    let mut board = Board::blank();
    let (left, top) = ((COLS - 7) / 2, (ROWS - 9) / 2);
    for (r, line) in rows.iter().enumerate() {
        for (c, ch) in line.chars().enumerate() {
            if ch == SOLID {
                board.cells[(top + r) * COLS + left + c] = Cell {
                    ch: SOLID,
                    style: Style::Heading,
                };
            }
        }
    }
    board
}

/// `text` alone in the middle of the board.
pub fn words(text: &str) -> Board {
    let mut board = Board::blank();
    let chars: Vec<char> = text.chars().collect();
    let left = (COLS.saturating_sub(chars.len())) / 2;
    for (k, ch) in chars.into_iter().take(COLS).enumerate() {
        board.cells[(ROWS / 2 - 1) * COLS + left + k] = Cell {
            ch,
            style: Style::Heading,
        };
    }
    board
}

/// Every flap on some character: the board wakes up.
pub fn scramble(seed: u64) -> Board {
    let wheel: Vec<char> = WHEEL.chars().filter(|c| c.is_alphanumeric()).collect();
    let mut board = Board::blank();
    for (i, cell) in board.cells.iter_mut().enumerate() {
        let h = crate::engines::hash01((i as u64 * 2654435761 + seed) as u32);
        cell.ch = wheel[(h * wheel.len() as f32) as usize % wheel.len()];
        cell.style = if h > 0.8 { Style::Heading } else { Style::Dim };
    }
    board
}

/// What the board does not show on `slide`, for `--check`.
pub fn problems(slide: &Slide) -> Vec<String> {
    let board = lay_out(slide, false, usize::MAX);
    let mut out: Vec<String> = board
        .unsupported
        .iter()
        .map(|what| format!("{what} are not shown on the split-flap board"))
        .map(|m| {
            m.replace("a second image are not", "a second image is not")
                .replace("cut to fit are not shown", "are cut to fit")
        })
        .collect();
    if board.needed > ROWS {
        out.push(format!(
            "the text needs {} rows and the board has {ROWS}; the rest is cut",
            board.needed
        ));
    }
    if !board.missing.is_empty() {
        let list: String = board.missing.iter().collect();
        out.push(format!(
            "the flaps do not carry {list:?}; those show as blanks"
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slide(md: &str) -> Slide {
        crate::parser::parse(md).slides.into_iter().next().unwrap()
    }

    #[test]
    fn the_wheel_turns_forward_and_wraps() {
        assert_eq!(wheel_next(' '), 'A');
        assert_eq!(wheel_next('A'), 'B');
        assert_eq!(wheel_next(SOLID), ' ');
        assert_eq!(wheel_distance(' ', 'C'), 3);
        assert_eq!(wheel_distance('C', ' '), WHEEL.chars().count() - 3);
        assert_eq!(wheel_distance('Q', 'Q'), 0);
        assert_eq!(wheel_distance('Q', SOLID), 1, "solid flaps turn at once");
        assert_eq!(step_toward('Q', SOLID), SOLID);
    }

    #[test]
    fn a_bullet_slide_reads_like_a_timetable() {
        let b = lay_out(
            &slide("# Departures\n\n- Stockholm 10:42\n- Göteborg on time\n"),
            false,
            usize::MAX,
        );
        let t = b.text();
        assert_eq!(t[0], " DEPARTURES");
        assert_eq!(t[1], "");
        assert_eq!(t[2], " • STOCKHOLM 10:42");
        assert_eq!(t[3], " • GÖTEBORG ON TIME");
        assert_eq!(b.get(1, 0).style, Style::Heading);
        assert_eq!(b.get(1, 2).style, Style::Accent);
        assert!(b.unsupported.is_empty() && b.missing.is_empty());
    }

    #[test]
    fn long_items_wrap_under_their_text() {
        let b = lay_out(
            &slide("# T\n\n- one two three four five six seven eight nine ten eleven\n"),
            false,
            usize::MAX,
        );
        let t = b.text();
        assert!(t[2].starts_with(" • ONE TWO"), "{t:?}");
        assert!(
            t[3].starts_with("   "),
            "continuation hangs under the text: {t:?}"
        );
        assert!(t[2].chars().count() < COLS);
    }

    #[test]
    fn a_title_is_centred() {
        let b = lay_out(&slide("# On Time\n\nEvery train, every day\n"), true, 0);
        let t = b.text();
        let row = t.iter().position(|l| l.contains("ON TIME")).unwrap();
        assert!(row >= 3, "centred vertically: {t:?}");
        let lead = t[row].len() - t[row].trim_start().len();
        assert!(lead > 8, "centred horizontally: {t:?}");
        assert_eq!(b.get(lead, row).style, Style::Heading);
    }

    #[test]
    fn unrevealed_items_keep_their_rows_blank() {
        let s = slide("# T\n\n- a\n+ b\n+ c\n");
        let before = lay_out(&s, false, 0).text();
        assert_eq!(before[3], "");
        assert_eq!(before[4], "");
        let after = lay_out(&s, false, 2).text();
        assert_eq!(after[3], " • B");
        assert_eq!(after[4], " • C");
    }

    #[test]
    fn tables_align_and_numbers_go_right() {
        let b = lay_out(
            &slide("# T\n\n| Train | Track |\n|---|---|\n| X2000 | 4 |\n| Regional | 12 |\n"),
            false,
            usize::MAX,
        );
        let t = b.text();
        assert_eq!(t[2], " TRAIN     TRACK");
        assert_eq!(t[3], " X2000         4");
        assert_eq!(t[4], " REGIONAL     12");
        assert_eq!(b.get(1, 2).style, Style::Dim);
    }

    #[test]
    fn overflow_and_missing_characters_are_reported() {
        let long: String = (0..20).map(|i| format!("- item {i}\n")).collect();
        let s = slide(&format!("# Many\n\n{long}"));
        let b = lay_out(&s, false, usize::MAX);
        assert!(b.needed > ROWS);
        assert!(b.text()[ROWS - 1].ends_with('…'), "{:?}", b.text());
        let p = problems(&s);
        assert!(p.iter().any(|m| m.contains("rows")), "{p:?}");
        let cjk = problems(&slide("# 你好\n\n- ok\n"));
        assert!(cjk.iter().any(|m| m.contains("do not carry")), "{cjk:?}");
    }

    #[test]
    fn code_charts_and_formulas_are_reported() {
        let p = problems(&slide(
            "# T\n\nThe $x^2$ rule\n\n```rust\nfn main() {}\n```\n\n```@barchart\nA: 1\n```\n",
        ));
        let all = p.join("|");
        assert!(all.contains("code blocks"), "{all}");
        assert!(all.contains("bar charts"), "{all}");
        assert!(all.contains("formulas"), "{all}");
    }

    #[test]
    fn an_image_takes_the_panel_and_narrows_the_text() {
        let s = slide(
            "# Platform\n\n![map](map.png)\n\n- A very long line of text that will need to wrap\n",
        );
        let b = lay_out(&s, false, usize::MAX);
        assert_eq!(b.image, Some(1));
        for row in b.text() {
            assert!(row.chars().count() <= COLS - PANEL, "{row}");
        }
    }

    #[test]
    fn progress_bars_are_solid_flaps() {
        let b = lay_out(
            &slide("# Build\n\n```@progress\n- Tests: 50%\n```\n"),
            false,
            usize::MAX,
        );
        let row: Vec<Cell> = (0..COLS).map(|c| b.get(c, 2)).collect();
        let lit = row
            .iter()
            .filter(|c| c.ch == SOLID && c.style == Style::Heading)
            .count();
        let dim = row
            .iter()
            .filter(|c| c.ch == SOLID && c.style == Style::Dim)
            .count();
        assert!(
            lit > 0 && (lit as i32 - dim as i32).abs() <= 1,
            "{lit} {dim}"
        );
        assert!(b.text()[2].ends_with("50%"));
    }

    #[test]
    fn countdown_digits_are_centred_blocks() {
        let b = digit(3);
        let solid = b.cells.iter().filter(|c| c.ch == SOLID).count();
        assert!(solid > 20);
        assert_eq!(words("THE END").text()[ROWS / 2 - 1].trim(), "THE END");
    }
}
