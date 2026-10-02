# 06. Visuals and pictures

Everything visual a slide can contain or be decorated with. Today this is the most confusing part
of mdeck: charts, diagrams, illustrations, figures, art, stories, scenes, backdrops and
formations overlap in name and purpose. This document sorts them into three groups and defines
each group.

## The map

| Group | What it is | Who decides | Examples |
|---|---|---|---|
| **Content** | carries the author's meaning; shown by every theme and engine | the author writes it | text, images, math, **visuals** (charts, diagrams, thermal images) |
| **Picture** | the one image on the slide's stage, rendered in the engine's medium | the author names it, the engine draws it | a point cloud (rocket), a generated artwork, a story cast |
| **Decoration** | what the engine adds by itself | the engine, never authored | backdrops, formations, embers off bar tops, the countdown, the end act |

Chrome (logo, background image, counter, footer) is the fourth, smaller group and belongs to the
theme and deck settings ([04](04-themes.md), [07](07-authoring-language.md)).

When the owner asks "what are the forms the particles take?", the answer is that the particles
engine shows one of these each frame, in this order (`src/engines/particles.rs:120`):

1. opening/ending (digits, burst, end words, end dance), which is **decoration**
2. a story, which is a **picture** (particles only; removed in v2, PIC-06)
3. the slide's illustration, which is a **picture**
4. reactions to visuals on the slide (runners along lines, embers off bars), which is
   **decoration**
5. a scene inferred from the layout (constellation, candle, code rain, bullet clusters in a
   formation, on a rotating backdrop), which is **decoration**

## Today: content

### Visuals: charts and diagrams

A fenced block whose info string starts with an `@` tag. There are 20 tags:

| Group | Tags |
|---|---|
| Data charts | `@barchart`, `@linechart`, `@scatter`, `@stackedbar`, `@piechart`, `@donut`, `@funnel`, `@radar`, `@progress`, `@kpi` |
| Text and time | `@wordcloud`, `@timeline`, `@gantt` |
| Structure (diagrams) | `@architecture`, `@orgchart`, `@gitgraph`, `@flower`, `@artifactflow`, `@venn` |
| Image | `@thermal` |

How they are parsed and drawn:

- **Matching is by prefix.** `@donutchart` matches `@donut`, and `@storyboard` matches `@story`.
  A typo such as `@barchat` becomes an ordinary code block, and nothing reports it.
- **Parsing happens at draw time.** The block content stays raw text, and each kind parses it
  itself.
- **`@architecture` is separate.** It has its own code path (`render/diagram/`) with A* edge
  routing, a grid via `pos:`, auto-layout, five arrow kinds and three tiers of icons:
  - a PNG in `media/diagram-icons/`;
  - 21 geometric fallbacks;
  - AI-generated icons.
- **Inside the fences there are five setting grammars:**
  - `# key: value` (charts);
  - `#` as a comment (architecture, except the undocumented `# scale:`);
  - bare `key: value` (thermal);
  - keyword-first lines (`petal`, `producer`, `lens`, `spot`);
  - `(key: value)` attributes on items.
- **Reveal is the one consistent rule.** Items use `-`, `+` and `*` everywhere.

**Spec drift: syntax the format reference documents but the parser does not accept.**

| Visual | Documented | Parser accepts |
|---|---|---|
| `@orgchart` | `(parent: X)` | `A -> B` |
| `@kpi` | `(trend: up, change: +12%)` | `(trend: +12%)` |
| `@venn` | `Set: items` | `Name (size: N)` and `A & B: label` |
| `@architecture` | `sequence` qualifier, `label`/`style` keys, tree auto-layout | none of these exist |
| `@timeline` | a vertical orientation | no orientation handling |

### Thermal images

```` ```@thermal ```` shows a thermal image (a display image, an image with a temperature mapping,
or 16-bit data with a `.yaml` sidecar). It has steps:

- `lens` over a visible-light photo;
- `reveal`;
- `above` thresholds;
- `spot` markers.

How it fits with the rest:

- **Palette.** Set by the block, by the deck's `@palette`, or live with `C`.
- **Slide settings.** `@thermal-window` sets a shared temperature scale for every block on a slide.
  `@zoom` makes the next slide zoom into a named spot.
- **Engines.** It works on every engine. It is unrelated to the thermal theme and engine except by
  name ([04](04-themes.md)).
- **Loading.** It is a `Chart` enum variant but is drawn by its own module, with its sources loaded
  when the deck opens.

### Images and math

**Images.** `![alt](path)` on its own line.

- **Options in the alt text.** `@width:80%`, `@fill`, `@fit`, `@left`, `@right`, `@center` and
  `@height`. Only `@fill` and `@width` are implemented; the others are parsed and ignored.
- **Generated images.** `![prompt](image-generation)` is replaced by a generated image when you
  run `mdeck ai generate`.
- **Math.** `$...$` and `$$...$$` are laid out with RaTeX.

## Today: pictures

**Illustrations.** `@illustration: rocket` names a `.mdpc` point cloud (38 built-ins). The lookup
order is deck `illustrations/`, then user, then built-in. They are made with
`mdeck illustration generate|import`.

**How each engine draws an illustration:**

| Engine | Medium |
|---|---|
| particles | particles settle into the shape |
| LED | lit LEDs |
| laser | etched |
| blocks | falling blocks |
| thermal | glowing heat signature |
| art engines | pen strokes |
| plain, splitflap | not shown |

**Placement.** On title slides it is a large, dim backdrop. On other slides it sits on the stage to
the right of the copy. It is shown only on slides the editorial layout handles.

**Generated art.** On the five art engines, `mdeck ai art` generates an artwork per slide in the
engine's medium.

- **Storage.** Artworks go in `art/` and `<deck>.art.yaml`.
- **Prompts.** `@art:` in the frontmatter sets the deck's world; `@art:` on a slide sets its scene;
  `@art: none` opts a slide out.
- **Fallback.** Without an artwork, the slide's `@illustration` is drawn in the medium.

**Stories.** On the particles engine only, a story stages a cast of point clouds in a 3×3 grid,
with flows of light between them and up to six **beats** that add steps, each with a spoken line.
They are generated by `mdeck ai story`, steered by a ```` ```@story ```` fence and the deck's
`@story:`, and stored in `<deck>.scenes.yaml`. The obsolete `@scene` fence still parses and warns.

## Today: decoration

All of this belongs to the particles engine unless noted.

- **Inferred scenes, by layout.** Title gets a constellation, section a warm mass, quote a candle,
  code a rain, and lists one cluster per item placed in one of six **formations** (by slide
  number). Media slides stay quiet.
- **Backdrops.** One of Stars, Dust, Galaxy or Nebula, chosen by slide number.
- **Reactions to visuals ("hints").** Embers off bars, runners along lines, sparks around circles.
  The LED, laser and thermal engines react too, in their own ways.
- **Countdown and end act.** Every engine except plain has its own.

## Assessment

1. **Visuals are content and are in good shape, but inconsistently specified.**
   - There are five settings grammars across fences.
   - Tag names are inconsistent: some have a `chart` suffix and some do not.
   - Prefix matching causes false positives.
   - Typos and bad syntax go unreported.
   - The format reference documents syntax that does not exist.
2. **"Diagram" is the name of one visual (`@architecture`) and of a whole family.** The visuals in
   that family live in different code paths for no user-visible reason.
3. **The picture slot has three names for one idea.** `@illustration`, `@art` and the story cast
   all answer the question "what stands on this slide's stage?" They share placement (`Figure` and
   `Art` are near-identical structs), eligibility and fallback. Authors should not need to know
   which engine they are on to say "put a rocket here".
4. **Stories are a large feature in a small corner.** The schema, an AI generator and a sidecar
   with pinning and staleness exist for one engine and four layouts. The spec itself says "most
   decks will not carry any".
5. **Decoration is presented as features.** Formations and backdrops appear in user docs and in
   `CLAUDE.md` as if they were mdeck concepts. They are particle internals, which is fine, and
   should be described that way.
6. **Two icon vocabularies.** Diagram icons (`database`) and point clouds (`db`) mean the same
   things under different spellings, and never cross over.

## Requirements

### Visuals

- **VIZ-01** MUST `keep`: A visual is a fenced code block whose info string names a visual kind.
  On any other markdown renderer it degrades to a readable code block of its source.
- **VIZ-02** MUST `change`: The info string must match a visual kind *exactly*. Each kind has
  exactly one name; there are no aliases. The `chart` suffix is dropped where it only says "this
  is a chart":
  - charts: `@bar`, `@line`, `@pie`, `@donut`, `@scatter`, `@stackedbar`, `@funnel`, `@radar`,
    `@progress`, `@kpi`, `@wordcloud`, `@timeline`, `@gantt`;
  - diagrams: `@architecture`, `@orgchart` (an "org chart" is the thing's name, not a suffix),
    `@gitgraph`, `@flower`, `@artifactflow`, `@venn`;
  - images: `@thermal`.

  An info string that starts with `@` and matches no kind is a `--check` error with a "did you
  mean" suggestion. (`@notes`, MD-14, is the one other mdeck fence; it is not a visual.)
- **VIZ-03** MUST `change`: Every visual kind uses one grammar inside the fence:
  - **settings** are `key: value` lines before the first item;
  - **items** are list lines (`- `, `+ `), with optional `(key: value, ...)` attributes;
  - **relations** are `A -> B: label` lines (with the five arrow kinds where the visual supports
    them);
  - `#` always starts a comment.

  Kind-specific verbs (for example `lens`, `spot`, `petal`) are item keywords inside this grammar.
- **VIZ-04** MUST `new`: Every visual kind validates its own source and reports problems to
  `--check` with line numbers (category `visual`). Malformed lines are never silently
  reinterpreted as labels.
- **VIZ-05** MUST `keep`: Visuals reveal their items with the same step markers as lists, in the
  slide's single step sequence (MD-18).
- **VIZ-06** MUST `keep`: Visuals publish their geometry, so engines can react to them and keep
  clear of them.
- **VIZ-07** MUST `change`: All visual kinds, including `@architecture` and `@thermal`, are
  implementations of one visual interface. That interface is what extensions implement to add a
  kind (see [08](08-extensibility.md)). Kinds that need extra context (thermal images need the
  image cache and the live palette) get it through that interface, not through special cases.
- **VIZ-08** MUST `change`: The format reference documents exactly the syntax the code accepts.
  Every example in it is parsed in a test.
- **VIZ-09** SHOULD `change`: The architecture visual's tag stays `@architecture`. "Diagram" is
  the family name, not a tag.
- **VIZ-10** SHOULD `new`: Diagram node icons and point clouds share one name vocabulary, so
  `server` means the same thing in a diagram and on a stage.

### Images and math

- **VIZ-11** MUST `change`: Every image option the format reference lists works, or is removed.
  Today `@height`, `@left`, `@right`, `@center` and `@fit` are parsed but ignored.
- **VIZ-12** MUST `keep`: Math renders wherever text renders (except code and visuals), in the
  window and in export.

### The picture

- **PIC-01** MUST `change`: A slide has at most one **picture**, set with one slide setting
  (`picture: rocket`). Today this is split across `@illustration` and `@art`.
- **PIC-02** MUST `change`: A picture name resolves, in order, to:
  1. a generated artwork for this slide in the current engine's medium, if one exists and is
     current;
  2. a point cloud of that name (deck, user, extensions, built-ins);
  3. an image file path.

  The engine draws whichever source it receives in its medium. An engine that cannot draw
  pictures ignores it, and `--check` reports that.
- **PIC-03** MUST `new`: `picture: none` keeps a slide empty, including on engines that would
  generate or infer one.
- **PIC-04** MUST `keep`: The picture appears on the slide's stage: beside the copy, or behind
  the title as a dim backdrop on title designs. Which designs have a stage is decided by the
  arrangement (ENG-14).
- **PIC-05** MUST `change`: The prompt for a generated artwork is a separate, optional setting
  (`picture-prompt:` on a slide; `art-world:` in the deck settings), used only by
  `mdeck ai art`. It is never needed to present.
- **PIC-06** MUST `remove`: Stories are removed: the `@story` fence and deck key, the
  `<deck>.scenes.yaml` sidecar, `mdeck ai story`, the `stories` capability and beats as steps.
  If staged pictures come back later, they come back as a picture source every picture engine can
  render, not as a particles-only feature.

### Decoration

- **DEC-01** MUST `keep`: Decoration is engine-owned, automatic and never authored (ENG-15).
- **DEC-02** MUST `change`: User documentation describes decoration per engine, under the engine
  ("the particles engine forms constellations on titles and clusters for bullets"), never as a
  general mdeck feature.
- **DEC-03** MUST `keep`: The opening (countdown) and ending (end act) are decoration, owned by
  the engine, and switched on or off by the theme and the deck setting (see [09](09-presenting-export-check.md)).
