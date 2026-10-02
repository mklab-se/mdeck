//! What extensions read from the theme: its colour tokens and the typed
//! settings of its `engine:` block.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use crate::paint::Color;
use crate::problem::Problem;

/// The theme colours engines, visuals and design sets read. The host fills
/// them from the theme; extensions never see the theme file itself.
///
/// ```
/// use mdeck_sdk::tokens::Tokens;
/// let t = Tokens::default();
/// assert!(!t.light);
/// assert_eq!(t.series.len(), 8);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Tokens {
    /// The slide background.
    pub background: Color,
    /// Running text.
    pub text: Color,
    /// Headings.
    pub heading: Color,
    /// The accent (links, highlights, the brand colour).
    pub accent: Color,
    /// A softer accent for large areas.
    pub accent_soft: Color,
    /// The secondary accent.
    pub secondary: Color,
    /// Muted text (captions, labels).
    pub muted: Color,
    /// Rules and hairlines.
    pub rule: Color,
    /// Code block background.
    pub code_background: Color,
    /// Code text.
    pub code_text: Color,
    /// Good news (a rising KPI).
    pub positive: Color,
    /// Bad news (a falling KPI).
    pub negative: Color,
    /// The chart series colours, in order.
    pub series: [Color; 8],
    /// Whether the background is light (ink on paper rather than light on dark).
    pub light: bool,
    /// The warm light particle-style engines glow with.
    pub particle_light: Color,
    /// The cool light particle-style engines glow with.
    pub particle_cool: Color,
}

impl Default for Tokens {
    /// A neutral dark theme, for tests and previews.
    fn default() -> Self {
        let c = Color::from_rgb;
        Self {
            background: c(12, 12, 14),
            text: c(230, 230, 232),
            heading: c(250, 250, 250),
            accent: c(255, 77, 28),
            accent_soft: c(120, 48, 28),
            secondary: c(255, 176, 74),
            muted: c(140, 140, 146),
            rule: c(48, 48, 54),
            code_background: c(24, 24, 28),
            code_text: c(220, 220, 224),
            positive: c(80, 200, 120),
            negative: c(240, 80, 80),
            series: [
                c(255, 77, 28),
                c(255, 176, 74),
                c(90, 170, 255),
                c(80, 200, 120),
                c(200, 120, 255),
                c(255, 120, 170),
                c(90, 220, 220),
                c(200, 200, 90),
            ],
            light: false,
            particle_light: c(255, 140, 70),
            particle_cool: c(150, 190, 255),
        }
    }
}

impl Tokens {
    /// Series colour `i`, wrapping around after the eighth.
    ///
    /// ```
    /// use mdeck_sdk::tokens::Tokens;
    /// let t = Tokens::default();
    /// assert_eq!(t.series_color(8), t.series[0]);
    /// ```
    pub fn series_color(&self, i: usize) -> Color {
        self.series[i % self.series.len()]
    }
}

/// A settings value as written in a theme: the shape of a YAML value,
/// without any YAML library's types.
///
/// ```
/// use mdeck_sdk::tokens::Value;
/// let v = Value::Map(vec![("glow".into(), Value::Number(0.5))]);
/// assert!(matches!(v, Value::Map(_)));
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// No value (`~` or an empty entry).
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// A number.
    Number(f64),
    /// Text.
    String(String),
    /// A sequence.
    List(Vec<Value>),
    /// A mapping, in the order written.
    Map(Vec<(String, Value)>),
}

impl Value {
    /// What kind of value this is, for messages (`a number`, `text`, ...).
    ///
    /// ```
    /// use mdeck_sdk::tokens::Value;
    /// assert_eq!(Value::Bool(true).kind(), "a boolean");
    /// ```
    pub fn kind(&self) -> &'static str {
        match self {
            Value::Null => "empty",
            Value::Bool(_) => "a boolean",
            Value::Number(_) => "a number",
            Value::String(_) => "text",
            Value::List(_) => "a list",
            Value::Map(_) => "a mapping",
        }
    }
}

/// The theme's `engine:` block as typed settings (ENG-11). The engine reads
/// each setting through a typed getter; a value of the wrong type is
/// recorded as a [`Problem`] and read as absent, and [`EngineSettings::problems`]
/// also reports every key the engine never read (unknown or inert).
///
/// ```
/// use mdeck_sdk::tokens::{EngineSettings, Value};
/// let s = EngineSettings::from_pairs([
///     ("surface", Value::String("slate".into())),
///     ("glow", Value::String("lots".into())),
///     ("sparkle", Value::Bool(true)),
/// ]);
/// assert_eq!(s.one_of("surface", &["sheet", "slate"]), Some("slate"));
/// assert_eq!(s.f32("glow"), None); // not a number: recorded
/// let problems = s.problems();
/// assert_eq!(problems.len(), 2); // `glow` is invalid, `sparkle` was never read
/// ```
#[derive(Debug, Default)]
pub struct EngineSettings {
    entries: BTreeMap<String, Value>,
    line: Option<usize>,
    read: RefCell<BTreeSet<String>>,
    invalid: RefCell<Vec<Problem>>,
}

impl Clone for EngineSettings {
    fn clone(&self) -> Self {
        Self {
            entries: self.entries.clone(),
            line: self.line,
            read: RefCell::new(self.read.borrow().clone()),
            invalid: RefCell::new(self.invalid.borrow().clone()),
        }
    }
}

impl EngineSettings {
    /// No settings.
    ///
    /// ```
    /// assert!(mdeck_sdk::tokens::EngineSettings::new().is_empty());
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Settings from key/value pairs (later keys win).
    ///
    /// See [`EngineSettings`] for an example.
    pub fn from_pairs<K: Into<String>>(pairs: impl IntoIterator<Item = (K, Value)>) -> Self {
        Self {
            entries: pairs.into_iter().map(|(k, v)| (k.into(), v)).collect(),
            ..Self::default()
        }
    }

    /// Settings from the `engine:` block's value. A mapping gives one setting
    /// per key; anything else gives no settings and one problem (except
    /// [`Value::Null`], which is simply empty).
    ///
    /// ```
    /// use mdeck_sdk::tokens::{EngineSettings, Value};
    /// let s = EngineSettings::from_value(Value::Map(vec![("x".into(), Value::Number(1.0))]));
    /// assert_eq!(s.f32("x"), Some(1.0));
    /// let bad = EngineSettings::from_value(Value::Number(3.0));
    /// assert_eq!(bad.problems().len(), 1);
    /// ```
    pub fn from_value(value: Value) -> Self {
        match value {
            Value::Map(pairs) => Self::from_pairs(pairs),
            Value::Null => Self::default(),
            other => {
                let s = Self::default();
                s.invalid.borrow_mut().push(Problem::new(
                    "engine",
                    format!(
                        "the engine settings must be a mapping, not {}",
                        other.kind()
                    ),
                ));
                s
            }
        }
    }

    /// The same settings, with problems reported at `line` (the `engine:`
    /// block's line in the theme file).
    ///
    /// ```
    /// use mdeck_sdk::tokens::{EngineSettings, Value};
    /// let s = EngineSettings::from_pairs([("x", Value::Null)]).at_line(4);
    /// assert_eq!(s.problems()[0].line, Some(4));
    /// ```
    pub fn at_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    /// Whether there are no settings.
    ///
    /// See [`EngineSettings::new`] for an example.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The keys, sorted.
    ///
    /// ```
    /// use mdeck_sdk::tokens::{EngineSettings, Value};
    /// let s = EngineSettings::from_pairs([("b", Value::Null), ("a", Value::Null)]);
    /// assert_eq!(s.keys().collect::<Vec<_>>(), ["a", "b"]);
    /// ```
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    /// The raw value of `key`, marking it read.
    ///
    /// ```
    /// use mdeck_sdk::tokens::{EngineSettings, Value};
    /// let s = EngineSettings::from_pairs([("a", Value::Bool(true))]);
    /// assert_eq!(s.get("a"), Some(&Value::Bool(true)));
    /// ```
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.read.borrow_mut().insert(key.to_owned());
        self.entries.get(key)
    }

    fn invalid(&self, key: &str, expected: &str, got: &Value) {
        let mut p = Problem::new(
            "engine",
            format!("`{key}` should be {expected}, not {}", got.kind()),
        );
        p.line = self.line;
        self.invalid.borrow_mut().push(p);
    }

    /// `key` as text. Numbers and booleans are not text.
    ///
    /// ```
    /// use mdeck_sdk::tokens::{EngineSettings, Value};
    /// let s = EngineSettings::from_pairs([("ink", Value::String("blue".into()))]);
    /// assert_eq!(s.str("ink"), Some("blue"));
    /// ```
    pub fn str(&self, key: &str) -> Option<&str> {
        match self.get(key)? {
            Value::String(s) => Some(s),
            other => {
                self.invalid(key, "text", other);
                None
            }
        }
    }

    /// `key` as a number. Text that parses as a number is accepted.
    ///
    /// ```
    /// use mdeck_sdk::tokens::{EngineSettings, Value};
    /// let s = EngineSettings::from_pairs([("glow", Value::String("0.5".into()))]);
    /// assert_eq!(s.f32("glow"), Some(0.5));
    /// ```
    pub fn f32(&self, key: &str) -> Option<f32> {
        match self.get(key)? {
            Value::Number(n) => Some(*n as f32),
            Value::String(s) if s.trim().parse::<f32>().is_ok() => s.trim().parse().ok(),
            other => {
                self.invalid(key, "a number", other);
                None
            }
        }
    }

    /// `key` as a number, or `default` when absent or invalid.
    ///
    /// ```
    /// use mdeck_sdk::tokens::EngineSettings;
    /// assert_eq!(EngineSettings::new().f32_or("glow", 0.8), 0.8);
    /// ```
    pub fn f32_or(&self, key: &str, default: f32) -> f32 {
        self.f32(key).unwrap_or(default)
    }

    /// `key` as a boolean (`true`/`false`, also `yes`/`no` and `on`/`off` as text).
    ///
    /// ```
    /// use mdeck_sdk::tokens::{EngineSettings, Value};
    /// let s = EngineSettings::from_pairs([("trails", Value::String("off".into()))]);
    /// assert_eq!(s.bool("trails"), Some(false));
    /// ```
    pub fn bool(&self, key: &str) -> Option<bool> {
        match self.get(key)? {
            Value::Bool(b) => Some(*b),
            Value::String(s) => match s.trim().to_ascii_lowercase().as_str() {
                "true" | "yes" | "on" => Some(true),
                "false" | "no" | "off" => Some(false),
                _ => {
                    self.invalid(key, "true or false", &Value::String(s.clone()));
                    None
                }
            },
            other => {
                self.invalid(key, "true or false", other);
                None
            }
        }
    }

    /// `key` as a boolean, or `default` when absent or invalid.
    ///
    /// ```
    /// use mdeck_sdk::tokens::EngineSettings;
    /// assert!(EngineSettings::new().bool_or("trails", true));
    /// ```
    pub fn bool_or(&self, key: &str, default: bool) -> bool {
        self.bool(key).unwrap_or(default)
    }

    /// `key` as a colour written `#rrggbb` (or `#rgb`, `#rrggbbaa`).
    ///
    /// ```
    /// use mdeck_sdk::{paint::Color, tokens::{EngineSettings, Value}};
    /// let s = EngineSettings::from_pairs([("ink", Value::String("#ff0000".into()))]);
    /// assert_eq!(s.color("ink"), Some(Color::from_rgb(255, 0, 0)));
    /// ```
    pub fn color(&self, key: &str) -> Option<Color> {
        match self.get(key)? {
            Value::String(s) => match Color::from_hex(s) {
                Some(c) => Some(c),
                None => {
                    let mut p = Problem::new(
                        "engine",
                        format!("`{key}` should be a colour like #ff4d1c, not `{s}`"),
                    );
                    p.line = self.line;
                    self.invalid.borrow_mut().push(p);
                    None
                }
            },
            other => {
                self.invalid(key, "a colour like #ff4d1c", other);
                None
            }
        }
    }

    /// `key` as one of `options` (case-insensitive), returning the option
    /// as written in `options`.
    ///
    /// ```
    /// use mdeck_sdk::tokens::{EngineSettings, Value};
    /// let s = EngineSettings::from_pairs([("surface", Value::String("Paper".into()))]);
    /// assert_eq!(s.one_of("surface", &["sheet", "slate"]), None);
    /// assert_eq!(s.problems().len(), 1);
    /// ```
    pub fn one_of<'o>(&self, key: &str, options: &[&'o str]) -> Option<&'o str> {
        let value = self.get(key)?;
        if let Value::String(s) = value
            && let Some(o) = options.iter().find(|o| o.eq_ignore_ascii_case(s.trim()))
        {
            return Some(o);
        }
        let shown = match value {
            Value::String(s) => format!("`{s}`"),
            other => other.kind().to_owned(),
        };
        let mut p = Problem::new(
            "engine",
            format!(
                "`{key}` should be one of {}, not {shown}",
                options.join(", ")
            ),
        );
        p.line = self.line;
        self.invalid.borrow_mut().push(p);
        None
    }

    /// Record a problem with `key` that the typed getters cannot see, such as
    /// a number out of range. It is reported by [`EngineSettings::problems`]
    /// with the `engine:` block's line, like a value of the wrong type.
    ///
    /// ```
    /// use mdeck_sdk::tokens::{EngineSettings, Value};
    /// let s = EngineSettings::from_pairs([("glow", Value::Number(5.0))]).at_line(7);
    /// let glow = s.f32_or("glow", 1.0);
    /// if !(0.0..=2.0).contains(&glow) {
    ///     s.report("glow", format!("should be between 0 and 2, not {glow}"));
    /// }
    /// let p = s.problems();
    /// assert_eq!(p[0].message, "`glow` should be between 0 and 2, not 5");
    /// assert_eq!(p[0].line, Some(7));
    /// ```
    pub fn report(&self, key: &str, message: impl AsRef<str>) {
        let mut p = Problem::new("engine", format!("`{key}` {}", message.as_ref()));
        p.line = self.line;
        self.invalid.borrow_mut().push(p);
    }

    /// Everything wrong so far: values of the wrong type that were read, and
    /// every key that was never read (unknown to the engine, or inert).
    /// Call it after the engine has read its settings (after `create`).
    ///
    /// See [`EngineSettings`] for an example.
    pub fn problems(&self) -> Vec<Problem> {
        let mut out = self.invalid.borrow().clone();
        let read = self.read.borrow();
        for key in self.entries.keys().filter(|k| !read.contains(*k)) {
            let mut p = Problem::new("engine", format!("unknown setting `{key}`"));
            p.line = self.line;
            out.push(p);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrong_types_are_recorded_once_per_read_and_read_as_absent() {
        let s = EngineSettings::from_pairs([
            ("n", Value::Bool(true)),
            ("b", Value::Number(1.0)),
            ("t", Value::List(vec![])),
        ]);
        assert_eq!(s.f32("n"), None);
        assert_eq!(s.bool("b"), None);
        assert_eq!(s.str("t"), None);
        let p = s.problems();
        assert_eq!(p.len(), 3, "{p:?}");
        assert!(
            p[0].message
                .contains("`n` should be a number, not a boolean")
        );
    }

    #[test]
    fn absent_keys_are_not_problems() {
        let s = EngineSettings::new();
        assert_eq!(s.f32("glow"), None);
        assert!(s.problems().is_empty());
    }

    #[test]
    fn unread_keys_are_unknown() {
        let s = EngineSettings::from_pairs([("glow", Value::Number(1.0))]).at_line(9);
        let p = s.problems();
        assert_eq!(p[0].message, "unknown setting `glow`");
        assert_eq!(p[0].line, Some(9));
        s.f32("glow");
        assert!(s.problems().is_empty());
    }

    #[test]
    fn series_wraps() {
        let t = Tokens::default();
        assert_eq!(t.series_color(9), t.series[1]);
    }
}
