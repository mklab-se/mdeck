//! The mdeck language in one table: every deck and slide setting and every
//! mdeck fence. The parser reads settings by these names, `--check`
//! validates against them and the references (`mdeck spec --short`, the
//! format reference) are generated from them, so they cannot drift.

mod suggest;

pub use suggest::suggestion;

/// Where a setting may be written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// In the frontmatter only.
    Deck,
    /// In a slide's settings comment only.
    Slide,
    /// In both: the slide's value overrides the deck's.
    Both,
}

impl Scope {
    pub fn in_deck(self) -> bool {
        matches!(self, Scope::Deck | Scope::Both)
    }

    pub fn in_slide(self) -> bool {
        matches!(self, Scope::Slide | Scope::Both)
    }
}

/// What a setting's value may be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Any text.
    Text,
    /// A theme name or a path to a theme file.
    Theme,
    /// An engine name.
    Engine,
    /// A file path, or `none`.
    Path,
    /// One of these words.
    Choice(&'static [&'static str]),
    /// A whole number in this range.
    Integer(u32, u32),
    /// 0 to 1, or a percentage.
    Opacity,
    /// A size in pixels on a 1920x1080 slide.
    Pixels,
    /// A thermal palette name.
    Palette,
    /// A temperature window, `25..90 °C`.
    Window,
}

impl Kind {
    /// The values as the reference shows them.
    fn describe(self) -> String {
        match self {
            Kind::Text => "text".into(),
            Kind::Theme => "theme name or file".into(),
            Kind::Engine => "engine name".into(),
            Kind::Path => "file, or `none`".into(),
            Kind::Choice(values) => values
                .iter()
                .map(|v| format!("`{v}`"))
                .collect::<Vec<_>>()
                .join(", "),
            Kind::Integer(lo, hi) => format!("{lo} to {hi}"),
            Kind::Opacity => "0 to 1, or a percentage".into(),
            Kind::Pixels => "pixels on a 1920x1080 slide".into(),
            Kind::Palette => crate::render::thermal::Palette::ALL
                .iter()
                .map(|p| format!("`{}`", p.name()))
                .collect::<Vec<_>>()
                .join(", "),
            Kind::Window => "`low..high °C`".into(),
        }
    }
}

/// One setting of the language.
#[derive(Debug, Clone, Copy)]
pub struct SettingDef {
    pub name: &'static str,
    pub scope: Scope,
    pub kind: Kind,
    /// What applies when the setting is not written (empty: nothing).
    pub default: &'static str,
    pub summary: &'static str,
    pub since: &'static str,
}

/// The design names a slide may choose (`design:`).
pub const DESIGNS: &[&str] = &[
    "title",
    "section",
    "statement",
    "points",
    "split",
    "media",
    "gallery",
    "quote",
    "code",
    "visual",
    "columns",
    "table",
    "content",
];

pub const TRANSITIONS: &[&str] = &["fade", "slide", "spatial", "zoom", "none"];

const fn def(
    name: &'static str,
    scope: Scope,
    kind: Kind,
    default: &'static str,
    summary: &'static str,
) -> SettingDef {
    SettingDef {
        name,
        scope,
        kind,
        default,
        summary,
        since: "2.0",
    }
}

/// Every setting, deck settings first, in the order the reference lists them.
pub const SETTINGS: &[SettingDef] = &[
    def("title", Scope::Deck, Kind::Text, "", "The deck's title"),
    def("author", Scope::Deck, Kind::Text, "", "The deck's author"),
    def(
        "theme",
        Scope::Deck,
        Kind::Theme,
        "dark",
        "The theme: a built-in name, a deck or user theme, or a file",
    ),
    def(
        "engine",
        Scope::Deck,
        Kind::Engine,
        "the theme's",
        "Run the deck on this engine instead of the theme's",
    ),
    def(
        "transition",
        Scope::Both,
        Kind::Choice(TRANSITIONS),
        "the theme's",
        "How slides change; on a slide, how it is entered (`zoom` needs `zoom-to`)",
    ),
    def(
        "slide-level",
        Scope::Deck,
        Kind::Integer(1, 6),
        "inferred",
        "Headings at this level and above start slides",
    ),
    def(
        "countdown",
        Scope::Deck,
        Kind::Choice(&["on", "off"]),
        "the theme's",
        "The engine's opening countdown",
    ),
    def(
        "reveal",
        Scope::Both,
        Kind::Choice(&["steps", "none"]),
        "steps",
        "`none` shows `+` items at once, without steps",
    ),
    def(
        "footer",
        Scope::Deck,
        Kind::Text,
        "",
        "Text at the foot of every slide",
    ),
    def(
        "logo",
        Scope::Both,
        Kind::Path,
        "the theme's",
        "A PNG or SVG logo on every slide; `none` hides it",
    ),
    def(
        "logo-position",
        Scope::Deck,
        Kind::Choice(&["top-left", "top-right", "bottom-left", "bottom-right"]),
        "the theme's",
        "The logo's corner",
    ),
    def(
        "logo-opacity",
        Scope::Deck,
        Kind::Opacity,
        "the theme's",
        "How strongly the logo shows",
    ),
    def(
        "logo-height",
        Scope::Deck,
        Kind::Pixels,
        "the theme's",
        "The logo's height",
    ),
    def(
        "background",
        Scope::Both,
        Kind::Path,
        "",
        "An image behind the slides (png, jpg, webp, svg); `none` for no image",
    ),
    def(
        "background-opacity",
        Scope::Both,
        Kind::Opacity,
        "30%",
        "How strongly the background image shows",
    ),
    def(
        "palette",
        Scope::Deck,
        Kind::Palette,
        "iron",
        "The palette of `@thermal` images that name none",
    ),
    def(
        "art-world",
        Scope::Deck,
        Kind::Text,
        "",
        "The deck's world for `mdeck ai art`: setting, era, recurring characters",
    ),
    def(
        "image-style",
        Scope::Deck,
        Kind::Text,
        "",
        "The style of images made by `mdeck ai generate`",
    ),
    def(
        "icon-style",
        Scope::Deck,
        Kind::Text,
        "",
        "The style of diagram icons made by `mdeck ai generate`",
    ),
    def(
        "design",
        Scope::Slide,
        Kind::Choice(DESIGNS),
        "recognised",
        "The slide's design instead of the recognised one",
    ),
    def(
        "picture",
        Scope::Slide,
        Kind::Text,
        "",
        "The picture on the slide's stage: a point cloud name; `none` keeps it empty",
    ),
    def(
        "picture-prompt",
        Scope::Slide,
        Kind::Text,
        "",
        "What `mdeck ai art` draws for this slide",
    ),
    def(
        "zoom-to",
        Scope::Slide,
        Kind::Text,
        "",
        "Enter this slide by zooming into the named thermal spot of the slide before",
    ),
    def(
        "thermal-window",
        Scope::Slide,
        Kind::Window,
        "",
        "One temperature scale for every `@thermal` image on the slide",
    ),
];

/// The setting called `name`.
pub fn setting(name: &str) -> Option<&'static SettingDef> {
    SETTINGS.iter().find(|s| s.name == name)
}

/// Why `value` is not a valid value of `def`, if it is not.
pub fn invalid_value(def: &SettingDef, value: &str) -> Option<String> {
    let v = value.trim();
    let bad = |what: String| Some(format!("`{}: {v}`: {what}", def.name));
    match def.kind {
        Kind::Text | Kind::Theme | Kind::Engine | Kind::Path => {
            v.is_empty().then(|| format!("`{}` has no value", def.name))
        }
        Kind::Choice(values) => {
            if values.contains(&v) {
                None
            } else {
                let mut msg = format!("expected {}", def.kind.describe());
                if let Some(s) = suggestion(v, values.iter().copied()) {
                    msg.push_str(&format!("; did you mean `{s}`?"));
                }
                bad(msg)
            }
        }
        Kind::Integer(lo, hi) => match v.parse::<u32>() {
            Ok(n) if (lo..=hi).contains(&n) => None,
            _ => bad(format!("expected a whole number from {lo} to {hi}")),
        },
        Kind::Opacity => {
            let n = v
                .strip_suffix('%')
                .map(|p| p.trim().parse::<f32>().map(|p| p / 100.0))
                .unwrap_or_else(|| v.parse::<f32>());
            match n {
                Ok(n) if (0.0..=1.0).contains(&n) => None,
                _ => bad("expected 0 to 1, or a percentage".into()),
            }
        }
        Kind::Pixels => match v.trim_end_matches("px").trim().parse::<f32>() {
            Ok(n) if n > 0.0 => None,
            _ => bad("expected a size in pixels".into()),
        },
        Kind::Palette => match crate::render::thermal::Palette::from_name(v) {
            Some(_) => None,
            None => bad(format!("expected {}", def.kind.describe())),
        },
        Kind::Window => {
            let ok = v
                .split_once("..")
                .is_some_and(|(lo, _)| lo.trim().parse::<f32>().is_ok());
            if ok {
                None
            } else {
                bad("expected `low..high °C`".into())
            }
        }
    }
}

/// What kind of mdeck fence a tag opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FenceKind {
    /// A chart, diagram or thermal image drawn on the slide.
    Visual,
    /// Speaker notes.
    Notes,
}

/// One mdeck fence (```` ```@tag ````).
#[derive(Debug, Clone, Copy)]
pub struct Fence {
    pub tag: &'static str,
    pub kind: FenceKind,
    pub summary: &'static str,
}

const fn visual(tag: &'static str, summary: &'static str) -> Fence {
    Fence {
        tag,
        kind: FenceKind::Visual,
        summary,
    }
}

/// Every mdeck fence tag. Tags match exactly; there are no aliases.
pub const FENCES: &[Fence] = &[
    visual("@bar", "Bar chart"),
    visual("@line", "Line chart"),
    visual("@pie", "Pie chart"),
    visual("@donut", "Donut chart"),
    visual("@scatter", "Scatter plot"),
    visual("@stackedbar", "Stacked bar chart"),
    visual("@funnel", "Funnel chart"),
    visual("@radar", "Radar chart"),
    visual("@progress", "Progress bars"),
    visual("@kpi", "KPI cards"),
    visual("@wordcloud", "Word cloud"),
    visual("@timeline", "Timeline"),
    visual("@gantt", "Gantt chart"),
    visual("@architecture", "Architecture diagram"),
    visual("@orgchart", "Org chart"),
    visual("@gitgraph", "Git branch graph"),
    visual(
        "@flower",
        "A platform in the middle and the teams around it",
    ),
    visual(
        "@artifactflow",
        "Artifacts from producers through services to consumers",
    ),
    visual("@venn", "Venn diagram"),
    visual("@thermal", "Thermal image with lens, reveals and spots"),
    Fence {
        tag: "@notes",
        kind: FenceKind::Notes,
        summary: "Speaker notes, in markdown",
    },
];

/// The fence a code block's info string opens: its first word, exactly.
pub fn fence(info: &str) -> Option<&'static Fence> {
    let tag = info.split_whitespace().next()?;
    FENCES.iter().find(|f| f.tag == tag)
}

/// v1 names and the v2 form that replaces them, for `--check` messages.
/// Frontmatter keys were written `@key`; slide directives `@key: value`.
pub fn v1_replacement(v1_name: &str, in_slide: bool) -> Option<String> {
    let comment = |key: &str| format!("<!-- {key}: ... -->");
    let v2 = match v1_name {
        "layout" => return Some(comment("design")),
        "illustration" => return Some(comment("picture")),
        "art" if in_slide => return Some(comment("picture-prompt")),
        "art" => return Some("`art-world:` in the frontmatter".into()),
        "zoom" => return Some(comment("zoom-to")),
        "story" | "aspect" | "code-theme" | "class" | "date" => {
            return Some("nothing: it was removed in v2".into());
        }
        name => setting(name)?,
    };
    Some(if in_slide && v2.scope.in_slide() {
        comment(v2.name)
    } else {
        format!("`{}:` in the frontmatter", v2.name)
    })
}

/// v1 fence tags renamed in v2.
pub fn v1_fence(tag: &str) -> Option<&'static str> {
    match tag {
        "@barchart" => Some("@bar"),
        "@linechart" => Some("@line"),
        "@piechart" => Some("@pie"),
        "@donutchart" => Some("@donut"),
        _ => None,
    }
}

/// The settings reference as markdown: one table each for deck and slide
/// settings. Used by `mdeck spec --short` and the format reference.
pub fn settings_reference() -> String {
    let mut out = String::new();
    for (title, slide) in [
        ("Deck settings (frontmatter)", false),
        ("Slide settings (<!-- key: value -->)", true),
    ] {
        out.push_str(&format!(
            "### {title}\n\n| Key | Values | Default | Since | |\n|---|---|---|---|---|\n"
        ));
        for s in SETTINGS {
            let fits = if slide {
                s.scope.in_slide()
            } else {
                s.scope.in_deck()
            };
            if !fits {
                continue;
            }
            let default = if s.default.is_empty() { "" } else { s.default };
            out.push_str(&format!(
                "| `{}` | {} | {} | {} | {} |\n",
                s.name,
                s.kind.describe(),
                default,
                s.since,
                s.summary
            ));
        }
        out.push('\n');
    }
    out
}

/// The settings as plain text lines for the quick reference card.
pub fn settings_card() -> String {
    let mut out = String::new();
    for (title, slide) in [
        (
            "DECK SETTINGS (frontmatter, plain YAML: `theme: ember`)",
            false,
        ),
        (
            "SLIDE SETTINGS (an HTML comment in the slide: <!-- design: quote -->)",
            true,
        ),
    ] {
        out.push_str(title);
        out.push('\n');
        for s in SETTINGS {
            let fits = if slide {
                s.scope.in_slide()
            } else {
                s.scope.in_deck()
            };
            if fits {
                out.push_str(&format!("  {:<20} {}\n", s.name, s.summary));
                let mut values = s.kind.describe().replace('`', "");
                if !s.default.is_empty() {
                    values.push_str(&format!("; default {}", s.default));
                }
                out.push_str(&format!("  {:<20} ({values})\n", ""));
            }
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_unique_and_lowercase() {
        for (i, s) in SETTINGS.iter().enumerate() {
            assert!(
                SETTINGS[i + 1..].iter().all(|o| o.name != s.name),
                "{} twice",
                s.name
            );
            assert!(
                s.name.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
                "{}",
                s.name
            );
        }
    }

    #[test]
    fn values_are_validated() {
        let check = |name: &str, value: &str| invalid_value(setting(name).unwrap(), value);
        assert_eq!(check("transition", "fade"), None);
        assert!(check("transition", "wipe").is_some());
        assert!(
            check("transition", "fad")
                .unwrap()
                .contains("did you mean `fade`")
        );
        assert_eq!(check("slide-level", "2"), None);
        assert!(check("slide-level", "7").is_some());
        assert_eq!(check("logo-opacity", "40%"), None);
        assert_eq!(check("logo-opacity", "0.4"), None);
        assert!(check("logo-opacity", "140%").is_some());
        assert_eq!(check("palette", "white-hot"), None);
        assert!(check("palette", "purple").is_some());
        assert_eq!(check("thermal-window", "25..90 °C"), None);
        assert!(check("thermal-window", "hot").is_some());
        assert_eq!(check("design", "statement"), None);
        assert!(check("title", " ").is_some());
    }

    #[test]
    fn fences_match_exactly() {
        assert_eq!(fence("@bar").unwrap().tag, "@bar");
        assert_eq!(fence("@notes").unwrap().kind, FenceKind::Notes);
        assert!(fence("@barchart").is_none());
        assert!(fence("@bars").is_none());
        assert!(fence("@donutchart").is_none());
        assert_eq!(v1_fence("@barchart"), Some("@bar"));
    }

    #[test]
    fn every_visual_fence_parses_to_a_visual() {
        for f in FENCES.iter().filter(|f| f.kind == FenceKind::Visual) {
            let block = crate::parser::blocks::parse(&format!("```{}\n- a: 1\n```", f.tag));
            assert!(
                matches!(
                    block[0],
                    crate::parser::Block::Chart { .. } | crate::parser::Block::Diagram { .. }
                ),
                "{}: {:?}",
                f.tag,
                block[0]
            );
        }
    }

    #[test]
    fn v1_names_point_at_v2() {
        assert_eq!(
            v1_replacement("layout", true).as_deref(),
            Some("<!-- design: ... -->")
        );
        assert_eq!(
            v1_replacement("theme", false).as_deref(),
            Some("`theme:` in the frontmatter")
        );
        assert_eq!(
            v1_replacement("art", false).as_deref(),
            Some("`art-world:` in the frontmatter")
        );
        assert_eq!(
            v1_replacement("logo", true).as_deref(),
            Some("<!-- logo: ... -->")
        );
        assert!(v1_replacement("team", true).is_none());
    }

    #[test]
    fn the_reference_lists_every_setting() {
        let md = settings_reference();
        for s in SETTINGS {
            assert!(md.contains(&format!("`{}`", s.name)), "{}", s.name);
        }
        let card = settings_card();
        assert!(card.contains("zoom-to"));
    }
}
