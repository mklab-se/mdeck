# MDeck Co: Design System

> Senior advisory, quietly held. A monochrome, luxury design language for
> a modern consulting firm. Graphite on black, one ember accent, and the
> restraint of a black credit card you can only wish you'd reach.

---

## 1. The company

**MDeck Co** is a modern consulting firm built around senior operators:
the people who have carried the outcome themselves, not just advised on
it. The positioning is an **open offer, not a velvet rope**: *whatever the
challenge, MDeck Co can help.* The firm is inclusive by design (anyone can
come to MDeck Co), but it specialises in the hard problems, the projects
that demand the most senior people in the room and that others pass on.
The message is *"we have what it takes,"* never *"do you?"*

The brand should feel luxurious and hard-to-*match* (in capability) without
ever being hard-to-*reach*. The heat, glow, and restraint signal a
standard of work, not a gate.

**Influences**
- **A rangefinder camera**: subtlety, precision, and a logo shown quietly.
  Never loud.
- **A dark cocktail bar** (see `assets/brand/mood.jpg`): candle amber
  glow, deep red backlighting, black leather and wood. Warmth in the dark.
- **The black card**: monochrome, weighty, understated, exclusive.

### Provided source material
No codebase or Figma was supplied. This system was authored from a brand
brief plus three assets (now in `assets/`):
- `mdeckco-white.svg` / `mdeckco-black.svg`: the MDeck Co mark, three cards
  of a deck with the front one solid.
- `signature-line.svg`: a single-stroke horizon used as a signature.
- `mood.jpg`: the reference bar photograph (mood and light).

Because no component inventory was provided, a **standard component set**
was authored (see §7). No brand logo was invented; the supplied MDeck Co
mark is used throughout.

---

## 2. Content fundamentals: how MDeck Co writes

**Voice:** confident, spare, adult, and **welcoming**. The firm never
oversells and never gatekeeps; it offers. The reader is addressed as
**you**; the firm is **we**. Short sentences. Full stops used as
punctuation for emphasis. *("Less deck. More decision.")* The seniority
belongs to MDeck Co, never framed as a bar the client must clear.

**Casing:** Sentence case for headlines and body. **Tracked UPPERCASE**
(mono) reserved for eyebrows, labels, and metadata: the "precision"
signal. Roman numerals appear as a quiet luxury tell (`EST. MMXIX`).

**Tone examples (use these as a north star):**
- Hero: *"Bring us the hard problems."*
- Sub: *"Whatever the challenge, MDeck Co can help. We're built for the ones
  that demand the most senior people in the room: the projects others
  pass on. Need experience? We have what it takes."*
- Ethos: *"Every engagement is led by someone who has done it before."*
- Access: *"However hard, we can help."* / *"Tell us what's hard."*
- Confirmation: *"We've got it."*

**Do:** be brief, be certain, be generous with the offer. Let one line
carry weight. **Don't:** gatekeep, imply the reader may not qualify, use
exclamation marks, hype adjectives ("world-class", "cutting-edge"), or
emoji. **No emoji, ever.**

---

## 3. Visual foundations

**Colour.** The system is monochrome by rule. Backgrounds and surfaces are
the **Ink** scale (near-black `#050505` up through white). The **only**
chromatic voice is **Ember** (`#ff4d1c`), a red-orange drawn from the
bar's backlight, used for the single most important action and its glow.
**Candle** (amber `#f5a623`) is a rare warm secondary. An extended
**cocktail palette** (bright drink colours) exists strictly for data
visualisation, never for chrome, buttons, or fills.

**Type.** Editorial-luxury pairing:
- **Spectral** (serif, weight 300): display and big moments. Tight tracking.
- **Hanken Grotesk**: UI and body. Precise, modern, faintly warm.
- **JetBrains Mono**: eyebrows, labels, data, metadata.
*(These were chosen fresh; no source font files were provided. Swap if
the firm has licensed faces; see §8.)*

**Spacing.** 4px base grid, but the rhythm is **generous**: luxury
breathes. Sections use large vertical padding (96 to 128px).

**Backgrounds.** Predominantly flat near-black. Photography is used
full-bleed but **heavily darkened** (opacity ~0.5) with a black gradient
scrim so text sits comfortably. One subtle radial ember wash is permitted
on feature panels (the "black card" rail). No repeating patterns, no
busy gradients, no purple.

**Elevation & the glow.** Shadows are deep, soft, and *black*, with no
gray haze (`--shadow-sm…xl`). The signature is the **ember glow**
(`--glow-ember-sm…lg`): a tight 1px ring plus a soft bloom, applied only
to primary/interactive moments (primary buttons, focused fields, checked
controls). It's what makes controls feel alive and expensive. A softer
`--glow-candle` exists for warm secondary highlights.

**Borders.** Structure on black is carried by light **hairlines**
(white at 6% / 10% / 18%). Accent borders glow.

**Corner radii.** Restrained, nothing bubbly. Controls `8px`, cards
`12px`, dialogs `18px`, pills full. Never larger than needed.

**Motion.** Confident and slow: *nothing bounces*. Decelerating
`--ease-out` (cubic-bezier(0.16,1,0.3,1)); durations 140 to 380ms. Things
**glow in**, they don't pop. Reduced-motion respected in animations.

**Hover / press.** Hover: primary intensifies its glow and darkens one
step; ghost/secondary lift from transparent to a faint surface. Press:
a small `scale(0.99)` / `0.94`, a tactile, machined click. Focus: a
2px ember ring offset from the surface.

**Transparency & blur.** Used sparingly. The sticky nav and dialog
backdrop blur the "room" behind them (`--blur-overlay`), echoing the
bar's depth of field.

**Imagery vibe.** Warm-in-the-dark: amber and deep red highlights over
near-black, shallow depth of field, no cold tones. Always darkened before
type is placed over it.

**Cards.** Graphite surface (`--surface-card`), a 6% hairline, soft black
shadow, 12px radius. Interactive cards lift 2px and firm their border on
hover. One `glow` variant reserves the ember halo for the single card you
want looked at first.

---

## 4. Iconography

- **System:** [Lucide](https://lucide.dev), loaded from CDN
  (`https://unpkg.com/lucide@latest`). Chosen because its **1.5px stroke**
  matches the brand's quiet, precise line. Wrapped by the `Icon` component
  so every glyph inherits the correct size/stroke/colour.
- **This is a substitution/addition**: no icon set was supplied. If MDeck Co
  adopts a bespoke set, replace the `Icon` internals; the API stays.
- **No emoji.** No unicode-glyph icons. No hand-rolled decorative SVGs.
- The **MDeck Co mark** and the **signature line** are the only bespoke
  marks (in `assets/`), used as brand signatures, not UI icons.

---

## 5. Assets (`assets/`)
- `logos/mdeckco-white.svg`: primary mark for dark surfaces (shown at low
  opacity for the "subtle gray" treatment, or full white on a surface).
- `logos/mdeckco-black.svg`: mark for light surfaces (rare; system is dark).
- `brand/signature-line.svg`: single-stroke horizon signature.
- `brand/mood.jpg`: reference light / hero photography.

---

## 6. Tokens (`styles.css` → `tokens/`)
Consumers link **`styles.css`** only. It `@import`s:
- `tokens/fonts.css`: Google Fonts (Spectral, Hanken Grotesk, JetBrains Mono).
- `tokens/colors.css`: Ink scale, Ember, Candle, cocktail palette,
  semantic status, and semantic aliases (`--surface-*`, `--text-*`,
  `--border-*`, `--accent*`).
- `tokens/typography.css`: families, weights, scale, tracking, roles,
  and the `.mdc-eyebrow` utility.
- `tokens/spacing.css`: 4px grid, containers, gutters.
- `tokens/effects.css`: radii, borders, shadows, **the glow**, focus ring.
- `tokens/motion.css`: durations, easings, composed transitions.

---

## 7. Components (`components/`)
Reusable React primitives. Import in card HTML via
`const { X } = window.MDeckCoDesignSystem_4d3c0a` after loading `_ds_bundle.js`.

**core/**: `Button`, `IconButton`, `Icon`, `Card`, `Badge`, `Tag`, `Divider`
**forms/**: `Input`, `Textarea`, `Select`, `Checkbox`, `Radio`, `Switch`
**feedback/**: `Dialog`, `Toast`, `Tooltip`
**navigation/**: `Tabs`

Every component ships a `.jsx`, a `.d.ts` (props contract), and a
`.prompt.md` (usage). Each directory has a `@dsCard` showcase HTML.

**Intentional additions** (no source inventory existed, so a standard set
was authored):
- `Icon`: a thin wrapper over Lucide so the brand controls glyph stroke
  and sizing centrally.

---

## 8. Caveats / substitutions
- **Fonts** are Google-Fonts stand-ins chosen for the brand, not supplied
  files. If MDeck Co has licensed typefaces, drop them in and swap
  `tokens/fonts.css`.
- **Icons** are Lucide via CDN (a substitution, see §4).
- No slide template was provided, so no sample slides were authored.

---

## 9. Index / manifest
- `styles.css`: global entry (imports only).
- `tokens/`: the CSS custom-property foundations.
- `assets/`: logos, signature line, reference photography.
- `foundations/*.card.html`: specimen cards (Colors, Type, Spacing,
  Effects, Brand) shown on the Design System tab.
- `components/{core,forms,feedback,navigation}/`: the primitives.
- `ui_kits/website/`: the MDeck Co consulting site (interactive recreation).
- `SKILL.md`: Agent-Skills-compatible entry point.
