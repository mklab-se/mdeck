//! How one slide gives way to the next: the built-in transitions (fade,
//! slide, spatial, none) and any transition an extension registers
//! (`mdeck_sdk::transition::Transition`, EXT-05), drawn the same way in the
//! window and in export (`--moment transition`).

use std::time::Instant;

use eframe::egui;
use mdeck_sdk::host as h;
use mdeck_sdk::transition::Transition;

const TRANSITION_DURATION: f32 = 0.3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransitionKind {
    Fade,
    SlideHorizontal,
    Spatial,
    None,
    /// A transition an extension registered, by its name.
    Extension(&'static str),
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
    /// Zoom into a spot of the outgoing slide's thermal image (`zoom-to`).
    pub zoom: Option<Zoom>,
}

/// A zoom into a named spot: the anchor is resolved to a fraction of the
/// slide once the outgoing slide has drawn it.
#[derive(Debug, Clone, PartialEq)]
pub struct Zoom {
    pub spot: String,
    pub anchor: Option<(f32, f32)>,
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
            zoom: None,
        }
    }

    pub fn progress(&self) -> f32 {
        let d = self.kind.duration();
        if d <= 0.0 {
            return 1.0;
        }
        ease_in_out((self.start.elapsed().as_secs_f32() / d).clamp(0.0, 1.0))
    }

    pub fn is_complete(&self) -> bool {
        self.start.elapsed().as_secs_f32() >= self.kind.duration()
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
    /// The built-ins in the order `T` cycles through them.
    pub const BUILTIN: [TransitionKind; 4] =
        [Self::SlideHorizontal, Self::Fade, Self::Spatial, Self::None];

    /// A transition by name: a built-in (`slide`, `fade`, `spatial`,
    /// `none`, any case) or one an extension registered (exactly as it
    /// registered it); `None` for anything else.
    pub fn parse(name: &str) -> Option<Self> {
        let name = name.trim();
        match name.to_ascii_lowercase().as_str() {
            "fade" => return Some(Self::Fade),
            "slide" => return Some(Self::SlideHorizontal),
            "spatial" => return Some(Self::Spatial),
            "none" => return Some(Self::None),
            _ => {}
        }
        crate::registry::get()
            .transition_for(name)
            .map(|t| Self::Extension(t.name()))
    }

    /// Every transition `T` cycles through: the built-ins, then the
    /// registered ones by name.
    pub fn all() -> Vec<Self> {
        let mut all = Self::BUILTIN.to_vec();
        all.extend(
            crate::registry::get()
                .transitions()
                .map(|t| Self::Extension(t.name())),
        );
        all
    }

    /// The registered transition behind an extension kind.
    pub fn extension(self) -> Option<&'static dyn Transition> {
        match self {
            Self::Extension(name) => crate::registry::get().transition_for(name),
            _ => None,
        }
    }

    /// Seconds it takes: the built-ins 0.3, an extension what it says
    /// (never negative).
    pub fn duration(self) -> f32 {
        match self.extension() {
            Some(t) => t.duration().max(0.0),
            None => TRANSITION_DURATION,
        }
    }
}

/// Every transition name a deck or theme may write: the built-ins and the
/// registered ones (`zoom` is a slide's own, set up with `zoom-to`).
pub fn names() -> Vec<String> {
    let mut names: Vec<String> = crate::language::TRANSITIONS
        .iter()
        .map(|s| s.to_string())
        .collect();
    names.extend(
        crate::registry::get()
            .transitions()
            .map(|t| t.name().to_string()),
    );
    names
}

/// Whether `name` is a transition a deck or theme may write.
pub fn is_known(name: &str) -> bool {
    let name = name.trim();
    crate::language::TRANSITIONS.contains(&name) || TransitionKind::parse(name).is_some()
}

/// Resolve the transition by precedence: the deck, then the theme, then
/// the user config, then the built-in `fade`. A blank or unknown value is
/// skipped, so the next one in line applies.
pub fn resolve(deck: Option<&str>, theme: Option<&str>, config: Option<&str>) -> TransitionKind {
    [deck, theme, config]
        .into_iter()
        .flatten()
        .find_map(TransitionKind::parse)
        .unwrap_or(TransitionKind::Fade)
}

/// A slide's own transition into it (RUN-09): its `transition` setting.
/// `zoom` is set up by the navigation from `zoom-to`, so it is not one here.
pub fn slide_transition(slide: &crate::parser::Slide) -> Option<TransitionKind> {
    crate::parser::setting(&slide.settings, "transition")
        .filter(|t| *t != "zoom")
        .and_then(TransitionKind::parse)
}

/// One slide on screen during a transition: where, how opaque, and how
/// much it is magnified.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Side {
    pub rect: egui::Rect,
    pub opacity: f32,
    pub zoom: f32,
}

impl Side {
    fn at(rect: egui::Rect, opacity: f32) -> Self {
        Side {
            rect,
            opacity,
            zoom: 1.0,
        }
    }
}

/// How the leaving and the arriving slide are drawn in `rect` at eased
/// progress `p` of `kind`, going `forward`: the leaving one first, left out
/// when it is not seen at all. `spatial` is the direction of a spatial
/// transition ([`ActiveTransition::spatial_direction`]).
pub fn sides(
    kind: TransitionKind,
    p: f32,
    forward: bool,
    rect: egui::Rect,
    spatial: (f32, f32),
) -> Vec<Side> {
    match kind {
        TransitionKind::Fade => vec![Side::at(rect, 1.0 - p), Side::at(rect, p)],
        TransitionKind::None => vec![Side::at(rect, 1.0)],
        TransitionKind::SlideHorizontal => {
            let w = rect.width();
            let sign = if forward { -1.0 } else { 1.0 };
            let from = sign * p * w;
            let to = from - sign * w;
            vec![
                Side::at(rect.translate(egui::vec2(from, 0.0)), 1.0),
                Side::at(rect.translate(egui::vec2(to, 0.0)), 1.0),
            ]
        }
        TransitionKind::Spatial => {
            let (dx, dy) = spatial;
            let (w, h) = (rect.width(), rect.height());
            vec![
                Side::at(rect.translate(egui::vec2(-dx * p * w, -dy * p * h)), 1.0),
                Side::at(
                    rect.translate(egui::vec2(dx * (1.0 - p) * w, dy * (1.0 - p) * h)),
                    1.0,
                ),
            ]
        }
        TransitionKind::Extension(_) => {
            let Some(t) = kind.extension() else {
                return vec![Side::at(rect, 1.0)];
            };
            let (from, to) = t.look(p.clamp(0.0, 1.0), forward, h::rect(rect));
            vec![side_of(from, rect), side_of(to, rect)]
        }
    }
}

/// A side as an extension transition describes it: moved, scaled about the
/// slide's centre, faded.
fn side_of(look: mdeck_sdk::transition::SideLook, rect: egui::Rect) -> Side {
    let scale = if look.scale.is_finite() && look.scale > 0.0 {
        look.scale
    } else {
        1.0
    };
    let offset = egui::vec2(look.offset.x, look.offset.y);
    let rect = egui::Rect::from_center_size(rect.center() + offset, rect.size() * scale);
    Side {
        rect,
        opacity: look.opacity.clamp(0.0, 1.0),
        zoom: scale,
    }
}

/// Let an extension transition paint over both slides at eased progress
/// `p` (a wipe line, a flash); the built-ins paint nothing.
pub fn paint_over(
    kind: TransitionKind,
    ui: &egui::Ui,
    theme: &crate::theme::Theme,
    rect: egui::Rect,
    p: f32,
    forward: bool,
) {
    let Some(t) = kind.extension() else {
        return;
    };
    let tokens = crate::engines::host::convert::tokens(theme);
    h::set_font_families(
        ui.ctx(),
        crate::engines::host::convert::font_families(theme),
    );
    let painter = h::painter(ui.painter().with_clip_rect(rect), h::Backend::Glow);
    let mut cx = h::transition_cx(painter, &tokens, h::rect(rect), forward);
    t.paint_over(&mut cx, p.clamp(0.0, 1.0));
}

/// Columns of the overview grid for a deck of `count` slides: a spatial
/// transition moves the way the grid lays slides out.
pub fn overview_columns(count: usize) -> usize {
    match count {
        0..=4 => 2,
        5..=9 => 3,
        _ => 4,
    }
}

/// Cubic ease-in-out: gentle start, brisk middle, soft landing.
pub fn ease_in_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transitions_parse_by_name() {
        assert_eq!(
            TransitionKind::parse("spatial"),
            Some(TransitionKind::Spatial)
        );
        assert_eq!(TransitionKind::parse("fade"), Some(TransitionKind::Fade));
        assert_eq!(
            TransitionKind::parse(" Slide "),
            Some(TransitionKind::SlideHorizontal)
        );
        assert_eq!(TransitionKind::parse("none"), Some(TransitionKind::None));
        assert_eq!(TransitionKind::parse("unknown"), None);
        assert_eq!(TransitionKind::parse(""), None);
    }

    /// EXT-05: a transition an extension registered is found by name and
    /// drives the slide change: its duration, its look, its paint.
    #[test]
    fn a_registered_transition_drives_the_change() {
        use crate::registry::test_extensions::TRANSITION;
        let kind = TransitionKind::parse(TRANSITION).unwrap();
        assert_eq!(kind, TransitionKind::Extension(TRANSITION));
        assert!(kind.extension().is_some());
        assert_eq!(kind.duration(), 1.0);
        assert_eq!(TransitionKind::Fade.duration(), TRANSITION_DURATION);
        assert!(is_known(TRANSITION) && is_known("zoom") && !is_known("wipe"));
        assert!(names().iter().any(|n| n == TRANSITION));
        assert_eq!(resolve(Some(TRANSITION), None, None), kind);
        assert!(TransitionKind::all().contains(&kind));
        // the test transition fades the old slide and drops the new one in
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920.0, 1080.0));
        let s = sides(kind, 0.5, true, rect, (0.0, 0.0));
        assert_eq!(s.len(), 2);
        assert_eq!((s[0].rect, s[0].opacity), (rect, 0.5));
        assert_eq!(s[1].rect, rect.translate(egui::vec2(0.0, -540.0)));
        assert_eq!((s[1].opacity, s[1].zoom), (1.0, 1.0));
        // the built-ins are unchanged
        let fade = sides(TransitionKind::Fade, 0.25, true, rect, (0.0, 0.0));
        assert_eq!((fade[0].opacity, fade[1].opacity), (0.75, 0.25));
        assert_eq!(
            sides(TransitionKind::None, 0.5, true, rect, (0.0, 0.0)).len(),
            1
        );
        let slid = sides(
            TransitionKind::SlideHorizontal,
            0.5,
            false,
            rect,
            (0.0, 0.0),
        );
        assert_eq!(slid[0].rect.left(), 960.0);
        assert_eq!(slid[1].rect.left(), -960.0);
    }

    #[test]
    fn a_side_scales_about_the_centre_and_ignores_nonsense() {
        use mdeck_sdk::transition::SideLook;
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(100.0, 50.0));
        let s = side_of(SideLook::SHOWN.with_scale(0.5).with_opacity(2.0), rect);
        assert_eq!(s.rect.center(), rect.center());
        assert_eq!(s.rect.width(), 50.0);
        assert_eq!((s.opacity, s.zoom), (1.0, 0.5));
        let s = side_of(SideLook::SHOWN.with_scale(f32::NAN), rect);
        assert_eq!((s.rect, s.zoom), (rect, 1.0));
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
        // Out-of-range input is clamped
        assert_eq!(ease_in_out(-1.0), 0.0);
        assert_eq!(ease_in_out(2.0), 1.0);
    }

    #[test]
    fn ease_in_out_is_cubic_and_symmetric() {
        // Cubic: 4 * 0.25^3 = 0.0625 (quadratic would give 0.125)
        assert!((ease_in_out(0.25) - 0.0625).abs() < 1e-6);
        assert!((ease_in_out(0.75) - 0.9375).abs() < 1e-6);
        // Monotonic
        let mut prev = 0.0;
        for i in 1..=100 {
            let v = ease_in_out(i as f32 / 100.0);
            assert!(v >= prev);
            prev = v;
        }
    }
}
