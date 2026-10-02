//! Arrangements (DES-09..DES-13): how each slide design looks in a theme.
//!
//! A design set (`crates/mdeck/designs/<set>.yaml`) gives an arrangement for
//! every design: a `base` every design starts from, then each design's own
//! keys. A theme picks a set with `designs:` and overrides any key with
//! `arrangements: { <design>: { ... } }`; `extends` merges those overrides
//! key by key. Everything here is data: the renderer in
//! `render::designs` draws whatever the arrangement says.

use std::collections::HashMap;
use std::sync::Arc;

use serde::Deserialize;
use serde_norway::Value;

use super::ThemeError;
use crate::parser::Design;

/// The built-in design sets.
pub const SETS: &[(&str, &str)] = &[
    ("standard", include_str!("../../designs/standard.yaml")),
    ("editorial", include_str!("../../designs/editorial.yaml")),
];

/// The design set a theme gets when it names none.
pub const DEFAULT_SET: &str = "standard";

/// The names of the built-in sets, for messages.
pub fn set_names() -> Vec<&'static str> {
    SETS.iter().map(|(n, _)| *n).collect()
}

/// A rectangle in fractions of the slide: `[x, y, width, height]`.
pub type Frac = [f32; 4];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum HAlign {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum VAlign {
    Top,
    #[default]
    Middle,
    Bottom,
}

/// Where a design leaves room for the engine's picture of the slide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Stage {
    /// No room: the content fills the slide.
    #[default]
    None,
    /// The right side of the slide (the copy sits on the left).
    Right,
    /// The left side of the slide.
    Left,
    /// Behind the copy (a title page's backdrop).
    Backdrop,
}

/// The line over the copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Eyebrow {
    #[default]
    None,
    /// The slide's Roman numeral in the accent, then the deck title.
    Numeral,
    /// The author and the deck title (a title page).
    Deck,
}

/// How copy enters when the slide is entered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Motion {
    /// Shown at once (the transition moves the slide).
    #[default]
    None,
    /// Everything fades in together.
    Fade,
    /// Everything fades in and rises into place together.
    Rise,
    /// Each element fades and rises in turn.
    Stagger,
}

/// How a `+` item revealed with Next comes in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum RevealMotion {
    /// Slides in from the left.
    #[default]
    Slide,
    /// Rises into place.
    Rise,
    /// Fades in where it stands.
    Fade,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case", default)]
pub struct Entry {
    pub kind: Motion,
    /// Between two elements of a stagger.
    pub step_ms: f32,
    /// How long one element takes to settle.
    pub duration_ms: f32,
    /// How far an element rises, px at 1920x1080.
    pub rise: f32,
    pub reveal: RevealMotion,
    /// How long a revealed item takes to settle.
    pub reveal_ms: f32,
}

/// A region of the slide and how content sits in it.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Region {
    pub region: Frac,
    #[serde(default)]
    pub align: HAlign,
    #[serde(default)]
    pub valign: VAlign,
}

impl Default for Region {
    fn default() -> Self {
        Region {
            region: [0.0, 0.0, 1.0, 1.0],
            align: HAlign::Left,
            valign: VAlign::Middle,
        }
    }
}

/// Where the plate (the design's image, code, table, visual or columns) goes
/// relative to the copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Place {
    /// Under the copy, in the plate's columns, down to the plate's bottom.
    #[default]
    Below,
    /// In its own region beside the copy.
    Beside,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Plate {
    pub region: Frac,
    #[serde(default)]
    pub place: Place,
    #[serde(default)]
    pub valign: VAlign,
    /// Between gallery cells, columns or stacked visuals.
    #[serde(default = "default_gap")]
    pub gap: Space,
    /// A hairline above each column (columns) or under the plate's caption.
    #[serde(default)]
    pub rule: bool,
}

fn default_gap() -> Space {
    Space::Token("md".into())
}

/// A size: a theme size token (`h1`, `h2`, `h3`, `body`, `code`, or `level`
/// for a heading's own level) or px at 1920x1080.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Size {
    Px(f32),
    Token(String),
}

impl Default for Size {
    fn default() -> Self {
        Size::Token("body".into())
    }
}

/// A gap: a theme spacing token (`xs`, `sm`, `md`, `lg`, `xl`) or px at
/// 1920x1080.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Space {
    Px(f32),
    Token(String),
}

impl Default for Space {
    fn default() -> Self {
        Space::Px(0.0)
    }
}

/// The font roles a theme names faces for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Font {
    Display,
    #[default]
    Body,
    Lead,
    Strong,
    Mono,
}

/// Colour roles of the theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Ink {
    #[default]
    Text,
    Heading,
    Muted,
    Strong,
    Accent,
    AccentSoft,
    Secondary,
    /// Text one step brighter (between text and heading).
    Bright,
    Rule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Case {
    #[default]
    None,
    Upper,
    Lower,
}

/// How `*emphasis*` shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Emphasis {
    #[default]
    Italic,
    /// In the soft accent, upright.
    Accent,
}

/// How one role's text looks.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case", default)]
pub struct RoleStyle {
    pub font: Font,
    pub size: Size,
    /// Multiplies the size.
    pub scale: f32,
    pub color: Ink,
    pub opacity: f32,
    pub case: Case,
    /// Letter spacing as a fraction of the size.
    pub tracking: f32,
    /// Line height as a multiple of the size (the theme's when unset).
    pub line_height: Option<f32>,
    /// Space after the element.
    pub gap: Space,
    /// Overrides the region's alignment for this role.
    pub align: Option<HAlign>,
    /// Drawn slanted.
    pub italic: bool,
}

impl Default for RoleStyle {
    fn default() -> Self {
        RoleStyle {
            font: Font::Body,
            size: Size::default(),
            scale: 1.0,
            color: Ink::Text,
            opacity: 1.0,
            case: Case::None,
            tracking: 0.0,
            line_height: None,
            gap: Space::default(),
            align: None,
            italic: false,
        }
    }
}

/// The named places content goes (DES-01). Every set defines every role in
/// its `base`.
#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case", default)]
pub struct Roles {
    pub eyebrow: RoleStyle,
    pub title: RoleStyle,
    pub subtitle: RoleStyle,
    pub kicker: RoleStyle,
    pub byline: RoleStyle,
    /// Headings after the title.
    pub heading: RoleStyle,
    pub statement: RoleStyle,
    pub lead: RoleStyle,
    pub body: RoleStyle,
    pub list: RoleStyle,
    /// Nested list items.
    pub nested: RoleStyle,
    pub quote: RoleStyle,
    pub attribution: RoleStyle,
    pub caption: RoleStyle,
}

/// Where a quote's bar goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Bar {
    #[default]
    None,
    Left,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case", default)]
pub struct Ornaments {
    /// The bullet before list items: a glyph, or `dot` for a drawn dot.
    pub bullet: String,
    pub bullet_color: Ink,
    /// How far list items step in from the copy edge, px at 1920x1080.
    pub indent: f32,
    /// How much further each nesting level steps in.
    pub nested_indent: f32,
    /// Space between list items.
    pub item_gap: Space,
    /// Ordered lists show their numbers (in the bullet colour).
    pub numbering: bool,
    /// Curly quotation marks around a quote.
    pub quote_marks: bool,
    pub quote_bar: Bar,
    pub bar_color: Ink,
    /// Bar width, px at 1920x1080.
    pub bar_width: f32,
    /// A short accent hairline under the title.
    pub title_rule: bool,
    pub emphasis: Emphasis,
    /// A soft dark pillow behind the copy, so it reads over a picture.
    pub pillow: bool,
    /// A quote's attribution gets a typographic dash in front.
    pub attribution_dash: bool,
    /// The deck's first slide, live, shows a quiet "Space to begin".
    pub begin_hint: bool,
}

impl Default for Ornaments {
    fn default() -> Self {
        Ornaments {
            bullet: "\u{2022}".into(),
            bullet_color: Ink::Text,
            indent: 45.0,
            nested_indent: 30.0,
            item_gap: Space::Px(8.0),
            numbering: true,
            quote_marks: true,
            quote_bar: Bar::Left,
            bar_color: Ink::Accent,
            bar_width: 4.0,
            title_rule: false,
            emphasis: Emphasis::Italic,
            pillow: false,
            attribution_dash: false,
            begin_hint: false,
        }
    }
}

/// How one design looks in one theme.
#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case", default)]
pub struct Arrangement {
    /// Where the copy (the text roles) goes.
    pub copy: Region,
    /// Where the copy goes on a slide that holds a wide block (an image,
    /// code, a table or a visual) in its copy; the stage is then given up.
    pub wide: Option<Region>,
    /// Where the plate goes, for designs that have one.
    pub plate: Option<Plate>,
    pub stage: Stage,
    pub eyebrow: Eyebrow,
    /// A title page shows the deck's author under the subtitle.
    pub byline: bool,
    pub entry: Entry,
    pub roles: Roles,
    pub ornaments: Ornaments,
}

/// Every design's arrangement in a theme.
#[derive(Debug, Clone, PartialEq)]
pub struct Arrangements {
    /// The design set the theme names.
    pub set: String,
    by_design: HashMap<Design, Arrangement>,
}

impl Arrangements {
    pub fn get(&self, design: Design) -> &Arrangement {
        &self.by_design[&design]
    }

    /// Whether the set is the editorial one.
    pub fn is_editorial(&self) -> bool {
        self.set == "editorial"
    }

    /// `set`'s arrangements with `overrides` (the theme's merged
    /// `arrangements:` mapping) applied key by key.
    pub fn resolve(set: &str, overrides: Option<&Value>) -> Result<Arc<Self>, ThemeError> {
        let source = SETS
            .iter()
            .find(|(n, _)| *n == set)
            .map(|(_, s)| *s)
            .ok_or_else(|| {
                ThemeError::invalid(
                    "designs",
                    format!("'{set}' is not a design set ({})", set_names().join(", ")),
                )
            })?;
        let file: Value = serde_norway::from_str(source)
            .map_err(|e| ThemeError::invalid("designs", format!("built-in set {set}: {e}")))?;
        let base = file.get("base").cloned().unwrap_or(Value::Null);
        let designs = file.get("designs").cloned().unwrap_or(Value::Null);
        if let Some(Value::Mapping(map)) = overrides {
            for key in map.keys() {
                let name = key.as_str().unwrap_or_default();
                if Design::from_name(name).is_none() && name != "all" {
                    return Err(ThemeError::invalid(
                        "arrangements",
                        format!(
                            "'{name}' is not a design ({}, or all)",
                            crate::language::DESIGNS.join(", ")
                        ),
                    ));
                }
            }
        } else if overrides.is_some_and(|v| !v.is_null()) {
            return Err(ThemeError::invalid(
                "arrangements",
                "must be a mapping of design names".to_string(),
            ));
        }
        let mut by_design = HashMap::new();
        for design in Design::ALL {
            let mut v = base.clone();
            merge(&mut v, designs.get(design.name()).unwrap_or(&Value::Null));
            if let Some(o) = overrides {
                merge(&mut v, o.get("all").unwrap_or(&Value::Null));
                merge(&mut v, o.get(design.name()).unwrap_or(&Value::Null));
            }
            let a: Arrangement = serde_norway::from_value(v).map_err(|e| {
                ThemeError::invalid(format!("arrangements.{}", design.name()), e.to_string())
            })?;
            check(&a).map_err(|reason| {
                ThemeError::invalid(format!("arrangements.{}", design.name()), reason)
            })?;
            by_design.insert(design, a);
        }
        Ok(Arc::new(Arrangements {
            set: set.to_string(),
            by_design,
        }))
    }
}

/// Values serde cannot rule out: fractions, tokens, ranges.
fn check(a: &Arrangement) -> Result<(), String> {
    let frac = |what: &str, r: &Frac| {
        let ok = r.iter().all(|v| (0.0..=1.0).contains(v))
            && r[0] + r[2] <= 1.0001
            && r[1] + r[3] <= 1.0001
            && r[2] > 0.0
            && r[3] > 0.0;
        if ok {
            Ok(())
        } else {
            Err(format!(
                "{what}.region {r:?} must lie inside the slide: [x, y, width, height] in fractions 0 to 1"
            ))
        }
    };
    frac("copy", &a.copy.region)?;
    if let Some(w) = &a.wide {
        frac("wide", &w.region)?;
    }
    if let Some(p) = &a.plate {
        frac("plate", &p.region)?;
        space_ok("plate.gap", &p.gap)?;
    }
    let r = &a.roles;
    for (name, style) in [
        ("eyebrow", &r.eyebrow),
        ("title", &r.title),
        ("subtitle", &r.subtitle),
        ("kicker", &r.kicker),
        ("byline", &r.byline),
        ("heading", &r.heading),
        ("statement", &r.statement),
        ("lead", &r.lead),
        ("body", &r.body),
        ("list", &r.list),
        ("nested", &r.nested),
        ("quote", &r.quote),
        ("attribution", &r.attribution),
        ("caption", &r.caption),
    ] {
        let key = format!("roles.{name}");
        match &style.size {
            Size::Token(t) if !SIZE_TOKENS.contains(&t.as_str()) => {
                return Err(format!(
                    "{key}.size: '{t}' is not {} or a number of px",
                    SIZE_TOKENS.join(", ")
                ));
            }
            Size::Px(p) if !(1.0..=1000.0).contains(p) => {
                return Err(format!("{key}.size: {p} must be 1 to 1000 px"));
            }
            _ => {}
        }
        if !(0.05..=10.0).contains(&style.scale) {
            return Err(format!("{key}.scale: {} must be 0.05 to 10", style.scale));
        }
        if !(0.0..=1.0).contains(&style.opacity) {
            return Err(format!("{key}.opacity: {} must be 0 to 1", style.opacity));
        }
        if let Some(lh) = style.line_height
            && !(0.5..=4.0).contains(&lh)
        {
            return Err(format!("{key}.line-height: {lh} must be 0.5 to 4"));
        }
        space_ok(&format!("{key}.gap"), &style.gap)?;
    }
    space_ok("ornaments.item-gap", &a.ornaments.item_gap)?;
    if a.ornaments.bullet.chars().count() > 3 {
        return Err(format!(
            "ornaments.bullet: '{}' must be a glyph (up to 3 characters) or dot",
            a.ornaments.bullet
        ));
    }
    Ok(())
}

/// Size tokens an arrangement may name.
pub const SIZE_TOKENS: [&str; 6] = ["h1", "h2", "h3", "body", "code", "level"];

fn space_ok(key: &str, s: &Space) -> Result<(), String> {
    match s {
        Space::Token(t)
            if !super::spacing::TOKENS.contains(&t.as_str())
                && super::spacing::em_factor(t).is_none() =>
        {
            Err(format!(
                "{key}: '{t}' is not {}, a number of px or a multiple of the size (0.5em)",
                super::spacing::TOKENS.join(", ")
            ))
        }
        Space::Px(p) if !(0.0..=1000.0).contains(p) => {
            Err(format!("{key}: {p} must be 0 to 1000 px"))
        }
        _ => Ok(()),
    }
}

/// Merge `over` into `base` key by key: mappings merge, anything else in
/// `over` replaces (lists included). A null in `over` changes nothing.
pub fn merge(base: &mut Value, over: &Value) {
    match (base, over) {
        (_, Value::Null) => {}
        (Value::Mapping(b), Value::Mapping(o)) => {
            for (k, v) in o {
                match b.get_mut(k) {
                    Some(slot) => merge(slot, v),
                    None => {
                        b.insert(k.clone(), v.clone());
                    }
                }
            }
        }
        (b, o) => *b = o.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn yaml(s: &str) -> Value {
        serde_norway::from_str(s).unwrap()
    }

    #[test]
    fn every_set_arranges_every_design() {
        for (set, _) in SETS {
            let a = Arrangements::resolve(set, None).unwrap_or_else(|e| panic!("{set}: {e}"));
            for d in Design::ALL {
                let _ = a.get(d);
            }
        }
    }

    #[test]
    fn the_sets_differ_where_it_counts() {
        let s = Arrangements::resolve("standard", None).unwrap();
        let e = Arrangements::resolve("editorial", None).unwrap();
        assert!(!s.is_editorial() && e.is_editorial());
        let points = (s.get(Design::Points), e.get(Design::Points));
        assert_eq!(points.0.stage, Stage::None);
        assert_eq!(points.1.stage, Stage::Right);
        assert_eq!(points.0.entry.kind, Motion::None);
        assert_eq!(points.1.entry.kind, Motion::Stagger);
        assert_eq!(points.1.eyebrow, Eyebrow::Numeral);
        // DES-10a: editorial arranges image, code, table and chart slides
        // itself: an eyebrow and the entry stagger on every one
        for d in [
            Design::Media,
            Design::Code,
            Design::Table,
            Design::Visual,
            Design::Split,
        ] {
            assert_eq!(e.get(d).eyebrow, Eyebrow::Numeral, "{d:?}");
            assert_eq!(e.get(d).entry.kind, Motion::Stagger, "{d:?}");
        }
    }

    #[test]
    fn overrides_merge_key_by_key() {
        let o = yaml(
            "quote: { roles: { attribution: { color: accent } }, ornaments: { quote-bar: none } }\n\
             all: { ornaments: { bullet: '◆' } }\n",
        );
        let a = Arrangements::resolve("editorial", Some(&o)).unwrap();
        let base = Arrangements::resolve("editorial", None).unwrap();
        let q = a.get(Design::Quote);
        assert_eq!(q.roles.attribution.color, Ink::Accent);
        assert_eq!(q.ornaments.quote_bar, Bar::None);
        // untouched keys keep the set's values
        assert_eq!(
            q.roles.attribution.size,
            base.get(Design::Quote).roles.attribution.size
        );
        assert_eq!(q.copy, base.get(Design::Quote).copy);
        assert_eq!(a.get(Design::Points).ornaments.bullet, "◆");
    }

    #[test]
    fn bad_overrides_name_their_key() {
        let bad = |s: &str| {
            Arrangements::resolve("standard", Some(&yaml(s)))
                .unwrap_err()
                .to_string()
        };
        assert!(bad("sideways: {}").contains("not a design"));
        assert!(bad("quote: { colour: red }").contains("arrangements.quote"));
        assert!(bad("title: { copy: { region: [0.5, 0, 0.8, 1] } }").contains("inside the slide"));
        assert!(bad("title: { roles: { title: { size: huge } } }").contains("roles.title.size"));
        assert!(bad("title: { roles: { title: { gap: roomy } } }").contains("roles.title.gap"));
        assert!(bad("[1, 2]").contains("mapping"));
        assert!(
            Arrangements::resolve("baroque", None)
                .unwrap_err()
                .to_string()
                .contains("design set")
        );
    }

    #[test]
    fn merge_replaces_scalars_and_lists_and_merges_maps() {
        let mut b = yaml("a: { x: 1, y: [1, 2] }\nb: 2\n");
        merge(&mut b, &yaml("a: { y: [3] }\nc: 4\n"));
        assert_eq!(b, yaml("a: { x: 1, y: [3] }\nb: 2\nc: 4\n"));
    }
}
