//! Units and ranges: `°C`, `°F`, `K` (converted between each other) or any
//! other unit (kept as written), and `lo..hi unit` ranges for mappings,
//! windows and the slide's common scale.

/// A temperature unit (or another quantity's unit, kept as written).
#[derive(Debug, Clone, PartialEq)]
pub enum Unit {
    Celsius,
    Fahrenheit,
    Kelvin,
    /// Not a temperature: compared only with the same unit.
    Other(String),
}

impl Unit {
    pub fn parse(s: &str) -> Option<Unit> {
        let t = s.trim();
        Some(
            match t.trim_start_matches('°').to_ascii_lowercase().as_str() {
                "" => return None,
                "c" | "celsius" => Unit::Celsius,
                "f" | "fahrenheit" => Unit::Fahrenheit,
                "k" | "kelvin" => Unit::Kelvin,
                _ => Unit::Other(t.to_string()),
            },
        )
    }

    pub fn symbol(&self) -> &str {
        match self {
            Unit::Celsius => "°C",
            Unit::Fahrenheit => "°F",
            Unit::Kelvin => "K",
            Unit::Other(s) => s,
        }
    }

    /// `value` in this unit, converted to `to`; `None` between a temperature
    /// and another quantity, or two different other units.
    pub fn convert(&self, value: f32, to: &Unit) -> Option<f32> {
        if self == to {
            return Some(value);
        }
        let kelvin = match self {
            Unit::Celsius => value + 273.15,
            Unit::Fahrenheit => (value - 32.0) * 5.0 / 9.0 + 273.15,
            Unit::Kelvin => value,
            Unit::Other(_) => return None,
        };
        Some(match to {
            Unit::Celsius => kelvin - 273.15,
            Unit::Fahrenheit => (kelvin - 273.15) * 9.0 / 5.0 + 32.0,
            Unit::Kelvin => kelvin,
            Unit::Other(_) => return None,
        })
    }
}

/// `lo..hi unit`, e.g. a mapping's or a window's range.
#[derive(Debug, Clone, PartialEq)]
pub struct Range {
    pub lo: f32,
    pub hi: f32,
    pub unit: Unit,
}

impl Range {
    /// `18..92 °C`, `18 .. 92°C`, `-20..40 C`.
    pub fn parse(s: &str) -> Result<Range, String> {
        let s = s.trim();
        let (lo, rest) = s
            .split_once("..")
            .ok_or_else(|| format!("'{s}' is not a range like 18..92 °C"))?;
        let rest = rest.trim();
        let split = rest
            .char_indices()
            .find(|&(i, c)| !(c.is_ascii_digit() || c == '.' || (i == 0 && (c == '-' || c == '+'))))
            .map_or(rest.len(), |(i, _)| i);
        let (hi, unit) = rest.split_at(split);
        let num = |v: &str| {
            v.trim()
                .parse::<f32>()
                .ok()
                .filter(|v| v.is_finite())
                .ok_or_else(|| format!("'{s}': '{}' is not a number", v.trim()))
        };
        let (lo, hi) = (num(lo)?, num(hi)?);
        if hi <= lo {
            return Err(format!("'{s}': the top must be above the bottom"));
        }
        let unit =
            Unit::parse(unit).ok_or_else(|| format!("'{s}' needs a unit, e.g. 18..92 °C"))?;
        Ok(Range { lo, hi, unit })
    }

    pub fn span(&self) -> f32 {
        self.hi - self.lo
    }

    /// This range in `unit`.
    pub fn to_unit(&self, unit: &Unit) -> Option<Range> {
        Some(Range {
            lo: self.unit.convert(self.lo, unit)?,
            hi: self.unit.convert(self.hi, unit)?,
            unit: unit.clone(),
        })
    }
}
