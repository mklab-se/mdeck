# 06. Visuals and pictures

Everything visual a slide can contain or be decorated with, sorted into three groups.

## The map

| Group | What it is | Who decides | Examples |
|---|---|---|---|
| **Content** | carries the author's meaning; shown by every theme and engine (or reported by `--check` when it cannot be) | the author writes it | text, images, math, **visuals** (charts, diagrams, thermal images) |
| **Picture** | the one image on the slide's stage, rendered in the engine's medium | the author names it, the engine draws it | a point cloud (rocket), a generated artwork |
| **Decoration** | what the engine adds by itself | the engine, never authored | backdrops, formations, embers off bar tops, the countdown, the end act |

Chrome (logo, background image, counter, footer) is the fourth, smaller group and belongs to the
theme and deck settings ([04](04-themes.md), [07](07-authoring-language.md)).

As an example of how the groups meet, the particles engine shows one of these each frame, in
this order:

1. the opening or ending (digits, burst, end words), which is **decoration**;
2. the slide's picture, which is a **picture**;
3. reactions to visuals on the slide (runners along lines, embers off bars), which is
   **decoration**;
4. a scene inferred from the design (constellation, candle, code rain, bullet clusters in a
   formation, on a rotating backdrop), which is **decoration**.

## Visuals

A visual is a fenced block whose info string is an exact visual tag. There are 20 built-in
kinds:

| Group | Tags |
|---|---|
| Charts | `@bar`, `@line`, `@pie`, `@donut`, `@scatter`, `@stackedbar`, `@funnel`, `@radar`, `@progress`, `@kpi`, `@wordcloud`, `@timeline`, `@gantt` |
| Diagrams | `@architecture`, `@orgchart`, `@gitgraph`, `@flower`, `@artifactflow`, `@venn` |
| Images | `@thermal` |

Every kind is a `Visual` registered under its tag in the same registry extensions use
([08](08-extensibility.md)); it checks its own source, counts its steps and draws.

## Requirements

### Visuals

- **VIZ-01** MUST `implemented`: A visual is a fenced code block whose info string names a visual
  kind. On any other markdown renderer it degrades to a readable code block of its source.
- **VIZ-02** MUST `implemented`: The info string must match a visual kind *exactly*. Each kind has
  exactly one name; there are no aliases and no prefix matching. The `chart` suffix is dropped
  where it only says "this is a chart":
  - charts: `@bar`, `@line`, `@pie`, `@donut`, `@scatter`, `@stackedbar`, `@funnel`, `@radar`,
    `@progress`, `@kpi`, `@wordcloud`, `@timeline`, `@gantt`;
  - diagrams: `@architecture`, `@orgchart` (an "org chart" is the thing's name, not a suffix),
    `@gitgraph`, `@flower`, `@artifactflow`, `@venn`;
  - images: `@thermal`.

  `--check` (category `visual`) reports an info string that starts with `@` and matches no kind,
  with a "did you mean" suggestion when one is close, and names the v2 tag for a v1 tag such as
  `@barchart`. (`@notes`, MD-14, is the one other mdeck fence; it is not a visual.)
- **VIZ-03** MUST `implemented`: Every visual kind uses one grammar inside the fence:
  - **settings** are `key: value` lines before the first item;
  - **items** are list lines (`- `, `+ `), with optional trailing `(key: value, ...)`
    attributes;
  - **relations** are items of the form `A -> B: label` (with the five arrow kinds `->`, `<-`,
    `<->`, `--`, `-->` where the visual supports them);
  - `#` starts a comment, on a whole line or after whitespace.

  Kind-specific verbs (for example `lens`, `spot`, `petal`) are the first word of an item.
- **VIZ-04** MUST `implemented`: Every visual kind validates its own source and reports problems
  to `--check` with line numbers (category `visual`). Malformed lines are never silently
  reinterpreted as labels.
- **VIZ-05** MUST `implemented`: Visuals reveal their items with the same step markers as lists,
  in the slide's single step sequence (MD-18).
- **VIZ-06** MUST `implemented`: Visuals publish their geometry (bars, paths, circles, frames), so
  engines can react to them and keep clear of them.
- **VIZ-07** MUST `deferred to 2.x`: All visual kinds, including `@architecture` and `@thermal`, are
  registered implementations of the one `Visual` interface that extensions implement to add a
  kind ([08](08-extensibility.md)); checking, steps and drawing go through the registry. In 2.0
  the built-ins still draw with egui through an internal bridge rather than the SDK painter, and
  `@thermal` and `@architecture` are drawn by their own renderers because they need the deck's
  images; moving them onto the SDK's drawing interface is deferred to 2.x.
- **VIZ-08** MUST `implemented`: The format reference documents exactly the syntax the code
  accepts. Every visual example in it (and in `docs/visualizations.md`) is parsed in a test, and
  every kind has an example.
- **VIZ-09** SHOULD `implemented`: The architecture visual's tag is `@architecture`. "Diagram" is
  the family name, not a tag.
- **VIZ-10** SHOULD `deferred to 2.x`: Diagram node icons and point clouds share one name
  vocabulary, so `server` means the same thing in a diagram and on a stage. *Deferred:* the two
  sets are still separate (a diagram's `database` is the point cloud `db`).

### Images and math

- **VIZ-11** MUST `implemented`: Every image option the format reference lists works: `@width`,
  `@height` and `@fill`, written in the settings grammar in the alt text (`@width: 80%`). Where
  an image goes is up to the slide's design. `@fit`, `@left`, `@right`, `@center` and any other
  `@` word are reported by `--check` (category `content`).
- **VIZ-12** MUST `implemented`: Math (`$...$`, `$$...$$`) renders wherever text renders (except
  code and visuals), in the window and in export.

### The picture

- **PIC-01** MUST `implemented`: A slide has at most one **picture**, set with one slide setting
  (`<!-- picture: rocket -->`). It replaces v1's `@illustration` and `@art`, which `--check`
  reports with their v2 form.
- **PIC-02** MUST `implemented`: A picture name resolves, in order, to:
  1. a current generated artwork for this slide, on an art engine;
  2. a point cloud of that name: the deck's generated point clouds
     (`<deck>.assets/point-clouds/`), `point-clouds/` next to the deck, the user's
     `point-clouds/`, installed packs' `point-clouds/`, then the clouds built into mdeck and its
     extensions;
  3. an image file path, relative to the deck.

  The engine draws an artwork or a point cloud in its medium. An engine that cannot draw
  pictures ignores it, and `--check` (category `engine`) reports that. An image file is content:
  mdeck draws it on the design's stage itself, on every engine but a board (which draws the whole
  slide, and `--check` says so), and engines see it as a `Frame` hint. `--check` reports an image
  path that names no file.
- **PIC-03** MUST `implemented`: `picture: none` keeps a slide's stage empty, including on art
  engines that would draw a generated artwork.
- **PIC-04** MUST `implemented`: The picture appears on the slide's stage: beside the copy, or
  behind the title as a large, dim backdrop on title slides. Which designs have a stage is
  decided by the arrangement (ENG-14); in the `editorial` set statement, points, quote, section,
  title and text-only content slides have one, and the `standard` set has none.
- **PIC-05** MUST `implemented`: The prompt for a generated artwork is a separate, optional
  setting (`picture-prompt` on a slide, `art-world` in the deck settings), used only by
  `mdeck ai pictures`. It is never needed to present.
- **PIC-06** MUST `implemented`: There are no stories: no `@story` or `@scene` fence, no `story`
  deck key, no `<deck>.scenes.yaml` sidecar, no `mdeck ai story`, no `stories` capability and no
  beats as steps; `--check` reports the v1 forms. If staged pictures come back later, they come
  back as a picture source every picture engine can render, not as a particles-only feature.

### Decoration

- **DEC-01** MUST `implemented`: Decoration is engine-owned, automatic and never authored
  (ENG-15).
- **DEC-02** MUST `implemented`: User documentation describes decoration per engine, under the
  engine ("the particles engine forms constellations on titles and clusters for bullets"), never
  as a general mdeck feature (`docs/engines.md`).
- **DEC-03** MUST `implemented`: The opening (countdown) and ending (end act) are decoration,
  owned by the engine, and switched on or off by the theme and the deck's `countdown: on|off`
  setting (see [09](09-presenting-export-check.md)).
