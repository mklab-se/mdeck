//! The script schema: cast, flows and beats, parsed from YAML or JSON and
//! validated so staging can rely on its structure.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::render::illustration::Library;

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
    pub(super) fn uv(self) -> (f32, f32) {
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
    /// Illustration name (`person`, `laptop`, `server`, ...).
    pub kind: String,
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
        let script: Script = serde_norway::from_str(text).map_err(|e| e.to_string())?;
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
            crate::render::illustration::validate_name(&m.kind)
                .map_err(|e| format!("cast `{}`: kind {e}", m.id))?;
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

    /// Kinds that do not resolve in `lib`, each once, in cast order.
    pub fn unknown_kinds(&self, lib: &mut Library) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for m in &self.cast {
            if !lib.has(&m.kind) && !out.contains(&m.kind) {
                out.push(m.kind.clone());
            }
        }
        out
    }

    /// Number of extra reveal steps the story adds to its slide.
    pub fn extra_steps(&self) -> usize {
        self.beats.len().saturating_sub(1)
    }

    /// Beat on which `id` first appears (0 when no beat shows it).
    pub(super) fn show_step(&self, id: &str) -> usize {
        self.beats
            .iter()
            .position(|b| b.show.iter().any(|s| s == id))
            .unwrap_or(0)
    }

    pub(super) fn hot_step(&self, id: &str) -> Option<usize> {
        self.beats
            .iter()
            .position(|b| b.hot.iter().any(|s| s == id))
    }

    /// The spoken line for a reveal step, if any.
    pub fn line(&self, step: usize) -> Option<&str> {
        self.beats.get(step).and_then(|b| b.say.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::story::fixtures::SAMPLE;

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
        let bad_kind = "cast:\n  - { id: a, kind: 'Big Server', cell: left }\n";
        assert!(Script::parse(bad_kind).unwrap_err().contains("kind"));
    }
}
