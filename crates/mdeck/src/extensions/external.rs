//! External visual programs (EXT-18): a visual drawn by a program in any
//! language, for teams that do not write Rust.
//!
//! The user config maps a fence tag to a command:
//!
//! ```yaml
//! visuals:
//!   plantuml: ~/bin/plantuml-png
//! ```
//!
//! For a ```` ```@plantuml ```` block mdeck runs the command through the
//! shell with a JSON [`Request`] on stdin and reads a PNG from stdout. The
//! PNG is cached next to the deck in `<stem>.assets/visuals/`, keyed by a
//! hash of the command and the request (GEN-03), so presenting and export
//! read the cache: a program runs at most once per block, when the deck
//! opens and its image is missing ([`prepare_deck`]), never per frame.
//!
//! PNG is the one output format. SVG is not accepted: mdeck's SVG
//! rasteriser is built without text support, and the diagrams such
//! programs draw are mostly text.
//!
//! Integration seam for the visual registry: [`lookup`] says whether a tag
//! is an external visual; [`ExternalVisual::cached`] gives its image and
//! [`draw`] paints it into a rect. A fence with an external tag that is not
//! cached yet should draw as its source (the EXT-07 fallback).

#![allow(
    dead_code,
    reason = "integration seam: the visual registry (phase 2b) calls lookup, prepare_deck and draw"
)]

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};
use eframe::egui;
use serde::Serialize;

use crate::config::Config;
use crate::parser;

/// How long a program may take before it is stopped.
pub const TIMEOUT: Duration = Duration::from_secs(30);

/// The pixel density mdeck asks for: twice the 1920x1080 slide units, so the
/// image stays sharp on large and high-density displays.
pub const SCALE: f32 = 2.0;

const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

/// A configured external visual.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalVisual {
    /// The fence tag, with its `@`.
    pub tag: String,
    /// The shell command that draws it.
    pub command: String,
}

/// What the program reads on stdin, as JSON.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Request {
    /// The fence tag, with its `@`.
    pub tag: String,
    /// The fence's content.
    pub source: String,
    /// The theme's colours by token name, as `#rrggbb`.
    pub tokens: BTreeMap<String, String>,
    /// The visual's box on a 1920x1080 slide.
    pub width: u32,
    pub height: u32,
    /// Pixels to draw per slide unit: the PNG should be about
    /// `width * scale` by `height * scale`.
    pub scale: f32,
}

fn normalise(tag: &str) -> String {
    format!("@{}", tag.trim().trim_start_matches('@'))
}

/// The external visual for `tag` in the user config, if one is configured.
pub fn lookup(tag: &str) -> Option<ExternalVisual> {
    lookup_in(&Config::load_or_default(), tag)
}

/// [`lookup`] in a given config.
pub fn lookup_in(config: &Config, tag: &str) -> Option<ExternalVisual> {
    let tag = normalise(tag);
    config
        .visuals
        .as_ref()?
        .iter()
        .find(|(k, _)| normalise(k) == tag)
        .map(|(_, command)| ExternalVisual {
            tag,
            command: command.clone(),
        })
}

/// Every configured external visual, in tag order.
pub fn all(config: &Config) -> Vec<ExternalVisual> {
    config
        .visuals
        .iter()
        .flatten()
        .map(|(tag, command)| ExternalVisual {
            tag: normalise(tag),
            command: command.clone(),
        })
        .collect()
}

/// `<stem>.assets/visuals/` for the deck file `deck`.
pub fn cache_dir(deck: &Path) -> PathBuf {
    crate::assets::manifest::folder_for(deck).join("visuals")
}

/// 64-bit FNV-1a: stable across Rust releases, so caches survive upgrades.
fn fnv(parts: &[&[u8]]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for part in parts {
        for &b in *part {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        // Separate the parts so ("ab", "c") and ("a", "bc") differ.
        h ^= 0xff;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

impl ExternalVisual {
    /// The cache file for `request`: `<tag>-<hash>.png`.
    pub fn cache_path(&self, deck: &Path, request: &Request) -> PathBuf {
        let json = serde_json::to_string(request).unwrap_or_default();
        let hash = fnv(&[self.command.as_bytes(), json.as_bytes()]);
        cache_dir(deck).join(format!(
            "{}-{hash:016x}.png",
            self.tag.trim_start_matches('@')
        ))
    }

    /// The cached image for `request`, if there is one. Presenting uses only
    /// this.
    pub fn cached(&self, deck: &Path, request: &Request) -> Option<PathBuf> {
        let path = self.cache_path(deck, request);
        path.is_file().then_some(path)
    }

    /// The cached image for `request`, running the program first when it is
    /// missing.
    pub fn ensure(&self, deck: &Path, request: &Request, timeout: Duration) -> Result<PathBuf> {
        if let Some(path) = self.cached(deck, request) {
            return Ok(path);
        }
        let png = run(&self.command, request, timeout)?;
        let path = self.cache_path(deck, request);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        std::fs::write(&path, png).with_context(|| format!("writing {}", path.display()))?;
        Ok(path)
    }
}

/// Run `command` through the shell with `request` as JSON on stdin and
/// return the PNG it writes to stdout. Stopped after `timeout`.
pub fn run(command: &str, request: &Request, timeout: Duration) -> Result<Vec<u8>> {
    let json = serde_json::to_vec(request)?;
    let mut child = shell(command)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| anyhow!("could not start `{command}`: {e}"))?;

    // Feed stdin and drain stdout/stderr on threads so a program that
    // writes before it has read everything cannot deadlock.
    let mut stdin = child.stdin.take().expect("piped stdin");
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&json);
    });
    let mut stdout = child.stdout.take().expect("piped stdout");
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        buf
    });
    let mut stderr = child.stderr.take().expect("piped stderr");
    let err_reader = std::thread::spawn(move || {
        let mut buf = String::new();
        let _ = stderr.read_to_string(&mut buf);
        buf
    });

    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if start.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            bail!(
                "`{command}` did not finish within {} s and was stopped",
                timeout.as_secs()
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let _ = writer.join();
    let out = reader.join().unwrap_or_default();
    let err = err_reader.join().unwrap_or_default();
    let err = err.trim();
    if !status.success() {
        bail!(
            "`{command}` failed ({status}){}",
            if err.is_empty() {
                String::new()
            } else {
                format!(": {}", last_lines(err, 5))
            }
        );
    }
    if !out.starts_with(PNG_SIGNATURE) {
        bail!(
            "`{command}` did not write a PNG to stdout ({} bytes{})",
            out.len(),
            if out.starts_with(b"<") {
                ", it looks like SVG or XML: external visuals must write PNG"
            } else {
                ""
            }
        );
    }
    Ok(out)
}

fn last_lines(text: &str, n: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(n)..].join(" / ")
}

#[cfg(unix)]
fn shell(command: &str) -> Command {
    let mut c = Command::new("sh");
    c.arg("-c").arg(command);
    c
}

#[cfg(windows)]
fn shell(command: &str) -> Command {
    let mut c = Command::new("cmd");
    c.arg("/C").arg(command);
    c
}

/// One external visual fence in a deck: its slide (1-based), tag and source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fence {
    pub slide: usize,
    pub tag: String,
    pub source: String,
}

/// Every fence in `presentation` whose tag is one of `visuals`.
pub fn fences(presentation: &parser::Presentation, visuals: &[ExternalVisual]) -> Vec<Fence> {
    let mut out = Vec::new();
    for (i, slide) in presentation.slides.iter().enumerate() {
        let mut tracker = parser::splitter::FenceTracker::new();
        let mut current: Option<Fence> = None;
        for line in slide.raw_source.lines() {
            let was_open = tracker.is_open();
            tracker.observe(line);
            match (was_open, tracker.is_open()) {
                (false, true) => {
                    let info = line.trim().trim_start_matches(['`', '~']).trim_start();
                    let tag = info.split_whitespace().next().unwrap_or("");
                    current = visuals.iter().any(|v| v.tag == tag).then(|| Fence {
                        slide: i + 1,
                        tag: tag.to_string(),
                        source: String::new(),
                    });
                }
                (true, false) => out.extend(current.take()),
                (true, true) => {
                    if let Some(f) = current.as_mut() {
                        f.source.push_str(line);
                        f.source.push('\n');
                    }
                }
                (false, false) => {}
            }
        }
    }
    out
}

/// Run, once, every external visual in the deck whose image is not cached.
/// Called when the deck opens (and by export); `request` builds the request
/// for a fence (the box comes from the design that shows it, the tokens
/// from the theme). Returns one problem per program that failed.
pub fn prepare_deck(
    deck: &Path,
    presentation: &parser::Presentation,
    config: &Config,
    request: impl Fn(&Fence) -> Request,
) -> Vec<String> {
    let visuals = all(config);
    if visuals.is_empty() {
        return Vec::new();
    }
    let mut problems = Vec::new();
    for fence in fences(presentation, &visuals) {
        let Some(visual) = visuals.iter().find(|v| v.tag == fence.tag) else {
            continue;
        };
        if let Err(e) = visual.ensure(deck, &request(&fence), TIMEOUT) {
            problems.push(format!("slide {}: {}: {e:#}", fence.slide, fence.tag));
        }
    }
    problems
}

/// Paint a cached image into `rect`, as large as fits with its aspect kept,
/// centred. Draws nothing while the image is loading.
pub fn draw(
    ui: &egui::Ui,
    images: &crate::render::image_cache::ImageCache,
    path: &Path,
    rect: egui::Rect,
    opacity: f32,
) {
    let Some(texture) = images.get_or_load(ui, &path.to_string_lossy()) else {
        return;
    };
    let size = texture.size_vec2();
    if size.x <= 0.0 || size.y <= 0.0 {
        return;
    }
    let k = (rect.width() / size.x).min(rect.height() / size.y);
    let target = egui::Rect::from_center_size(rect.center(), size * k);
    ui.painter().image(
        texture.id(),
        target,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE.gamma_multiply(opacity),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 1x1 PNG.
    const PNG: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f,
        0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0xf8,
        0xcf, 0xc0, 0xf0, 0x1f, 0x00, 0x05, 0x00, 0x01, 0xff, 0x89, 0x99, 0x3d, 0x1d, 0x00, 0x00,
        0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];

    fn request(source: &str) -> Request {
        Request {
            tag: "@echo".into(),
            source: source.into(),
            tokens: BTreeMap::from([("accent".into(), "#ff4d1c".into())]),
            width: 800,
            height: 450,
            scale: SCALE,
        }
    }

    /// A fixture program: copies its stdin to `log`, then writes the PNG.
    fn fixture(dir: &Path) -> String {
        let png = dir.join("fixture.png");
        std::fs::write(&png, PNG).unwrap();
        let script = dir.join("echo-png.sh");
        std::fs::write(
            &script,
            format!(
                "#!/bin/sh\ncat > '{}'\ncat '{}'\n",
                dir.join("stdin.json").display(),
                png.display()
            ),
        )
        .unwrap();
        format!("sh '{}'", script.display())
    }

    fn config(command: &str) -> Config {
        Config {
            visuals: Some(BTreeMap::from([("echo".into(), command.into())])),
            ..Config::default()
        }
    }

    #[test]
    fn tags_match_with_or_without_the_at() {
        let c = config("x");
        assert_eq!(lookup_in(&c, "@echo").unwrap().tag, "@echo");
        assert_eq!(lookup_in(&c, "echo").unwrap().command, "x");
        assert!(lookup_in(&c, "@bar").is_none());
        assert!(lookup_in(&Config::default(), "@echo").is_none());
    }

    #[cfg(unix)]
    #[test]
    fn a_program_gets_json_and_its_png_is_cached() {
        let dir = crate::extensions::packs::tempdir("ext-visual").unwrap();
        let command = fixture(&dir);
        let deck = dir.join("talk.md");
        let visual = lookup_in(&config(&command), "@echo").unwrap();
        let req = request("a -> b\n");

        assert!(visual.cached(&deck, &req).is_none());
        let path = visual.ensure(&deck, &req, TIMEOUT).unwrap();
        assert!(path.starts_with(dir.join("talk.assets/visuals")));
        assert_eq!(std::fs::read(&path).unwrap(), PNG);

        let sent: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("stdin.json")).unwrap())
                .unwrap();
        assert_eq!(sent["tag"], "@echo");
        assert_eq!(sent["source"], "a -> b\n");
        assert_eq!(sent["tokens"]["accent"], "#ff4d1c");
        assert_eq!(sent["width"], 800);
        assert_eq!(sent["scale"], 2.0);

        // Cached: the program does not run again.
        std::fs::remove_file(dir.join("stdin.json")).unwrap();
        assert_eq!(visual.ensure(&deck, &req, TIMEOUT).unwrap(), path);
        assert!(!dir.join("stdin.json").exists());

        // A different source is a different image.
        assert_ne!(visual.cache_path(&deck, &request("b\n")), path);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn failures_are_explained() {
        let req = request("x");
        let err = run("echo nope >&2; exit 3", &req, TIMEOUT)
            .unwrap_err()
            .to_string();
        assert!(err.contains("failed") && err.contains("nope"), "{err}");

        let err = run("cat > /dev/null; echo '<svg/>'", &req, TIMEOUT)
            .unwrap_err()
            .to_string();
        assert!(err.contains("must write PNG"), "{err}");

        let start = Instant::now();
        let err = run("sleep 5", &req, Duration::from_millis(200))
            .unwrap_err()
            .to_string();
        assert!(err.contains("did not finish"), "{err}");
        assert!(start.elapsed() < Duration::from_secs(3));
    }

    #[cfg(unix)]
    #[test]
    fn prepare_deck_runs_each_missing_block_once() {
        let dir = crate::extensions::packs::tempdir("ext-prepare").unwrap();
        let command = fixture(&dir);
        let deck = dir.join("talk.md");
        let md = "# One\n\n```@echo\nhello\n```\n\n# Two\n\n```@other\nx\n```\n";
        let pres = parser::parse(md);
        let c = config(&command);
        let found = fences(&pres, &all(&c));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].source, "hello\n");
        let problems = prepare_deck(&deck, &pres, &c, |f| request(&f.source));
        assert!(problems.is_empty(), "{problems:?}");
        let cached: Vec<_> = std::fs::read_dir(cache_dir(&deck)).unwrap().collect();
        assert_eq!(cached.len(), 1);

        let broken = config("exit 1");
        let problems = prepare_deck(&dir.join("other.md"), &pres, &broken, |f| {
            request(&f.source)
        });
        assert_eq!(problems.len(), 1);
        assert!(problems[0].starts_with("slide 1: @echo"), "{problems:?}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_hash_is_stable() {
        assert_eq!(fnv(&[b"abc"]), fnv(&[b"abc"]));
        assert_ne!(fnv(&[b"ab", b"c"]), fnv(&[b"a", b"bc"]));
    }
}
