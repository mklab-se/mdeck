use eframe::egui;
use std::time::Instant;

const TRANSITION_DURATION: f32 = 0.3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransitionKind {
    Fade,
    SlideHorizontal,
    Spatial,
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

    /// Compute the normalized direction vector for a spatial transition.
    /// Returns `(dx, dy)` where each is -1.0, 0.0, or 1.0.
    pub fn spatial_direction(&self, cols: usize) -> (f32, f32) {
        let from_col = self.from % cols;
        let from_row = self.from / cols;
        let to_col = self.to % cols;
        let to_row = self.to / cols;

        let dx = (to_col as isize - from_col as isize).signum() as f32;
        let dy = (to_row as isize - from_row as isize).signum() as f32;
        (dx, dy)
    }
}

impl TransitionKind {
    pub fn from_name(name: &str) -> Self {
        match name {
            "fade" => Self::Fade,
            "slide" => Self::SlideHorizontal,
            "spatial" => Self::Spatial,
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
            TransitionKind::Spatial => {
                let (dx, dy) = transition.spatial_direction(4);
                let w = rect.width();
                let h = rect.height();

                let from_rect = rect.translate(egui::vec2(-dx * progress * w, -dy * progress * h));
                let to_rect = rect.translate(egui::vec2(
                    dx * (1.0 - progress) * w,
                    dy * (1.0 - progress) * h,
                ));

                draw_fn(transition.from, from_rect, 1.0);
                draw_fn(transition.to, to_rect, 1.0);
            }
            TransitionKind::None => {
                draw_fn(transition.to, rect, 1.0);
            }
        }
    }
}

pub fn ease_in_out(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_name_spatial() {
        assert_eq!(
            TransitionKind::from_name("spatial"),
            TransitionKind::Spatial
        );
    }

    #[test]
    fn from_name_known_variants() {
        assert_eq!(TransitionKind::from_name("fade"), TransitionKind::Fade);
        assert_eq!(
            TransitionKind::from_name("slide"),
            TransitionKind::SlideHorizontal
        );
        assert_eq!(TransitionKind::from_name("none"), TransitionKind::None);
        // Unknown falls back to SlideHorizontal
        assert_eq!(
            TransitionKind::from_name("unknown"),
            TransitionKind::SlideHorizontal
        );
    }

    #[test]
    fn spatial_direction_same_row() {
        let t = ActiveTransition::new(0, 1, TransitionKind::Spatial, TransitionDirection::Forward);
        let (dx, dy) = t.spatial_direction(4);
        assert_eq!(dx, 1.0);
        assert_eq!(dy, 0.0);
    }

    #[test]
    fn spatial_direction_row_wrap() {
        // Slide 3 (row 0, col 3) -> Slide 4 (row 1, col 0) with 4 cols
        let t = ActiveTransition::new(3, 4, TransitionKind::Spatial, TransitionDirection::Forward);
        let (dx, dy) = t.spatial_direction(4);
        assert_eq!(dx, -1.0);
        assert_eq!(dy, 1.0);
    }

    #[test]
    fn spatial_direction_backward() {
        let t = ActiveTransition::new(2, 1, TransitionKind::Spatial, TransitionDirection::Backward);
        let (dx, dy) = t.spatial_direction(4);
        assert_eq!(dx, -1.0);
        assert_eq!(dy, 0.0);
    }

    #[test]
    fn spatial_direction_same_column() {
        // Slide 0 (row 0, col 0) -> Slide 4 (row 1, col 0) with 4 cols
        let t = ActiveTransition::new(0, 4, TransitionKind::Spatial, TransitionDirection::Forward);
        let (dx, dy) = t.spatial_direction(4);
        assert_eq!(dx, 0.0);
        assert_eq!(dy, 1.0);
    }

    #[test]
    fn ease_in_out_boundaries() {
        assert_eq!(ease_in_out(0.0), 0.0);
        assert_eq!(ease_in_out(1.0), 1.0);
        // Midpoint
        let mid = ease_in_out(0.5);
        assert!((mid - 0.5).abs() < 0.01);
    }
}
