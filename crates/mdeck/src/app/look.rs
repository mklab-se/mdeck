//! Switching the theme (`Shift+T`, a reload), the transition (`T`) and
//! the thermal palette (`C`, `Shift+C`).

use crate::deck;
use crate::render::transition::TransitionKind;
use crate::theme::Theme;

use super::PresentationApp;
use super::toast::Toast;

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

    /// Switch to `theme` now.
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

    /// The transition to use: the chosen one, or none with reduced motion.
    pub(super) fn transition_kind(&self) -> TransitionKind {
        if self.reduced_motion {
            TransitionKind::None
        } else {
            self.default_transition
        }
    }

    /// `C`: the next thermal palette for every `@thermal` image and legend.
    pub(super) fn cycle_palette(&mut self) {
        use crate::render::thermal::Palette;
        if !self.has_thermal() {
            self.toast = Some(Toast::new("No thermal images in this deck".to_string()));
            return;
        }
        let next = match self.live_palette {
            Some(p) => p.next(),
            None => self.deck_palette().unwrap_or(Palette::DEFAULT).next(),
        };
        self.live_palette = Some(next);
        self.toast = Some(Toast::new(format!("Thermal palette: {}", next.name())));
    }

    /// `Shift+C`: back to the palettes the deck was written with.
    pub(super) fn reset_palette(&mut self) {
        if self.live_palette.take().is_some() {
            self.toast = Some(Toast::new("Thermal palette: as written".to_string()));
        }
    }

    /// The deck's `@palette`, if it names a palette.
    pub(super) fn deck_palette(&self) -> Option<crate::render::thermal::Palette> {
        self.deck
            .presentation
            .meta
            .palette
            .as_deref()
            .and_then(crate::render::thermal::Palette::from_name)
    }

    fn has_thermal(&self) -> bool {
        self.deck
            .presentation
            .slides
            .iter()
            .any(|s| !crate::render::thermal::blocks(s).is_empty())
    }
}
