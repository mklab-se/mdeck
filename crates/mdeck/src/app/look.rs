//! Switching the theme (`Shift+T`, a reload), the transition (`T`) and
//! the thermal palette (`C`, `Shift+C`).

use crate::deck;
use crate::render::transition::TransitionKind;
use crate::theme::Theme;

use super::PresentationApp;
use super::toast::Toast;

use crate::render::transition::{resolve as resolve_transition, slide_transition};

/// The transition after `current` in `all`, wrapping round (the first
/// when `current` is not in it).
fn next_transition(current: TransitionKind, all: &[TransitionKind]) -> TransitionKind {
    let i = all.iter().position(|k| *k == current);
    match i {
        Some(i) => all[(i + 1) % all.len()],
        None => all.first().copied().unwrap_or(TransitionKind::Fade),
    }
}

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

    /// `T`: the next transition, for the rest of the session.
    pub(super) fn cycle_transition(&mut self) {
        let current = self
            .cycled_transition
            .unwrap_or_else(|| self.resolved_transition());
        let next = next_transition(current, &TransitionKind::all());
        self.cycled_transition = Some(next);
        let name = match next {
            TransitionKind::SlideHorizontal => "Slide",
            TransitionKind::Fade => "Fade",
            TransitionKind::Spatial => "Spatial",
            TransitionKind::None => "None",
            TransitionKind::Extension(name) => name,
        };
        self.toast = Some(Toast::new(format!("Transition: {name}")));
    }

    /// The deck's transition: its own setting, then the theme's (the one
    /// about to be shown when a theme waits for its fonts), then the user
    /// config, then `fade`.
    pub(super) fn resolved_transition(&self) -> TransitionKind {
        let theme = self.pending_theme.as_ref().unwrap_or(&self.theme);
        resolve_transition(
            self.deck.presentation.meta.transition.as_deref(),
            theme.transition.as_deref(),
            self.config_transition.as_deref(),
        )
    }

    /// The transition into slide `to` from `from`: none with reduced
    /// motion; otherwise a slide's own transition (the one being entered
    /// going forward, the one being left going back, so the way back
    /// mirrors the way in), then the one picked with `T`, then the deck's.
    pub(super) fn transition_between(&self, from: usize, to: usize) -> TransitionKind {
        if self.reduced_motion {
            return TransitionKind::None;
        }
        // a board engine owns its transitions: per-slide ones are ignored
        let own = if self.theme.engine.is_board() {
            None
        } else {
            let entered = if to >= from { to } else { from };
            self.deck
                .presentation
                .slides
                .get(entered)
                .and_then(slide_transition)
        };
        own.or(self.cycled_transition)
            .unwrap_or_else(|| self.resolved_transition())
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

    /// The deck's `palette`, if it names a palette.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_precedence_is_deck_theme_config_fade() {
        let r = resolve_transition;
        assert_eq!(
            r(Some("none"), Some("spatial"), Some("slide")),
            TransitionKind::None
        );
        assert_eq!(
            r(None, Some("spatial"), Some("slide")),
            TransitionKind::Spatial
        );
        assert_eq!(
            r(None, None, Some("slide")),
            TransitionKind::SlideHorizontal
        );
        assert_eq!(r(None, None, None), TransitionKind::Fade);
    }

    /// D19: a blank deck value used to skip the config default and fall
    /// through to the built-in one.
    #[test]
    fn a_blank_or_unknown_deck_transition_falls_to_the_next_in_line() {
        let r = resolve_transition;
        assert_eq!(r(Some(""), None, Some("spatial")), TransitionKind::Spatial);
        assert_eq!(r(Some("  "), Some("none"), None), TransitionKind::None);
        assert_eq!(
            r(Some("wipe"), None, Some("slide")),
            TransitionKind::SlideHorizontal
        );
        assert_eq!(r(Some(" Fade "), None, None), TransitionKind::Fade);
    }

    #[test]
    fn t_cycles_through_the_builtins_then_the_registered_ones() {
        let all = [
            TransitionKind::SlideHorizontal,
            TransitionKind::Fade,
            TransitionKind::Spatial,
            TransitionKind::None,
            TransitionKind::Extension("drop"),
        ];
        let n = |k| next_transition(k, &all);
        assert_eq!(n(TransitionKind::SlideHorizontal), TransitionKind::Fade);
        assert_eq!(n(TransitionKind::None), TransitionKind::Extension("drop"));
        assert_eq!(
            n(TransitionKind::Extension("drop")),
            TransitionKind::SlideHorizontal
        );
        assert_eq!(
            n(TransitionKind::Extension("gone")),
            TransitionKind::SlideHorizontal
        );
    }

    #[test]
    fn a_slide_sets_its_own_transition_but_not_zoom() {
        let slides = crate::parser::parse(
            "# A\n\n# B\n<!-- transition: spatial -->\n\n# C\n<!-- transition: zoom -->\n",
        )
        .slides;
        assert_eq!(slide_transition(&slides[0]), None);
        assert_eq!(slide_transition(&slides[1]), Some(TransitionKind::Spatial));
        assert_eq!(slide_transition(&slides[2]), None);
    }
}
