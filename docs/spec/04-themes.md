# 04. Themes

What a theme is, what comes with one, and what it can and cannot change.

## The model in brief

A theme is a YAML file (or a folder with `theme.yaml` and its fonts, logos and reference images)
containing data only. It packages a **look** (colours, fonts, sizes, spacing, the code syntax
theme), a **design set** with arrangement overrides ([03](03-slide-designs.md)), an **engine**
with that engine's settings ([05](05-engines.md)), a default **transition**, whether decks open
with a **countdown**, a **logo**, and optionally a **page** (the slide as a sheet on a surface).

```yaml
name: Thermal
extends: ember
engine:
  name: thermal
  palette: iron
  drift: false
countdown: on
colors:
  background: "#05030d"
  accent: "#f37a0c"
```

Every theme without `extends` inherits `dark`, the default theme, so there is one root. Built-in
themes are written in the same format as user themes (`crates/mdeck/themes/*.yaml`, embedded in
the binary). The built-ins come in two tiers: 13 **themes** (`dark`, `light`, `nord`, `ember`,
`thermal`, `marquee`, `departures`, `stack`, `blueprint`, `chalkboard`, `sketchbook`,
`watercolour`, `darkroom`) and 4 **variants**, recolourings that say `variant-of:` (`spring` and
`summer` of `light`, `autumn` and `winter` of `ember`).

### Thermal, in plain words

The word "thermal" names three independent things:

1. **The `thermal` theme** is a look: indigo-black, white-hot headings, an iron-orange accent,
   on the thermal engine.
2. **The thermal engine** paints a contour-banded heat field under the slides; headings form in
   heat on title and section slides, pictures glow as heat signatures. Its colours are its
   `engine: { palette: ... }` setting.
3. **A ```` ```@thermal ```` block** is content: a real thermal image that can be stepped
   through (a lens over a visible-light photo, a threshold, spots). It works in every theme and
   every engine, and its palette is set by the block, by the deck's `palette` setting, or live
   with `C`, never by the theme.

You do not need the Thermal theme to show thermal images, and the Thermal theme does not make
your images thermal.

## Requirements

### What a theme is

- **THM-01** MUST `implemented`: A theme is a named package of:
  - a **look** (tokens);
  - a **design set** plus arrangement overrides (`designs:`, `arrangements:`;
    [03](03-slide-designs.md));
  - an **engine** choice plus that engine's settings (`engine:`; [05](05-engines.md));
  - **chrome**: a logo, and the counter, eyebrow and progress line its design set draws;
  - a **default transition** (`transition:`);
  - an **opening**: `countdown: on | off` (the countdown's look and the end act are the
    engine's);
  - optionally a **page** (the slide as a sheet on a surface).
- **THM-02** MUST `implemented`: A theme is data. Creating a theme never requires code. Fonts,
  logos, syntax themes and reference images are files inside the theme's folder.
- **THM-03** MUST `implemented`: `extends` merges key by key; lists are replaced whole. A theme
  states only what differs. A theme without `extends` inherits from the default theme (`dark`),
  so there is exactly one root, and the default theme is complete.
- **THM-04** MUST `implemented`: Lookup order is the deck's `themes/` folder, then the user
  folder's `themes/`, then installed packs, then built-ins. A deck-local theme overrides
  everything, so a deck folder is self-contained.
- **THM-05** MUST `implemented`: A theme is a folder (`<name>/theme.yaml` plus assets) or a single
  file (`<name>.yaml`). A theme folder can be shared and installed as is, alone or in a pack (see
  [08](08-extensibility.md)).

### Look

- **THM-06** MUST `implemented`: The token set covers:
  - colours: the semantic roles plus a chart series and annotation pens;
  - typography: font per role (display, body, lead, strong, mono), a size scale, line height;
  - the code syntax theme.
- **THM-07** MUST `implemented`: Spacing and radius are tokens too (`spacing: xs..xl` and
  `radius`), used by every arrangement, so a theme can make everything airier or tighter in one
  place.
- **THM-08** MUST `implemented`: Chart grids, axes and muted chart text use the `rule` and `muted`
  tokens, not fixed opacities of `text`.
- **THM-09** MUST `implemented`: Every built-in theme passes WCAG AA for body text and large text.
  The test checks every text/background pair the themes actually render, including links and
  captions, and `mdeck theme check` gives the same advice for any theme.

### Designs and engine inside the theme

- **THM-10** MUST `implemented`: The design set (`standard`, `editorial`, or one provided as data
  by the user folder or a pack) is chosen by the theme, independently of the engine. Ember's look
  on a still screen is `designs: editorial` with `engine: plain`.
- **THM-11** MUST `deferred to 2.x`: Engine settings live in one block named after the engine,
  and are validated against the settings that engine declares (`mdeck theme check` warns about a
  key the engine does not read):
  ```yaml
  engine:
    name: thermal
    palette: iron
    drift: false
  ```
  `engine: name` is the shorthand without settings. When the deck or the command line overrides
  the engine, the theme's settings for the old engine are ignored, and `--check -v` says so.
  *Deferred:* everything but the last clause is built; `--check -v` does not yet say that the
  theme's engine settings are ignored.
- **THM-12** MUST `implemented`: An engine declares what it needs from the theme (in 2.0: a
  page, declared by the `line`, `sketch` and `watercolour` engines). `mdeck theme check` reports
  a theme that selects an engine without what the engine needs; the engine then draws on the bare
  background.
- **THM-13** MUST `implemented`: Keys that have no effect with the theme's engine and design set
  (an engine setting the engine does not read, a `fonts.lead` no role of the design set uses) are
  warnings in `mdeck theme check`, not silently accepted.

### Built-ins

- **THM-14** MUST `implemented`: Built-in themes are listed in two tiers in the docs, in
  `mdeck theme list` and in the theme picker:
  - **Themes:** distinctive packages, at least one showcase per engine plus the plain looks
    (`dark`, `light`, `nord`). There is no `etch` theme: it went with the laser engine
    (ENG-17a).
  - **Variants:** recolourings of a theme that say `variant-of:` (`autumn` and `winter` of
    `ember`, `spring` and `summer` of `light`).

  Shift+T cycles the themes first, then the variants.
- **THM-15** MUST `implemented`: The default theme is `dark`: a plain dark background with a
  bright foreground, `designs: standard`, `engine: plain`, fade transitions and no countdown
  (VIS-07). It shows that mdeck is simple; the showcase themes show the wow effect.
- **THM-16** MUST `implemented`: No code branches on a theme's name. Behaviour comes from the
  theme's data and its engine's capabilities.

### Tools

- **THM-17** MUST `implemented`: `mdeck theme new` writes a commented starter that lists every
  key, generated from the schema key table (`theme/schema.rs`) so it cannot drift.
- **THM-18** MUST `implemented`: `mdeck theme new --from <dir>` creates a theme from a design
  system with AI, validated by loading the result.
- **THM-19** MUST `implemented`: `mdeck theme preview` exports a sampler deck in the theme.
- **THM-20** MUST `implemented`: The sampler deck in THM-19 has one slide per design in the
  catalogue ([03](03-slide-designs.md)), in catalogue order, so a designer can see every
  arrangement at once.
