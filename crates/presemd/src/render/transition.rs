use eframe::egui;
use std::time::Instant;

const TRANSITION_DURATION: f32 = 0.3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransitionKind {
    Fade,
    SlideHorizontal,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransitionDirection {
    Forward,
    Backward,
}

pub struct ActiveTransition {
    pub from: usize,
    pub to: usize,
    pub kind: TransitionKind,
    pub direction: TransitionDirection,
    pub start: Instant,
}

impl ActiveTransition {
    pub fn new(
        from: usize,
        to: usize,
        kind: TransitionKind,
        direction: TransitionDirection,
    ) -> Self {
        Self {
            from,
            to,
            kind,
            direction,
            start: Instant::now(),
        }
    }

    pub fn progress(&self) -> f32 {
        let raw = (self.start.elapsed().as_secs_f32() / TRANSITION_DURATION).clamp(0.0, 1.0);
        ease_in_out(raw)
    }

    pub fn is_complete(&self) -> bool {
        self.start.elapsed().as_secs_f32() >= TRANSITION_DURATION
    }
}

impl TransitionKind {
    pub fn from_name(name: &str) -> Self {
        match name {
            "fade" => Self::Fade,
            "slide" => Self::SlideHorizontal,
            "none" => Self::None,
            _ => Self::SlideHorizontal,
        }
    }

    /// Render a transition between two slides.
    /// Calls `draw_fn` with (slide_index, rect, opacity) for each visible slide.
    #[allow(dead_code)]
    pub fn render(
        &self,
        transition: &ActiveTransition,
        rect: egui::Rect,
        draw_fn: &mut dyn FnMut(usize, egui::Rect, f32),
    ) {
        let progress = transition.progress();

        match self {
            TransitionKind::Fade => {
                draw_fn(transition.from, rect, 1.0 - progress);
                draw_fn(transition.to, rect, progress);
            }
            TransitionKind::SlideHorizontal => {
                let w = rect.width();
                let sign = match transition.direction {
                    TransitionDirection::Forward => -1.0,
                    TransitionDirection::Backward => 1.0,
                };
                let from_offset = sign * progress * w;
                let to_offset = from_offset - sign * w;

                let from_rect = rect.translate(egui::vec2(from_offset, 0.0));
                let to_rect = rect.translate(egui::vec2(to_offset, 0.0));

                draw_fn(transition.from, from_rect, 1.0);
                draw_fn(transition.to, to_rect, 1.0);
            }
            TransitionKind::None => {
                draw_fn(transition.to, rect, 1.0);
            }
        }
    }
}

fn ease_in_out(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}
