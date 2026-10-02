# Writing a visual kind

A visual is a block mdeck draws from structured text in a fence. A deck writes it like code:

````markdown
```@roadmap
title: 2027
- Search: Q1
+ Sync: Q2 (owner: platform)
```
````

On GitHub or in any other markdown renderer it degrades to a readable code block. In mdeck, the
visual kind registered under the tag `roadmap` draws it. Your own kinds work exactly like the
built-in `@bar`, `@line` and `@architecture`.

Start with the scaffold, which builds and tests as it is:

```bash
mdeck sdk new visual roadmap
```

## The `Visual` trait

```rust
pub trait Visual: Send + Sync {
    fn tag(&self) -> &str;                       // "roadmap" for ```@roadmap
    fn summary(&self) -> &str;                   // one sentence for `mdeck spec` and the docs
    fn check(&self, src: &str) -> Vec<Problem> { Vec::new() }   // for --check
    fn steps(&self, src: &str) -> usize { 0 }    // reveal steps it adds to the slide
    fn draw(&self, cx: &mut VisualCx, src: &str, rect: Rect, step: usize) -> f32;
}
```

Register it from your crate's entry point:

```rust
pub fn register(r: &mut Registry) -> Result<(), RegistryError> {
    r.visual(Box::new(Roadmap))
}
```

Tags are unique. A tag mdeck or another extension already uses is an error at startup that names
both. Tags match exactly; there are no aliases.

## The fence grammar

Every visual kind uses one grammar inside its fence, so deck authors learn it once:

- **Settings** are `key: value` lines before the first item.
- **Items** are list lines, starting with `- ` or `+ `, with optional `(key: value, ...)`
  attributes at the end.
- **Relations** are `A -> B: label` lines. The arrow kinds are `->`, `<-`, `<->`, `--` and `-->`;
  support the ones that mean something for your kind.
- `#` always starts a comment.

Kind-specific verbs (a `milestone` in a roadmap, a `lens` in a thermal image) are item keywords
inside this grammar, not new syntax. A `+ ` item is revealed on the next step, like a `+` list item
in the slide; a `- ` item is shown with the slide.

```text
title: 2027                       # a setting
max: 4                            # another setting
- Search: Q1                      # an item, shown with the slide
+ Sync: Q2 (owner: platform)      # an item with an attribute, revealed on the next step
Search -> Sync: unblocks          # a relation
```

## `check`: report, never reinterpret

`check` returns every problem in the source, one `Problem` per mistake, with the 1-based line
**inside the fence** (mdeck adds the fence's position in the deck, so `--check` points at the
right line of the file). Use the category `visual`.

```rust
fn check(&self, src: &str) -> Vec<Problem> {
    parse(src).problems
}
```

The rule: a malformed line is a problem, never silently turned into a label. Settings after the
first item, unknown settings, unknown attributes, values that do not parse and an empty fence are
all reported. `draw` must still cope with any input (skip what it cannot use) and never panic.

The scaffold's `parse` returns the parsed fence and its problems together, so `check`, `steps` and
`draw` all agree.

## `steps`: reveal in the slide's sequence

If the visual reveals its items step by step, `steps` returns how many steps it adds. The slide's
reveal steps are one sequence: list items, visuals and everything else take their turns in order.
`draw` receives the current `step` (0 is the slide's first state) and shows what is due.

## `draw`

```rust
fn draw(&self, cx: &mut VisualCx, src: &str, rect: Rect, step: usize) -> f32
```

- Draw inside `rect` and return the height you used (at most `rect.height()`). mdeck uses it to
  stack blocks and to scroll an overflowing slide.
- `cx.painter()` draws, `cx.tokens()` gives the theme's colours (`series_color(i)` for data
  series, `text`, `muted`, `rule`, `accent`), `cx.scale()` the scale to multiply every size by.
- `cx.animate()` is false for stills and reduced motion: draw the finished state then.
- `cx.image(path)` loads an image relative to the deck, cached by mdeck.
- `cx.publish(hint)` tells the engine what you drew: `Hint::Bar` for bars, `Hint::Path` for lines
  and edges (in reading order: runners travel from the first point), `Hint::Circle`, `Hint::Point`,
  and `Hint::Frame` for boxes the engine must stay out of (an image, a card, a node). Engines use
  this to react to your visual and to keep calm behind it.

## Testing

`mdeck_sdk::testing::Headless::render_visual` draws a visual without a window and returns the
image, the published hints and the height:

```rust
let out = Headless::new(480, 270).render_visual(&Roadmap, SRC, 1, &Tokens::default());
assert_eq!(out.hints.len(), 3);
assert_golden(path, &out.image, GOLDEN_TOLERANCE);
```

Test `check` with a table of broken inputs and the lines you expect: it is the part deck authors
meet most.

## Visuals in any language

If your team does not write Rust, a visual can be any program that writes a PNG. Map a fence tag
to a command in the user config (`config.yaml` in the user folder: `~/.config/mdeck/` on Linux, `~/Library/Application Support/mdeck/`
on macOS; `mdeck config show` prints it):

```yaml
visuals:
  plantuml: ~/bin/plantuml-png      # draws ```@plantuml blocks
```

For each ```` ```@plantuml ```` block, mdeck runs the command through the shell (`sh -c`, or
`cmd /C` on Windows) and writes one JSON object to its stdin:

```json
{
  "tag": "@plantuml",
  "source": "Alice -> Bob: hello\n",
  "tokens": { "background": "#101014", "text": "#e8e8ec", "accent": "#ff4d1c" },
  "width": 1600,
  "height": 700,
  "scale": 2.0
}
```

- `source` is the fence's content, exactly as written.
- `tokens` are the theme's colours by token name, as `#rrggbb`, so the picture matches the deck.
- `width` and `height` are the visual's box on a 1920x1080 slide; draw the image about
  `width * scale` by `height * scale` pixels. mdeck fits it into the box with its aspect kept.

The program writes a **PNG** to stdout and exits with status 0. Anything else is an error, shown
with the last lines of the program's stderr. SVG is not accepted: mdeck's SVG rasteriser is built
without text support, and the diagrams such programs draw are mostly text. A program that runs
longer than 30 seconds is stopped.

The PNG is cached next to the deck as `<deck>.assets/visuals/<tag>-<hash>.png`, keyed by the
command and everything in the request. A program runs once per block when the deck opens and its
image is missing, never while you present; change the block or the theme and it runs again for
that block. Commit the `visuals/` folder with the deck and it presents on machines without the
program. A minimal program in Python:

```python
#!/usr/bin/env python3
import json, sys
from PIL import Image, ImageDraw   # pip install pillow

req = json.load(sys.stdin)
w, h = int(req["width"] * req["scale"]), int(req["height"] * req["scale"])
img = Image.new("RGBA", (w, h), (0, 0, 0, 0))
ImageDraw.Draw(img).text((20, 20), req["source"], fill=req["tokens"].get("text", "#ffffff"))
img.save(sys.stdout.buffer, "PNG")
```

Such visuals are static: no reveal steps, no engine reactions, no `check` of the fence. Use the
`Visual` trait when you need more. `mdeck extensions list` shows the configured programs.
