//! Spacing and radius tokens (THM-07): the gaps arrangements name (`xs` to
//! `xl`) and the corner radius of cards (code, tables, callouts). A theme
//! makes everything airier or tighter by changing them in one place.

use super::ThemeError;
use super::arrangement::Space;

/// The spacing tokens, smallest first.
pub const TOKENS: [&str; 5] = ["xs", "sm", "md", "lg", "xl"];

/// Their values when a theme sets none, px at 1920x1080.
pub const DEFAULTS: [f32; 5] = [8.0, 16.0, 24.0, 40.0, 64.0];

/// The default corner radius, px at 1920x1080.
pub const DEFAULT_RADIUS: f32 = 8.0;

/// `0.5em` as 0.5.
pub fn em_factor(token: &str) -> Option<f32> {
    token
        .strip_suffix("em")
        .and_then(|n| n.trim().parse::<f32>().ok())
        .filter(|f| (0.0..=10.0).contains(f))
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spacing {
    pub steps: [f32; 5],
}

impl Default for Spacing {
    fn default() -> Self {
        Spacing { steps: DEFAULTS }
    }
}

impl Spacing {
    /// The value of `space`, px at 1920x1080, for an element whose size is
    /// `em` (for `0.5em`). Unknown tokens, which theme building rules out,
    /// are 0.
    pub fn px(&self, space: &Space, em: f32) -> f32 {
        match space {
            Space::Px(p) => *p,
            Space::Token(t) => match em_factor(t) {
                Some(f) => f * em,
                None => TOKENS
                    .iter()
                    .position(|k| k == t)
                    .map_or(0.0, |i| self.steps[i]),
            },
        }
    }

    /// The theme file's `spacing:` over the defaults.
    pub fn resolve(f: &super::file::Spacing) -> Result<Self, ThemeError> {
        let mut steps = DEFAULTS;
        for (i, v) in [f.xs, f.sm, f.md, f.lg, f.xl].into_iter().enumerate() {
            if let Some(v) = v {
                if !(0.0..=400.0).contains(&v) {
                    return Err(ThemeError::OutOfRange {
                        key: format!("spacing.{}", TOKENS[i]),
                        value: v,
                        lo: 0.0,
                        hi: 400.0,
                    });
                }
                steps[i] = v;
            }
        }
        if steps.windows(2).any(|w| w[0] > w[1]) {
            return Err(ThemeError::invalid(
                "spacing",
                format!("the steps must grow from xs to xl (they are {steps:?})"),
            ));
        }
        Ok(Spacing { steps })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_and_px() {
        let s = Spacing::default();
        assert_eq!(s.px(&Space::Token("md".into()), 50.0), 24.0);
        assert_eq!(s.px(&Space::Px(13.0), 50.0), 13.0);
        assert_eq!(s.px(&Space::Token("0.5em".into()), 50.0), 25.0);
        let f = super::super::file::Spacing {
            md: Some(30.0),
            ..Default::default()
        };
        assert_eq!(Spacing::resolve(&f).unwrap().steps[2], 30.0);
        let f = super::super::file::Spacing {
            xs: Some(100.0),
            ..Default::default()
        };
        assert!(
            Spacing::resolve(&f)
                .unwrap_err()
                .to_string()
                .contains("grow")
        );
    }
}
