//! A `@thermal` block as written: its source (`image:` or `data:`), the
//! optional visible photo, palette, polarity, mapping and window, and the
//! steps that reveal it (`lens`, `reveal`, `above`, `spot`).

use super::palette::Palette;
pub use super::units::{Range, Unit};
use crate::render::visualizations::{VizReveal, assign_steps, parse_reveal_prefix};

/// Which way the source runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Polarity {
    /// Brighter is hotter (the baseline).
    #[default]
    WhiteHot,
    /// Darker is hotter: inverted before the palette and thresholds.
    BlackHot,
}

/// A threshold for the reveal: a share of the intensity range (display
/// images) or a value in a unit (mapped or data sources).
#[derive(Debug, Clone, PartialEq)]
pub enum Threshold {
    Relative(f32),
    Value(f32, Unit),
}

/// One revealing line of a block.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// The lens over the visible photo: centre and radius as fractions of
    /// the image (radius of its width).
    Lens { x: f32, y: f32, r: f32 },
    /// The thermal image fills its frame (the lens opens all the way).
    Reveal,
    /// Colour only what is above the threshold; the rest goes gray.
    Above(Threshold),
    /// A marked point, its label, and an optional text the author supplies.
    Spot {
        name: String,
        x: f32,
        y: f32,
        text: Option<String>,
    },
}

/// What a block's source can do, which decides the steps it keeps.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Support {
    /// The source takes a palette (not a colour image).
    pub palette: bool,
    /// The unit of the source's values, when it has one (mapped or data).
    pub unit: Option<Unit>,
}

impl Support {
    /// Everything a display image can do (the parser's default count).
    pub const DISPLAY: Support = Support {
        palette: true,
        unit: None,
    };
}

impl Action {
    /// Whether the source can show this step. Steps it cannot show are left
    /// out of the step count, so no click does nothing.
    pub fn supported(&self, support: &Support) -> bool {
        match self {
            Action::Above(Threshold::Relative(_)) => support.palette,
            Action::Above(Threshold::Value(_, unit)) => {
                support.palette
                    && support
                        .unit
                        .as_ref()
                        .is_some_and(|u| unit.convert(0.0, u).is_some())
            }
            _ => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub action: Action,
    pub reveal: VizReveal,
    /// 0-based line within the block.
    pub offset: usize,
}

/// Something in the block that does not parse, with its line in the block.
#[derive(Debug, Clone, PartialEq)]
pub struct Problem {
    pub offset: usize,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Spec {
    pub image: Option<String>,
    pub data: Option<String>,
    pub visible: Option<String>,
    pub palette: Option<Palette>,
    pub polarity: Polarity,
    /// `mapping: linear 18..92 °C`.
    pub mapping: Option<Range>,
    pub window: Option<Range>,
    /// The window comes from the slide's `@thermal-window` (a comparison).
    pub slide_window: bool,
    pub label: Option<String>,
    pub lines: Vec<Line>,
    pub problems: Vec<Problem>,
    /// The line each key was written on.
    pub keys: std::collections::BTreeMap<String, usize>,
}

impl Spec {
    pub fn parse(content: &str) -> Spec {
        let mut spec = Spec::default();
        for (offset, raw) in content.lines().enumerate() {
            let t = raw.trim();
            if t.is_empty() {
                continue;
            }
            let problem = |message: String| spec_problem(offset, message);
            // keys may be written `key: value` or `# key: value`
            let keyed = t.strip_prefix('#').map_or(t, |r| r.trim_start());
            let is_step = t.starts_with(['-', '+', '*']);
            if !is_step {
                let Some((key, value)) = keyed.split_once(':') else {
                    if !t.starts_with('#') {
                        spec.problems
                            .push(problem(format!("'{t}' is not a key or a step")));
                    }
                    continue;
                };
                let value = value.trim();
                let key = key.trim().to_ascii_lowercase();
                spec.keys.insert(key.clone(), offset);
                match key.as_str() {
                    "image" => spec.image = Some(value.to_string()),
                    "data" => spec.data = Some(value.to_string()),
                    "visible" => spec.visible = Some(value.to_string()),
                    "label" => spec.label = Some(value.to_string()),
                    "palette" => match Palette::from_name(value) {
                        Some(p) => spec.palette = Some(p),
                        None => spec.problems.push(problem(format!(
                            "palette: '{value}' is not one of iron, white-hot, black-hot, rainbow, arctic, lava"
                        ))),
                    },
                    "polarity" => match value.to_ascii_lowercase().as_str() {
                        "white-hot" => spec.polarity = Polarity::WhiteHot,
                        "black-hot" => spec.polarity = Polarity::BlackHot,
                        _ => spec.problems.push(problem(format!(
                            "polarity: '{value}' is not white-hot or black-hot"
                        ))),
                    },
                    "mapping" => {
                        let rest = value.strip_prefix("linear").map(str::trim);
                        match rest.map(Range::parse) {
                            Some(Ok(r)) => spec.mapping = Some(r),
                            Some(Err(e)) => spec.problems.push(problem(format!("mapping: {e}"))),
                            None => spec.problems.push(problem(format!(
                                "mapping: '{value}' must be linear, e.g. linear 18..92 °C"
                            ))),
                        }
                    }
                    "window" if !spec.slide_window => match Range::parse(value) {
                        Ok(r) => spec.window = Some(r),
                        Err(e) => spec.problems.push(problem(format!("window: {e}"))),
                    },
                    "window" => {}
                    // added by the parser from the slide's @thermal-window
                    "slide-window" => match Range::parse(value) {
                        Ok(r) => {
                            spec.window = Some(r);
                            spec.slide_window = true;
                        }
                        Err(e) => spec
                            .problems
                            .push(problem(format!("@thermal-window: {e}"))),
                    },
                    other if t.starts_with('#') => {
                        let _ = other; // a comment that happens to have a colon
                    }
                    other => spec
                        .problems
                        .push(problem(format!("'{other}' is not a @thermal key"))),
                }
                continue;
            }
            let (text, reveal) = parse_reveal_prefix(t);
            match parse_action(text) {
                Ok(action) => spec.lines.push(Line {
                    action,
                    reveal,
                    offset,
                }),
                Err(e) => spec.problems.push(problem(e)),
            }
        }
        if spec.image.is_some() && spec.data.is_some() {
            spec.problems
                .push(spec_problem(0, "use image: or data:, not both".to_string()));
        }
        if spec.image.is_none() && spec.data.is_none() {
            spec.problems
                .push(spec_problem(0, "needs an image: or data: line".to_string()));
        }
        spec
    }

    /// The lines the source can show and their reveal steps; the others are
    /// left out (see [`Action::supported`]).
    pub fn steps(&self, support: &Support) -> Vec<(&Line, usize)> {
        let kept: Vec<&Line> = self
            .lines
            .iter()
            .filter(|l| l.action.supported(support))
            .collect();
        let reveals: Vec<VizReveal> = kept.iter().map(|l| l.reveal).collect();
        kept.into_iter().zip(assign_steps(&reveals)).collect()
    }

    /// How many reveal steps the block adds.
    pub fn step_count(&self, support: &Support) -> usize {
        self.steps(support)
            .iter()
            .map(|(_, s)| *s)
            .max()
            .unwrap_or(0)
    }

    /// The block line of a key, if it was written.
    pub fn key_line(&self, key: &str) -> Option<usize> {
        self.keys.get(key).copied()
    }
}

fn spec_problem(offset: usize, message: String) -> Problem {
    Problem { offset, message }
}

/// `76%`, `0.76` → 0.76 (a fraction of the image).
fn fraction(s: &str) -> Result<f32, String> {
    let t = s.trim();
    let v = match t.strip_suffix('%') {
        Some(p) => p.trim().parse::<f32>().map(|v| v / 100.0),
        None => t.parse::<f32>(),
    }
    .map_err(|_| format!("'{t}' is not a position like 40%"))?;
    if (0.0..=1.0).contains(&v) {
        Ok(v)
    } else {
        Err(format!("'{t}' must be between 0% and 100%"))
    }
}

fn parse_action(text: &str) -> Result<Action, String> {
    let (word, rest) = text
        .split_once(char::is_whitespace)
        .map_or((text, ""), |(w, r)| (w, r.trim()));
    match word.to_ascii_lowercase().as_str() {
        "reveal" => Ok(Action::Reveal),
        "lens" => {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() != 3 {
                return Err(format!(
                    "lens needs x, y and a radius, e.g. lens 76% 43% 18% (got '{rest}')"
                ));
            }
            Ok(Action::Lens {
                x: fraction(parts[0])?,
                y: fraction(parts[1])?,
                r: fraction(parts[2])?.max(0.01),
            })
        }
        "above" => {
            let r = rest.trim();
            if let Some(p) = r.strip_suffix('%') {
                let v = p
                    .trim()
                    .parse::<f32>()
                    .map_err(|_| format!("above: '{r}' is not a share like 85%"))?;
                if !(0.0..=100.0).contains(&v) {
                    return Err(format!("above: '{r}' must be between 0% and 100%"));
                }
                return Ok(Action::Above(Threshold::Relative(v / 100.0)));
            }
            let split = r
                .char_indices()
                .find(|&(i, c)| {
                    !(c.is_ascii_digit() || c == '.' || (i == 0 && (c == '-' || c == '+')))
                })
                .map_or(r.len(), |(i, _)| i);
            let (n, unit) = r.split_at(split);
            let v = n
                .trim()
                .parse::<f32>()
                .map_err(|_| format!("above: '{r}' is not 85% or a value like 60 °C"))?;
            let unit = Unit::parse(unit)
                .ok_or_else(|| format!("above: '{r}' needs % or a unit, e.g. 60 °C"))?;
            Ok(Action::Above(Threshold::Value(v, unit)))
        }
        "spot" => {
            let (head, text) = match rest.split_once(':') {
                Some((h, t)) => (h, Some(t.trim().to_string()).filter(|t| !t.is_empty())),
                None => (rest, None),
            };
            let parts: Vec<&str> = head.split_whitespace().collect();
            if parts.len() < 3 {
                return Err(format!(
                    "spot needs a name and a position, e.g. spot Sp1 76% 43% (got '{rest}')"
                ));
            }
            let n = parts.len();
            Ok(Action::Spot {
                name: parts[..n - 2].join(" "),
                x: fraction(parts[n - 2])?,
                y: fraction(parts[n - 1])?,
                text,
            })
        }
        _ => Err(format!(
            "'{text}' is not a step; use lens, reveal, above or spot"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHOWCASE: &str = "\
image: cabinet-4.png
visible: cabinet-4-visible.jpg
palette: lava
# a comment
+ lens 76% 43% 18%
+ reveal
+ above 85%
* spot Hotspot 76% 43%
* spot Reference 30% 52%: 31.2 °C
";

    #[test]
    fn keys_and_steps() {
        let s = Spec::parse(SHOWCASE);
        assert!(s.problems.is_empty(), "{:?}", s.problems);
        assert_eq!(s.image.as_deref(), Some("cabinet-4.png"));
        assert_eq!(s.visible.as_deref(), Some("cabinet-4-visible.jpg"));
        assert_eq!(s.palette, Some(Palette::Lava));
        assert_eq!(s.lines.len(), 5);
        assert_eq!(
            s.lines[0].action,
            Action::Lens {
                x: 0.76,
                y: 0.43,
                r: 0.18
            }
        );
        let steps: Vec<usize> = s.steps(&Support::DISPLAY).iter().map(|(_, n)| *n).collect();
        assert_eq!(steps, [1, 2, 3, 3, 3]);
        assert_eq!(s.step_count(&Support::DISPLAY), 3);
        match &s.lines[4].action {
            Action::Spot { name, text, .. } => {
                assert_eq!(name, "Reference");
                assert_eq!(text.as_deref(), Some("31.2 °C"));
            }
            a => panic!("{a:?}"),
        }
    }

    #[test]
    fn colour_input_drops_the_palette_steps_so_no_click_is_dead() {
        let s = Spec::parse(SHOWCASE);
        let colour = Support {
            palette: false,
            unit: None,
        };
        let steps: Vec<usize> = s.steps(&colour).iter().map(|(_, n)| *n).collect();
        // the threshold is gone; its spots join the reveal's step
        assert_eq!(steps, [1, 2, 2, 2]);
        assert_eq!(s.step_count(&colour), 2);
    }

    #[test]
    fn a_value_threshold_needs_a_source_in_a_compatible_unit() {
        let s = Spec::parse("image: a.png\n+ above 60 °C\n+ above 50%\n");
        assert_eq!(
            s.step_count(&Support::DISPLAY),
            1,
            "the °C step is left out"
        );
        let celsius = Support {
            palette: true,
            unit: Some(Unit::Celsius),
        };
        assert_eq!(s.step_count(&celsius), 2);
        let fahrenheit = Support {
            palette: true,
            unit: Some(Unit::Fahrenheit),
        };
        assert_eq!(s.step_count(&fahrenheit), 2, "converted");
        let percent = Support {
            palette: true,
            unit: Some(Unit::Other("%".into())),
        };
        assert_eq!(s.step_count(&percent), 1);
    }

    #[test]
    fn ranges_units_and_thresholds() {
        assert_eq!(
            Range::parse("18..92 °C").unwrap(),
            Range {
                lo: 18.0,
                hi: 92.0,
                unit: Unit::Celsius
            }
        );
        assert_eq!(Range::parse("-20 .. 40F").unwrap().unit, Unit::Fahrenheit);
        assert!(Range::parse("92..18 °C").is_err());
        assert!(Range::parse("18..92").is_err(), "a unit is required");
        assert!(Range::parse("warm").is_err());
        let s = Spec::parse(
            "image: a.png\nmapping: linear 10..110 °C\nwindow: 40..90 °C\n+ above 60 °C\n",
        );
        assert!(s.problems.is_empty(), "{:?}", s.problems);
        assert_eq!(s.mapping.as_ref().unwrap().span(), 100.0);
        assert_eq!(
            s.lines[0].action,
            Action::Above(Threshold::Value(60.0, Unit::Celsius))
        );
        let f = Unit::Celsius.convert(100.0, &Unit::Fahrenheit).unwrap();
        assert!((f - 212.0).abs() < 1e-3);
        assert_eq!(Unit::Celsius.convert(1.0, &Unit::Other("%".into())), None);
    }

    #[test]
    fn the_slides_window_wins_over_the_blocks() {
        let s = Spec::parse("image: a.png\nwindow: 10..20 °C\nslide-window: 40..90 °C\n");
        assert!(s.slide_window);
        assert_eq!(s.window.as_ref().unwrap().lo, 40.0);
        assert_eq!(s.key_line("window"), Some(1));
    }

    #[test]
    fn problems_name_their_line() {
        let s = Spec::parse(
            "image: a.png\ndata: b.png\npalette: plasma\npolarity: sideways\nmapping: log 1..2 °C\n+ lens 50%\n+ above lots\n+ spin\nzoom: 2\n",
        );
        let lines: Vec<usize> = s.problems.iter().map(|p| p.offset).collect();
        assert_eq!(lines, [2, 3, 4, 5, 6, 7, 8, 0], "{:?}", s.problems);
        assert!(
            Spec::parse("+ reveal").problems[0]
                .message
                .contains("image:")
        );
    }
}
