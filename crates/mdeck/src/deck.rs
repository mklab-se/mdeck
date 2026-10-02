//! A deck as the presenting window and export both hold it: the parsed
//! presentation and everything resolved for it (reveal steps,
//! illustrations, generated art, logos, background images, images), and the
//! engine that draws under its slides. Both draw through
//! [`Deck::draw_background`], [`Deck::engine_layer`], [`Deck::draw_slide`]
//! and [`Deck::draw_logo`], so an export shows what the window shows.

use std::path::{Path, PathBuf};
use std::time::Instant;

use eframe::egui;

use crate::engines::{self, CountPhase, EngineKind, Host};
use crate::parser::{self, Presentation};
use crate::render::art::gallery::DeckArt;
use crate::render::background::{Backgrounds, FadeIn};
use crate::render::illustration::Library;
use crate::render::image_cache::ImageCache;
use crate::render::image_cache::ImageState;
use crate::render::logo::Logos;
use crate::render::{self, SlideContext};
use crate::theme::{Theme, lookup};

pub struct Deck {
    pub presentation: Presentation,
    /// The deck's markdown file.
    pub file: PathBuf,
    pub image_cache: ImageCache,
    /// Reveal steps per slide.
    pub max_steps: Vec<usize>,
    /// Point cloud illustrations resolved for this deck.
    pub illustrations: Library,
    /// Generated art for the art engines.
    pub art: DeckArt,
    /// The logo on each slide (theme `logo:`, the deck's `logo`, the slide's `logo`).
    pub logos: Logos,
    /// The background image on each slide (the deck's `background`, the slide's).
    pub backgrounds: Backgrounds,
    /// When each background image appeared, so it fades in once loaded.
    background_fade: FadeIn,
    /// The theme's engine: its layer under the slides, countdown and end act.
    pub engine: Host,
}

/// What the engine layer shows in one frame.
pub struct EngineFrame {
    pub rect: egui::Rect,
    pub scale: f32,
    /// The slide on screen (the target during a transition).
    pub index: usize,
    pub reveal: usize,
    /// The virtual end slide.
    pub end: bool,
    pub countdown: Option<(CountPhase, f32)>,
    /// Settle to a still frame (export).
    pub still: bool,
}

/// Where and how far one slide's content is drawn.
pub struct SlideFrame {
    pub rect: egui::Rect,
    pub opacity: f32,
    pub reveal: usize,
    /// When the latest step was revealed; `None` draws it settled.
    pub reveal_timestamp: Option<Instant>,
    pub scale: f32,
}

impl Deck {
    /// Resolve everything `presentation` needs in `theme`. Art loads in the
    /// background when `background_art` (the window) and inline otherwise
    /// (export). Problems are printed unless `quiet`.
    pub fn open(
        file: PathBuf,
        mut presentation: Presentation,
        theme: &Theme,
        background_art: bool,
        quiet: bool,
    ) -> Self {
        let dir = deck_dir(&file).to_path_buf();
        let mut image_cache = ImageCache::new(dir);
        load_thermal(&mut image_cache, &presentation, quiet);
        let max_steps = slide_max_steps(&mut presentation, image_cache.thermal());
        let mut art = DeckArt::new(Some(&file), background_art);
        art.sync(&presentation, theme);
        let mut deck = Self {
            image_cache,
            illustrations: Library::for_deck(file.parent()),
            presentation,
            file,
            max_steps,
            art,
            logos: Logos::default(),
            backgrounds: Backgrounds::default(),
            background_fade: FadeIn::default(),
            engine: Host::new(EngineKind::Plain),
        };
        deck.refresh_logos(theme);
        deck.refresh_backgrounds(quiet);
        deck
    }

    pub fn slide_count(&self) -> usize {
        self.presentation.slides.len()
    }

    /// Follow a theme switch: logos depend on the theme.
    pub fn retheme(&mut self, theme: &Theme) {
        self.refresh_logos(theme);
    }

    /// Swap in a re-parsed presentation and drop everything resolved for the
    /// old one.
    pub fn replace(&mut self, presentation: Presentation, theme: &Theme) {
        self.presentation = presentation;
        self.image_cache.clear();
        load_thermal(&mut self.image_cache, &self.presentation, false);
        self.illustrations.reset();
        self.art.invalidate();
        self.max_steps = slide_max_steps(&mut self.presentation, self.image_cache.thermal());
        self.refresh_logos(theme);
        self.refresh_backgrounds(false);
        self.background_fade.clear();
    }

    fn refresh_logos(&mut self, theme: &Theme) {
        let (logos, problems) =
            render::logo::resolve_slides(theme, &self.presentation, deck_dir(&self.file));
        report_theme_problems(&problems);
        self.logos = logos;
    }

    fn refresh_backgrounds(&mut self, quiet: bool) {
        let (backgrounds, problems) =
            render::background::resolve(&self.presentation, deck_dir(&self.file));
        if !quiet {
            for p in problems {
                eprintln!("warning: {p}");
            }
        }
        self.backgrounds = backgrounds;
    }

    /// Start loading the background images of slides `indices`.
    pub fn preload_backgrounds(&self, ctx: &egui::Context, indices: std::ops::Range<usize>) {
        for i in indices {
            if let Some(bg) = self.backgrounds.get(i) {
                self.image_cache.preload(ctx, &bg.path);
            }
        }
    }

    /// Paint slide `index`'s background image over `rect` (clipped by the
    /// painter), at `fade` times its opacity, with corners of `radius`. With
    /// `ease_in` an image fades in when its decode lands instead of popping
    /// up (the window; export waits for the image).
    pub fn draw_background(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        index: usize,
        fade: f32,
        radius: f32,
        ease_in: bool,
    ) {
        let Some(bg) = self.backgrounds.get(index) else {
            return;
        };
        let ImageState::Ready(texture) = self.image_cache.state(painter.ctx(), &bg.path) else {
            return;
        };
        let appear = if ease_in {
            let a = self.background_fade.amount(&bg.path, Instant::now());
            if a < 1.0 {
                painter.ctx().request_repaint();
            }
            a
        } else {
            1.0
        };
        render::background::draw(painter, rect, &texture, bg.opacity * fade * appear, radius);
    }

    /// Paint the engine's layer for `frame`: from where it is on screen, or,
    /// with `rehearse_at`, from a cold start that many seconds in (the
    /// `--at` stills in export).
    pub fn engine_layer(
        &mut self,
        ui: &egui::Ui,
        theme: &Theme,
        frame: EngineFrame,
        rehearse_at: Option<f32>,
    ) {
        let count = self.slide_count();
        let index = frame.index.min(count.saturating_sub(1));
        self.art.sync(&self.presentation, theme);
        let art = if frame.end {
            None
        } else {
            self.art.picture(index)
        };
        let shot = engines::Shot {
            rect: frame.rect,
            slide: (!frame.end).then(|| &self.presentation.slides[index]),
            art: art.as_ref(),
            index,
            reveal: frame.reveal,
            end: frame.end,
            countdown: frame.countdown,
            theme,
            scale: frame.scale,
            opacity: 1.0,
            still: frame.still,
            deck_title: self.presentation.meta.title.as_deref(),
            count,
        };
        match rehearse_at {
            Some(t) => self.engine.rehearse(ui, shot, &mut self.illustrations, t),
            None => self.engine.frame(ui, shot, &mut self.illustrations),
        }
    }

    /// Draw slide `index`'s content in `theme`.
    pub fn draw_slide(
        &self,
        ui: &egui::Ui,
        theme: &Theme,
        index: usize,
        frame: SlideFrame,
        cx: &SlideContext,
    ) {
        let Some(slide) = self.presentation.slides.get(index) else {
            return;
        };
        let block = render::BlockCx {
            ui,
            theme,
            opacity: frame.opacity,
            scale: frame.scale,
            image_cache: &self.image_cache,
            reveal_step: frame.reveal,
            reveal_timestamp: frame.reveal_timestamp,
        };
        render::render_slide(&block, slide, frame.rect, cx);
    }

    /// The chrome over a slide: the editorial counter and hairline, or the
    /// footer and the slide counter (a board engine prints its own). The
    /// window and export both draw it, so an export shows what the window
    /// shows.
    pub fn draw_chrome(
        &self,
        painter: &egui::Painter,
        theme: &Theme,
        rect: egui::Rect,
        cx: &SlideContext,
        scale: f32,
    ) {
        if theme.engine.capabilities().editorial {
            render::ember::draw_chrome(painter, theme, rect, cx, scale);
            return;
        }
        if theme.engine.is_board() {
            return;
        }
        if let Some(footer) = &self.presentation.meta.footer {
            let color = Theme::with_opacity(theme.foreground, 0.4);
            let galley = painter.layout_no_wrap(
                footer.clone(),
                egui::FontId::proportional(14.0 * scale),
                color,
            );
            let pos = egui::pos2(
                rect.center().x - galley.rect.width() / 2.0,
                rect.bottom() - 30.0 * scale,
            );
            painter.galley(pos, galley, color);
        }
        let color = Theme::with_opacity(theme.foreground, 0.3);
        let galley = painter.layout_no_wrap(
            format!("{} / {}", cx.index + 1, cx.count),
            egui::FontId::monospace(14.0 * scale),
            color,
        );
        let pos = egui::pos2(
            rect.right() - galley.rect.width() - 16.0 * scale,
            rect.bottom() - 30.0 * scale,
        );
        painter.galley(pos, galley, color);
    }

    /// Draw slide `index`'s logo, if it has one.
    pub fn draw_logo(&self, painter: &egui::Painter, rect: egui::Rect, index: usize, scale: f32) {
        if let Some(logo) = self.logos.get(index) {
            render::logo::draw(painter, rect, logo, scale, 1.0);
        }
    }
}

/// Read the deck's thermal sources and tell the author what will not be
/// shown as written (unless `quiet`).
fn load_thermal(cache: &mut ImageCache, presentation: &Presentation, quiet: bool) {
    let diagnostics = cache.thermal_mut().load(presentation);
    if !quiet {
        for d in diagnostics {
            eprintln!("warning: {d}");
        }
    }
}

fn deck_dir(file: &Path) -> &Path {
    file.parent().unwrap_or(Path::new("."))
}

/// Reveal steps per slide. A thermal block's steps depend on what its
/// source can show, so with the sources read every slide is numbered again
/// (the steps after a thermal block follow its real count).
fn slide_max_steps(
    presentation: &mut Presentation,
    thermal: &render::thermal::Library,
) -> Vec<usize> {
    let visual_steps = |block: &parser::Block| match block {
        parser::Block::Chart {
            kind: parser::Chart::Thermal,
            content,
            ..
        } => render::thermal::block_steps(content, thermal),
        other => parser::steps::default_visual_steps(other),
    };
    presentation
        .slides
        .iter_mut()
        .map(|s| {
            s.steps = parser::steps::number(&mut s.blocks, s.reveal, &visual_steps);
            s.steps
        })
        .collect()
}

/// The theme a deck asks for: its `theme`, then the config default, then
/// the built-in default. Returns the theme and the name it was looked up by.
pub fn deck_theme(
    themes: &lookup::Lookup,
    presentation: &Presentation,
    config_default: Option<&str>,
) -> (Theme, String) {
    let key = lookup::select(presentation.meta.theme.as_deref(), config_default);
    let (theme, problems) = lookup::resolve_or_default(themes, &key);
    report_theme_problems(&problems);
    (theme, key)
}

/// The engine override for a deck: `--engine`, then its `engine` (whose
/// problems are printed unless quiet). `None` keeps the theme's own.
pub fn deck_engine(
    cli: Option<EngineKind>,
    presentation: &Presentation,
    quiet: bool,
) -> Option<EngineKind> {
    if cli.is_some() {
        return cli;
    }
    // Only a CLI name can fail; the deck's name only warns.
    let (kind, warnings) =
        engines::choose(None, presentation.meta.engine.as_deref()).unwrap_or((None, Vec::new()));
    if !quiet {
        for w in warnings {
            eprintln!("warning: {w}");
        }
    }
    kind
}

/// Print theme problems (unknown name, invalid file, fallbacks) to stderr.
pub fn report_theme_problems(problems: &[String]) {
    for p in problems {
        eprintln!("warning: theme: {p}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cli_engine_wins_over_the_deck() {
        let pres = parser::parse("---\nengine: led\n---\n# A\n");
        assert_eq!(
            deck_engine(Some(EngineKind::Plain), &pres, true),
            Some(EngineKind::Plain)
        );
    }
}
