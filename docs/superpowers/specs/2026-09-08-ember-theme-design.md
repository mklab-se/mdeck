# Ember: from spike to product

Design for shipping the Ember theme, its particle field and its stories as a
real part of MDeck. Written 2026-09-08 after three spikes on the
`spike/ember-theme` branch (theme and field, AI stories, stage and beat rules,
export selection, countdown).

## 1. Decisions already taken

- **The theme ships as `ember`**, MKLab's brand: ink palette, one accent,
  Spectral / Hanken Grotesk / JetBrains Mono bundled, a left copy column, and
  a living particle field behind every slide.
- **Out-of-the-box scenes come first.** Most decks will never carry a story.
  The inferred scenes (what a slide gets with no authoring at all) must look
  finished on every layout before any more story vocabulary is added.
- **Stories live in the sidecar only.** `deck.scenes.yaml` (or `.yml`) is the
  single home for scripts, written by `mdeck ai story` from `@story` hints,
  copy and notes, and hand-edited there when wanted. The inline `@scene`
  fence from the spike is removed. `@story` hints stay in the markdown.
- **Stories play only where there is a stage**: bullet, content, quote and
  section slides. Title slides are the gap between two sections and carry no
  story; code, chart, diagram, table, image and two-column slides keep the
  quiet, content-aware field.
- **Beats belong to Ember.** Under any other theme a slide steps through its
  own `+` reveals only.

## 2. Goals and non-goals

Goals:

1. Every layout looks deliberate under Ember with no authoring.
2. The field never fights the content: on content slides it serves what is
   drawn (diagram edges, chart axes, code), on staged slides it tells the
   story or lights the bullets.
3. Stories are reliable to generate, safe to render (the engine owns
   geometry), and cheap to keep current.
4. Everything is covered by the repo's usual bar: tests, `--check`, the five
   documentation places, samples and the gallery.

Non-goals for this release: hand-authored choreography files, a JavaScript
twin for HTML export, particle-native charts (bars made of particles), phone
remote, and audio.

## 3. Architecture as it stands

```
render/
  ember.rs              Ember layouts (title, section, quote, copy column), chrome, say line
  fonts.rs              bundled faces, installed on the egui context
  particles/
    mod.rs              Field: pool, homes, drift, life/heat, links, paint
    gl.rs               additive glow sprites via egui_glow paint callback
    scenes.rs           inferred scenes per layout, countdown digit and burst
  story/
    mod.rs              Script schema, validation, staging (cells, silhouettes, flows, labels)
    silhouettes.rs      particle outlines for cast kinds
    sidecar.rs          deck.scenes.yaml load/save, hashing, resolution, staleness
app/
  ember.rs              EmberState: field lifetime, scene selection, countdown digits
  mod.rs                stories, step counting (beats), countdown, S key
commands/
  story.rs              mdeck ai story (Ailloy, JSON on the wire, YAML on disk)
  export.rs             --slide / --range, settled field per still
```

The spike's boundaries hold. What follows changes behaviour inside them and
adds two capabilities: wakes and content-aware fields.

## 4. Work packages

Ordered so each one leaves `main` releasable. Sizes: S under a day, M a few
days, L a week or more.

### WP1 Out-of-the-box scenes (L)

The field with no story on any layout.

- **Wakes.** Particles leave faint trails as on the site. Render the sprite
  pass into an offscreen framebuffer that is faded (not cleared) each frame,
  then composited additively. Two textures and one extra draw in `gl.rs`.
  Off under reduced motion and in export.
- **Content-aware fields.** Renderers publish geometry to the field through a
  small `SceneHints` value returned from `render_slide`: diagram edge
  polylines and their direction, chart axis line and bar extents, code block
  rect, table rect, image rect. `scenes.rs` turns hints into scenes: runners
  along diagram edges in edge direction, dust drifting along a chart's
  baseline, a slow current down a code block, a quiet frame around an image.
  Slides without hints keep today's quiet scene.
- **Per-layout polish.** Title constellation, section mass, candle, rain and
  bullet clusters reviewed on a projector-sized window; cluster placement
  avoids the copy column's measured extent, not a fixed fraction.
- **Reveal pacing.** A newly lit cluster rises and brightens over 600 ms with
  the item's fade-up, instead of easing brightness alone.
- **Budget.** Particle count and link count scale with the window and drop
  under a `--reduced-motion` flag and the OS reduced-motion setting where
  egui exposes it. Target: 60 fps on integrated graphics at 1080p.

Acceptance: exported stills of every sample deck under Ember pass a visual
review; no slide has dead black regions or particles crossing copy; frame time
under 8 ms on the development machine at 1080p.

### WP2 Ember layouts (M)

- Ember treatments for code, table, image, gallery and two-column slides:
  serif heading, eyebrow, hairline frame, palette, and the copy column where
  the layout has copy.
- A `Layout::measure` for the Ember copy column so overflow and scrolling use
  the same geometry as drawing (closes backlog 5.1 for Ember).
- Pillow and text sizes tuned on a 1080p projector window and a 4K monitor.
- Chrome: counter, hairline, beat ticks, footer text from `@footer`.

Acceptance: `samples/layouts/*.md` and `samples/visualizations/all.md` export
cleanly under Ember with no overflow indicators where none are needed.

### WP3 Countdown (S)

Already implemented; finish with a live check of the 3 appearing in full on
a cold start, the burst reading as a burst, and the Nord numerals' timing.
Document the `@countdown` switch in the spec (done) and README (done).

### WP4 Stories (M)

- Remove the inline `@scene` fence and its parser variant; `--check` explains
  the change for decks that still carry one.
- Label collision test over every script in the sidecar and over generated
  scripts before saving (reject and retry once on overlap).
- Beat theatrics: cast entering rises into place; a flow visibly starts at
  its beat; a hot element pulses once when it heats.
- `mdeck ai story --dry-run` prints the scripts and their `say` lines without
  writing; the interactive run shows each slide's lines before saving.
- A per-deck cast sheet (names and roles) generated once and reused, so
  people stay consistent across slides and regenerations.
- Stale story handling in the app: a HUD note and a `Shift+S` to refresh only
  stale slides.

Acceptance: regenerating `samples/ember.md` produces scripts that pass
validation and the collision test without retries on at least four of five
runs; the hinted slide follows its hint on every run.

### WP5 Presenter surface (M, after WP4)

- The `say` line and the next beat's line in the HUD, and in the presenter
  view when backlog item 1.1 lands. This is the reason stories carry lines.
- Beat ticks in the counter for every theme that plays beats (Ember only for
  now).

### WP6 Code health and release (M)

- `Theme` becomes an enum; the six string comparisons go.
- Bundled fonts for every theme, with dark and light gaining real bold, which
  closes backlog item 1.8.
- Samples reorganised: `samples/ember/` with one deck per feature (layouts,
  scenes, stories, countdown) plus the showcase; gallery images regenerated;
  README gallery preview gains one Ember still.
- Visual regression: export the Ember showcase in CI on a software GL context
  if available, otherwise as a manual pre-release step, and compare with a
  perceptual hash and tolerance.
- The five documentation places updated for every change; the spec's Ember
  section rewritten as a proper chapter; a "story your first deck" walkthrough
  in the README.
- Release as 0.20.0 "Ember" with the changelog entry consolidated.

## 5. Testing strategy

- Unit tests where the spike put them: scene inference per layout, script
  validation and staging, sidecar resolution and staleness, step counting per
  theme, countdown phases, digit masks, export selection.
- New: collision test for staged scripts; `SceneHints` extraction per
  renderer; wakes framebuffer creation guarded so a failure falls back to the
  crisp sprites with a logged incident.
- Visual: single-slide exports (`--slide N --debug`) are the review unit for
  humans and agents; every WP ends with a reviewed set of stills.
- Runtime: the app is smoke-run through the countdown and the showcase deck
  before each commit that touches the field; incidents must be empty.

## 6. Risks

- Wakes on macOS OpenGL (deprecated, but working): the offscreen framebuffer
  must survive window resizes and the export tiling. Fallback is the current
  crisp field.
- Content-aware hints couple renderers to the field. Kept one-way (renderers
  publish, the field reads) and optional, so a renderer without hints costs
  nothing.
- Story generation cost and latency are the presenter's choice (explicit
  command or key), never automatic.

## 7. Open questions

- Whether `mdeck ai create` should write stories as part of creating a deck,
  or leave that to an explicit `mdeck ai story` afterwards. Recommendation:
  explicit, until WP4's quality gates are in.
- Whether a `--reduced-motion` flag is enough, or Ember also needs a
  `@motion: calm` frontmatter level that lowers particle count and drift.
