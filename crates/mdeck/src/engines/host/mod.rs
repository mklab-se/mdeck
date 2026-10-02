//! The engine host: the engine-neutral half of drawing an engine, owned by
//! the presentation window and export. It keeps the clock, follows the
//! geometry renderers publish, times the end slide, draws the countdown
//! digits and end words as masks, resolves the slide's picture, and hands
//! all of it to the engine through the SDK ([`mdeck_sdk::stage::Stage`],
//! [`mdeck_sdk::stage::Frame`], [`mdeck_sdk::paint::Painter`]). Engines see
//! nothing of mdeck beyond that.

mod choice;
pub mod convert;
mod masks;
pub mod place;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use eframe::egui;
use mdeck_sdk::cloud::{Cloud as SdkCloud, Mask};
use mdeck_sdk::engine::{Annotation, Engine};
use mdeck_sdk::geometry::{Hint as SdkHint, fingerprint};
use mdeck_sdk::host::{self as h, Backend};
use mdeck_sdk::paint::ImageData;
use mdeck_sdk::stage::{Frame, Moment, Picture, PictureSource, Stage};
use mdeck_sdk::tokens::EngineSettings;

pub use choice::{choose, unsupported, unsupported_summary, with_engine};
use masks::{glyph_mask, text_mask, trim_flag};
use place::figure_box;

use super::EngineId;
use crate::parser::Slide;
use crate::render::art::prepare::Prepared;
use crate::render::hints;
use crate::render::illustration::Library;
use crate::theme::Theme;

/// Where the opening countdown is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CountPhase {
    /// Showing this digit (3, 2 or 1).
    Digit(u8),
    /// The last digit leaves and the first slide arrives.
    Burst,
}

/// One frame's worth of input from the presentation window or the export.
#[derive(Clone, Copy)]
pub struct Shot<'a> {
    pub rect: egui::Rect,
    /// The slide to show (the target during a transition); `None` on the end slide.
    pub slide: Option<&'a Slide>,
    /// The slide's generated picture, when it has one and it is loaded.
    pub art: Option<&'a Arc<Prepared>>,
    pub index: usize,
    pub reveal: usize,
    /// On the end slide.
    pub end: bool,
    pub countdown: Option<(CountPhase, f32)>,
    pub theme: &'a Theme,
    pub scale: f32,
    pub opacity: f32,
    /// Export: settle at once.
    pub still: bool,
    pub deck_title: Option<&'a str>,
    /// Slides in the deck.
    pub count: usize,
    /// The deck's folder, for pictures that are image paths.
    pub deck_dir: &'a Path,
}

/// One step of the engine's clock: seconds since the last, seconds into
/// the end slide, whether to settle at once (export) and whether to paint
/// (a rehearsal paints only its last step).
#[derive(Clone, Copy)]
struct Tick {
    dt: f32,
    end_elapsed: f32,
    still: bool,
    paint: bool,
}

pub struct Host {
    id: EngineId,
    /// The settings the engine was made with, and what they were read from.
    settings: EngineSettings,
    settings_key: String,
    engine: Box<dyn Engine>,
    /// How painters draw sprites: GL in the window and export.
    backend: Backend,
    /// When the end slide was entered, for its choreography.
    end_started: Option<Instant>,
    /// Geometry the current slide's renderers published, and its fingerprint.
    hints: Vec<SdkHint>,
    hints_key: u64,
    /// Slide shown last frame (hints belong to one slide).
    last_index: Option<usize>,
    /// "THE END" as a mask, rasterised once per display face.
    end_words: Option<Mask>,
    /// Glyph masks for the countdown digits.
    digits: Vec<(u8, Mask)>,
    /// The display face the masks were drawn in.
    mask_face: Option<egui::FontFamily>,
    last_tick: Option<Instant>,
    /// Slides as the SDK sees them, by index, with the source they were made from.
    slides: HashMap<usize, (String, Arc<mdeck_sdk::content::Slide>)>,
    /// Point clouds as the SDK sees them, by name.
    clouds: HashMap<String, Arc<SdkCloud>>,
    /// Pictures that are image paths, decoded once (`None`: unreadable).
    images: HashMap<PathBuf, Option<Arc<ImageData>>>,
}

impl Host {
    pub fn new(id: EngineId) -> Self {
        let settings = EngineSettings::new();
        Self {
            id,
            engine: (id.def().create)(&settings),
            settings,
            settings_key: String::new(),
            backend: Backend::Glow,
            end_started: None,
            hints: Vec::new(),
            hints_key: 0,
            last_index: None,
            end_words: None,
            digits: Vec::new(),
            mask_face: None,
            last_tick: None,
            slides: HashMap::new(),
            clouds: HashMap::new(),
            images: HashMap::new(),
        }
    }

    /// Seconds since the end slide was entered (0 when not on it).
    pub fn end_elapsed(&self) -> f32 {
        self.end_started
            .map(|t| t.elapsed().as_secs_f32())
            .unwrap_or(0.0)
    }

    /// Make sure the runtime is the theme's engine with the theme's
    /// settings: a new engine or new settings start a fresh runtime.
    fn follow(&mut self, theme: &Theme) {
        let settings = crate::theme::engine_settings(theme);
        let key = settings_key(&settings);
        if theme.engine != self.id || key != self.settings_key {
            *self = Host::new(theme.engine).with_settings(settings, key, self.backend);
        }
    }

    fn with_settings(mut self, settings: EngineSettings, key: String, backend: Backend) -> Self {
        self.engine = (self.id.def().create)(&settings);
        self.settings = settings;
        self.settings_key = key;
        self.backend = backend;
        self
    }

    /// Advance the engine one frame and paint it into `shot.rect`. A theme
    /// on another engine (or with other engine settings) swaps the runtime
    /// first.
    pub fn frame(&mut self, ui: &egui::Ui, shot: Shot, lib: &mut Library) {
        self.follow(shot.theme);
        if !self.id.paints() {
            return;
        }
        let now = Instant::now();
        let dt = self
            .last_tick
            .map(|t| now.duration_since(t).as_secs_f32())
            .unwrap_or(1.0 / 60.0);
        self.last_tick = Some(now);
        // The end slide runs its own clock from the moment it is entered.
        let end_elapsed = if shot.end {
            let started = *self.end_started.get_or_insert(now);
            now.duration_since(started).as_secs_f32()
        } else {
            self.end_started = None;
            0.0
        };
        let tick = Tick {
            dt,
            end_elapsed,
            still: shot.still,
            paint: true,
        };
        self.step(ui, &shot, lib, tick);
    }

    /// Run the engine from a cold start through `seconds` of simulated time
    /// at 60 frames a second, then paint that frame: a still of the motion
    /// (`mdeck export --at`, for looking at animations in export). A burst
    /// runs its progress over the rehearsal.
    pub fn rehearse(&mut self, ui: &egui::Ui, shot: Shot, lib: &mut Library, seconds: f32) {
        let backend = self.backend;
        *self = Host::new(shot.theme.engine);
        self.backend = backend;
        self.follow(shot.theme);
        if !self.id.paints() {
            return;
        }
        let dt = 1.0 / 60.0;
        let steps = (seconds.max(0.0) / dt).round() as usize;
        for k in 0..=steps {
            let t = k as f32 * dt;
            let mut s = shot;
            if let Some((CountPhase::Burst, _)) = s.countdown {
                s.countdown = Some((CountPhase::Burst, (t / 1.2).min(1.0)));
            }
            let tick = Tick {
                dt,
                end_elapsed: t,
                still: false,
                paint: k == steps,
            };
            self.step(ui, &s, lib, tick);
        }
    }

    /// Let the engine draw the presenter's pen strokes its own way (the
    /// thermal heat trace). `false`: the core draws its plain ink.
    pub fn annotate(
        &mut self,
        ui: &egui::Ui,
        theme: &Theme,
        rect: egui::Rect,
        strokes: &[Annotation],
    ) -> bool {
        if theme.engine != self.id || strokes.is_empty() {
            return false;
        }
        let tokens = convert::tokens(theme);
        let frame = Frame::new(h::rect(rect), &tokens, &self.settings);
        let mut painter = h::painter(ui.painter().clone(), self.backend);
        let drew = self.engine.annotate(&mut painter, &frame, strokes);
        if drew {
            ui.ctx().request_repaint();
        }
        drew
    }

    fn step(&mut self, ui: &egui::Ui, shot: &Shot, lib: &mut Library, tick: Tick) {
        self.follow_hints(ui, shot.index, shot.theme);
        let moment = self.moment(ui, shot, tick.end_elapsed);
        let title = shot
            .slide
            .is_some_and(|s| crate::render::ember::is_title(s, shot.index));
        let picture = self.picture(shot, lib, title);
        let slide = shot.slide.map(|s| self.sdk_slide(shot.index, s));
        let stage = Stage {
            moment,
            index: shot.index,
            step: shot.reveal,
            slide: slide.as_deref(),
            title,
            picture,
            geometry: &self.hints,
            geometry_key: self.hints_key,
            deck_title: shot.deck_title,
            count: shot.count,
        };
        h::set_font_families(ui.ctx(), convert::font_families(shot.theme));
        let tokens = convert::tokens(shot.theme);
        let mut frame = Frame::new(h::rect(shot.rect), &tokens, &self.settings);
        frame.scale = shot.scale;
        frame.opacity = shot.opacity;
        frame.dt = tick.dt;
        frame.still = tick.still;
        self.engine.update(&frame, &stage);
        if tick.paint {
            let mut painter = h::painter(ui.painter().clone(), self.backend);
            self.engine.paint(&mut painter, &frame, &stage);
            if !tick.still && self.engine.animating() {
                ui.ctx().request_repaint();
            }
        }
        self.last_index = Some(shot.index);
    }

    /// The slide as the SDK's content model, made once per slide source.
    fn sdk_slide(&mut self, index: usize, slide: &Slide) -> Arc<mdeck_sdk::content::Slide> {
        if let Some((src, s)) = self.slides.get(&index)
            && *src == slide.raw_source
        {
            return Arc::clone(s);
        }
        let s = Arc::new(convert::slide(slide));
        self.slides
            .insert(index, (slide.raw_source.clone(), Arc::clone(&s)));
        s
    }

    /// The slide's picture (D13), on engines that show pictures: its
    /// current generated artwork (art engines), else the point cloud it
    /// names, else the image it names. Clouds and images show only where
    /// the design has a stage ([`crate::render::design_has_stage`]).
    fn picture(&mut self, shot: &Shot, lib: &mut Library, title: bool) -> Option<Picture> {
        let caps = self.id.capabilities();
        if !caps.picture {
            return None;
        }
        let slide = shot.slide?;
        let rect_aspect = shot.rect.width() / shot.rect.height();
        let placed = |source: PictureSource, aspect: f32| Picture {
            source,
            backdrop: title,
            place: figure_box(aspect, slide.layout, rect_aspect, title),
        };
        if caps.medium.is_some()
            && let Some(art) = shot.art
        {
            return Some(placed(
                PictureSource::Artwork(Arc::clone(art)),
                art.aspect(),
            ));
        }
        if !crate::render::design_has_stage(slide, shot.theme) {
            return None;
        }
        let name = slide.illustration.as_deref()?;
        if let Some(cloud) = lib.get(name) {
            let sdk = self
                .clouds
                .entry(name.to_string())
                .or_insert_with(|| {
                    Arc::new(SdkCloud::new(
                        cloud.name.clone(),
                        cloud.points.to_vec(),
                        cloud.aspect,
                    ))
                })
                .clone();
            let aspect = sdk.aspect;
            return Some(placed(PictureSource::Cloud(sdk), aspect));
        }
        let image = self.image(&shot.deck_dir.join(name))?;
        let aspect = image.height() as f32 / image.width().max(1) as f32;
        Some(placed(PictureSource::Image(image), aspect))
    }

    /// An image picture, decoded once (only files that look like images).
    fn image(&mut self, path: &Path) -> Option<Arc<ImageData>> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)?;
        if !matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp") {
            return None;
        }
        self.images
            .entry(path.to_path_buf())
            .or_insert_with(|| {
                let img = image::open(path).ok()?.to_rgba8();
                let size = [img.width() as usize, img.height() as usize];
                ImageData::from_rgba_unmultiplied(size, img.as_raw()).map(Arc::new)
            })
            .clone()
    }

    /// Take the geometry the renderers published. They publish while the
    /// slide draws (after the engine), so what arrives is last frame's;
    /// collection stays on only while an engine is drawing.
    fn follow_hints(&mut self, ui: &egui::Ui, index: usize, theme: &Theme) {
        hints::set_enabled(ui.ctx(), true);
        let fresh: Vec<SdkHint> = hints::take(ui.ctx())
            .iter()
            .flat_map(|h| convert::hints(ui.ctx(), h, theme))
            .collect();
        let changed = self.last_index.is_some_and(|i| i != index);
        adopt_hints(&mut self.hints, &mut self.hints_key, fresh, changed);
    }

    /// What the frame shows: a countdown digit or its burst, the end slide
    /// with its words, or a slide. Masks are drawn in the theme's display
    /// face, again when the face changes.
    fn moment(&mut self, ui: &egui::Ui, shot: &Shot, end_elapsed: f32) -> Moment {
        let face = shot.theme.display_family();
        if self.mask_face.as_ref() != Some(&face) {
            self.digits.clear();
            self.end_words = None;
            self.mask_face = Some(face);
        }
        if let Some((phase, progress)) = shot.countdown {
            match phase {
                CountPhase::Digit(digit) => Moment::Countdown {
                    digit,
                    mask: self.digit_mask(ui, shot.theme, digit),
                    progress,
                },
                CountPhase::Burst => Moment::Burst { progress },
            }
        } else if shot.end {
            let words = self
                .end_words
                .get_or_insert_with(|| text_mask(ui, shot.theme, "THE END"))
                .clone();
            Moment::End {
                elapsed: end_elapsed,
                words,
            }
        } else {
            Moment::Slide
        }
    }

    /// Mask points and aspect (width / height) for a digit, rasterised once
    /// through egui in the theme's display face.
    fn digit_mask(&mut self, ui: &egui::Ui, theme: &Theme, digit: u8) -> Mask {
        if let Some((_, mask)) = self.digits.iter().find(|(d, _)| *d == digit) {
            return mask.clone();
        }
        let mut mask = glyph_mask(ui, theme, char::from(b'0' + digit));
        if digit == 1 {
            mask = trim_flag(mask);
        }
        self.digits.push((digit, mask.clone()));
        mask
    }
}

/// What a set of settings was read from: changes when any value does.
fn settings_key(settings: &EngineSettings) -> String {
    let probe = settings.clone();
    let keys: Vec<String> = probe.keys().map(str::to_string).collect();
    keys.iter()
        .map(|k| format!("{k}={:?};", probe.get(k)))
        .collect()
}

/// Fold the hints taken this frame into the held set. On the first frame of
/// a new slide, `fresh` is what the previous slide drew last frame, so it is
/// dropped with the held set instead of adopted (a slide without visuals
/// would otherwise keep the previous chart's geometry, D24).
fn adopt_hints(held: &mut Vec<SdkHint>, key: &mut u64, fresh: Vec<SdkHint>, slide_changed: bool) {
    if slide_changed {
        held.clear();
        *key = 0;
        return;
    }
    if !fresh.is_empty() {
        let fp = fingerprint(&fresh);
        if fp != *key {
            *held = fresh;
            *key = fp;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mdeck_sdk::paint::{Pos2, Rect, Vec2};

    fn bar() -> Vec<SdkHint> {
        vec![SdkHint::Bar(Rect::from_min_size(
            Pos2::new(10.0, 20.0),
            Vec2::new(40.0, 80.0),
        ))]
    }

    #[test]
    fn hints_taken_on_a_slide_change_belong_to_the_previous_slide() {
        let mut held = bar();
        let mut key = fingerprint(&held);
        // First frame of the next slide: what arrives is the chart's.
        adopt_hints(&mut held, &mut key, bar(), true);
        assert!(held.is_empty(), "the previous slide's chart leaked");
        assert_eq!(key, 0);
        // A slide without visuals publishes nothing afterwards.
        adopt_hints(&mut held, &mut key, Vec::new(), false);
        assert!(held.is_empty());
    }

    #[test]
    fn hints_on_the_same_slide_are_adopted_and_kept() {
        let mut held = Vec::new();
        let mut key = 0;
        adopt_hints(&mut held, &mut key, bar(), false);
        assert_eq!(held.len(), 1);
        assert_ne!(key, 0);
        // A frame that published nothing keeps the held geometry.
        adopt_hints(&mut held, &mut key, Vec::new(), false);
        assert_eq!(held.len(), 1);
    }

    #[test]
    fn settings_keys_change_with_values() {
        use mdeck_sdk::tokens::Value;
        let a = EngineSettings::from_pairs([("surface", Value::String("sheet".into()))]);
        let b = EngineSettings::from_pairs([("surface", Value::String("slate".into()))]);
        assert_ne!(settings_key(&a), settings_key(&b));
        assert_eq!(settings_key(&a), settings_key(&a.clone()));
        // reading for the key does not mark settings read
        assert_eq!(a.problems().len(), 1);
    }
}
