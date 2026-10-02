//! What renderers draw with: the deck facts a slide shows ([`SlideContext`])
//! and the drawing contexts threaded through text and layouts ([`TextCx`],
//! [`BlockCx`]).

use std::time::Instant;

use eframe::egui;

use crate::render::image_cache::ImageCache;
use crate::render::visualizations::VizCtx;
use crate::theme::Theme;

/// Facts about the deck that the eyebrow and chrome show.
#[derive(Clone, Debug, Default)]
pub struct SlideContext {
    pub index: usize,
    pub count: usize,
    pub deck_title: Option<String>,
    pub author: Option<String>,
    /// While the logo intro runs, the title copy holds back.
    pub hold_copy: bool,
    /// Play entry and reveal animations (false for export and thumbnails).
    pub animate: bool,
    /// The engine drew this slide live already (a board engine's board);
    /// false for grid thumbnails and the overview zoom.
    pub engine_drew: bool,
}

/// What text draws with: where, in which theme, how faded and at what scale.
#[derive(Clone, Copy)]
pub struct TextCx<'a> {
    pub ui: &'a egui::Ui,
    pub theme: &'a Theme,
    pub opacity: f32,
    pub scale: f32,
}

/// What a block flow and the layouts draw with: the text context plus the
/// images the blocks show and how far the slide is revealed.
#[derive(Clone, Copy)]
pub struct BlockCx<'a> {
    pub ui: &'a egui::Ui,
    pub theme: &'a Theme,
    pub opacity: f32,
    pub scale: f32,
    pub image_cache: &'a ImageCache,
    /// Reveal steps shown so far.
    pub reveal_step: usize,
    /// When the latest step was revealed (animates it in); `None` draws it
    /// settled.
    pub reveal_timestamp: Option<Instant>,
}

impl<'a> BlockCx<'a> {
    /// The same context drawing with `theme` instead.
    pub fn with_theme<'b>(&self, theme: &'b Theme) -> BlockCx<'b>
    where
        'a: 'b,
    {
        BlockCx { theme, ..*self }
    }

    /// This context as a visual whose steps follow `base` earlier steps on
    /// the slide sees it: its own step count starts after them.
    pub fn after_steps(&self, base: usize) -> BlockCx<'a> {
        BlockCx {
            reveal_step: self.reveal_step.saturating_sub(base),
            ..*self
        }
    }

    /// The text half of this context.
    pub fn text(&self) -> TextCx<'a> {
        TextCx {
            ui: self.ui,
            theme: self.theme,
            opacity: self.opacity,
            scale: self.scale,
        }
    }

    /// This context as a chart sees it.
    pub fn viz(&self) -> VizCtx<'a> {
        VizCtx {
            ui: self.ui,
            theme: self.theme,
            opacity: self.opacity,
            scale: self.scale,
            reveal_step: self.reveal_step,
            reveal_timestamp: self.reveal_timestamp,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::test_support::with_ui;

    #[test]
    fn block_context_hands_its_fields_to_text_and_charts() {
        with_ui(|ui| {
            let theme = Theme::dark();
            let light = Theme::light();
            let cache = ImageCache::new(std::path::PathBuf::new());
            let now = Instant::now();
            let cx = BlockCx {
                ui,
                theme: &theme,
                opacity: 0.5,
                scale: 2.0,
                image_cache: &cache,
                reveal_step: 3,
                reveal_timestamp: Some(now),
            };
            let t = cx.text();
            assert_eq!((t.opacity, t.scale), (0.5, 2.0));
            assert!(std::ptr::eq(t.theme, &theme));
            let v = cx.viz();
            assert_eq!((v.opacity, v.scale, v.reveal_step), (0.5, 2.0, 3));
            assert_eq!(v.reveal_timestamp, Some(now));
            let swapped = cx.with_theme(&light);
            assert!(std::ptr::eq(swapped.theme, &light));
            assert_eq!(swapped.reveal_step, 3);
        });
    }
}
