//! The split-flap engine: the slide is a departure board. A fixed grid of
//! flaps carries every piece of text in capitals; going to the next slide
//! turns each flap forward through its wheel of characters until it shows
//! the new one, cells starting a moment apart, left to right. A slide's
//! first image sits in a panel on the board. The countdown is drawn in solid
//! flaps, the board scrambles awake, and the end words clear flap by flap.

pub mod design;
pub mod draw;
mod flaps;
pub mod layout;
mod wheel;
mod writer;

use mdeck_sdk::engine::{Capabilities, Engine, EngineDef};
use mdeck_sdk::paint::{Painter, Rect};
use mdeck_sdk::stage::{Frame, Look, Moment, Stage};

use draw::{Geometry, Labels, Scene, View};
use layout::{Board, COLS, Cell, PANEL, ROWS, Style};

/// Seconds into the end slide when the caption fades in: the words have
/// shown and the board has cleared.
pub const END_CAPTION_DELAY: f32 = 5.8;

pub static DEF: EngineDef = EngineDef::new(
    "splitflap",
    "A departure board: every slide in split flaps that turn to the next.",
    |_| Box::new(SplitFlap::new()),
)
.with_capabilities(
    Capabilities::NONE
        .with_transition()
        .with_countdown()
        .with_ending(),
)
.with_board(&design::BOARD)
.with_ending_caption_delay(END_CAPTION_DELAY);
/// The end words stay this long, then the board clears.
const END_WORDS: f32 = 3.6;
/// A cell's whole turn from one character to another takes about this long,
/// however far apart they are on the wheel.
const TURN: f32 = 0.85;

type Key = (usize, usize, Look, bool);

pub struct SplitFlap {
    /// The character each flap shows now, and its style.
    shown: Vec<char>,
    style: Vec<Style>,
    /// What each flap is turning toward.
    target: Vec<Cell>,
    /// Progress through the current flip (0..1), seconds per flip, and the
    /// wait before the cell starts turning.
    t: Vec<f32>,
    per_flip: Vec<f32>,
    wait: Vec<f32>,
    key: Option<Key>,
    /// The current slide has an image in the panel.
    panel: bool,
}

impl SplitFlap {
    pub fn new() -> Self {
        let n = COLS * ROWS;
        Self {
            shown: vec![' '; n],
            style: vec![Style::Normal; n],
            target: vec![Cell::BLANK; n],
            t: vec![0.0; n],
            per_flip: vec![0.05; n],
            wait: vec![0.0; n],
            key: None,
            panel: false,
        }
    }

    /// Point every flap at `board`. `brisk` (the countdown, the end) turns
    /// the whole board in about half the time.
    fn retarget(&mut self, board: &Board, still: bool, brisk: bool) {
        for (i, cell) in board.cells.iter().enumerate() {
            self.target[i] = *cell;
            if still {
                self.shown[i] = cell.ch;
                self.style[i] = cell.style;
                self.t[i] = 0.0;
                self.wait[i] = 0.0;
                continue;
            }
            if self.shown[i] == cell.ch {
                self.style[i] = cell.style;
                self.t[i] = 0.0;
                continue;
            }
            let flips = wheel::wheel_distance(self.shown[i], cell.ch).max(1);
            let pace = if brisk { 0.45 } else { 1.0 };
            self.per_flip[i] = (TURN * pace / flips as f32).clamp(0.022, 0.075);
            // a wave left to right and down, with each cell a little late
            let (col, row) = ((i % COLS) as f32, (i / COLS) as f32);
            self.wait[i] =
                (col * 0.011 + row * 0.018 + super::hash01(i as u32 * 31 + 7) * 0.22) * pace;
            self.t[i] = 0.0;
        }
    }

    fn advance(&mut self, dt: f32) -> bool {
        let mut moving = false;
        for i in 0..self.shown.len() {
            let target = self.target[i];
            if self.shown[i] == target.ch {
                continue;
            }
            moving = true;
            if self.wait[i] > 0.0 {
                self.wait[i] -= dt;
                continue;
            }
            self.t[i] += dt / self.per_flip[i];
            while self.t[i] >= 1.0 {
                self.t[i] -= 1.0;
                self.shown[i] = wheel::step_toward(self.shown[i], target.ch);
                self.style[i] = target.style;
                if self.shown[i] == target.ch {
                    self.t[i] = 0.0;
                    break;
                }
            }
        }
        moving
    }

    fn views(&self) -> Vec<View> {
        (0..self.shown.len())
            .map(|i| {
                let from = Cell {
                    ch: self.shown[i],
                    style: self.style[i],
                };
                let target = self.target[i];
                if from.ch == target.ch || self.wait[i] > 0.0 {
                    View {
                        from,
                        to: from,
                        t: 0.0,
                    }
                } else {
                    View {
                        from,
                        to: Cell {
                            ch: wheel::step_toward(from.ch, target.ch),
                            style: target.style,
                        },
                        t: self.t[i].max(0.001),
                    }
                }
            })
            .collect()
    }
}

impl Default for SplitFlap {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for SplitFlap {
    fn update(&mut self, frame: &Frame, stage: &Stage) {
        let look = stage.moment.look(END_WORDS);
        let key = (stage.index, stage.step, look, stage.title);
        if self.key != Some(key) {
            let board = match look {
                Look::Slide => stage
                    .slide
                    .map(|s| layout::lay_out(s, stage.title, stage.step))
                    .unwrap_or_else(Board::blank),
                Look::Digit(d) => layout::digit(d),
                Look::Burst => layout::scramble(7),
                Look::EndWords => layout::words("THE END"),
                _ => Board::blank(),
            };
            self.panel = look == Look::Slide && board.image.is_some();
            let brisk = !matches!(look, Look::Slide);
            self.retarget(&board, frame.still, brisk);
            self.key = Some(key);
        }
        if !frame.still {
            self.advance(frame.dt);
        }
    }

    fn paint(&mut self, painter: &mut Painter, frame: &Frame, stage: &Stage) {
        let geo = Geometry::new(frame.rect, frame.scale);
        let views = self.views();
        let hidden = self.panel.then(|| panel_rect(&geo));
        let right = format!("{:02} / {:02}", stage.index + 1, stage.count.max(1));
        let labels = matches!(stage.moment, Moment::Slide).then(|| Labels {
            left: stage.deck_title.unwrap_or(""),
            right: &right,
        });
        let scene = Scene {
            views: &views,
            hidden,
            labels,
        };
        draw::paint(
            painter,
            &geo,
            frame.tokens,
            &scene,
            frame.opacity,
            frame.scale,
        );
    }

    /// Repaint while any flap is still turning (or waiting to).
    fn animating(&self) -> bool {
        self.shown
            .iter()
            .zip(&self.target)
            .any(|(shown, target)| *shown != target.ch)
    }
}

/// The image panel: the right-hand columns of the board, inside its frame.
fn panel_rect(geo: &Geometry) -> Rect {
    geo.area(COLS - PANEL + 1, 0, COLS, ROWS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cell_turns_through_the_wheel_to_its_character() {
        let mut sf = SplitFlap::new();
        let mut board = Board::blank();
        board.cells[0] = Cell {
            ch: 'C',
            style: Style::Heading,
        };
        sf.retarget(&board, false, false);
        sf.wait[0] = 0.0;
        let mut seen = vec![sf.shown[0]];
        for _ in 0..200 {
            sf.advance(1.0 / 60.0);
            if *seen.last().unwrap() != sf.shown[0] {
                seen.push(sf.shown[0]);
            }
        }
        assert_eq!(
            seen,
            vec![' ', 'A', 'B', 'C'],
            "one flap at a time, forward"
        );
        assert_eq!(sf.style[0], Style::Heading);
        assert!(!sf.advance(1.0 / 60.0), "at rest once there");
    }

    #[test]
    fn export_shows_the_board_finished() {
        let mut sf = SplitFlap::new();
        let board = layout::words("ON TIME");
        sf.retarget(&board, true, false);
        assert!(sf.views().iter().all(|v| v.t == 0.0 && v.from == v.to));
        let text: String = sf.shown.iter().collect();
        assert!(text.contains("ON TIME"));
    }

    /// The board draws its flaps and the heading's characters (accent ink)
    /// headless, and settles at once in a still.
    #[test]
    fn the_board_draws_headless() {
        use mdeck_sdk::content::{Block, Inline, Slide};
        use mdeck_sdk::paint::Color;
        use mdeck_sdk::testing::Headless;
        use mdeck_sdk::tokens::{EngineSettings, Tokens};

        let mut slide = Slide::new("points");
        slide.blocks = vec![Block::heading(
            1,
            vec![Inline::text("MMMMMMMMMMMMMMMMMMMMMMMMMMMMMM")],
        )];
        let mut h = Headless::new(480, 270);
        let tokens = Tokens::default();
        let settings = EngineSettings::new();
        let mut frame = Frame::new(h.rect(), &tokens, &settings);
        frame.still = true;
        let mut stage = Stage::new(Moment::Slide);
        stage.slide = Some(&slide);
        let mut sf = SplitFlap::new();
        let img = h.render_engine(&mut sf, &frame, &stage);
        assert!(!sf.animating(), "a still is at rest");
        let reddish = |c: Color| c.r() > 120 && c.g() < 90 && c.b() < 70;
        let ink = (0..img.width())
            .flat_map(|x| (0..img.height()).map(move |y| (x, y)))
            .filter(|&(x, y)| img.get(x, y).is_some_and(reddish))
            .count();
        assert!(ink > 100, "the heading's characters show: {ink}");
    }

    #[test]
    fn a_whole_turn_takes_about_a_second_however_far() {
        let mut sf = SplitFlap::new();
        let mut board = Board::blank();
        board.cells[0] = Cell {
            ch: '9',
            style: Style::Normal,
        };
        sf.retarget(&board, false, false);
        sf.wait[0] = 0.0;
        let mut t = 0.0;
        while sf.shown[0] != '9' && t < 5.0 {
            sf.advance(1.0 / 60.0);
            t += 1.0 / 60.0;
        }
        assert!((0.6..2.6).contains(&t), "{t}");
    }
}
