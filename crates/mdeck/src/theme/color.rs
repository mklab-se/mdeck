//! Colour arithmetic shared by theme building and readability advice.

use eframe::egui::Color32;

/// `a` moved towards `b` by `t` (0 is `a`, 1 is `b`), per channel, opaque.
pub(crate) fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

/// `c` at `f` of its brightness.
pub(crate) fn darken(c: Color32, f: f32) -> Color32 {
    mix(Color32::BLACK, c, f)
}

/// `c` composited over the opaque `background` once, so a translucent
/// colour from a design system becomes the solid colour it shows as.
pub(crate) fn flatten(c: Color32, background: Color32) -> Color32 {
    if c.a() == 255 {
        return c;
    }
    let [r, g, b, a] = c.to_srgba_unmultiplied();
    mix(background, Color32::from_rgb(r, g, b), a as f32 / 255.0)
}

/// Relative luminance (0..1) of a colour, ignoring alpha (Rec. 709 weights
/// on the stored values; good enough to rank colours).
pub fn luminance(c: Color32) -> f32 {
    (0.2126 * c.r() as f32 + 0.7152 * c.g() as f32 + 0.0722 * c.b() as f32) / 255.0
}

/// WCAG 2 contrast ratio between two colours (1..21).
pub fn contrast(a: Color32, b: Color32) -> f32 {
    let lin = |v: u8| {
        let c = v as f32 / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let rl = |c: Color32| 0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b());
    let (x, y) = (rl(a), rl(b));
    let (hi, lo) = if x > y { (x, y) } else { (y, x) };
    (hi + 0.05) / (lo + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contrast_matches_wcag() {
        assert!((contrast(Color32::BLACK, Color32::WHITE) - 21.0).abs() < 0.01);
        assert!((contrast(Color32::WHITE, Color32::WHITE) - 1.0).abs() < 0.01);
    }

    #[test]
    fn mix_and_darken_blend_per_channel() {
        let red = Color32::from_rgb(200, 0, 0);
        assert_eq!(mix(Color32::BLACK, red, 0.5), Color32::from_rgb(100, 0, 0));
        assert_eq!(mix(Color32::BLACK, red, 0.0), Color32::BLACK);
        assert_eq!(darken(red, 0.6), Color32::from_rgb(120, 0, 0));
    }

    #[test]
    fn flatten_composites_translucent_colours_only() {
        let bg = Color32::BLACK;
        let half_white = Color32::from_rgba_unmultiplied(255, 255, 255, 128);
        assert_eq!(flatten(half_white, bg), Color32::from_rgb(128, 128, 128));
        let solid = Color32::from_rgb(1, 2, 3);
        assert_eq!(flatten(solid, bg), solid);
    }
}
