//! Laying a slide out on the board: a fixed grid of character cells, the way
//! a departure board shows a timetable. Every piece of text goes on the
//! board in capitals; what cannot (code, charts, diagrams, formulas, a
//! second image, characters the flaps do not carry) is reported, never
//! typeset outside the board.

use std::collections::BTreeSet;

use super::wheel::{SOLID, WHEEL};
use super::writer::{Line, Writer};
use crate::parser::{Block, Layout, Slide};

/// The board: columns and rows of flaps, the same on every slide.
pub const COLS: usize = 32;
pub const ROWS: usize = 12;
/// Blank columns left and right of the text.
const MARGIN: usize = 1;
/// Columns the image panel takes on the right, when a slide has an image.
pub const PANEL: usize = 13;

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
    let mut w = Writer::new();
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
        let mut left = Writer::new();
        left.blocks(&slide.blocks[lead..sep], half, false, &mut counter);
        let mut right = Writer::new();
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
    use std::path::Path;

    fn slide(md: &str) -> Slide {
        crate::parser::parse(md, Path::new("."))
            .slides
            .into_iter()
            .next()
            .unwrap()
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
