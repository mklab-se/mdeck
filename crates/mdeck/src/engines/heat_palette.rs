//! Heat palettes: how a relative temperature (0 cold .. 1 hot) becomes a
//! colour. Iron, white-hot and black-hot are the everyday set; rainbow,
//! arctic and lava are there for audiences that expect them.
//!
//! Shared by the thermal engine's heat field and the core's `@thermal`
//! visual (`render::thermal`), so it lives here, outside any feature.

use mdeck_sdk::paint::Color;

/// A palette for thermal images and their legends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Palette {
    Iron,
    WhiteHot,
    BlackHot,
    Rainbow,
    Arctic,
    Lava,
}

impl Palette {
    /// Every palette, in the order the palette key cycles through them.
    pub const ALL: [Palette; 6] = [
        Palette::Iron,
        Palette::WhiteHot,
        Palette::BlackHot,
        Palette::Rainbow,
        Palette::Arctic,
        Palette::Lava,
    ];

    /// The palette a `@thermal` block uses when it names none.
    pub const DEFAULT: Palette = Palette::Iron;

    pub fn name(self) -> &'static str {
        match self {
            Palette::Iron => "iron",
            Palette::WhiteHot => "white-hot",
            Palette::BlackHot => "black-hot",
            Palette::Rainbow => "rainbow",
            Palette::Arctic => "arctic",
            Palette::Lava => "lava",
        }
    }

    pub fn from_name(name: &str) -> Option<Palette> {
        let n = name.trim().to_ascii_lowercase().replace(['_', ' '], "-");
        Palette::ALL.into_iter().find(|p| p.name() == n)
    }

    /// The next palette after this one, for the palette key.
    pub fn next(self) -> Palette {
        let i = Palette::ALL.iter().position(|&p| p == self).unwrap_or(0);
        Palette::ALL[(i + 1) % Palette::ALL.len()]
    }

    /// Whether brightness rises steadily from cold to hot (or falls, for
    /// black-hot), so the order of temperatures survives a gray print and
    /// colour-vision deficiencies. Rainbow and arctic trade that for hue.
    #[cfg(test)]
    pub fn ordered_by_lightness(self) -> bool {
        !matches!(self, Palette::Rainbow | Palette::Arctic)
    }

    fn stops(self) -> &'static [u32] {
        match self {
            Palette::Iron => &[
                0x000000, 0x1d0b4f, 0x57108c, 0xa3176f, 0xd93a2b, 0xf37a0c, 0xfbb41c, 0xfde35a,
                0xffffff,
            ],
            Palette::WhiteHot => &[0x000000, 0xffffff],
            Palette::BlackHot => &[0xffffff, 0x000000],
            Palette::Rainbow => &[
                0x000000, 0x20125e, 0x1f4fd1, 0x13a5c9, 0x2ec25a, 0xd6d61f, 0xf28a12, 0xe5251c,
                0xffffff,
            ],
            Palette::Arctic => &[
                0x03081f, 0x0b2a6b, 0x1c6fbf, 0x62b6e6, 0xd7f0fb, 0xf6d18a, 0xf28c28, 0xffffff,
            ],
            Palette::Lava => &[
                0x000000, 0x2b0a06, 0x6e1308, 0xb5260b, 0xe3561a, 0xf59a3c, 0xfcd987, 0xffffff,
            ],
        }
    }

    /// The palette as 256 colours, from cold to hot.
    pub fn lut(self) -> [Color; 256] {
        let stops = self.stops();
        let rgb = |c: u32| {
            [
                (c >> 16) as f32,
                ((c >> 8) & 0xff) as f32,
                (c & 0xff) as f32,
            ]
        };
        let n = stops.len() - 1;
        std::array::from_fn(|i| {
            let t = i as f32 / 255.0 * n as f32;
            let k = (t as usize).min(n - 1);
            let f = t - k as f32;
            let (a, b) = (rgb(stops[k]), rgb(stops[k + 1]));
            let ch = |j: usize| (a[j] + (b[j] - a[j]) * f).round() as u8;
            Color::from_rgb(ch(0), ch(1), ch(2))
        })
    }

    /// The colour at `t` (0 cold .. 1 hot).
    #[cfg(test)]
    pub fn at(self, t: f32) -> Color {
        self.lut()[(t.clamp(0.0, 1.0) * 255.0).round() as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Relative luminance (Rec. 709 weights), as the theme computes it.
    fn luminance(c: Color) -> f32 {
        (0.2126 * c.r() as f32 + 0.7152 * c.g() as f32 + 0.0722 * c.b() as f32) / 255.0
    }

    #[test]
    fn names_round_trip_and_the_key_cycles_through_all() {
        for p in Palette::ALL {
            assert_eq!(Palette::from_name(p.name()), Some(p));
        }
        assert_eq!(Palette::from_name("White Hot"), Some(Palette::WhiteHot));
        assert_eq!(Palette::from_name("plasma"), None);
        let mut p = Palette::Iron;
        for _ in 0..Palette::ALL.len() {
            p = p.next();
        }
        assert_eq!(p, Palette::Iron);
    }

    #[test]
    fn luts_run_from_the_first_stop_to_the_last() {
        let iron = Palette::Iron.lut();
        assert_eq!(iron[0], Color::BLACK);
        assert_eq!(iron[255], Color::WHITE);
        assert_eq!(Palette::BlackHot.lut()[0], Color::WHITE);
        assert_eq!(Palette::WhiteHot.at(0.5), Color::from_rgb(128, 128, 128));
    }

    /// The palettes we recommend keep their order in lightness, so hotter is
    /// always lighter (darker for black-hot): readable in gray and under the
    /// common colour-vision deficiencies.
    #[test]
    fn ordered_palettes_rise_steadily_in_lightness() {
        for p in Palette::ALL
            .into_iter()
            .filter(|p| p.ordered_by_lightness())
        {
            let lut = p.lut();
            let l: Vec<f32> = lut.iter().map(|&c| luminance(c)).collect();
            let falling = p == Palette::BlackHot;
            for w in l.windows(2) {
                let step = if falling { w[0] - w[1] } else { w[1] - w[0] };
                assert!(step >= -1e-3, "{p:?} is not ordered by lightness");
            }
            // and the ends are far apart: cold and hot never look alike
            assert!((l[255] - l[0]).abs() > 0.9, "{p:?}");
        }
        assert!(!Palette::Rainbow.ordered_by_lightness());
    }
}
