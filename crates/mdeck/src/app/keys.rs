//! Keyboard mapping and small pure helpers for the presentation app.
//!
//! Everything in this module is free of window state so it can be unit tested.
//! The [`SHORTCUTS`] table is the single source of truth for the in-app HUD
//! (`H`) and the `mdeck spec --short` quick reference card.

use eframe::egui::{Key, Modifiers, Pos2};
use std::time::{Duration, Instant};

/// Human-readable shortcut table: (keys, description).
///
/// Keep this in sync with [`map_key`] — a unit test checks that every key
/// mentioned here actually maps to an action.
pub const SHORTCUTS: &[(&str, &str)] = &[
    ("Space/N/→/PgDn/Enter", "Next slide / reveal"),
    ("P/←/PgUp/Backspace", "Previous slide / hide"),
    ("↑ / ↓ / Wheel", "Scroll slide content"),
    ("Home / End", "First / last slide"),
    ("Left click", "Next slide"),
    ("Right click", "Previous slide"),
    ("Left drag", "Freehand pen"),
    ("Right drag", "Draw arrow"),
    ("Esc", "Clear drawings / ×2 exit"),
    ("Q ×2 / Ctrl+C ×2", "Quit"),
    ("G", "Grid view / overview"),
    ("Enter / E", "Grid: open slide"),
    ("T", "Cycle transition"),
    ("Shift+T", "Cycle theme"),
    ("F", "Toggle fullscreen"),
    ("M", "Move to next monitor"),
    ("H", "Toggle HUD"),
    (". / B", "Blackout screen"),
    ("R", "Debug overlay (L/R/off)"),
];

/// Which input context the key is interpreted in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyMode {
    Presentation,
    Grid,
    /// Overview zoom animation in progress: only global actions apply.
    Blocked,
}

/// A user intent derived from a key press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    // Global (any mode)
    Quit,
    CtrlC,
    Escape,
    ToggleFullscreen,
    MoveMonitor,
    CycleTheme,
    CycleTransition,
    ToggleBlackout,
    // Presentation mode
    Next,
    Previous,
    ScrollUp,
    ScrollDown,
    FirstSlide,
    LastSlide,
    EnterGrid,
    ToggleHud,
    CycleRawOverlay,
    // Grid mode
    GridRight,
    GridLeft,
    GridDown,
    GridUp,
    GridSelect,
}

impl Action {
    /// Global actions work in every mode, including while blacked out.
    pub fn is_global(self) -> bool {
        matches!(
            self,
            Action::Quit
                | Action::CtrlC
                | Action::Escape
                | Action::ToggleFullscreen
                | Action::MoveMonitor
                | Action::CycleTheme
                | Action::CycleTransition
                | Action::ToggleBlackout
        )
    }
}

/// Map a key press to an action for the given mode.
pub fn map_key(key: Key, modifiers: Modifiers, mode: KeyMode) -> Option<Action> {
    // Global bindings first
    let global = match key {
        Key::Q => Some(Action::Quit),
        Key::C if modifiers.ctrl => Some(Action::CtrlC),
        Key::Escape => Some(Action::Escape),
        Key::F => Some(Action::ToggleFullscreen),
        Key::M => Some(Action::MoveMonitor),
        Key::T if modifiers.shift => Some(Action::CycleTheme),
        Key::T => Some(Action::CycleTransition),
        Key::Period | Key::B => Some(Action::ToggleBlackout),
        _ => None,
    };
    if global.is_some() {
        return global;
    }

    match mode {
        KeyMode::Presentation => match key {
            Key::ArrowRight | Key::N | Key::Space | Key::PageDown | Key::Enter => {
                Some(Action::Next)
            }
            Key::ArrowLeft | Key::P | Key::PageUp | Key::Backspace => Some(Action::Previous),
            Key::ArrowUp => Some(Action::ScrollUp),
            Key::ArrowDown => Some(Action::ScrollDown),
            Key::Home => Some(Action::FirstSlide),
            Key::End => Some(Action::LastSlide),
            Key::G => Some(Action::EnterGrid),
            Key::H => Some(Action::ToggleHud),
            Key::R => Some(Action::CycleRawOverlay),
            _ => None,
        },
        KeyMode::Grid => match key {
            Key::ArrowRight => Some(Action::GridRight),
            Key::ArrowLeft => Some(Action::GridLeft),
            Key::ArrowDown => Some(Action::GridDown),
            Key::ArrowUp => Some(Action::GridUp),
            Key::Enter | Key::Space | Key::E => Some(Action::GridSelect),
            _ => None,
        },
        KeyMode::Blocked => None,
    }
}

/// Tracks a "press twice within a window" gesture (Esc, Q, Ctrl+C).
#[derive(Debug, Clone)]
pub struct DoubleTap {
    last: Option<Instant>,
    window: Duration,
}

impl DoubleTap {
    pub fn new(window: Duration) -> Self {
        Self { last: None, window }
    }

    /// Register a tap at `now`. Returns `true` when this tap completes a double tap.
    pub fn tap(&mut self, now: Instant) -> bool {
        if let Some(last) = self.last
            && now.duration_since(last) < self.window
        {
            self.last = None;
            return true;
        }
        self.last = Some(now);
        false
    }

    pub fn reset(&mut self) {
        self.last = None;
    }
}

/// Frame-rate independent smoothing factor for exponential easing.
///
/// `rate` is the decay rate per second; `rate = 10` moves ~15% of the
/// remaining distance per frame at 60 Hz (and ~8% at 120 Hz, so two 120 Hz
/// frames cover the same distance as one 60 Hz frame).
pub fn smooth_factor(dt: f32, rate: f32) -> f32 {
    let dt = dt.clamp(0.0, 0.25);
    (1.0 - (-dt * rate).exp()).clamp(0.0, 1.0)
}

/// Default smoothing rate for scroll animations (see [`smooth_factor`]).
pub const SCROLL_SMOOTH_RATE: f32 = 10.0;

/// Whether a reveal timestamp counts as an in-flight animation at `reference`.
pub fn reveal_in_flight(timestamp: Option<Instant>, reference: Instant, window: Duration) -> bool {
    timestamp.is_some_and(|t| reference.saturating_duration_since(t) < window)
}

/// Scroll target so that content ending at `item_bottom` (relative to the
/// content top) is visible, given the current target and the viewport height.
/// Returns `None` when no scrolling is needed.
pub fn scroll_target_to_show(
    item_bottom: f32,
    current_target: f32,
    available_height: f32,
    overflow: f32,
    margin: f32,
) -> Option<f32> {
    if overflow <= 0.0 || item_bottom <= current_target + available_height {
        return None;
    }
    Some((item_bottom - available_height + margin).clamp(0.0, overflow))
}

/// Outcome of a fullscreen monitor hop, judged after the window has settled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitorMoveOutcome {
    /// The window ended up on the requested monitor.
    Landed,
    /// No monitor there — wrap around to the primary monitor at the origin.
    Wrap,
    /// Already at the origin (or wrap attempted): there is no other monitor.
    Failed,
}

/// Decide what to do after a monitor move: compare where the window landed
/// (`actual_x`) with where we asked it to go (`target_x`).
pub fn evaluate_monitor_move(
    target_x: f32,
    actual_x: f32,
    monitor_width: f32,
    wrapped: bool,
) -> MonitorMoveOutcome {
    let half = (monitor_width / 2.0).max(1.0);
    if (actual_x - target_x).abs() < half {
        MonitorMoveOutcome::Landed
    } else if !wrapped && actual_x.abs() >= half {
        MonitorMoveOutcome::Wrap
    } else {
        MonitorMoveOutcome::Failed
    }
}

/// Position of the monitor to the right of the current one.
pub fn next_monitor_position(current: Pos2, monitor_width: f32) -> Pos2 {
    Pos2::new(current.x + monitor_width + 100.0, current.y)
}

/// Render the shortcut table as a two-column plain-text block for the
/// `mdeck spec --short` card.
pub fn shortcut_card() -> String {
    const KEY_W: usize = 22;
    const DESC_W: usize = 26;
    let mut out = String::new();
    let mut rows = SHORTCUTS.chunks(2);
    for pair in &mut rows {
        let (k1, d1) = pair[0];
        let mut line = format!("  {}{}", pad(k1, KEY_W), pad(d1, DESC_W));
        if let Some((k2, d2)) = pair.get(1) {
            line.push_str(&format!("{}{}", pad(k2, KEY_W), d2));
        }
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

/// Pad to `width` characters (not bytes — the table contains arrows).
fn pad(s: &str, width: usize) -> String {
    let len = s.chars().count();
    let mut out = s.to_string();
    for _ in len..width {
        out.push(' ');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(key: Key) -> Option<Action> {
        map_key(key, Modifiers::NONE, KeyMode::Presentation)
    }

    #[test]
    fn clicker_keys_navigate() {
        assert_eq!(map(Key::PageDown), Some(Action::Next));
        assert_eq!(map(Key::Enter), Some(Action::Next));
        assert_eq!(map(Key::PageUp), Some(Action::Previous));
        assert_eq!(map(Key::Backspace), Some(Action::Previous));
    }

    #[test]
    fn existing_navigation_keys_still_work() {
        assert_eq!(map(Key::Space), Some(Action::Next));
        assert_eq!(map(Key::N), Some(Action::Next));
        assert_eq!(map(Key::ArrowRight), Some(Action::Next));
        assert_eq!(map(Key::P), Some(Action::Previous));
        assert_eq!(map(Key::ArrowLeft), Some(Action::Previous));
        assert_eq!(map(Key::Home), Some(Action::FirstSlide));
        assert_eq!(map(Key::End), Some(Action::LastSlide));
        assert_eq!(map(Key::G), Some(Action::EnterGrid));
        assert_eq!(map(Key::H), Some(Action::ToggleHud));
        assert_eq!(map(Key::R), Some(Action::CycleRawOverlay));
        assert_eq!(map(Key::ArrowUp), Some(Action::ScrollUp));
        assert_eq!(map(Key::ArrowDown), Some(Action::ScrollDown));
    }

    #[test]
    fn blackout_on_period_and_b() {
        assert_eq!(map(Key::Period), Some(Action::ToggleBlackout));
        assert_eq!(map(Key::B), Some(Action::ToggleBlackout));
        assert_eq!(
            map_key(Key::B, Modifiers::NONE, KeyMode::Grid),
            Some(Action::ToggleBlackout)
        );
    }

    #[test]
    fn shift_t_is_theme_and_t_is_transition() {
        assert_eq!(map(Key::T), Some(Action::CycleTransition));
        assert_eq!(
            map_key(Key::T, Modifiers::SHIFT, KeyMode::Presentation),
            Some(Action::CycleTheme)
        );
    }

    #[test]
    fn ctrl_c_only_with_ctrl() {
        assert_eq!(map(Key::C), None);
        assert_eq!(
            map_key(Key::C, Modifiers::CTRL, KeyMode::Presentation),
            Some(Action::CtrlC)
        );
    }

    #[test]
    fn global_keys_work_in_every_mode() {
        for mode in [KeyMode::Presentation, KeyMode::Grid, KeyMode::Blocked] {
            assert_eq!(map_key(Key::Q, Modifiers::NONE, mode), Some(Action::Quit));
            assert_eq!(
                map_key(Key::Escape, Modifiers::NONE, mode),
                Some(Action::Escape)
            );
            assert_eq!(
                map_key(Key::F, Modifiers::NONE, mode),
                Some(Action::ToggleFullscreen)
            );
            assert_eq!(
                map_key(Key::M, Modifiers::NONE, mode),
                Some(Action::MoveMonitor)
            );
        }
    }

    #[test]
    fn grid_mode_keys() {
        let g = |k| map_key(k, Modifiers::NONE, KeyMode::Grid);
        assert_eq!(g(Key::ArrowRight), Some(Action::GridRight));
        assert_eq!(g(Key::ArrowLeft), Some(Action::GridLeft));
        assert_eq!(g(Key::ArrowDown), Some(Action::GridDown));
        assert_eq!(g(Key::ArrowUp), Some(Action::GridUp));
        assert_eq!(g(Key::Enter), Some(Action::GridSelect));
        assert_eq!(g(Key::Space), Some(Action::GridSelect));
        assert_eq!(g(Key::E), Some(Action::GridSelect));
        // Presentation-only keys are inert in the grid
        assert_eq!(g(Key::N), None);
        assert_eq!(g(Key::Home), None);
    }

    #[test]
    fn blocked_mode_ignores_navigation() {
        let b = |k| map_key(k, Modifiers::NONE, KeyMode::Blocked);
        assert_eq!(b(Key::Space), None);
        assert_eq!(b(Key::ArrowRight), None);
        assert_eq!(b(Key::Enter), None);
    }

    #[test]
    fn global_flag_matches_mapping() {
        assert!(Action::Quit.is_global());
        assert!(Action::ToggleBlackout.is_global());
        assert!(!Action::Next.is_global());
        assert!(!Action::GridSelect.is_global());
    }

    #[test]
    fn double_tap_within_window() {
        let mut dt = DoubleTap::new(Duration::from_secs(1));
        let t0 = Instant::now();
        assert!(!dt.tap(t0));
        assert!(dt.tap(t0 + Duration::from_millis(500)));
        // A completed double tap resets — the next tap starts over
        assert!(!dt.tap(t0 + Duration::from_millis(600)));
    }

    #[test]
    fn double_tap_outside_window_restarts() {
        let mut dt = DoubleTap::new(Duration::from_secs(1));
        let t0 = Instant::now();
        assert!(!dt.tap(t0));
        assert!(!dt.tap(t0 + Duration::from_millis(1500)));
        assert!(dt.tap(t0 + Duration::from_millis(1900)));
    }

    #[test]
    fn double_tap_reset_clears_pending() {
        let mut dt = DoubleTap::new(Duration::from_secs(1));
        let t0 = Instant::now();
        assert!(!dt.tap(t0));
        dt.reset();
        assert!(!dt.tap(t0 + Duration::from_millis(100)));
    }

    #[test]
    fn smooth_factor_is_frame_rate_independent() {
        let f60 = smooth_factor(1.0 / 60.0, SCROLL_SMOOTH_RATE);
        let f120 = smooth_factor(1.0 / 120.0, SCROLL_SMOOTH_RATE);
        assert!((f60 - 0.15).abs() < 0.01, "60Hz factor {f60}");
        assert!(f120 < f60);
        // Two 120 Hz steps ≈ one 60 Hz step
        let two_steps = 1.0 - (1.0 - f120) * (1.0 - f120);
        assert!((two_steps - f60).abs() < 1e-5);
    }

    #[test]
    fn smooth_factor_clamps_edge_cases() {
        assert_eq!(smooth_factor(0.0, SCROLL_SMOOTH_RATE), 0.0);
        assert_eq!(smooth_factor(-1.0, SCROLL_SMOOTH_RATE), 0.0);
        let big = smooth_factor(10.0, SCROLL_SMOOTH_RATE);
        assert!(big <= 1.0 && big > 0.9);
    }

    #[test]
    fn reveal_in_flight_window() {
        let now = Instant::now();
        let w = Duration::from_secs(3);
        assert!(!reveal_in_flight(None, now, w));
        assert!(reveal_in_flight(Some(now - Duration::from_secs(1)), now, w));
        assert!(!reveal_in_flight(
            Some(now - Duration::from_secs(5)),
            now,
            w
        ));
    }

    #[test]
    fn scroll_target_to_show_only_when_below_fold() {
        // Visible region [0, 500); item ends at 300 → nothing to do
        assert_eq!(scroll_target_to_show(300.0, 0.0, 500.0, 400.0, 40.0), None);
        // Item ends at 700 → scroll so it is visible with margin
        assert_eq!(
            scroll_target_to_show(700.0, 0.0, 500.0, 400.0, 40.0),
            Some(240.0)
        );
        // Clamped to overflow
        assert_eq!(
            scroll_target_to_show(2000.0, 0.0, 500.0, 400.0, 40.0),
            Some(400.0)
        );
        // No overflow → never scroll
        assert_eq!(scroll_target_to_show(700.0, 0.0, 500.0, 0.0, 40.0), None);
    }

    #[test]
    fn monitor_move_outcomes() {
        // Asked for x=2020, landed at 1920 → on the next monitor
        assert_eq!(
            evaluate_monitor_move(2020.0, 1920.0, 1920.0, false),
            MonitorMoveOutcome::Landed
        );
        // Asked for 3940 from the last monitor, stayed at 1920 → wrap
        assert_eq!(
            evaluate_monitor_move(3940.0, 1920.0, 1920.0, false),
            MonitorMoveOutcome::Wrap
        );
        // Single monitor at origin: stayed at 0 → fail (nothing to wrap to)
        assert_eq!(
            evaluate_monitor_move(2020.0, 0.0, 1920.0, false),
            MonitorMoveOutcome::Failed
        );
        // Already wrapped once and still not there → fail
        assert_eq!(
            evaluate_monitor_move(0.0, 1920.0, 1920.0, true),
            MonitorMoveOutcome::Failed
        );
        // Wrapped to the origin and landed
        assert_eq!(
            evaluate_monitor_move(0.0, 0.0, 1920.0, true),
            MonitorMoveOutcome::Landed
        );
    }

    #[test]
    fn next_monitor_position_moves_right() {
        let p = next_monitor_position(Pos2::new(0.0, 0.0), 1920.0);
        assert_eq!(p, Pos2::new(2020.0, 0.0));
    }

    #[test]
    fn shortcut_table_keys_all_map_to_actions() {
        // Every single-key token in the table must be a real binding.
        let known: &[(&str, Key)] = &[
            ("Space", Key::Space),
            ("N", Key::N),
            ("→", Key::ArrowRight),
            ("PgDn", Key::PageDown),
            ("Enter", Key::Enter),
            ("P", Key::P),
            ("←", Key::ArrowLeft),
            ("PgUp", Key::PageUp),
            ("Backspace", Key::Backspace),
            ("↑", Key::ArrowUp),
            ("↓", Key::ArrowDown),
            ("Home", Key::Home),
            ("End", Key::End),
            ("Esc", Key::Escape),
            ("Q", Key::Q),
            ("G", Key::G),
            ("E", Key::E),
            ("T", Key::T),
            ("F", Key::F),
            ("M", Key::M),
            ("H", Key::H),
            (".", Key::Period),
            ("B", Key::B),
            ("R", Key::R),
        ];
        for (token, key) in known {
            let in_table = SHORTCUTS.iter().any(|(k, _)| {
                k.split(['/', ' '])
                    .map(|s| s.trim_matches(|c| c == '×' || c == '2'))
                    .any(|s| s == *token)
            });
            assert!(in_table, "token {token} missing from SHORTCUTS");
            let mapped = map_key(*key, Modifiers::NONE, KeyMode::Presentation).is_some()
                || map_key(*key, Modifiers::NONE, KeyMode::Grid).is_some();
            assert!(mapped, "key {key:?} listed in SHORTCUTS but not mapped");
        }
    }

    #[test]
    fn shortcut_card_has_two_columns_and_correct_theme_key() {
        let card = shortcut_card();
        assert!(card.contains("Shift+T"));
        assert!(
            !card.lines().any(|l| l.trim_start().starts_with("D ")),
            "stale 'D' theme binding must be gone"
        );
        assert!(card.contains("PgDn"));
        assert!(card.contains(". / B"));
        assert!(card.contains("Home / End"));
        let first = card.lines().next().unwrap();
        // Two shortcut pairs per line
        assert!(first.contains("Next slide") && first.contains("Previous slide"));
        for line in card.lines() {
            assert!(line.starts_with("  "));
            assert!(!line.ends_with(' '));
        }
    }

    #[test]
    fn pad_counts_chars_not_bytes() {
        assert_eq!(pad("→", 3).chars().count(), 3);
        assert_eq!(pad("abcd", 2), "abcd");
    }
}
