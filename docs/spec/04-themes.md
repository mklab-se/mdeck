# 04. Themes

What a theme is, what comes with one, and what it can and cannot change.

## Today

### What a theme is

A theme is a YAML file containing data only. Built-ins are `crates/mdeck/themes/*.yaml`, embedded
in the binary. The resolved `Theme` (`src/theme/mod.rs:110`) holds:

- 13 named colours, an 8-slot chart series, 4 annotation colours, 2 particle tints
- 5 font roles (display, body, lead, strong, mono) and 5 sizes (h1, h2, h3, body, code)
- line height, chart fill opacity, the code syntax theme
- an optional logo, an optional page (the slide as a sheet on a surface)
- an `art:` block (style for art engines), a `heat:` block (thermal engine)
- an **engine** name and a **countdown** style (`none`, `plain` or `burst`)

**Schema.** Every key is kebab-case, and unknown keys are errors.

**Inheritance.** `extends` merges key by key; lists are replaced whole. A user theme with no
`extends` inherits from `dark`, while the default theme is `light`.

**Lookup.** Deck `themes/`, then the user folder (`dirs::config_dir()/mdeck/themes`, which on macOS
is `~/Library/Application Support/mdeck/themes`), then built-ins.

**Validation.** `mdeck theme check` reports errors and WCAG contrast advice for four colour pairs.

**Commands.** `mdeck theme list|check|new|preview`. `theme new --from <design-system-dir>` asks an AI
to write a theme from a design system's tokens, CSS, fonts and logos.

### The 18 built-ins

| Theme | Engine | Notes |
|---|---|---|
| dark | plain | The root that everything extends. |
| light | plain | The default theme. |
| nord, spring, summer | plain | Colours and type only. |
| ember, autumn, winter | particles | Autumn and winter are Ember recoloured. |
| thermal | thermal | Extends Ember, adds `heat:`. |
| marquee | led | Showcase for the LED engine. |
| departures | splitflap | Showcase for the split-flap engine. |
| etch | laser | Showcase for the laser engine. |
| stack | blocks | Showcase for the blocks engine. |
| blueprint | blueprint | Has a `page:` and paints its own sheet in the engine. |
| sketchbook | sketch | Has a `page:`. |
| chalkboard | chalkboard | Has a `page:`. |
| watercolour | watercolour | Has a `page:`. |
| darkroom | darkroom | Accent is the safelight. |

15 of the 18 have a countdown, so the spec's "Ember and Nord open with a countdown" is out of
date.

### What a theme can and cannot control

| A theme CAN set | A theme CANNOT set (hardcoded or engine-owned) |
|---|---|
| every colour role, chart series, annotation pens | layout geometry: columns, padding, title position, image split |
| fonts per role, five sizes, line height | whether slides use the editorial layouts (the engine decides) |
| code syntax theme | bullet glyph, quote ornament, caption style |
| logo, page/sheet | transitions (deck setting or config only) |
| engine, countdown style | backdrops, formations (particles internals) |
| art style, heat palette | chart axis and grid colours (fixed opacities of `text`) |
| | slide counter, eyebrow, footer chrome |

### Theme versus engine

- **Precedence.** `--engine` beats `@engine`, which beats the theme's `engine:`.
- **What switching engines keeps.** Switching engine keeps colours, fonts and logo, and downgrades
  a `burst` countdown to `plain`.
- **Engine-specific keys.** `heat:` matters only on the thermal engine, `art:` only on art engines,
  and `fonts.lead` only on editorial engines. These keys pass validation silently on other engines.
- **The `particles:` section.** Its tints are read by six engines, not only the particles engine.
- **Coupling.** Sketch, chalkboard and watercolour depend on the theme having a `page:` (the
  paper or slate). This coupling is not declared anywhere: `@engine: sketch` on a theme without a
  page draws pencil on a bare background.

### Thermal, in plain words

The word "thermal" names three independent things:

1. **The `thermal` theme** is a look: indigo-black, white-hot headings, an iron-orange accent. It
   is Ember with the thermal engine.
2. **The thermal engine** paints a contour-banded heat field under the slides.
   - Titles "form in heat" before the copy appears (the "cold open").
   - Illustrations glow as heat signatures.
   - The field stays dark around images and charts.
   - Pen annotations are drawn white-hot and cool away.
   - Its colours are the theme's `heat.palette`.
3. **A ```` ```@thermal ```` block** is content: a real thermal image (a photo, an image with a
   temperature mapping, or 16-bit data).
   - You can step through it with clicks: a lens over a visible-light photo, a threshold, spots.
   - It switches palette live with `C`.
   - It works in **every theme and every engine**.
   - Its palette is set by the block, by the deck's `@palette`, or live with `C`, never by the
     theme's `heat.palette`.

So: you do not need the Thermal theme to show thermal images, and the Thermal theme does not make
your images thermal. The shared word and the shared palette names make them look like one feature.

## Assessment

1. **A theme is only half a design.** It controls tokens but none of the arrangement, so two
   themes on the same engine can differ only in colour and type. The thing that makes Ember look
   like Ember (the editorial copy column) is not in the theme at all.
2. **Engine settings live in theme sections named after one engine** (`particles:`, `heat:`,
   `art:`). They are not validated against the engine that will use them.
3. **The engine and the theme are coupled by convention, not by contract.** Paper engines need a
   page; editorial engines need a lead font.
4. **Several built-ins are variants, not themes.** Autumn and winter are recoloured Ember; spring
   and summer are recoloured Light. That is fine as a variant mechanism, but it inflates the list
   of "18 themes" and dilutes the showcase.
5. **Docs drift.** Engine lists in comments and in the `theme new` starter name only 6 of 12
   engines. The user theme path is documented as `~/.config`. The page margin limit is
   documented as 200 when it is 300. The spec's WCAG claim is stronger than what the test
   checks.

## Requirements

### What a theme is

- **THM-01** MUST `change`: A theme is a named package of:
  - a **look** (tokens);
  - a **design set** plus arrangement overrides ([03](03-slide-designs.md));
  - an **engine** choice plus that engine's settings ([05](05-engines.md));
  - **chrome** (logo, counter, footer, progress);
  - a **default transition** (`new`; today transitions cannot be set by a theme);
  - an **opening and ending** (countdown and end act style);
  - optionally a **page** (the slide as a sheet on a surface).
- **THM-02** MUST `keep`: A theme is data. Creating a theme never requires code. Fonts, logos,
  syntax themes and reference images are files inside the theme's folder.
- **THM-03** MUST `keep`: `extends` merges key by key. A theme states only what differs. A theme
  without `extends` inherits from the default theme (`dark`), so there is exactly one root, and
  the default theme is complete.
- **THM-04** MUST `keep`: Lookup order is deck, then user, then installed extensions, then
  built-ins. A deck-local theme overrides everything, so a deck folder is self-contained.
- **THM-05** MUST `new`: A theme is a folder (`<name>/theme.yaml` plus assets) or a single file.
  A theme folder can be zipped, shared and installed as is (see [08](08-extensibility.md)).

### Look

- **THM-06** MUST `keep`: The token set covers:
  - colours: the semantic roles plus a chart series;
  - typography: font per role, a size scale, line height;
  - the code syntax theme.
- **THM-07** MUST `new`: Spacing and radius are tokens too (a spacing scale and a corner radius),
  used by every arrangement, so a theme can make everything airier or tighter in one place.
- **THM-08** MUST `change`: Chart grid, axis and muted text use the `rule` and `muted` tokens,
  not fixed opacities of `text`.
- **THM-09** MUST `keep`: Every built-in theme passes WCAG AA for body text and large text. The
  test checks every text/background pair the themes actually render, including links and
  captions.

### Designs and engine inside the theme

- **THM-10** MUST `new`: The design set (`standard`, `editorial`, or one provided by an
  extension) is chosen by the theme, independently of the engine. Ember's look on a still screen
  is `designs: editorial` with `engine: plain`.
- **THM-11** MUST `change`: Engine settings live in one block named after the engine, and are
  validated by that engine:
  ```yaml
  engine:
    name: thermal
    palette: iron
    drift: false
  ```
  When the deck or the command line overrides the engine, the theme's settings for the old
  engine are ignored, and `--check -v` says so.
- **THM-12** MUST `new`: An engine declares what it needs from the theme (for example a page),
  and what it provides when the theme lacks it. `mdeck theme check` reports a theme that selects
  an engine without what the engine needs.
- **THM-13** MUST `new`: Keys that have no effect with the theme's engine and design set are
  warnings in `mdeck theme check`, not silently accepted.

### Built-ins

- **THM-14** MUST `new`: Built-in themes are listed in two tiers in the docs and the theme picker:
  - **Themes:** distinctive packages, at least one showcase per engine plus the plain looks. The
    `etch` theme is removed with the laser engine (ENG-17a).
  - **Variants:** recolourings of a theme (autumn and winter of Ember, spring and summer of
    Light).

  Shift+T cycles themes first.
- **THM-15** MUST `change`: The default theme is `dark`: a plain dark background with a bright
  foreground, `designs: standard`, `engine: plain`, fade transitions and no countdown (VIS-07).
  It shows that mdeck is simple; the showcase themes show the wow effect.
- **THM-16** MUST `keep`: No code branches on a theme's name. Behaviour comes from the theme's
  data and its engine's capabilities.

### Tools

- **THM-17** MUST `keep`: `mdeck theme new` writes a commented starter that lists every key,
  generated from the schema so it cannot drift.
- **THM-18** MUST `keep`: `mdeck theme new --from <dir>` creates a theme from a design system with
  AI, validated by loading the result.
- **THM-19** MUST `keep`: `mdeck theme preview` exports a sampler deck that shows every design in
  the theme.
- **THM-20** MUST `new`: The sampler deck in THM-19 includes one slide per design in the catalogue
  ([03](03-slide-designs.md)), so a designer can see every arrangement at once.
