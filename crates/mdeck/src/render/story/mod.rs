//! Story scripts: a small declarative vocabulary that turns a slide into a
//! choreographed scene on the particle field.
//!
//! A script names a **cast** (people and props placed in stage cells), the
//! **flows** between them, and the **beats** the presenter releases with
//! Space. Authors write scripts by hand in a ```@scene fence, or let
//! `mdeck ai story` write them from an English ```@story hint. Either way the
//! model or the author states intent; geometry, colour, motion and label
//! placement are decided here, so a script can never draw off-brand.

pub mod sidecar;
pub mod silhouettes;

use std::collections::HashSet;
use std::sync::Arc;

use eframe::egui::{Pos2, Rect};
use serde::{Deserialize, Serialize};

use crate::parser::Layout;
use crate::render::particles::{Drift, Group, Home, Palette, Scene, Tint};

// ---------------------------------------------------------------------------
// Schema
// ---------------------------------------------------------------------------

/// What a cast member looks like.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Person,
    Hooded,
    Box,
    Orb,
    Doc,
    Docs,
    Inbox,
    Db,
    Cloud,
    Laptop,
    Folder,
    Mail,
    Gate,
}

impl Kind {
    pub const ALL: [Kind; 13] = [
        Kind::Person,
        Kind::Hooded,
        Kind::Box,
        Kind::Orb,
        Kind::Doc,
        Kind::Docs,
        Kind::Inbox,
        Kind::Db,
        Kind::Cloud,
        Kind::Laptop,
        Kind::Folder,
        Kind::Mail,
        Kind::Gate,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Kind::Person => "person",
            Kind::Hooded => "hooded",
            Kind::Box => "box",
            Kind::Orb => "orb",
            Kind::Doc => "doc",
            Kind::Docs => "docs",
            Kind::Inbox => "inbox",
            Kind::Db => "db",
            Kind::Cloud => "cloud",
            Kind::Laptop => "laptop",
            Kind::Folder => "folder",
            Kind::Mail => "mail",
            Kind::Gate => "gate",
        }
    }

    fn is_figure(self) -> bool {
        matches!(self, Kind::Person | Kind::Hooded)
    }
}

/// Where on the stage a cast member stands. The stage is the part of the
/// slide the copy does not use; cells are a 3×3 grid on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Cell {
    LeftTop,
    CenterTop,
    RightTop,
    Left,
    Center,
    Right,
    LeftBottom,
    CenterBottom,
    RightBottom,
}

impl Cell {
    pub const ALL: [Cell; 9] = [
        Cell::LeftTop,
        Cell::CenterTop,
        Cell::RightTop,
        Cell::Left,
        Cell::Center,
        Cell::Right,
        Cell::LeftBottom,
        Cell::CenterBottom,
        Cell::RightBottom,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Cell::LeftTop => "left-top",
            Cell::CenterTop => "center-top",
            Cell::RightTop => "right-top",
            Cell::Left => "left",
            Cell::Center => "center",
            Cell::Right => "right",
            Cell::LeftBottom => "left-bottom",
            Cell::CenterBottom => "center-bottom",
            Cell::RightBottom => "right-bottom",
        }
    }

    /// Centre of the cell as fractions of the stage box.
    fn uv(self) -> (f32, f32) {
        let col = match self {
            Cell::LeftTop | Cell::Left | Cell::LeftBottom => 0.18,
            Cell::CenterTop | Cell::Center | Cell::CenterBottom => 0.50,
            Cell::RightTop | Cell::Right | Cell::RightBottom => 0.82,
        };
        let row = match self {
            Cell::LeftTop | Cell::CenterTop | Cell::RightTop => 0.20,
            Cell::Left | Cell::Center | Cell::Right => 0.50,
            Cell::LeftBottom | Cell::CenterBottom | Cell::RightBottom => 0.80,
        };
        (col, row)
    }
}

/// How a prop is lit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Fill {
    /// Outline only (default).
    Outline,
    /// A pulsing mass inside: something that thinks.
    Brain,
    /// Filled with ember.
    Hot,
    /// Filled with pale light.
    Cold,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FlowColor {
    White,
    Ember,
    Candle,
    Pale,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Member {
    pub id: String,
    pub kind: Kind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub cell: Cell,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<Fill>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Flow {
    pub from: String,
    pub to: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<FlowColor>,
    /// Beat at which the flow starts running (default 0).
    #[serde(default)]
    pub at: usize,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Beat {
    /// Cast members that appear on this beat.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub show: Vec<String>,
    /// Cast members that start glowing hot on this beat.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hot: Vec<String>,
    /// What the presenter says on this beat.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub say: Option<String>,
}

/// A complete story for one slide.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Script {
    #[serde(default)]
    pub cast: Vec<Member>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub flows: Vec<Flow>,
    #[serde(default)]
    pub beats: Vec<Beat>,
}

pub const MAX_CAST: usize = 7;
pub const MAX_BEATS: usize = 6;
pub const MAX_SAY_CHARS: usize = 160;

impl Script {
    /// Parse a script from YAML or JSON (JSON is valid YAML).
    pub fn parse(text: &str) -> Result<Script, String> {
        let script: Script = serde_yaml::from_str(text).map_err(|e| e.to_string())?;
        script.validate()?;
        Ok(script)
    }

    /// Structural checks the renderer relies on.
    pub fn validate(&self) -> Result<(), String> {
        if self.cast.is_empty() {
            return Err("cast is empty".into());
        }
        if self.cast.len() > MAX_CAST {
            return Err(format!(
                "cast has {} members, at most {MAX_CAST} allowed",
                self.cast.len()
            ));
        }
        if self.beats.len() > MAX_BEATS {
            return Err(format!(
                "{} beats, at most {MAX_BEATS} allowed",
                self.beats.len()
            ));
        }
        let mut ids = HashSet::new();
        let mut cells = HashSet::new();
        for m in &self.cast {
            if m.id.trim().is_empty() {
                return Err("a cast member has an empty id".into());
            }
            if !ids.insert(m.id.as_str()) {
                return Err(format!("duplicate cast id `{}`", m.id));
            }
            if !cells.insert(m.cell) {
                return Err(format!(
                    "two cast members share the cell `{}`",
                    m.cell.name()
                ));
            }
        }
        for f in &self.flows {
            for end in [&f.from, &f.to] {
                if !ids.contains(end.as_str()) {
                    return Err(format!("flow references unknown cast id `{end}`"));
                }
            }
            if f.from == f.to {
                return Err(format!("flow from `{}` to itself", f.from));
            }
            if f.at >= self.beats.len().max(1) {
                return Err(format!(
                    "flow starts at beat {}, but there are {} beats",
                    f.at,
                    self.beats.len()
                ));
            }
        }
        for (i, b) in self.beats.iter().enumerate() {
            for id in b.show.iter().chain(b.hot.iter()) {
                if !ids.contains(id.as_str()) {
                    return Err(format!("beat {i} references unknown cast id `{id}`"));
                }
            }
            if let Some(say) = &b.say
                && say.chars().count() > MAX_SAY_CHARS
            {
                return Err(format!(
                    "beat {i} line is longer than {MAX_SAY_CHARS} characters"
                ));
            }
        }
        Ok(())
    }

    /// Number of extra reveal steps the story adds to its slide.
    pub fn extra_steps(&self) -> usize {
        self.beats.len().saturating_sub(1)
    }

    /// Beat on which `id` first appears (0 when no beat shows it).
    fn show_step(&self, id: &str) -> usize {
        self.beats
            .iter()
            .position(|b| b.show.iter().any(|s| s == id))
            .unwrap_or(0)
    }

    fn hot_step(&self, id: &str) -> Option<usize> {
        self.beats
            .iter()
            .position(|b| b.hot.iter().any(|s| s == id))
    }

    /// The spoken line for a reveal step, if any.
    pub fn line(&self, step: usize) -> Option<&str> {
        self.beats.get(step).and_then(|b| b.say.as_deref())
    }
}

// ---------------------------------------------------------------------------
// Staging
// ---------------------------------------------------------------------------

/// A label to draw over the field, in slide fractions, tied to a group so it
/// fades with it.
#[derive(Clone, Debug)]
pub struct Label {
    pub text: String,
    /// Anchor (top-centre of the label) as slide fractions.
    pub u: f32,
    pub v: f32,
    pub group: usize,
    pub figure: bool,
}

/// A staged script: the particle scene plus the labels to draw over it.
#[derive(Clone, Debug)]
pub struct Staged {
    pub scene: Scene,
    pub labels: Vec<Label>,
}

/// Whether a story can play on this slide at all. A story needs a stage:
/// the right half of a slide whose copy Ember lays out on the left (bullets,
/// content, quotes, section dividers). Slides whose content fills the frame
/// (code, charts, diagrams, tables, images, two columns) and title slides
/// (centred copy) never play a story; their field stays content-aware and
/// quiet, whatever a sidecar says.
pub fn allowed(slide: &crate::parser::Slide, index: usize) -> bool {
    crate::render::ember::handles(slide) && !crate::render::ember::is_title(slide, index)
}

/// The part of the slide the story plays on, as slide fractions.
pub fn stage_box(layout: Layout) -> Rect {
    match layout {
        // Quotes run wider, so the stage is narrower.
        Layout::Quote => Rect::from_min_max(Pos2::new(0.64, 0.10), Pos2::new(0.96, 0.90)),
        // Copy sits left; the stage is the right half.
        _ => Rect::from_min_max(Pos2::new(0.52, 0.10), Pos2::new(0.96, 0.90)),
    }
}

/// Geometry of a staged cast member, as slide fractions.
struct Placed<'a> {
    member: &'a Member,
    /// Centre.
    u: f32,
    v: f32,
    /// Half extents.
    hw: f32,
    hh: f32,
}

impl Placed<'_> {
    /// Point on this member's bounding box edge in the direction of `(tu, tv)`.
    fn edge_toward(&self, tu: f32, tv: f32, aspect: f32) -> [f32; 2] {
        let dx = tu - self.u;
        let dy = (tv - self.v) / aspect;
        let len = (dx * dx + dy * dy).sqrt().max(1e-4);
        let (nx, ny) = (dx / len, dy / len);
        // scale to the box edge (ellipse approximation, slightly outside)
        let k = 1.0
            / ((nx / (self.hw * 1.15)).powi(2) + (ny / (self.hh * 1.15 / aspect)).powi(2)).sqrt();
        [self.u + nx * k, self.v + ny * k * aspect]
    }
}

/// Turn a script into a particle scene for a slide of `layout`, drawn in a
/// rect with the given width/height `aspect`.
pub fn stage(script: &Script, layout: Layout, aspect: f32) -> Staged {
    let stage = stage_box(layout);
    let sw = stage.width();
    let sh = stage.height();
    // sizes as fractions of the slide width; heights are corrected by aspect
    let fig_w = 0.092;
    let prop_w = 0.096;
    // no member may be taller than this (fraction of slide height), so a
    // 3×3 grid with labels never collides
    let max_hh = 0.105;

    let placed: Vec<Placed> = script
        .cast
        .iter()
        .map(|m| {
            let (cu, cv) = m.cell.uv();
            let w = if m.kind.is_figure() { fig_w } else { prop_w };
            let ratio = silhouettes::aspect(m.kind); // height / width in a square space
            let mut hw = w / 2.0;
            let mut hh = w * ratio / 2.0 * aspect;
            if hh > max_hh {
                let k = max_hh / hh;
                hh = max_hh;
                hw *= k;
            }
            Placed {
                member: m,
                u: stage.left() + cu * sw,
                v: stage.top() + cv * sh,
                hw,
                hh,
            }
        })
        .collect();

    let cast_n = placed.len().max(1) as f32;
    let mut groups: Vec<Group> = Vec::new();
    let mut labels: Vec<Label> = Vec::new();

    for p in &placed {
        let m = p.member;
        let points: Arc<Vec<[f32; 2]>> = silhouettes::outline(m.kind);
        let step = script.show_step(&m.id);
        let mut g = Group::new(
            0.60 / cast_n,
            Home::Mask {
                points,
                u: p.u - p.hw,
                v: p.v - p.hh,
                w: p.hw * 2.0,
                h: p.hh * 2.0,
            },
        )
        .drift(Drift::Breathe {
            amp: 0.0015,
            speed: 0.6,
        })
        .size(0.42, 0.78)
        .dim(0.0)
        .step(step);
        g = match m.kind {
            Kind::Hooded => g.palette(Palette::Solid(Tint::Pale)).alpha(0.45, 0.9),
            Kind::Person => g.palette(Palette::Site).alpha(0.6, 1.0),
            _ => g.palette(Palette::Cold).alpha(0.5, 0.95),
        };
        if let Some(h) = script.hot_step(&m.id) {
            g = g.hot(h);
        }
        labels.push(Label {
            text: m.label.clone().unwrap_or_else(|| m.id.clone()),
            u: p.u,
            v: p.v + p.hh + 0.012 * aspect,
            group: groups.len(),
            figure: m.kind.is_figure(),
        });
        groups.push(g);

        // Fills
        if let Some(fill) = m.fill.filter(|f| *f != Fill::Outline) {
            let inner = Group::new(
                0.10 / cast_n,
                Home::Cluster {
                    u: p.u,
                    v: p.v,
                    r: p.hw * 0.55 / aspect.max(0.01) * aspect,
                    falloff: 0.7,
                },
            )
            .size(0.25, 0.5)
            .dim(0.0)
            .step(step);
            let inner = match fill {
                Fill::Brain => inner
                    .palette(Palette::Warm)
                    .alpha(0.25, 0.8)
                    .drift(Drift::Breathe {
                        amp: 0.006,
                        speed: 2.2,
                    })
                    .links(2),
                Fill::Hot => inner.palette(Palette::Solid(Tint::Ember)).alpha(0.2, 0.6),
                Fill::Cold => inner.palette(Palette::Cold).alpha(0.15, 0.5),
                Fill::Outline => inner,
            };
            let inner = match script.hot_step(&m.id) {
                Some(h) => inner.hot(h),
                None => inner,
            };
            groups.push(inner);
        }
    }

    // Flows: bezier from the edge of `from` to the edge of `to`.
    for (fi, f) in script.flows.iter().enumerate() {
        let (Some(a), Some(b)) = (
            placed.iter().find(|p| p.member.id == f.from),
            placed.iter().find(|p| p.member.id == f.to),
        ) else {
            continue;
        };
        let p0 = a.edge_toward(b.u, b.v, aspect);
        let p1 = b.edge_toward(a.u, a.v, aspect);
        let points = bezier_points(p0, p1, if fi % 2 == 0 { 0.10 } else { -0.10 }, aspect);
        // Runners per flow scale with its length so a short hop is a few
        // sparks and a long one is a stream, never a pile.
        let length: f32 = points
            .windows(2)
            .map(|w| ((w[1][0] - w[0][0]).powi(2) + ((w[1][1] - w[0][1]) / aspect).powi(2)).sqrt())
            .sum();
        let share = (0.012 + length * 0.22).min(0.08);
        let tint = match f.color.unwrap_or(FlowColor::White) {
            FlowColor::White => Tint::White,
            FlowColor::Ember => Tint::Ember,
            FlowColor::Candle => Tint::Candle,
            FlowColor::Pale => Tint::Pale,
        };
        groups.push(
            Group::new(
                share,
                Home::Path {
                    points,
                    spread: 0.006,
                },
            )
            .palette(Palette::Solid(tint))
            .alpha(0.35, 0.9)
            .size(0.35, 0.7)
            .dim(0.0)
            .step(f.at)
            .drift(Drift::Flow { speed: 1.0 }),
        );
    }

    // The rest is quiet dust so the stage never sits in dead black.
    let used: f32 = groups.iter().map(|g| g.share).sum();
    groups.push(
        Group::new(
            (1.0 - used).max(0.12),
            Home::Field {
                u0: 0.02,
                v0: 0.04,
                u1: 0.98,
                v1: 0.96,
            },
        )
        .alpha(0.04, 0.16)
        .size(0.4, 0.8),
    );

    let mut scene = Scene::new(groups);
    scene.link_alpha = 0.10;
    Staged { scene, labels }
}

/// A cubic bezier from `p0` to `p1` bent sideways by `bend` (fraction of the
/// chord), sampled into a polyline in slide fractions.
fn bezier_points(p0: [f32; 2], p1: [f32; 2], bend: f32, aspect: f32) -> Vec<[f32; 2]> {
    let dx = p1[0] - p0[0];
    let dy = (p1[1] - p0[1]) / aspect;
    let dist = (dx * dx + dy * dy).sqrt().max(1e-4);
    let (nx, ny) = (-dy / dist, dx / dist);
    let b = dist * bend;
    let c0 = [
        p0[0] + dx * 0.3 + nx * b * 0.5,
        p0[1] + (dy * 0.3 + ny * b * 0.5) * aspect,
    ];
    let c1 = [
        p0[0] + dx * 0.72 + nx * b,
        p0[1] + (dy * 0.72 + ny * b) * aspect,
    ];
    (0..=24)
        .map(|i| {
            let t = i as f32 / 24.0;
            let u = 1.0 - t;
            [
                u * u * u * p0[0]
                    + 3.0 * u * u * t * c0[0]
                    + 3.0 * u * t * t * c1[0]
                    + t * t * t * p1[0],
                u * u * u * p0[1]
                    + 3.0 * u * u * t * c0[1]
                    + 3.0 * u * t * t * c1[1]
                    + t * t * t * p1[1],
            ]
        })
        .collect()
}

/// Draw the cast labels over the field: tracked mono, fading with each
/// member's group and warming when it runs hot.
pub fn draw_labels(
    painter: &eframe::egui::Painter,
    labels: &[Label],
    field: &crate::render::particles::Field,
    rect: Rect,
    theme: &crate::theme::Theme,
    scale: f32,
    opacity: f32,
) {
    use eframe::egui::{Color32, FontId, text::LayoutJob, text::TextFormat};
    let size = 13.0 * scale;
    for l in labels {
        let life = field.group_life(l.group);
        if life < 0.02 {
            continue;
        }
        let heat = field.group_heat(l.group);
        let base = if l.figure {
            Color32::from_rgb(0xD6, 0xD6, 0xDB)
        } else {
            Color32::from_rgb(0x8F, 0x8F, 0x98)
        };
        let ember = Color32::from_rgb(0xFF, 0x8A, 0x66);
        let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * heat) as u8;
        let color = Color32::from_rgba_unmultiplied(
            mix(base.r(), ember.r()),
            mix(base.g(), ember.g()),
            mix(base.b(), ember.b()),
            (life * opacity * 255.0) as u8,
        );
        let mut job = LayoutJob::default();
        job.append(
            &l.text.to_uppercase(),
            0.0,
            TextFormat {
                font_id: FontId::new(size, theme.mono_family()),
                color,
                extra_letter_spacing: size * 0.18,
                ..Default::default()
            },
        );
        let galley = painter.layout_job(job);
        let x = rect.left() + l.u * rect.width() - galley.rect.width() / 2.0;
        let y = rect.top() + l.v * rect.height();
        painter.galley(Pos2::new(x, y), galley, color);
    }
}

/// Human-readable vocabulary, embedded in the AI prompt and the spec.
pub fn vocabulary() -> String {
    let kinds: Vec<&str> = Kind::ALL.iter().map(|k| k.name()).collect();
    let cells: Vec<&str> = Cell::ALL.iter().map(|c| c.name()).collect();
    format!(
        "kinds: {}\ncells: {}\nfills: outline, brain, hot, cold\nflow colors: white, ember, candle, pale",
        kinds.join(", "),
        cells.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
cast:
  - { id: anders, kind: person, label: Anders, cell: left }
  - { id: queue, kind: inbox, label: Support queue, cell: center-top }
  - { id: model, kind: orb, label: The model, cell: right, fill: brain }
flows:
  - { from: queue, to: model, color: white, at: 1 }
  - { from: model, to: anders, color: ember, at: 2 }
beats:
  - { show: [anders, queue], say: "Anders stopped reading the tickets." }
  - { show: [model], say: "He pointed the assistant at the queue." }
  - { hot: [model], say: "Nobody noticed what came back." }
"#;

    #[test]
    fn parses_yaml_and_json() {
        let s = Script::parse(SAMPLE).unwrap();
        assert_eq!(s.cast.len(), 3);
        assert_eq!(s.extra_steps(), 2);
        assert_eq!(s.show_step("model"), 1);
        assert_eq!(s.hot_step("model"), Some(2));
        assert_eq!(s.line(0), Some("Anders stopped reading the tickets."));
        let json = r#"{"cast":[{"id":"a","kind":"person","cell":"left"}],
            "flows":[],"beats":[{"show":["a"],"say":"hi"},{"hot":["a"]}]}"#;
        let again = Script::parse(json).unwrap();
        assert_eq!(again.beats.len(), 2);
        assert_eq!(again.hot_step("a"), Some(1));
    }

    #[test]
    fn validation_catches_bad_references_and_cells() {
        let bad = "cast:\n  - { id: a, kind: person, cell: left }\nflows:\n  - { from: a, to: zz }\nbeats: []\n";
        assert!(Script::parse(bad).unwrap_err().contains("unknown cast id"));
        let dup = "cast:\n  - { id: a, kind: person, cell: left }\n  - { id: b, kind: box, cell: left }\n";
        assert!(Script::parse(dup).unwrap_err().contains("share the cell"));
        assert!(Script::parse("cast: []\n").is_err());
    }

    #[test]
    fn staging_makes_a_group_per_member_plus_flows_and_labels() {
        let s = Script::parse(SAMPLE).unwrap();
        let staged = stage(&s, Layout::Bullet, 16.0 / 9.0);
        assert_eq!(staged.labels.len(), 3);
        // 3 outlines + 1 brain fill + 2 flows + dust
        assert_eq!(staged.scene.groups.len(), 7);
        let model_group = staged
            .labels
            .iter()
            .find(|l| l.text == "The model")
            .unwrap()
            .group;
        assert_eq!(staged.scene.groups[model_group].step, Some(1));
        assert_eq!(staged.scene.groups[model_group].hot_step, Some(2));
        // every label sits inside the stage
        let sb = stage_box(Layout::Bullet);
        for l in &staged.labels {
            assert!(l.u > sb.left() && l.u < sb.right(), "{}", l.text);
        }
    }
}
