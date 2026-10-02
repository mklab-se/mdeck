# Known defects

Bugs and documentation drift found while writing this specification (October 2026, v1.19.0).
These are worth fixing whether or not v2 happens. **Verified** means reproduced with
`--check -v` or an export; **by reading** means found in the code but not reproduced.

## Bugs

| # | Defect | Where | Status |
|---|---|---|---|
| D1 | An indented directive inside the last list item of a slide migrates to the next slide (the trailing-directive check uses `trim()`). `--check` then blames the wrong slide. | `parser/splitter.rs:296-329` | verified |
| D2 | Image layout drops a paragraph written before the image. | `render/layouts/image_slide.rs:28` | verified (export) |
| D3 | Quote layout drops all but the last quote and the last heading. | `render/layouts/quote.rs` | verified (export) |
| D4 | Diagram layout drops everything except the first heading and the diagram, including bullets. Visualization layout drops second and later charts and moves text written after a chart above it. | `render/layouts/diagram.rs`, `visualization.rs` | by reading |
| D5 | A heading or `---` inside speaker notes starts a new visible slide. | `parser/notes.rs` (notes are extracted after splitting) | verified |
| D6 | An unknown `@layout` value silently becomes Content. | `parser/layout.rs:53` | verified |
| D7 | Frontmatter keys without `@` (`theme: dark`), misspelt keys and invalid values are silently ignored. | `parser/frontmatter.rs:126` | verified |
| D8 | A misspelt visual tag (`@barchat`) becomes an ordinary code block without a warning. Prefix matching makes `@storyboard` a story hint. | `parser/model.rs:265` | verified |
| D9 | Gallery overflow is measured as stacked 400 px images, so a gallery that fits gets a scroll range and a ▼ cue. | `render/mod.rs:37` | by reading |
| D10 | The reveal auto-scroll measures text at `width - 160`, not the layout's column width, ignoring the side image and code fitting. | `app/drawing.rs:87` | by reading |
| D11 | Hidden `+` items do not reserve space, so content below jumps as items are revealed. Generic list items pop in without animation. | `render/text/list.rs:73-89` | by reading |
| D12 | Under the editorial layouts, nested `+` items inherit the parent's step, yet still count as steps: the presses reveal nothing. | `render/ember/pieces.rs:65`, `parser/mod.rs:116` | by reading |
| D13 | `@footer` is drawn in the window but not in export (violates export parity). | `app/drawing.rs:330` | by reading |
| D14 | Image options `@height`, `@left`, `@right`, `@center` and `@fit` are parsed but never applied. `@fill` in a side panel is not clipped and can overdraw the text column. | `parser/blocks/image.rs`, `render/text/image.rs:63` | by reading |
| D15 | Hint collection is enabled each engine frame and never disabled. After switching to `plain` (Shift+T), or in overview, hints are pushed and never drained. | `engines/host.rs:103,222` | by reading |
| D16 | Story beat ticks and the HUD "say" line show on editorial engines that cannot play stories. | `app/drawing.rs:62`, `render/ember/chrome.rs:47` | by reading |
| D17 | `--engine particles` on the default theme gets no countdown; `@countdown: true` cannot turn one on. | `engines/choice.rs:52`, `app/countdown.rs:73` | by reading |
| D18 | Theme error messages prefix non-colour keys with `colors.` (`colors.page.surface`, `colors.pen`). | `theme/build/colors.rs:48` | by reading |
| D19 | A blank `@transition` skips the config default and falls through to the built-in value. | `app/helpers.rs:73` | by reading |
| D20 | The art style id hashes a reference image's path, not its contents, so editing a swatch in place does not make artworks stale. | `render/art/style.rs:174` | by reading |
| D21 | Setext headings never split slides. | `parser/splitter.rs:262` | verified |
| D22 | Ordered list start numbers and table column alignment are ignored. | `render/text/list.rs`, `render/text/table.rs` | by reading |
| D23 | Ordered lists under the editorial layouts show dots, not numbers, and only one nesting level is drawn. | `render/ember/pieces.rs` | by reading |
| D24 | Chart reactions leak onto the next slide. On the first frame of a new slide, `follow_hints` clears the old hints and then adopts `fresh`, which holds the previous slide's geometry (renderers publish after the engine paints). A slide without visuals never replaces it, so LED peak markers and laser traces from a chart stay on the following slides. | `engines/host.rs:221-235` | fixed (regression test in `engines/host.rs`) |
| D25 | A blockquote with several paragraphs is flattened into one, so an attribution written as a second quote paragraph (as several engine samples do) reads as part of the quotation. | `parser/blocks` (blockquote), `samples/engines/*.md` | verified (export) |
| D26 | The thermal end act cools to black before the export still is taken, so the end slide exports as an empty frame (violates "stills look finished"). | `engines/thermal` | fixed (regression test in `engines/thermal`) |

## Documentation drift (format reference `mdeck-spec.md` and docs)

| # | Drift |
|---|---|
| R1 | `@orgchart` documented as `(parent: X)`; the parser only accepts `A -> B`. |
| R2 | `@kpi` documented as `(trend: up, change: +12%)`; the parser takes `(trend: +12%)`. |
| R3 | `@venn` documented as `Set: items`; the parser takes `Name (size: N)` and `A & B: label`. |
| R4 | `@architecture`: the `sequence` qualifier, the `label`/`style` node keys and tree auto-layout are documented but do not exist; `# scale:` exists but is undocumented. |
| R5 | `@timeline` "horizontal or vertical": no orientation exists. |
| R6 | Layout list documents `blank` (does not exist) and omits `visualization`, `bullet` and `architecture`; the pattern list omits Visualization and TwoColumn inference. |
| R7 | Two-column "requires `@layout: two-column`"; it is inferred from `+++`. |
| R8 | "Single image gives a full-screen image": only with `@fill`. "Two to four images give a gallery": five or more do too. |
| R9 | Nested blockquotes "supported": they are not. Quote attribution "italic": it is not. |
| R10 | Spec 9.1 says other themes ignore `@illustration`; six non-particle engines draw it. |
| R11 | Spec 9.1 says Ember and Nord have a countdown; 15 of 18 built-ins do. |
| R12 | User folders documented as `~/.config/mdeck/...`; on macOS they are `~/Library/Application Support/mdeck/...` (CLI help, quick reference, `docs/themes.md`, `CLAUDE.md`). |
| R13 | `page.margin` documented as 0 to 200; the code allows 0 to 300. Font fallback behaviour differs from the description. |
| R14 | Engine lists in `theme/file.rs`, the spec 9.4 key comment and the `theme new` starter name only 6 of 12 engines; the starter omits `annotations`, `particles`, `heat`, `page`, `art`, `fonts.lead`. `theme/mod.rs` says "four built-in themes". |
| R15 | `spec --short` presents `@aspect` and `@footer` as working, and describes notes as "presenter-only" though nothing shows them while presenting. |
| R16 | `@scene` (obsolete, still parsed and warned about) is undocumented. |
| R17 | Spec grammar 13.1 says headings split at `# `; splitting is level-based. The spec version still reads "0.1 Draft". |
| R18 | The engine guide's example engine (`example.rs`) is test-only and cannot be selected with `@engine`. |
| R19 | `CLAUDE.md` describes backdrops and formations as a top-level pattern; they are particles-engine internals. |
