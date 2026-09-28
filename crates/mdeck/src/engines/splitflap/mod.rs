//! The split-flap engine: the slide is a departure board. A fixed grid of
//! flaps carries every piece of text in capitals; going to the next slide
//! turns each flap forward through its wheel of characters until it shows
//! the new one, cells starting a moment apart, left to right. A slide's
//! first image sits in a panel on the board. The countdown is drawn in solid
//! flaps, the board scrambles awake, and the end words clear flap by flap.

pub mod draw;
mod flaps;
pub mod layout;
mod wheel;
mod writer;

use eframe::egui;

use super::stage::{FrameCx, Moment, Stage};
use super::{Capabilities, Engine, EngineDef};
use crate::parser::{Block, Slide};
use crate::render::illustration::Library;
use crate::render::image_cache::{ImageCache, ImageState};
use crate::theme::Theme;
use draw::{Geometry, Labels, Scene, View};
use layout::{Board, COLS, Cell, PANEL, ROWS, Style};

/// Seconds into the end slide when the caption fades in: the words have
/// shown and the board has cleared.
pub const END_CAPTION_DELAY: f32 = 5.8;

pub static DEF: EngineDef = EngineDef {
    capabilities: Capabilities {
        paints: true,
        board: true,
        countdown: true,
        end_act: true,
        ..Capabilities::NONE
    },
    create: || Box::new(SplitFlap::new()),
    end_caption_delay: END_CAPTION_DELAY,
    medium: None,
    render_slide: Some(render_slide),
    problems: Some(layout::problems),
};
/// The end words stay this long, then the board clears.
const END_WORDS: f32 = 3.6;
/// A cell's whole turn from one character to another takes about this long,
/// however far apart they are on the wheel.
const TURN: f32 = 0.85;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Look {
    Slide,
    Digit(u8),
    Wake,
    EndWords,
    EndClear,
}

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
    fn update(&mut self, cx: &FrameCx, stage: &Stage, _lib: &mut Library) {
        let look = match &stage.moment {
            Moment::Slide => Look::Slide,
            Moment::Countdown { digit, .. } => Look::Digit(*digit),
            Moment::Burst { .. } => Look::Wake,
            Moment::End { elapsed, .. } if *elapsed < END_WORDS => Look::EndWords,
            Moment::End { .. } => Look::EndClear,
        };
        let key = (stage.index, stage.reveal, look, stage.title);
        if self.key != Some(key) {
            let board = match look {
                Look::Slide => stage
                    .slide
                    .map(|s| layout::lay_out(s, stage.title, stage.reveal))
                    .unwrap_or_else(Board::blank),
                Look::Digit(d) => layout::digit(d),
                Look::Wake => layout::scramble(7),
                Look::EndWords => layout::words("THE END"),
                Look::EndClear => Board::blank(),
            };
            self.panel = look == Look::Slide && board.image.is_some();
            let brisk = !matches!(look, Look::Slide);
            self.retarget(&board, cx.still, brisk);
            self.key = Some(key);
        }
        if !cx.still {
            self.advance(cx.dt);
        }
    }

    fn paint(&mut self, ui: &egui::Ui, cx: &FrameCx, stage: &Stage) {
        let geo = Geometry::new(cx.rect, cx.scale);
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
        draw::paint(ui, &geo, cx.theme, &scene, cx.opacity, cx.scale);
        if !cx.still && views.iter().any(|v| v.t > 0.0) || self.wait.iter().any(|w| *w > 0.0) {
            ui.ctx().request_repaint();
        }
    }
}

/// The image panel: the right-hand columns of the board, inside its frame.
fn panel_rect(geo: &Geometry) -> egui::Rect {
    geo.area(COLS - PANEL + 1, 0, COLS, ROWS)
}

/// Draw a slide the way the board shows it when no engine runs live
/// (thumbnails in the grid, the overview zoom), and the slide's image in
/// its panel always. `engine_drew`: the engine already drew the board.
#[allow(clippy::too_many_arguments)]
pub fn render_slide(
    ui: &egui::Ui,
    slide: &Slide,
    theme: &Theme,
    rect: egui::Rect,
    opacity: f32,
    image_cache: &ImageCache,
    reveal: usize,
    scale: f32,
    cx: &crate::render::SlideContext,
) {
    let title = crate::render::ember::is_title(slide, cx.index);
    let board = layout::lay_out(slide, title, reveal);
    let geo = Geometry::new(rect, scale);
    if !cx.engine_drew {
        let views: Vec<View> = board
            .cells
            .iter()
            .map(|c| View {
                from: *c,
                to: *c,
                t: 0.0,
            })
            .collect();
        let right = format!("{:02} / {:02}", cx.index + 1, cx.count.max(1));
        let scene = Scene {
            views: &views,
            hidden: board.image.map(|_| panel_rect(&geo)),
            labels: Some(Labels {
                left: cx.deck_title.as_deref().unwrap_or(""),
                right: &right,
            }),
        };
        draw::paint(ui, &geo, theme, &scene, opacity, scale);
    }
    if let Some(i) = board.image
        && let Some(Block::Image { path, .. }) = slide.blocks.get(i)
    {
        draw_panel_image(
            ui,
            path,
            panel_rect(&geo),
            theme,
            opacity,
            image_cache,
            scale,
        );
    }
}

/// The image fills the panel (cropped to cover it), with the flaps' corner
/// radius and a hairline frame.
fn draw_panel_image(
    ui: &egui::Ui,
    path: &str,
    panel: egui::Rect,
    theme: &Theme,
    opacity: f32,
    image_cache: &ImageCache,
    scale: f32,
) {
    let painter = ui.painter();
    let radius = 6.0 * scale;
    match image_cache.state(ui.ctx(), path) {
        ImageState::Ready(texture) => {
            let ts = texture.size_vec2();
            let k = (panel.width() / ts.x).max(panel.height() / ts.y);
            let seen = egui::vec2(panel.width() / k / ts.x, panel.height() / k / ts.y);
            let uv = egui::Rect::from_center_size(egui::pos2(0.5, 0.5), seen);
            painter.add(
                egui::epaint::RectShape::filled(
                    panel,
                    radius,
                    egui::Color32::WHITE.gamma_multiply(opacity),
                )
                .with_texture(texture.id(), uv),
            );
            crate::render::hints::push(ui.ctx(), crate::render::hints::Hint::Frame(panel));
        }
        ImageState::Loading => {}
        ImageState::Missing => {
            painter.rect_filled(panel, radius, theme.code_background.gamma_multiply(opacity));
        }
    }
    painter.rect_stroke(
        panel,
        radius,
        egui::Stroke::new(1.0 * scale, theme.rule.gamma_multiply(opacity)),
        egui::StrokeKind::Outside,
    );
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
