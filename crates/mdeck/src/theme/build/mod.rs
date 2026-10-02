//! Building a [`Theme`] from a merged [`ThemeFile`]: one resolver per
//! section, called in the order their errors are reported.

use super::file::ThemeFile;
use super::{Built, Theme, ThemeError};

mod colors;
mod extras;
mod fonts;
mod settings;

use colors::Palette;

/// An optional number that must lie in `lo..=hi` when set.
fn range(key: &str, v: Option<f32>, lo: f32, hi: f32) -> Result<Option<f32>, ThemeError> {
    match v {
        Some(x) if !(lo..=hi).contains(&x) => Err(ThemeError::OutOfRange {
            key: key.to_string(),
            value: x,
            lo,
            hi,
        }),
        v => Ok(v),
    }
}

impl Theme {
    /// Build a theme from a fully merged file whose font and syntax paths
    /// are already absolute and confined (see [`super::lookup`]). Invalid
    /// values are errors; a font or syntax file that cannot be used falls
    /// back with a warning.
    pub fn build(name: &str, f: &ThemeFile) -> Result<Built, ThemeError> {
        let mut warnings = Vec::new();
        let palette = Palette::resolve(f, &mut warnings)?;
        let (engine, countdown, transition) = settings::engine_and_countdown(f, &mut warnings)?;
        settings::surface(f)?;
        let fonts = fonts::resolve(&f.fonts, &mut warnings)?;
        let line_height = settings::line_height(f)?;
        let fill_opacity = settings::fill_opacity(f)?;
        let syntax = settings::syntax(f, &mut warnings)?;
        let logo = extras::logo(&f.logo, &mut warnings)?;
        let page = extras::page(&f.page)?;
        let art = extras::art(f)?;
        extras::heat(&f.heat)?;
        let [h1_size, h2_size, h3_size, body_size, code_size] = settings::sizes(&f.sizes)?;

        let p = palette.flattened();
        let mut theme = Theme {
            name: f.name.clone().unwrap_or_else(|| name.to_string()),
            engine,
            copy_hold: 0.0,
            engine_numbers_slides: false,
            engine_keys: super::engine::engine_keys(f),
            countdown,
            transition,
            background: p.background,
            foreground: p.foreground,
            heading_color: p.heading,
            muted: p.muted,
            strong: p.strong,
            rule: p.rule,
            accent: p.accent,
            accent_soft: p.accent_soft,
            secondary: p.secondary,
            code_background: p.code_background,
            code_foreground: p.code_foreground,
            positive: p.positive,
            negative: p.negative,
            series: p.series,
            pen: p.pen,
            pen_outline: p.pen_outline,
            arrow: p.arrow,
            arrow_outline: p.arrow_outline,
            particle_light: p.particle_light,
            particle_cool: p.particle_cool,
            fonts,
            h1_size,
            h2_size,
            h3_size,
            body_size,
            code_size,
            line_height,
            fill_opacity,
            syntax,
            logo,
            page,
            art,
            source: None,
        };
        theme.set_engine(engine);
        Ok(Built { theme, warnings })
    }
}

#[cfg(test)]
mod tests {
    use super::super::lookup;
    use super::*;
    use eframe::egui::Color32;

    fn over_dark(yaml: &str) -> ThemeFile {
        ThemeFile::parse(yaml)
            .unwrap()
            .over(&lookup::builtin_file("dark").unwrap())
    }

    #[test]
    fn range_accepts_unset_and_bounds() {
        assert_eq!(range("k", None, 0.0, 1.0), Ok(None));
        assert_eq!(range("k", Some(1.0), 0.0, 1.0), Ok(Some(1.0)));
        assert_eq!(
            range("k", Some(1.5), 0.0, 1.0).unwrap_err().to_string(),
            "k: 1.5 must be between 0 and 1"
        );
        assert!(range("k", Some(f32::NAN), 0.0, 1.0).is_err());
    }

    #[test]
    fn translucent_colours_are_flattened_onto_the_background() {
        let f = over_dark(
            "colors: { background: '#000000', rule: '#ffffff80', series: ['#ff000080'] }",
        );
        let t = Theme::build("x", &f).unwrap().theme;
        assert_eq!(t.rule, Color32::from_rgb(128, 128, 128));
        assert_eq!(t.series[0], Color32::from_rgb(128, 0, 0));
        assert_eq!(t.background.a(), 255);
    }

    #[test]
    fn series_cycles_to_fill_the_palette() {
        let f = over_dark("colors: { series: ['#ff0000', '#00ff00'] }");
        let t = Theme::build("x", &f).unwrap().theme;
        assert_eq!(t.series[0], Color32::from_rgb(255, 0, 0));
        assert_eq!(t.series[1], Color32::from_rgb(0, 255, 0));
        assert_eq!(t.series[2], Color32::from_rgb(255, 0, 0));
    }

    #[test]
    fn invalid_values_are_errors() {
        let bad = |yaml: &str| Theme::build("x", &over_dark(yaml)).unwrap_err().to_string();
        assert!(bad("colors: { accent: 'orange' }").contains("colors.accent"));
        assert!(bad("countdown: loud").contains("countdown"));
        assert!(bad("sizes: { h1: -3 }").contains("sizes.h1"));
        assert!(bad("charts: { fill-opacity: 2 }").contains("fill-opacity"));
        assert!(bad("fonts: { body: comic-sans }").contains("bundled face"));
        assert!(bad("code: { syntax: rainbow }").contains("code.syntax"));
        assert!(bad("colors: { series: [] }").contains("series"));
        assert!(bad("page: { surface: paper }").contains("colors.page.surface"));
    }

    /// The first problem in file order is the one reported, as before the
    /// resolvers were split: colours, then engine, ..., then sizes last.
    #[test]
    fn errors_come_in_section_order() {
        let e = Theme::build(
            "x",
            &over_dark("sizes: { h1: -3 }\ncharts: { fill-opacity: 2 }\ncolors: { muted: nope }"),
        )
        .unwrap_err();
        assert!(e.to_string().starts_with("colors.muted"), "{e}");
        let e =
            Theme::build("x", &over_dark("sizes: { h1: -3 }\nlogo: { opacity: 2 }")).unwrap_err();
        assert!(e.to_string().starts_with("logo.opacity"), "{e}");
    }

    #[test]
    fn countdown_is_a_switch_and_transition_a_known_name() {
        let t = |yaml: &str| Theme::build("x", &over_dark(yaml)).unwrap().theme;
        assert!(t("countdown: on").countdown);
        assert!(!t("countdown: off").countdown);
        assert!(!t("engine: plain").countdown);
        assert_eq!(
            t("transition: spatial").transition.as_deref(),
            Some("spatial")
        );
        assert_eq!(t("engine: plain").transition, None);
        let bad = |yaml: &str| Theme::build("x", &over_dark(yaml)).unwrap_err().to_string();
        assert!(bad("countdown: burst").contains("on or off"));
        assert!(bad("transition: wipe").contains("transition"));
    }
}
