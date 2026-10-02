//! The theme file's keys as one table (THM-17): `mdeck theme new` writes its
//! commented starter from it, and a test checks the table against
//! [`super::file::ThemeFile`] field by field, so the starter cannot drift
//! from the format.

use super::EngineId;

/// One key of the starter: its path (`colors.accent`), an example value as
/// YAML, what it does, and whether the starter writes it uncommented.
pub struct Key {
    pub path: &'static str,
    pub example: &'static str,
    pub note: &'static str,
    pub on: bool,
}

const fn key(path: &'static str, example: &'static str, note: &'static str, on: bool) -> Key {
    Key {
        path,
        example,
        note,
        on,
    }
}

/// Every key of a theme file except `name` and the engine block (written
/// per engine from [`engine_settings`]) and `arrangements` (an example).
pub const KEYS: &[Key] = &[
    key(
        "extends",
        "dark",
        "unset keys come from this theme (default: dark)",
        true,
    ),
    key(
        "variant-of",
        "dark",
        "a recolouring of this theme: listed after the themes",
        false,
    ),
    key(
        "designs",
        "standard",
        "standard | editorial: how every slide design looks",
        false,
    ),
    key(
        "countdown",
        "off",
        "on | off: open with a 3-2-1 countdown (the engine decides its look)",
        false,
    ),
    key(
        "transition",
        "fade",
        "slide | fade | spatial | none (a deck's own wins)",
        false,
    ),
    key(
        "radius",
        "8",
        "corner radius of code, tables and callouts, px at 1920x1080",
        false,
    ),
    key(
        "spacing.xs",
        "8",
        "the spacing scale arrangements name, px at 1920x1080",
        false,
    ),
    key("spacing.sm", "16", "", false),
    key("spacing.md", "24", "", false),
    key("spacing.lg", "40", "", false),
    key("spacing.xl", "64", "", false),
    key("colors.background", "\"#1e1e1e\"", "slide background", true),
    key("colors.text", "\"#c8c8c8\"", "body text", true),
    key("colors.heading", "\"#ffffff\"", "headings", true),
    key(
        "colors.accent",
        "\"#5294e2\"",
        "links, quote bars, highlights",
        true,
    ),
    key(
        "colors.muted",
        "\"#8f8f8f\"",
        "captions, eyebrows, slide numbers, chart axes",
        false,
    ),
    key("colors.strong", "\"#ffffff\"", "**bold** text", false),
    key(
        "colors.rule",
        "\"#3a3a3a\"",
        "hairlines and chart grids",
        false,
    ),
    key(
        "colors.accent-soft",
        "\"#8fb8ee\"",
        "lighter accent: editorial emphasis, glows",
        false,
    ),
    key(
        "colors.secondary",
        "\"#e8a838\"",
        "a second, rarer highlight",
        false,
    ),
    key("colors.code-background", "\"#2d2d2d\"", "", false),
    key("colors.code-text", "\"#d4d4d4\"", "", false),
    key("colors.positive", "\"#5cdb95\"", "", false),
    key("colors.negative", "\"#ff6b6b\"", "", false),
    key(
        "colors.series",
        "[\"#5cb8ff\", \"#ff7e67\", \"#5cdb95\", \"#e8a838\"]",
        "chart colours, cycled",
        false,
    ),
    key(
        "annotations.pen",
        "\"#50c8ff\"",
        "the presenter's pen",
        false,
    ),
    key("annotations.pen-outline", "\"#1e82b4\"", "", false),
    key("annotations.arrow", "\"#ffc832\"", "", false),
    key("annotations.arrow-outline", "\"#c88c00\"", "", false),
    key(
        "fonts.display",
        "sans",
        "a bundled face or a .ttf/.otf file in this folder:",
        false,
    ),
    key(
        "fonts.body",
        "sans",
        "sans, mono, spectral-light, hanken-light,",
        false,
    ),
    key(
        "fonts.lead",
        "sans",
        "hanken-regular, hanken-medium, jetbrains-mono",
        false,
    ),
    key("fonts.strong", "sans", "", false),
    key("fonts.mono", "mono", "", false),
    key("sizes.h1", "96", "px at 1920x1080", false),
    key("sizes.h2", "72", "", false),
    key("sizes.h3", "52", "", false),
    key("sizes.body", "44", "", false),
    key("sizes.code", "30", "", false),
    key(
        "text.line-height",
        "1.4",
        "a multiple of the font size",
        false,
    ),
    key("charts.fill-opacity", "0.85", "", false),
    key(
        "code.syntax",
        "base16-ocean.dark",
        "a bundled syntax theme or a .tmTheme file in this folder",
        false,
    ),
    key(
        "logo.file",
        "logo.svg",
        "a PNG or SVG in this folder, in a corner of every slide",
        false,
    ),
    key(
        "logo.position",
        "top-right",
        "top-left | top-right | bottom-left | bottom-right",
        false,
    ),
    key("logo.height", "56", "px at 1920x1080", false),
    key("logo.opacity", "0.6", "", false),
    key(
        "page.surface",
        "\"#2a2a2a\"",
        "the slide as a sheet on a surface: the colour around it",
        false,
    ),
    key("page.margin", "56", "px at 1920x1080, 0 to 300", false),
    key("page.shadow", "0.5", "0 to 1", false),
    key("page.grain", "0.5", "0 to 1", false),
    key("page.radius", "6", "px at 1920x1080", false),
];

/// An example value and note for each engine setting.
pub fn engine_setting(key: &str) -> (&'static str, &'static str) {
    match key {
        "light" => ("\"#d7d7e1\"", "the brightest tint the engine draws in"),
        "cool" => ("\"#afc3f0\"", "a cool tint besides the accents"),
        "palette" => (
            "iron",
            "iron | white-hot | black-hot | rainbow | arctic | lava",
        ),
        "drift" => ("false", "embers drift on ordinary slides"),
        "surface" => ("sheet", "sheet | slate"),
        "kind" => (
            "line",
            "line | tonal: what kind of picture generated art is",
        ),
        "style" => ("\"ink on white\"", "the style prompt for generated art"),
        "references" => ("[swatch.png]", "style swatches in this folder"),
        _ => ("", ""),
    }
}

/// Engines in the order the starter lists their settings (the built-ins
/// this build has).
fn engines() -> Vec<EngineId> {
    [
        "plain",
        "particles",
        "led",
        "splitflap",
        "blocks",
        "thermal",
        "line",
        "sketch",
        "watercolour",
        "darkroom",
    ]
    .into_iter()
    .filter_map(EngineId::find)
    .collect()
}

/// The starter `mdeck theme new` writes: every key, commented except the
/// few a theme always sets. `all` writes every key uncommented (tests).
pub fn starter(name: &str, all: bool) -> String {
    let mut out = format!(
        "# {name}: a custom MDeck theme. Every key is optional; unset keys come\n\
         # from the theme named by `extends`. See `mdeck spec`, section 9.4.\n\
         # Check it with `mdeck theme check {name}` and look at it with\n\
         # `mdeck theme preview {name} --output-dir /tmp/{name}`.\n\
         name: {name}\n"
    );
    let line = |indent: &str, k: &str, v: &str, note: &str, on: bool, open: bool| {
        // a commented key in an open section keeps the section's indent
        let (hash, indent) = match (on || all, open && !indent.is_empty()) {
            (true, _) => ("", indent.to_string()),
            (false, true) => ("", format!("{indent}# ")),
            (false, false) => ("# ", indent.to_string()),
        };
        let body = format!("{indent}{k}: {v}");
        if note.is_empty() {
            format!("{hash}{body}\n")
        } else {
            format!("{hash}{body:<34} # {note}\n")
        }
    };
    let mut section = "";
    let mut emit_engine = true;
    for k in KEYS {
        let (sec, leaf) = k.path.split_once('.').unwrap_or(("", k.path));
        // the engine block goes after the top-level keys
        if !sec.is_empty() && emit_engine {
            out.push_str(&engine_block(all));
            emit_engine = false;
        }
        let open = all
            || KEYS
                .iter()
                .any(|x| x.on && x.path.starts_with(&format!("{sec}.")));
        if sec != section && !sec.is_empty() {
            out.push_str(&format!("{}{sec}:\n", if open { "" } else { "# " }));
        }
        section = sec;
        let indent = if sec.is_empty() { "" } else { "  " };
        out.push_str(&line(indent, leaf, k.example, k.note, k.on, open));
    }
    out.push_str(ARRANGEMENTS);
    if all {
        out = out.replace("# arrangements:", "arrangements:");
        out = out.replace("#   ", "  ");
    }
    out
}

/// The engine block: the engine's name, then each engine's settings.
fn engine_block(all: bool) -> String {
    let hash = if all { "" } else { "# " };
    let mut out = format!(
        "{hash}engine:                           # the engine and its settings (or `engine: plain`)\n\
         {hash}  name: {}\n",
        if all {
            "line"
        } else {
            "plain     # plain | particles | led | splitflap | blocks | thermal | line | sketch | watercolour | darkroom"
        }
    );
    // every setting once, grouped by the engines that read it
    let mut keys: Vec<&str> = Vec::new();
    for engine in engines() {
        for k in super::validate::engine_keys(engine) {
            if !keys.contains(&k) {
                keys.push(k);
            }
        }
    }
    let readers = |k: &str| -> Vec<&'static str> {
        engines()
            .into_iter()
            .filter(|e| super::validate::engine_keys(*e).contains(&k))
            .map(|e| e.name())
            .collect()
    };
    let mut group: Vec<&str> = Vec::new();
    for k in keys {
        let r = readers(k);
        if r != group {
            out.push_str(&format!("{hash}  # read by {}:\n", r.join(", ")));
            group = r;
        }
        let (v, note) = engine_setting(k);
        let body = format!("  {k}: {v}");
        out.push_str(&format!("{hash}{body:<34} # {note}\n"));
    }
    out
}

/// A short arrangement override, commented (the design sets are documented
/// in `docs/themes.md`).
const ARRANGEMENTS: &str = "\
# arrangements:                    # how each slide design looks, over the design set
#   quote:
#     roles: { attribution: { color: accent } }
#     ornaments: { quote-bar: none }
#   all:
#     ornaments: { bullet: \"◆\" }
";

#[cfg(test)]
mod tests {
    use super::super::file::{EngineBlock, ThemeFile};
    use super::*;

    /// Every key of the theme file, set: destructured without `..`, so a new
    /// field fails to compile here until the starter knows it.
    fn unset(f: &ThemeFile) -> Vec<&'static str> {
        let ThemeFile {
            name,
            extends,
            variant_of,
            engine,
            countdown,
            transition,
            designs,
            arrangements,
            spacing,
            radius,
            colors,
            annotations,
            fonts,
            sizes,
            text,
            charts,
            code,
            logo,
            page,
            moved_particles: _,
            moved_heat: _,
            moved_art: _,
            moved_surface: _,
        } = f;
        let mut out = Vec::new();
        let mut need = |k: &'static str, set: bool| {
            if !set {
                out.push(k)
            }
        };
        need("name", name.is_some());
        need("extends", extends.is_some());
        need("variant-of", variant_of.is_some());
        need(
            "engine",
            engine
                .as_ref()
                .is_some_and(|e: &EngineBlock| e.name.is_some()),
        );
        need("countdown", countdown.is_some());
        need("transition", transition.is_some());
        need("designs", designs.is_some());
        need("arrangements", arrangements.is_some());
        need("radius", radius.is_some());
        let s = spacing;
        need(
            "spacing",
            [s.xs, s.sm, s.md, s.lg, s.xl].iter().all(Option::is_some),
        );
        let c = colors;
        need(
            "colors",
            [
                &c.background,
                &c.text,
                &c.heading,
                &c.muted,
                &c.strong,
                &c.rule,
                &c.accent,
                &c.accent_soft,
                &c.secondary,
                &c.code_background,
                &c.code_text,
                &c.positive,
                &c.negative,
            ]
            .iter()
            .all(|v| v.is_some())
                && c.series.is_some(),
        );
        let a = annotations;
        need(
            "annotations",
            [&a.pen, &a.pen_outline, &a.arrow, &a.arrow_outline]
                .iter()
                .all(|v| v.is_some()),
        );
        let fo = fonts;
        need(
            "fonts",
            [&fo.display, &fo.body, &fo.lead, &fo.strong, &fo.mono]
                .iter()
                .all(|v| v.is_some()),
        );
        let z = sizes;
        need(
            "sizes",
            [z.h1, z.h2, z.h3, z.body, z.code]
                .iter()
                .all(Option::is_some),
        );
        need("text", text.line_height.is_some());
        need("charts", charts.fill_opacity.is_some());
        need("code", code.syntax.is_some());
        need(
            "logo",
            logo.file.is_some()
                && logo.position.is_some()
                && logo.height.is_some()
                && logo.opacity.is_some(),
        );
        need(
            "page",
            page.surface.is_some()
                && page.margin.is_some()
                && page.shadow.is_some()
                && page.grain.is_some()
                && page.radius.is_some(),
        );
        out
    }

    #[test]
    fn the_starter_names_every_key_of_the_format() {
        let full = ThemeFile::parse(&starter("acme", true)).unwrap();
        assert_eq!(unset(&full), Vec::<&str>::new());
        // and every setting of every engine
        let block = full.engine_block();
        for engine in engines() {
            for k in super::super::validate::engine_keys(engine) {
                assert!(block.get(k).is_some(), "engine.{k} ({})", engine.name());
                assert!(!engine_setting(k).0.is_empty(), "{k}");
            }
        }
    }

    #[test]
    fn the_starter_mostly_comments() {
        let s = starter("acme", false);
        let f = ThemeFile::parse(&s).unwrap();
        assert_eq!(f.name.as_deref(), Some("acme"));
        assert!(f.engine.is_none() && f.designs.is_none());
        assert!(s.contains("# designs: standard"));
        // engine settings come from the engines built in
        if cfg!(feature = "thermal") {
            assert!(s.contains("#   palette: iron"));
        }
        assert!(!s.contains('\u{2014}'));
    }
}
