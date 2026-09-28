//! Switching the theme (`Shift+T`, a reload) and the transition (`T`).

use super::*;

impl PresentationApp {
    /// `Shift+T`: the next theme visible from this deck (built-ins, then
    /// user and deck themes by name).
    pub(super) fn toggle_theme(&mut self) {
        let names: Vec<String> = self
            .themes
            .available()
            .into_iter()
            .map(|f| f.name)
            .collect();
        if names.is_empty() {
            return;
        }
        let next = names
            .iter()
            .position(|n| *n == self.theme_key)
            .map(|i| (i + 1) % names.len())
            .unwrap_or(0);
        let key = names[next].clone();
        match self.themes.load(&key) {
            Ok(built) => {
                deck::report_theme_problems(&built.warnings);
                self.theme_key = key;
                self.pending_theme = Some(built.theme);
            }
            Err(e) => {
                // Skip a broken theme rather than getting stuck on it.
                deck::report_theme_problems(&[e.to_string()]);
                self.theme_key = key;
                self.toast = Some(Toast::new(format!("Theme {}: {e}", names[next])));
            }
        }
    }

    /// Switch to `theme` now: step counts follow its engine (story beats are
    /// an ember feature; other engines step through the content's own reveals).
    pub(super) fn apply_theme(&mut self, theme: Theme) {
        self.theme = crate::engines::with_engine(theme, self.engine_override);
        self.deck.retheme(&self.theme);
        self.clamp_reveals();
        self.toast = Some(Toast::new(format!("Theme: {}", self.theme.name)));
    }

    pub(super) fn cycle_transition(&mut self) {
        self.default_transition = match self.default_transition {
            TransitionKind::SlideHorizontal => TransitionKind::Fade,
            TransitionKind::Fade => TransitionKind::Spatial,
            TransitionKind::Spatial => TransitionKind::None,
            TransitionKind::None => TransitionKind::SlideHorizontal,
        };
        let name = match self.default_transition {
            TransitionKind::SlideHorizontal => "Slide",
            TransitionKind::Fade => "Fade",
            TransitionKind::Spatial => "Spatial",
            TransitionKind::None => "None",
        };
        self.toast = Some(Toast::new(format!("Transition: {name}")));
    }
}
