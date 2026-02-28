const SPEC: &str = include_str!("../../../../doc/presemd-spec.md");

pub fn run(short: bool) {
    if short {
        print_short_reference();
    } else {
        println!("{SPEC}");
    }
}

fn print_short_reference() {
    println!(
        r#"presemd Quick Reference
=======================

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
  diagram      @diagram fenced block
  two-column   @layout: two-column with +++ separator
  content      Fallback

INCREMENTAL REVEAL (list markers)
  -   Static (always visible)
  +   Next step (appears on forward press)
  *   Same step as previous +

IMAGE DIRECTIVES (in alt text)
  @fill  @fit  @width:80%  @height:100px  @left  @right  @center

KEYBOARD SHORTCUTS
  Space/N/Right  Next slide       P/Left      Previous slide
  Up/Down        Scroll content   G           Grid view
  Enter/E        Back to present. T           Cycle transition
  D              Toggle theme     F           Toggle fullscreen
  H              Show/hide HUD    Esc x2      Exit
  Ctrl+C x2      Exit             Q           Quit

MOUSE CONTROLS
  Left click     Next slide       Right click Previous slide
  Left drag      Freehand pen     Right drag  Draw arrow
  Scroll wheel   Scroll content
  Drawings fade out after 8 seconds

COLUMN SEPARATOR
  +++   Separates left and right columns in two-column layout
"#
    );
}
