const SPEC: &str = include_str!("../../doc/mdeck-spec.md");

pub fn run(short: bool) {
    if short {
        print_short_reference();
    } else {
        println!("{SPEC}");
    }
}

fn print_short_reference() {
    print!("{}", short_reference());
}

/// The quick reference card up to its keyboard section.
const CARD_HEAD: &str = r#"MDeck Quick Reference
=====================

SLIDE SEPARATION
  ---              Explicit separator (blank lines above and below)
  3+ blank lines   Automatic slide break
  # Heading        Starts new slide when current slide has content

FRONTMATTER (YAML at top of file)
  title, author, date     Standard metadata
  @theme: name            Theme: dark, light, nord, ember, spring, summer,
                          autumn, winter, marquee, departures, etch, stack,
                          blueprint, sketchbook, chalkboard, watercolour,
                          darkroom, thermal, or a custom one (see THEMES)
  @engine: name           Run on this engine instead of the theme's: plain,
                          particles, led, splitflap, laser, blocks, blueprint,
                          sketch, chalkboard, watercolour, darkroom, thermal
                          (try one with --engine name)
  @art: "..."             The deck's world for generated art (art engines)
  @transition: slide|fade|spatial|none
  @aspect: 16:9|4:3|16:10
  @footer: "text"         Footer on every slide
  @background: file       Image behind every slide (png, jpg, webp, svg)
  @background-opacity: 30%  How strongly it shows (0-1 or %, default 30%)
  @palette: iron          Palette of @thermal images (iron, white-hot,
                          black-hot, rainbow, arctic, lava)

SLIDE DIRECTIVES (on their own line, under the slide's heading)
  @layout: name         Override the inferred layout
  @illustration: name   Point cloud illustration (particles engine)
  @logo: file|none      This slide's logo, or none to hide it
  @art: "..."|none      This slide's picture on an art engine, or none
  @background: file|none  This slide's background image, or none
  @background-opacity: 50%  This slide's background opacity
  @thermal-window: 25..90 °C  One scale for the slide's @thermal images
  @zoom: Spot           Enter by zooming into a spot of the last slide

LAYOUTS (auto-inferred, override with @layout: name)
  title        H1 + optional subtitle
  section      Lone heading, centered
  bullet       Heading + list
  quote        Blockquote + optional attribution
  code         Code block + optional heading
  image        Single image + optional heading/caption
  gallery      2+ images
  diagram      @architecture fenced block
  two-column   @layout: two-column with +++ separator
  content      Fallback

INCREMENTAL REVEAL (list markers)
  -   Static (always visible)
  +   Next step (appears on forward press)
  *   Same step as previous +

MATH (LaTeX, KaTeX syntax)
  $E = mc^2$            Inline, on the text baseline
  $$\frac{a}{b}$$       Display: own line, centred
  \$5                   Literal dollar sign ($5 and $10 stay text anyway)

IMAGE DIRECTIVES (in alt text)
  @fill  @fit  @width:80%  @height:100px  @left  @right  @center

PARTICLES ENGINE (ember, autumn, winter, and custom themes on it)
  @illustration: name   Point cloud beside the copy (title: behind it)
                        deck illustrations/ > ~/.config/mdeck/illustrations > built-in
  ```@story             English hint for `mdeck ai story` (cast, flows, beats)
  mdeck illustration generate --name NAME --description "..."   New cloud via AI
  mdeck illustration import IMAGE --name NAME | list | show NAME | contribute NAME

ART ENGINES (blueprint, sketch, chalkboard, watercolour, darkroom; spec 9.7)
  mdeck ai art deck.md  Draw a picture per slide (--slide N, --stale, --force,
                        --dry-run, --engine, --node); kept in art/ and deck.art.yaml
  S                     While presenting: draw this slide's picture
  Without art           The slide's @illustration is drawn in the medium

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

/// The quick reference card after its keyboard section.
const CARD_TAIL: &str = r#"  Drawings fade out after 8 seconds

COLUMN SEPARATOR
  +++   Separates left and right columns in two-column layout

SPEAKER NOTES
  ???   Notes separator (3+ question marks)
        Everything after ??? is presenter-only notes (not rendered)
        Supports full markdown formatting in notes content

VISUALIZATIONS (fenced code blocks with @ language tag)
  @barchart      Bar chart (vertical/horizontal, # orientation:, # x-label:, # y-label:)
  @linechart     Line chart (# x-labels:, # x-label:, # y-label:, multiple series)
  @scatter       Scatter plot (# x-label:, # y-label:, optional size per point)
  @stackedbar    Stacked bar (# categories:, # x-label:, # y-label:)
  @piechart      Pie chart (- Label: value%)
  @donutchart    Donut chart (# center: text)
  @wordcloud     Word cloud (- Word (size: N), auto-rotation)
  @timeline      Timeline (- Year: Event)
  @funnel        Funnel chart (- Stage: value)
  @kpi           KPI cards (- Metric: value (trend: up, change: +N%))
  @progress      Progress bars (- Label: value%)
  @radar         Radar chart (# axes: A, B, C)
  @venn          Venn diagram (- Set: item1, item2)
  @orgchart      Org chart (- Name (parent: Parent))
  @gantt         Gantt chart (- Task: date, duration, after Dep; # labels: inside)
  @gitgraph      Git branch graph (lane, commit, branch/merge with ->, tag)
  @flower        Platform and teams (- center Name, - petal Name: what, A -> B)
  @artifactflow  Artifact supply chain (producer/service/consumer, A -> B: artifact)
  @thermal       Thermal image: palette, lens, reveal, threshold, spots (spec 14.20)

GANTT CHART DURATION FORMATS
  Nd             Calendar days (e.g. 10d)
  Nwd            Working days, Mon-Fri (e.g. 5wd)
  Nw             Weeks (e.g. 2w)
  Nm             Months (e.g. 3m)
  after Task     Start when Task ends
  after Task+Nd  Start N days after Task ends

CHART AXIS LABELS
  # x-label: text    Horizontal axis label (centered below)
  # y-label: text    Vertical axis label (rotated 90° CCW)
  Supported by: @barchart, @linechart, @scatter, @stackedbar

THEMES (custom themes are YAML files; mdeck spec, section 9.4)
  themes/<name>.yaml     Next to the deck (or <name>/theme.yaml with fonts)
  user folder            ~/.config/mdeck/themes (macOS: ~/Library/Application Support)
  extends: dark          Unset keys come from another theme
  engine: plain|particles|...  What the theme does beyond colours (section 9.6)
  page: { surface, margin, shadow, grain, radius }   The slide as a sheet on a surface
  art: { kind, style, references }   House style of generated art
  mdeck theme list | check <n> | preview <n> -o <dir>
  mdeck theme new <n> [--from <design system folder>]
"#;

/// Build the quick reference card. The keyboard section is generated from
/// the same shortcut table the in-app HUD uses, so the two cannot drift.
pub fn short_reference() -> String {
    let card = crate::app::keys::shortcut_card();
    format!("{CARD_HEAD}{card}{CARD_TAIL}")
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
        assert!(card.ends_with("[--from <design system folder>]\n"));
        assert_eq!(
            card.len(),
            CARD_HEAD.len() + crate::app::keys::shortcut_card().len() + CARD_TAIL.len()
        );
    }

    #[test]
    fn short_reference_keeps_other_sections() {
        let card = short_reference();
        for section in [
            "SLIDE SEPARATION",
            "FRONTMATTER",
            "LAYOUTS",
            "INCREMENTAL REVEAL",
            "KEYBOARD & MOUSE",
            "SPEAKER NOTES",
            "VISUALIZATIONS",
            "GANTT CHART DURATION FORMATS",
            "CHART AXIS LABELS",
        ] {
            assert!(card.contains(section), "missing section {section}");
        }
    }
}
