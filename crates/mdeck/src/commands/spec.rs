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

/// Build the quick reference card. The keyboard section is generated from
/// the same shortcut table the in-app HUD uses, so the two cannot drift.
pub fn short_reference() -> String {
    let card = crate::app::keys::shortcut_card();
    let mut out = String::new();
    out.push_str(
        r#"MDeck Quick Reference
=====================

SLIDE SEPARATION
  ---              Explicit separator (blank lines above and below)
  3+ blank lines   Automatic slide break
  # Heading        Starts new slide when current slide has content

FRONTMATTER (YAML at top of file)
  title, author, date     Standard metadata
  @theme: dark|light      Global theme
  @transition: slide|fade|spatial|none
  @aspect: 16:9|4:3|16:10
  @footer: "text"         Footer on every slide

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

IMAGE DIRECTIVES (in alt text)
  @fill  @fit  @width:80%  @height:100px  @left  @right  @center

KEYBOARD & MOUSE
"#,
    );
    out.push_str(&card);
    out.push_str(
        r#"  Drawings fade out after 8 seconds

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
"#,
    );
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
