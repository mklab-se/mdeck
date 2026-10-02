//! Colours: 8-bit RGBA, stored premultiplied, and the blending helpers the
//! built-in engines share.

use super::geom::{FromEgui, ToEgui};

/// An 8-bit sRGB colour with alpha, stored **premultiplied**: the colour
/// channels are already multiplied by alpha, so a colour with alpha 0 and
/// non-zero channels *adds* light (see [`additive`]).
///
/// ```
/// use mdeck_sdk::paint::Color;
/// let c = Color::from_rgb(200, 100, 50);
/// assert_eq!(c.a(), 255);
/// assert_eq!(Color::from_rgba_unmultiplied(200, 100, 50, 0), Color::TRANSPARENT);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Color([u8; 4]);

impl Color {
    /// Fully transparent.
    pub const TRANSPARENT: Color = Color([0, 0, 0, 0]);
    /// Opaque black.
    pub const BLACK: Color = Color([0, 0, 0, 255]);
    /// Opaque white.
    pub const WHITE: Color = Color([255, 255, 255, 255]);

    /// An opaque colour.
    ///
    /// ```
    /// assert_eq!(mdeck_sdk::paint::Color::from_rgb(1, 2, 3).r(), 1);
    /// ```
    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Color([r, g, b, 255])
    }

    /// An opaque grey.
    ///
    /// ```
    /// use mdeck_sdk::paint::Color;
    /// assert_eq!(Color::from_gray(255), Color::WHITE);
    /// ```
    pub const fn from_gray(v: u8) -> Self {
        Color([v, v, v, 255])
    }

    /// A colour from channels that are already premultiplied by alpha.
    ///
    /// ```
    /// use mdeck_sdk::paint::Color;
    /// assert_eq!(Color::from_rgba_premultiplied(10, 10, 10, 0).a(), 0);
    /// ```
    pub const fn from_rgba_premultiplied(r: u8, g: u8, b: u8, a: u8) -> Self {
        Color([r, g, b, a])
    }

    /// A colour from straight (not premultiplied) channels.
    ///
    /// ```
    /// use mdeck_sdk::paint::Color;
    /// let c = Color::from_rgba_unmultiplied(200, 0, 0, 128);
    /// assert_eq!(c.r(), 100);
    /// ```
    pub fn from_rgba_unmultiplied(r: u8, g: u8, b: u8, a: u8) -> Self {
        let k = a as f32 / 255.0;
        let m = |c: u8| (c as f32 * k).round() as u8;
        Color([m(r), m(g), m(b), a])
    }

    /// Parse `#rgb`, `#rrggbb` or `#rrggbbaa` (straight alpha). The `#` is optional.
    ///
    /// ```
    /// use mdeck_sdk::paint::Color;
    /// assert_eq!(Color::from_hex("#ff8000"), Some(Color::from_rgb(255, 128, 0)));
    /// assert_eq!(Color::from_hex("nope"), None);
    /// ```
    pub fn from_hex(s: &str) -> Option<Self> {
        let s = s.trim().trim_start_matches('#');
        let byte = |i: usize| u8::from_str_radix(s.get(i..i + 2)?, 16).ok();
        match s.len() {
            3 => {
                let n = |i: usize| {
                    u8::from_str_radix(s.get(i..i + 1)?, 16)
                        .ok()
                        .map(|v| v * 17)
                };
                Some(Color::from_rgb(n(0)?, n(1)?, n(2)?))
            }
            6 => Some(Color::from_rgb(byte(0)?, byte(2)?, byte(4)?)),
            8 => Some(Color::from_rgba_unmultiplied(
                byte(0)?,
                byte(2)?,
                byte(4)?,
                byte(6)?,
            )),
            _ => None,
        }
    }

    /// The colour as `#rrggbb` (or `#rrggbbaa` when not opaque), straight alpha.
    ///
    /// ```
    /// use mdeck_sdk::paint::Color;
    /// assert_eq!(Color::from_rgb(255, 128, 0).to_hex(), "#ff8000");
    /// ```
    pub fn to_hex(self) -> String {
        let [r, g, b, a] = self.to_rgba_unmultiplied();
        if a == 255 {
            format!("#{r:02x}{g:02x}{b:02x}")
        } else {
            format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
        }
    }

    /// Red channel (premultiplied).
    ///
    /// ```
    /// assert_eq!(mdeck_sdk::paint::Color::from_rgb(7, 0, 0).r(), 7);
    /// ```
    pub const fn r(self) -> u8 {
        self.0[0]
    }

    /// Green channel (premultiplied).
    ///
    /// ```
    /// assert_eq!(mdeck_sdk::paint::Color::from_rgb(0, 7, 0).g(), 7);
    /// ```
    pub const fn g(self) -> u8 {
        self.0[1]
    }

    /// Blue channel (premultiplied).
    ///
    /// ```
    /// assert_eq!(mdeck_sdk::paint::Color::from_rgb(0, 0, 7).b(), 7);
    /// ```
    pub const fn b(self) -> u8 {
        self.0[2]
    }

    /// Alpha channel.
    ///
    /// ```
    /// assert_eq!(mdeck_sdk::paint::Color::WHITE.a(), 255);
    /// ```
    pub const fn a(self) -> u8 {
        self.0[3]
    }

    /// The premultiplied channels.
    ///
    /// ```
    /// assert_eq!(mdeck_sdk::paint::Color::BLACK.to_array(), [0, 0, 0, 255]);
    /// ```
    pub const fn to_array(self) -> [u8; 4] {
        self.0
    }

    /// The straight (not premultiplied) channels.
    ///
    /// ```
    /// use mdeck_sdk::paint::Color;
    /// let c = Color::from_rgba_unmultiplied(200, 100, 0, 128);
    /// let [r, _, _, a] = c.to_rgba_unmultiplied();
    /// assert!((r as i32 - 200).abs() <= 1 && a == 128);
    /// ```
    pub fn to_rgba_unmultiplied(self) -> [u8; 4] {
        let [r, g, b, a] = self.0;
        if a == 0 {
            return [0, 0, 0, 0];
        }
        let k = 255.0 / a as f32;
        let u = |c: u8| (c as f32 * k).round().min(255.0) as u8;
        [u(r), u(g), u(b), a]
    }

    /// The colour as floats 0..1, premultiplied.
    ///
    /// ```
    /// assert_eq!(mdeck_sdk::paint::Color::WHITE.to_f32(), [1.0; 4]);
    /// ```
    pub fn to_f32(self) -> [f32; 4] {
        self.0.map(|c| c as f32 / 255.0)
    }

    /// Every channel (alpha included) scaled by `k` (0..1): the colour at
    /// opacity `k` with normal blending.
    ///
    /// ```
    /// use mdeck_sdk::paint::Color;
    /// assert_eq!(Color::WHITE.gamma_multiply(0.0), Color::TRANSPARENT);
    /// ```
    pub fn gamma_multiply(self, k: f32) -> Color {
        let k = k.clamp(0.0, 1.0);
        Color(self.0.map(|c| (c as f32 * k).round() as u8))
    }

    /// The same colour, opaque with the channels it has now (alpha set to 255
    /// without unmultiplying).
    ///
    /// ```
    /// use mdeck_sdk::paint::Color;
    /// assert_eq!(Color::from_rgba_premultiplied(9, 9, 9, 0).opaque().a(), 255);
    /// ```
    pub const fn opaque(self) -> Color {
        Color([self.0[0], self.0[1], self.0[2], 255])
    }
}

/// `a` toward `b` by `t` (0..1), opaque.
///
/// ```
/// use mdeck_sdk::paint::{mix, Color};
/// assert_eq!(mix(Color::BLACK, Color::WHITE, 1.0), Color::WHITE);
/// assert_eq!(mix(Color::BLACK, Color::WHITE, 0.5), Color::from_gray(128));
/// ```
pub fn mix(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

/// `c` at opacity `a`, premultiplied (normal blending). `c`'s own alpha is
/// ignored: the colour channels are taken as opaque.
///
/// ```
/// use mdeck_sdk::paint::{premul, Color};
/// let c = Color::from_rgb(200, 100, 50);
/// assert_eq!(premul(c, 1.0), c);
/// assert_eq!(premul(c, 0.0), Color::TRANSPARENT);
/// ```
pub fn premul(c: Color, a: f32) -> Color {
    let a = a.clamp(0.0, 1.0);
    Color::from_rgba_premultiplied(
        (c.r() as f32 * a) as u8,
        (c.g() as f32 * a) as u8,
        (c.b() as f32 * a) as u8,
        (a * 255.0) as u8,
    )
}

/// `c` scaled by `k` with zero alpha: with premultiplied blending it is
/// added onto what is below (glow).
///
/// ```
/// use mdeck_sdk::paint::{additive, Color};
/// assert_eq!(additive(Color::WHITE, 0.5).a(), 0);
/// assert_eq!(additive(Color::WHITE, 0.5).r(), 127);
/// ```
pub fn additive(c: Color, k: f32) -> Color {
    let k = k.clamp(0.0, 1.0);
    Color::from_rgba_premultiplied(
        (c.r() as f32 * k) as u8,
        (c.g() as f32 * k) as u8,
        (c.b() as f32 * k) as u8,
        0,
    )
}

/// 0 below `a`, 1 above `b`, and a smooth S between.
///
/// ```
/// use mdeck_sdk::paint::smoothstep;
/// assert_eq!(smoothstep(0.0, 1.0, 0.5), 0.5);
/// assert_eq!(smoothstep(0.0, 1.0, 2.0), 1.0);
/// ```
pub fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl ToEgui for Color {
    type Out = egui::Color32;
    fn eg(self) -> egui::Color32 {
        let [r, g, b, a] = self.0;
        egui::Color32::from_rgba_premultiplied(r, g, b, a)
    }
}
impl FromEgui<Color> for egui::Color32 {
    fn sdk(self) -> Color {
        Color(self.to_array())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoothstep_is_clamped_and_symmetric() {
        assert_eq!(smoothstep(0.0, 1.0, -1.0), 0.0);
        assert_eq!(smoothstep(0.0, 1.0, 2.0), 1.0);
        assert_eq!(smoothstep(0.0, 1.0, 0.3), 0.3 * 0.3 * (3.0 - 2.0 * 0.3));
    }

    #[test]
    fn helpers_match_egui_premultiplied_semantics() {
        let c = Color::from_rgb(200, 100, 50);
        assert_eq!(premul(c, 0.5).eg(), {
            let e = egui::Color32::from_rgb(200, 100, 50);
            egui::Color32::from_rgba_premultiplied(
                (e.r() as f32 * 0.5) as u8,
                (e.g() as f32 * 0.5) as u8,
                (e.b() as f32 * 0.5) as u8,
                127,
            )
        });
        let straight = Color::from_rgba_unmultiplied(10, 20, 30, 77);
        assert_eq!(
            straight.eg(),
            egui::Color32::from_rgba_unmultiplied(10, 20, 30, 77)
        );
    }

    #[test]
    fn hex_round_trips() {
        for s in ["#000000", "#ff8000", "#ff000080"] {
            assert_eq!(Color::from_hex(s).unwrap().to_hex(), s);
        }
        assert_eq!(Color::from_hex("#fff"), Some(Color::WHITE));
        assert_eq!(Color::from_hex("#ggg"), None);
    }
}
