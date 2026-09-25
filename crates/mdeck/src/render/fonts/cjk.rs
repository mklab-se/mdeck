//! System CJK faces.
//!
//! Chinese, Japanese and Korean need a face far too large to bundle, so one
//! is borrowed from the system at startup when a known file exists (PingFang
//! or Hiragino on macOS, Microsoft YaHei on Windows, Noto Sans CJK or WenQuanYi
//! on Linux) or `MDECK_CJK_FONT` names one. It closes every family's chain
//! after the symbol faces; without it CJK text draws as boxes and `--check`
//! says so (GitHub issue 11).

use std::path::PathBuf;
use std::sync::{Arc, LazyLock};

use eframe::egui::{self, FontData, FontDefinitions, FontFamily};

/// Prefix the system CJK faces are registered under (`SystemCJK0`, ...).
const FONT_CJK: &str = "SystemCJK";

/// Environment variable naming a CJK font file (`.ttf`, `.otf` or `.ttc`) to
/// use instead of the platform search. It is assumed to cover every script.
pub const CJK_FONT_ENV: &str = "MDECK_CJK_FONT";

/// The three CJK scripts a font may cover. Han also stands for CJK
/// punctuation and the fullwidth forms, which every CJK face carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Scripts {
    pub han: bool,
    pub kana: bool,
    pub hangul: bool,
}

impl Scripts {
    pub const NONE: Self = Self {
        han: false,
        kana: false,
        hangul: false,
    };
    pub const ALL: Self = Self {
        han: true,
        kana: true,
        hangul: true,
    };
    const HAN: Self = Self {
        han: true,
        ..Self::NONE
    };
    const HAN_KANA: Self = Self {
        han: true,
        kana: true,
        hangul: false,
    };
    const HANGUL: Self = Self {
        hangul: true,
        ..Self::NONE
    };

    /// The scripts `text` uses: Han ideographs (all extensions), CJK
    /// punctuation and fullwidth forms count as Han; hiragana and katakana
    /// as kana; hangul syllables and jamo as hangul.
    pub fn of(text: &str) -> Self {
        let mut s = Self::NONE;
        for c in text.chars() {
            match c as u32 {
                0x2E80..=0x2FDF   // CJK and Kangxi radicals
                | 0x3000..=0x303F // CJK symbols and punctuation (，。「」)
                | 0x3100..=0x312F // Bopomofo
                | 0x3400..=0x4DBF // CJK Unified Ideographs Extension A
                | 0x4E00..=0x9FFF // CJK Unified Ideographs
                | 0xF900..=0xFAFF // CJK compatibility ideographs
                | 0xFF00..=0xFFEF // Halfwidth and fullwidth forms
                | 0x20000..=0x3134F => s.han = true, // Extensions B to G
                0x3040..=0x30FF | 0x31F0..=0x31FF => s.kana = true,
                0x1100..=0x11FF | 0x3130..=0x318F | 0xAC00..=0xD7AF => s.hangul = true,
                _ => {}
            }
        }
        s
    }

    pub fn is_empty(self) -> bool {
        !(self.han || self.kana || self.hangul)
    }

    pub fn union(self, o: Self) -> Self {
        Self {
            han: self.han || o.han,
            kana: self.kana || o.kana,
            hangul: self.hangul || o.hangul,
        }
    }

    /// The scripts in `self` that `o` does not cover.
    pub fn minus(self, o: Self) -> Self {
        Self {
            han: self.han && !o.han,
            kana: self.kana && !o.kana,
            hangul: self.hangul && !o.hangul,
        }
    }

    /// Human names, for warnings: "Chinese", "Japanese kana", "Korean hangul".
    pub fn names(self) -> Vec<&'static str> {
        let mut v = Vec::new();
        if self.han {
            v.push("Chinese");
        }
        if self.kana {
            v.push("Japanese kana");
        }
        if self.hangul {
            v.push("Korean hangul");
        }
        v
    }
}

/// A font file that may exist on this platform and the scripts it covers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub path: PathBuf,
    pub covers: Scripts,
}

/// Font files that carry CJK glyphs, in the order they are tried on this
/// platform. `MDECK_CJK_FONT` comes first when set.
pub fn cjk_font_candidates() -> Vec<Candidate> {
    let mut v = Vec::new();
    let mut push = |path: PathBuf, covers: Scripts| v.push(Candidate { path, covers });
    if let Ok(p) = std::env::var(CJK_FONT_ENV)
        && !p.trim().is_empty()
    {
        push(PathBuf::from(p), Scripts::ALL);
    }
    #[cfg(target_os = "macos")]
    for (path, covers) in [
        ("/System/Library/Fonts/PingFang.ttc", Scripts::HAN),
        ("/System/Library/Fonts/Hiragino Sans GB.ttc", Scripts::HAN),
        ("/System/Library/Fonts/STHeiti Light.ttc", Scripts::HAN),
        (
            "/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc",
            Scripts::HAN_KANA,
        ),
        (
            "/System/Library/Fonts/Hiragino Sans W3.ttc",
            Scripts::HAN_KANA,
        ),
        (
            "/System/Library/Fonts/AppleSDGothicNeo.ttc",
            Scripts::HANGUL,
        ),
        (
            "/System/Library/Fonts/Supplemental/AppleGothic.ttf",
            Scripts::HANGUL,
        ),
        (
            "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
            Scripts::ALL,
        ),
        ("/Library/Fonts/Arial Unicode.ttf", Scripts::ALL),
    ] {
        push(PathBuf::from(path), covers);
    }
    #[cfg(target_os = "windows")]
    {
        let root = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into());
        let fonts = std::path::Path::new(&root).join("Fonts");
        for (file, covers) in [
            ("msyh.ttc", Scripts::HAN),          // Microsoft YaHei (Simplified)
            ("msyhl.ttc", Scripts::HAN),         // Microsoft YaHei Light
            ("msjh.ttc", Scripts::HAN),          // Microsoft JhengHei (Traditional)
            ("simsun.ttc", Scripts::HAN),        // SimSun
            ("simhei.ttf", Scripts::HAN),        // SimHei
            ("yugothm.ttc", Scripts::HAN_KANA),  // Yu Gothic
            ("meiryo.ttc", Scripts::HAN_KANA),   // Meiryo
            ("msgothic.ttc", Scripts::HAN_KANA), // MS Gothic
            ("malgun.ttf", Scripts::HANGUL),     // Malgun Gothic
            ("gulim.ttc", Scripts::HANGUL),      // Gulim
            ("ARIALUNI.TTF", Scripts::ALL),      // Arial Unicode (Office)
        ] {
            push(fonts.join(file), covers);
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    for (path, covers) in [
        (
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            Scripts::ALL,
        ),
        (
            "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
            Scripts::ALL,
        ),
        (
            "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
            Scripts::ALL,
        ),
        (
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
            Scripts::ALL,
        ),
        (
            "/usr/share/fonts/opentype/noto/NotoSansCJKsc-Regular.otf",
            Scripts::ALL,
        ),
        (
            "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
            Scripts::ALL,
        ),
        (
            "/usr/share/fonts/wenquanyi/wqy-microhei/wqy-microhei.ttc",
            Scripts::ALL,
        ),
        (
            "/usr/share/fonts/wqy-microhei/wqy-microhei.ttc",
            Scripts::ALL,
        ),
        ("/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc", Scripts::ALL),
        ("/usr/share/fonts/wqy-zenhei/wqy-zenhei.ttc", Scripts::ALL),
        (
            "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
            Scripts::ALL,
        ),
        (
            "/usr/share/fonts/droid/DroidSansFallbackFull.ttf",
            Scripts::ALL,
        ),
        (
            "/usr/share/fonts/TTF/DroidSansFallbackFull.ttf",
            Scripts::ALL,
        ),
    ] {
        push(PathBuf::from(path), covers);
    }
    v
}

/// The system CJK font files to load: for each script, the first candidate
/// on disk that covers it, without duplicates. Empty when none exists.
pub fn system_cjk_fonts() -> Vec<Candidate> {
    let present: Vec<Candidate> = cjk_font_candidates()
        .into_iter()
        .filter(|c| c.path.is_file())
        .collect();
    let mut chosen: Vec<Candidate> = Vec::new();
    let mut covered = Scripts::NONE;
    for want in [
        Scripts::HAN,
        Scripts::HANGUL,
        Scripts::HAN_KANA.minus(Scripts::HAN),
    ] {
        if !want.minus(covered).is_empty()
            && let Some(c) = present.iter().find(|c| want.minus(c.covers).is_empty())
            && !chosen.contains(c)
        {
            covered = covered.union(c.covers);
            chosen.push(c.clone());
        }
    }
    chosen
}

/// The scripts the system faces found on this machine cover.
pub fn cjk_coverage() -> Scripts {
    system_cjk_fonts()
        .iter()
        .fold(Scripts::NONE, |acc, c| acc.union(c.covers))
}

/// Line metrics of a face in em units, read the way skrifa (and so epaint)
/// does: the OS/2 typographic values when the font asks for them
/// (`USE_TYPO_METRICS`) or its `hhea` values are zero, else `hhea`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VerticalMetrics {
    pub ascent: f32,
    pub descent: f32,
    pub line_gap: f32,
}

impl VerticalMetrics {
    /// Metrics of face `index` in a TrueType, OpenType or collection file.
    pub fn of(bytes: &[u8], index: u32) -> Option<Self> {
        let u16_at = |o: usize| {
            bytes
                .get(o..o + 2)
                .map(|b| u16::from_be_bytes([b[0], b[1]]))
        };
        let i16_at = |o: usize| u16_at(o).map(|v| v as i16);
        let u32_at = |o: usize| {
            bytes
                .get(o..o + 4)
                .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
        };
        let mut off = 0usize;
        if bytes.get(0..4) == Some(b"ttcf") {
            if index >= u32_at(8)? {
                return None;
            }
            off = u32_at(12 + 4 * index as usize)? as usize;
        } else if index != 0 {
            return None;
        }
        // 0x00010000 and "true" are TrueType, "OTTO" is CFF OpenType.
        if !matches!(u32_at(off)?, 0x0001_0000 | 0x7472_7565 | 0x4F54_544F) {
            return None;
        }
        let (mut head, mut hhea, mut os2) = (None, None, None);
        for i in 0..u16_at(off + 4)? as usize {
            let rec = off + 12 + 16 * i;
            let o = u32_at(rec + 8)? as usize;
            match bytes.get(rec..rec + 4)? {
                b"head" => head = Some(o),
                b"hhea" => hhea = Some(o),
                b"OS/2" => os2 = Some(o),
                _ => {}
            }
        }
        let upm = u16_at(head? + 18)? as f32;
        if upm == 0.0 {
            return None;
        }
        let hhea = hhea?;
        let (mut a, mut d, mut g) = (i16_at(hhea + 4)?, i16_at(hhea + 6)?, i16_at(hhea + 8)?);
        if let Some(o) = os2 {
            let use_typo = u16_at(o + 62)? & (1 << 7) != 0;
            if use_typo || (a == 0 && d == 0) {
                (a, d, g) = (i16_at(o + 68)?, i16_at(o + 70)?, i16_at(o + 72)?);
            }
        }
        Some(Self {
            ascent: a as f32 / upm,
            descent: d as f32 / upm,
            line_gap: g as f32 / upm,
        })
    }

    fn row_height(&self) -> f32 {
        self.ascent - self.descent + self.line_gap
    }

    /// The `y_offset_factor` (em, positive is down) that puts `fallback`'s
    /// baseline on this face's. epaint places a fallback glyph at its own
    /// ascent plus half the difference in row height, which only lands on
    /// the baseline when both faces split their height the same way.
    pub fn baseline_offset(&self, fallback: &Self) -> f32 {
        self.ascent - fallback.ascent - 0.5 * (self.row_height() - fallback.row_height())
    }
}

/// A system CJK face: its bytes, leaked once so every family can register
/// it with its own tweak, and its metrics.
struct CjkFace {
    bytes: &'static [u8],
    metrics: VerticalMetrics,
}

/// The system CJK faces, read once per process. Files that cannot be read
/// or parsed are skipped.
static CJK_FACES: LazyLock<Vec<CjkFace>> = LazyLock::new(|| {
    system_cjk_fonts()
        .iter()
        .filter_map(|c| std::fs::read(&c.path).ok())
        .filter_map(|bytes| {
            let metrics = VerticalMetrics::of(&bytes, 0)?;
            let bytes: &'static [u8] = Box::leak(bytes.into_boxed_slice());
            Some(CjkFace { bytes, metrics })
        })
        .collect()
});

/// Append the system CJK faces to `family`'s chain, each shifted onto the
/// baseline of the family's primary face.
pub(super) fn add_faces(defs: &mut FontDefinitions, family: &FontFamily) {
    let Some(chain) = defs.families.get(family) else {
        return;
    };
    let primary = chain
        .first()
        .and_then(|name| defs.font_data.get(name))
        .and_then(|fd| VerticalMetrics::of(&fd.font, fd.index));
    let key = match family {
        FontFamily::Proportional => "proportional".to_string(),
        FontFamily::Monospace => "monospace".to_string(),
        FontFamily::Name(n) => n.to_string(),
    };
    let mut names = Vec::new();
    for (i, face) in CJK_FACES.iter().enumerate() {
        let name = format!("{FONT_CJK}{i}-{key}");
        let y_offset_factor = primary.map_or(0.0, |p| p.baseline_offset(&face.metrics));
        let tweak = egui::FontTweak {
            y_offset_factor,
            ..Default::default()
        };
        defs.font_data.insert(
            name.clone(),
            Arc::new(FontData::from_static(face.bytes).tweak(tweak)),
        );
        names.push(name);
    }
    if let Some(chain) = defs.families.get_mut(family) {
        chain.extend(names);
    }
}

#[cfg(test)]
mod tests {
    use super::super::{DEJAVU_SANS, HANKEN_REGULAR, install};
    use super::*;
    use crate::theme::{FONT_BODY, FONT_BODY_LIGHT, FONT_BODY_MEDIUM, FONT_DISPLAY, FONT_MONO};

    #[test]
    fn scripts_of_recognises_ideographs_kana_hangul_and_fullwidth_punctuation() {
        assert_eq!(Scripts::of("中文"), Scripts::HAN);
        assert_eq!(Scripts::of("Mixed: English 和 中文"), Scripts::HAN);
        assert_eq!(Scripts::of("日本語のテキスト"), Scripts::HAN_KANA);
        assert_eq!(
            Scripts::of("カタカナ"),
            Scripts::HAN_KANA.minus(Scripts::HAN)
        );
        assert_eq!(Scripts::of("한국어"), Scripts::HANGUL);
        assert_eq!(Scripts::of("你好，世界。"), Scripts::HAN);
        assert_eq!(Scripts::of("ＦＵＬＬＷＩＤＴＨ"), Scripts::HAN);
        assert_eq!(Scripts::of("中 ひ 한"), Scripts::ALL);
        assert!(Scripts::of("Räksmörgås & friends").is_empty());
        assert!(Scripts::of("①②③ ✓ → ★").is_empty());
        assert!(Scripts::of("").is_empty());
        assert_eq!(
            Scripts::ALL.minus(Scripts::HAN).names(),
            ["Japanese kana", "Korean hangul"]
        );
    }

    /// Regression for GitHub issue 11: Chinese text drew as boxes in every
    /// theme. Every script the system faces claim to cover must have glyphs
    /// in every family, which also proves the coverage table right for the
    /// fonts on this machine.
    #[test]
    fn cjk_has_glyphs_in_every_family_for_the_scripts_the_system_covers() {
        let covered = cjk_coverage();
        if covered.is_empty() {
            eprintln!("no system CJK font on this machine, skipping glyph check");
            return;
        }
        let mut samples = Vec::new();
        if covered.han {
            samples.extend(["中文标题测试", "你好，世界。", "繁體中文"]);
        }
        if covered.kana {
            samples.extend(["ひらがな", "カタカナ"]);
        }
        if covered.hangul {
            samples.push("한국어");
        }
        let ctx = egui::Context::default();
        install(&ctx);
        let families = [
            FontFamily::Proportional,
            FontFamily::Monospace,
            FontFamily::Name(FONT_DISPLAY.into()),
            FontFamily::Name(FONT_BODY.into()),
            FontFamily::Name(FONT_BODY_LIGHT.into()),
            FontFamily::Name(FONT_BODY_MEDIUM.into()),
            FontFamily::Name(FONT_MONO.into()),
        ];
        let mut output = ctx.run_ui(Default::default(), |ui| {
            ui.fonts_mut(|f| {
                for fam in &families {
                    let id = egui::FontId::new(24.0, fam.clone());
                    for s in &samples {
                        assert!(f.has_glyphs(&id, s), "{fam:?} lacks glyphs for {s}");
                    }
                }
            });
        });
        output.textures_delta.clear();
    }

    #[test]
    fn cjk_font_candidates_name_absolute_paths_and_chosen_fonts_are_distinct() {
        let c = cjk_font_candidates();
        assert!(!c.is_empty());
        assert!(c.iter().all(|c| c.path.is_absolute()), "{c:?}");
        let chosen = system_cjk_fonts();
        assert!(chosen.len() <= 3, "{chosen:?}");
        for (i, a) in chosen.iter().enumerate() {
            assert!(!chosen[i + 1..].contains(a), "{chosen:?}");
        }
    }

    #[test]
    fn vertical_metrics_read_hhea_and_ttc_faces() {
        let m = VerticalMetrics::of(HANKEN_REGULAR, 0).unwrap();
        assert!((m.ascent - 1.0).abs() < 0.001, "{m:?}");
        assert!((m.descent + 0.303).abs() < 0.001, "{m:?}");
        assert!(m.line_gap.abs() < 0.001, "{m:?}");
        let m = VerticalMetrics::of(DEJAVU_SANS, 0).unwrap();
        assert!((m.ascent - 0.928).abs() < 0.001, "{m:?}");
        assert!(VerticalMetrics::of(b"not a font", 0).is_none());
        assert!(VerticalMetrics::of(HANKEN_REGULAR, 3).is_none());
        if let Some(c) = system_cjk_fonts().first() {
            let bytes = std::fs::read(&c.path).unwrap();
            let m = VerticalMetrics::of(&bytes, 0).expect("system face parses");
            assert!(m.ascent > 0.5 && m.descent < 0.0, "{m:?}");
        }
    }

    /// epaint centres a fallback face on the primary's row, so a face with a
    /// different ascent/descent split lands off the baseline; the offset
    /// (in em, positive = down) undoes that.
    #[test]
    fn baseline_offset_lowers_a_cjk_face_onto_a_latin_baseline() {
        let hanken = VerticalMetrics {
            ascent: 1.0,
            descent: -0.303,
            line_gap: 0.0,
        };
        let hiragino = VerticalMetrics {
            ascent: 0.88,
            descent: -0.12,
            line_gap: 0.5,
        };
        let off = hanken.baseline_offset(&hiragino);
        assert!((off - 0.2185).abs() < 0.001, "{off}");
        assert_eq!(hanken.baseline_offset(&hanken), 0.0);
    }
}
