//! The deck's thermal sources: read once when the deck opens (so step
//! counts and diagnostics are known up front and export matches the
//! window), with the composed pictures cached per look.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use eframe::egui;

use super::compose::{Look, compose};
use super::source::{self, Kind, Reading, Source};
use super::spec::{Action, Spec, Support, Threshold};
use crate::parser::{Block, Chart, Presentation, Slide};

/// Something worth telling the author about a block, with the deck file
/// line it is on.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    /// 1-based slide.
    pub slide: usize,
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "slide {} (line {}): {}",
            self.slide, self.line, self.message
        )
    }
}

type Loaded = Result<Arc<Source>, String>;

#[derive(Default)]
pub struct Library {
    base: PathBuf,
    sources: HashMap<String, Loaded>,
    textures: RefCell<HashMap<(String, LookKey), egui::TextureHandle>>,
}

type LookKey = (super::Palette, u32, u32, Option<u32>);

/// How `spec`'s source is read, and the key it is cached under.
fn reading(spec: &Spec) -> Option<(String, Reading)> {
    if let Some(d) = &spec.data {
        return Some((d.clone(), Reading::Data));
    }
    let image = spec.image.clone()?;
    Some((
        image,
        Reading::Image {
            polarity: spec.polarity,
            mapping: spec.mapping.clone(),
        },
    ))
}

fn cache_key(path: &str, reading: &Reading) -> String {
    format!("{path}|{reading:?}")
}

/// Every `@thermal` block of a slide, with the deck file line of its fence.
pub fn blocks(slide: &Slide) -> Vec<(&str, usize)> {
    let mut fences = slide
        .raw_source
        .lines()
        .enumerate()
        .filter(|(_, l)| {
            let t = l.trim_start();
            (t.starts_with("```") || t.starts_with("~~~"))
                && t.trim_start_matches(['`', '~'])
                    .trim_start()
                    .starts_with("@thermal")
        })
        .map(|(i, _)| slide.line_at(i));
    slide
        .blocks
        .iter()
        .filter_map(|b| match b {
            Block::Chart {
                kind: Chart::Thermal,
                content,
                ..
            } => Some((content.as_str(), fences.next().unwrap_or(slide.line))),
            _ => None,
        })
        .collect()
}

impl Library {
    pub fn new(base: PathBuf) -> Self {
        Self {
            base,
            ..Default::default()
        }
    }

    pub fn clear(&mut self) {
        self.sources.clear();
        self.textures.get_mut().clear();
    }

    /// Read every source the deck's `@thermal` blocks use, and return what
    /// the author should know (unreadable files, colour input, settings the
    /// source cannot honour).
    pub fn load(&mut self, presentation: &Presentation) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        for (i, slide) in presentation.slides.iter().enumerate() {
            for (content, fence) in blocks(slide) {
                let spec = Spec::parse(content);
                if let Some((path, reading)) = reading(&spec) {
                    let key = cache_key(&path, &reading);
                    if !self.sources.contains_key(&key) {
                        let loaded = source::load(&resolve(&self.base, &path), &reading)
                            .map(Arc::new)
                            .map_err(|e| format!("{e:#}"));
                        self.sources.insert(key, loaded);
                    }
                }
                for (offset, message) in self.diagnose(&spec) {
                    out.push(Diagnostic {
                        slide: i + 1,
                        line: fence + 1 + offset,
                        message,
                    });
                }
            }
        }
        out
    }

    /// The source a block shows: `None` before [`Library::load`] saw it.
    pub fn source(&self, spec: &Spec) -> Option<Result<&Arc<Source>, &str>> {
        let (path, reading) = reading(spec)?;
        self.sources
            .get(&cache_key(&path, &reading))
            .map(|r| r.as_ref().map_err(String::as_str))
    }

    /// What the block's source can do, which decides its steps.
    pub fn support(&self, spec: &Spec) -> Support {
        match self.source(spec) {
            Some(Ok(s)) => support_of(s),
            _ => Support::DISPLAY,
        }
    }

    /// The composed picture of `spec`'s source in `look`, cached.
    pub fn texture(
        &self,
        ctx: &egui::Context,
        spec: &Spec,
        source: &Source,
        look: &Look,
    ) -> Option<egui::TextureHandle> {
        let (path, reading) = reading(spec)?;
        let key = (cache_key(&path, &reading), look.key());
        if let Some(t) = self.textures.borrow().get(&key) {
            return Some(t.clone());
        }
        let image = compose(source, look);
        let tex = ctx.load_texture(
            format!("thermal:{}:{:?}", key.0, key.1),
            image,
            egui::TextureOptions {
                magnification: egui::TextureFilter::Linear,
                minification: egui::TextureFilter::Linear,
                wrap_mode: egui::TextureWrapMode::ClampToEdge,
                mipmap_mode: Some(egui::TextureFilter::Linear),
            },
        );
        self.textures.borrow_mut().insert(key, tex.clone());
        Some(tex)
    }

    /// What the author should know about `spec` given its source, each with
    /// its line in the block.
    pub fn diagnose(&self, spec: &Spec) -> Vec<(usize, String)> {
        let mut out: Vec<(usize, String)> = spec
            .problems
            .iter()
            .map(|p| (p.offset, p.message.clone()))
            .collect();
        let line_of = |key: &str| spec.key_line(key).unwrap_or(0);
        let source = match self.source(spec) {
            Some(Ok(s)) => s,
            Some(Err(e)) => {
                out.push((0, format!("@thermal: {e}")));
                return out;
            }
            None => return out,
        };
        if source.chromatic {
            let left_out = spec
                .lines
                .iter()
                .filter(|l| !l.action.supported(&support_of(source)))
                .count();
            let mut msg = format!(
                "@thermal: unsupported chromatic input: {} has colour in it, so it is shown as it is, without palette, legend or threshold reveal",
                spec.image.as_deref().unwrap_or("the image")
            );
            if left_out > 0 {
                msg.push_str(&format!(
                    " ({left_out} threshold step{} left out)",
                    if left_out == 1 { "" } else { "s" }
                ));
            }
            msg.push_str("; export it from the camera software as grayscale, white-hot");
            out.push((line_of("image"), msg));
            return out;
        }
        if source.kind == Kind::Display && spec.slide_window {
            out.push((
                0,
                "@thermal: the slide's thermal-window compares sources on one scale, but this image has no mapping: or data:; it is shown by relative intensity and cannot be compared".into(),
            ));
        } else if source.kind == Kind::Display && spec.window.is_some() {
            out.push((
                line_of("window"),
                "@thermal: window: needs a mapping: or a data: file, so it is not used; the image is shown by relative intensity".into(),
            ));
        }
        if let (Some(w), Some(unit)) = (&spec.window, source.unit())
            && w.to_unit(unit).is_none()
        {
            out.push((
                line_of("window"),
                format!(
                    "@thermal: a window in {} cannot apply to a source in {}, so it is not used",
                    w.unit.symbol(),
                    unit.symbol()
                ),
            ));
        }
        if spec.data.is_some() && (spec.mapping.is_some() || spec.polarity != Default::default()) {
            out.push((
                line_of("mapping").max(line_of("polarity")),
                "@thermal: mapping: and polarity: do not apply to a data: file (its sidecar says what the values are)".into(),
            ));
        }
        let support = support_of(source);
        for l in &spec.lines {
            match &l.action {
                Action::Above(Threshold::Value(v, unit)) if !l.action.supported(&support) => {
                    let why = match &support.unit {
                        None => "the image has no mapping: or data:, so its gray levels are not temperatures".to_string(),
                        Some(u) => format!("the source is in {}", u.symbol()),
                    };
                    out.push((
                        l.offset,
                        format!(
                            "@thermal: above {v} {} is left out: {why}; use a share like above 85%",
                            unit.symbol()
                        ),
                    ));
                }
                Action::Spot {
                    name,
                    text: Some(text),
                    ..
                } if source.kind == Kind::Display && looks_measured(text) => {
                    out.push((
                        l.offset,
                        format!(
                            "@thermal: spot {name} shows '{text}', which the image cannot measure; it is drawn as an author-supplied value (†)"
                        ),
                    ));
                }
                _ => {}
            }
        }
        out
    }
}

/// What `source` can do.
pub fn support_of(source: &Source) -> Support {
    Support {
        palette: !source.chromatic,
        unit: source.unit().cloned(),
    }
}

/// A spot text that reads like a measurement (a number with a unit).
fn looks_measured(text: &str) -> bool {
    text.chars().any(|c| c.is_ascii_digit())
        && ["°", " K", "K ", "deg"]
            .iter()
            .any(|u| text.contains(u) || text.ends_with('K'))
}

fn resolve(base: &Path, path: &str) -> PathBuf {
    if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        base.join(path)
    }
}
