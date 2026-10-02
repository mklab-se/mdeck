const SPEC: &str = include_str!("../../doc/mdeck-spec.md");

pub fn run(short: bool) {
    if short {
        print_short_reference();
    } else {
        println!("{}", full_reference());
    }
}

/// The format reference, with its settings tables generated from the
/// language table.
pub fn full_reference() -> String {
    SPEC.replace(
        SETTINGS_MARKER,
        crate::language::settings_reference().trim_end(),
    )
    .replace(
        DESIGNS_MARKER,
        crate::parser::design::rules_reference().trim_end(),
    )
}

/// Where the format reference takes the design catalogue and the
/// recognition table.
const DESIGNS_MARKER: &str = "<!-- generated: designs -->";

/// Where the format reference takes the generated settings tables.
const SETTINGS_MARKER: &str = "<!-- generated: settings -->";

fn print_short_reference() {
    print!("{}", short_reference());
}

/// The quick reference card up to its settings section.
const CARD_HEAD: &str = r#"MDeck Quick Reference
=====================

SLIDES
  # Heading        A heading at the slide level starts a new slide (ATX or
                   setext). Level: 2 with zero or one H1, else 1
  ---              Explicit break (blank lines above and below)
  # Title + ## Sub An H2 directly under a lone H1 is its subtitle

"#;

/// The quick reference card between its settings and its keyboard section.
const CARD_MIDDLE: &str = r#"DESIGNS (recognised from the content, or chosen with <!-- design: name -->)
  title        H1 + one short line (the first slide's lone H1 too)
  section      A lone heading, or a heading + a deeper one
  statement    A heading + 1 or 2 short paragraphs
  points       A heading + one list
  split        One image + text
  media        One image (lead and caption allowed)
  gallery      2+ images
  quote        One quote + attribution
  code         One code block + heading/short paragraph
  visual       One chart or diagram + heading/short paragraph
  columns      Columns split by +++
  table        One table + heading/short paragraph
  content      Anything else, in reading order (nothing is ever dropped)
  mdeck --check -v prints each slide's design and the rule that matched

STEPS (list markers)
  -  *  Static (always visible)
  +     Its own step; its children appear with it
  Steps count across the slide in reading order, visuals included.
  `reveal: none` (deck or slide) shows everything at once.

MATH (LaTeX, KaTeX syntax)
  $E = mc^2$            Inline, on the text baseline
  $$\frac{a}{b}$$       Display: own line, centred
  \$5                   Literal dollar sign ($5 and $10 stay text anyway)

IMAGE OPTIONS (in alt text: ![Team @width: 60%](team.jpg))
  @width: 60%           Width: % of the image's space, or px (400, 400px)
  @height: 400px        Height; with both, the image fits both
  @fill                 Cover the space, cropping (a media slide: the whole slide)

PICTURES (on the design's stage; the standard set opens one when a slide sets a picture)
  <!-- picture: name -->  The slide's artwork on an art engine, else the point
                        cloud of that name, else an image file of that path
                        (drawn by mdeck on every engine but splitflap)
                        clouds on plain: a stipple of accent dots, drawn by mdeck
                        clouds: deck.assets/point-clouds > deck point-clouds/ >
                        user folder point-clouds/ > packs > built-in
  <!-- picture: none -->  Keep the stage empty
  mdeck ai point-cloud deck.md   Generate the deck's missing names (deck.assets/)
  mdeck ai point-cloud --name NAME --description "..."   A library cloud via AI
  mdeck point-cloud import IMAGE --name NAME | list | show NAME | contribute NAME

ART ENGINES (line, sketch, watercolour, darkroom; spec 9.7)
  mdeck ai pictures deck.md  Draw a picture per slide (--slide N, --stale, --force,
                        --dry-run, --engine, --node); kept in deck.assets/
  S                     While presenting: draw this slide's picture
  Without art           The slide's picture (point cloud) is drawn in the medium

THERMAL IMAGES (```@thermal; spec 14.20)
  image: file.png       Grayscale, white-hot (brighter is hotter)
  data: file.png        Temperature data, with file.yaml (unit, scale, offset)
  visible: photo.jpg    Registered photo for the lens
  mapping: linear 18..92 °C   Gray levels as values; window: 40..90 °C
  palette: iron   label: text   polarity: black-hot
  + lens 76% 43% 16%    Lens over the photo; + reveal fills the frame
  + above 85%           Colour only the hottest (or: above 60 °C)
  - spot Name 76% 43%   Crosshair; sampled value, or ": text" (marked †)
  Ordinary images are never treated as thermal. C / Shift+C: palettes

KEYBOARD & MOUSE
"#;

/// The quick reference card after its keyboard section, before its fences.
const CARD_TAIL: &str = r#"  Drawings fade out after 8 seconds

COLUMN SEPARATOR
  +++   Separates left and right columns (the columns design)

SPEAKER NOTES
  ```@notes      A fenced block of markdown notes, anywhere in the slide;
  ...            several are joined in order. Nothing in them splits the
  ```            slide. Shown in the presenter view (V) and in PDF export
                 with --notes

GANTT CHART DURATION FORMATS
  Nd             Calendar days (e.g. 10d)
  Nwd            Working days, Mon-Fri (e.g. 5wd)
  Nw             Weeks (e.g. 2w)
  Nm             Months (e.g. 3m)
  after Task     Start when Task ends
  after Task + Nd  Start N days after Task ends

CHART AXIS LABELS
  x-label: text      Horizontal axis label (centered below)
  y-label: text      Vertical axis label (rotated 90° CCW)
  Supported by: @bar, @line, @scatter, @stackedbar

THEMES (custom themes are YAML files; mdeck spec, section 9.4)
  themes/<name>.yaml     Next to the deck (or <name>/theme.yaml with fonts)
  user folder            ~/.config/mdeck/themes (macOS: ~/Library/Application
                         Support/mdeck/themes, Windows: %APPDATA%\mdeck\themes)
  extends: dark          Unset keys come from another theme (dark is the default)
  variant-of: ember      A recolouring: listed after the themes
  engine: { name: thermal, palette: iron }   The engine and its settings (9.6)
  designs: standard|editorial   How the slide designs look; arrangements: overrides
  spacing: { md: 24 }  radius: 8   Gaps and corners every design uses
  page: { surface, margin, shadow, grain, radius }   The slide as a sheet on a surface
  mdeck theme list | check <n> | preview <n> -o <dir>
  mdeck theme new <n>    A commented starter
  mdeck ai theme <n> --from <design system folder>

PRESENT, EXPORT, CHECK
  mdeck deck.md          Fullscreen (--windowed, --slide N, --overview,
                         --presenter, --theme T, --engine E, --reduced-motion)
  mdeck export deck.md   PNGs in export/ (--format pdf, --notes, --width,
                         --height, --slide N, --range A-B, --at S, --moment M)
  mdeck --check deck.md  Report what will not show as written (-v: designs)

EXTENDING (spec section 18)
  mdeck pack install <folder|zip|git-url>   Themes, designs, point clouds, fonts
  mdeck sdk new engine|visual|design-set|transition <name>   A Rust crate
  mdeck sdk preview --engine NAME [--theme NAME] -o DIR   Every design and both moments, as PNGs
  mdeck build --with <path|crate|git-url>   An mdeck with the extension inside
  requires: [pack, extension]       In the frontmatter; --check names missing ones
"#;

/// Build the quick reference card. The settings and fences are generated
/// from the language table and the keyboard section from the shortcut table
/// the in-app HUD uses, so none of them can drift.
pub fn short_reference() -> String {
    let settings = crate::language::settings_card();
    let keys = crate::app::keys::shortcut_card();
    let fences = fences_card();
    format!("{CARD_HEAD}{settings}{CARD_MIDDLE}{keys}{CARD_TAIL}{fences}")
}

/// The mdeck fences, from the language table.
fn fences_card() -> String {
    use crate::language::{FENCES, FenceKind};
    let mut out = String::from(
        "\nFENCES (```@tag; tags match exactly, one name per kind)\n  Inside a visual: key: value settings before the first - or + item, items\n  end in (key: value) attributes, relations are A -> B: label, # comments\n",
    );
    for kind in [FenceKind::Visual, FenceKind::Notes] {
        for f in FENCES.iter().filter(|f| f.kind == kind) {
            out.push_str(&format!("  {:<15}{}\n", f.tag, f.summary));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_reference_matches_actual_key_bindings() {
        let card = short_reference();
        assert!(card.contains("Shift+T"), "theme is Shift+T, not D");
        assert!(!card.contains("\n  D "), "stale D binding");
        assert!(card.contains("PgDn"));
        assert!(card.contains("PgUp"));
        assert!(card.contains("Home / End"));
        assert!(card.contains(". / B"));
        assert!(card.contains("Q ×2"));
        assert!(card.contains("Debug overlay"));
        assert!(card.contains("Left click"));
    }

    #[test]
    fn short_reference_puts_the_keys_between_head_and_tail() {
        let card = short_reference();
        assert!(card.starts_with("MDeck Quick Reference\n"));
        assert!(card.contains("KEYBOARD & MOUSE\n"));
        assert!(card.ends_with("Speaker notes, in markdown\n"));
        let keys = card.find("KEYBOARD & MOUSE").unwrap();
        assert!(card.find("DECK SETTINGS").unwrap() < keys);
        assert!(card.find("FENCES").unwrap() > keys);
    }

    #[test]
    fn the_format_reference_takes_the_generated_settings() {
        assert!(SPEC.contains(SETTINGS_MARKER));
        assert!(SPEC.contains(DESIGNS_MARKER));
        assert!(full_reference().contains("| `statement` |"));
        let full = full_reference();
        assert!(!full.contains(SETTINGS_MARKER));
        assert!(full.contains("| `picture-prompt` |"));
    }

    #[test]
    fn short_reference_shows_no_v1_syntax() {
        let card = short_reference();
        for v1 in [
            "@theme",
            "@layout",
            "@illustration",
            "???",
            "@barchart",
            "@piechart",
        ] {
            assert!(!card.contains(v1), "{v1} is v1 syntax");
        }
        for s in crate::language::SETTINGS {
            assert!(card.contains(s.name), "{}", s.name);
        }
    }

    #[test]
    fn short_reference_keeps_other_sections() {
        let card = short_reference();
        for section in [
            "SLIDES",
            "DECK SETTINGS",
            "SLIDE SETTINGS",
            "DESIGNS",
            "STEPS",
            "KEYBOARD & MOUSE",
            "SPEAKER NOTES",
            "FENCES",
            "GANTT CHART DURATION FORMATS",
            "CHART AXIS LABELS",
        ] {
            assert!(card.contains(section), "missing section {section}");
        }
    }
}
