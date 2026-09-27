# Design systems

Sample design systems, used to design and test how MDeck turns a design
system into a theme (issue #14).

## `mdeck-co/`

A design system as people get it when they **export a design system project
from Claude** (claude.ai/design): an [Agent Skills](https://agentskills.io)
entry point, a readme with the brand rules, CSS custom properties as tokens,
React components, specimen cards and a UI kit. It is a real export with the
brand replaced by the fictional MDeck Co (new mark, signature line and a
generated mood photograph), so its shape, depth and quirks are what MDeck will
meet in the wild.

| Path | What it is |
|---|---|
| `SKILL.md` | Agent entry point (`name`, `description`, how to use the folder) |
| `readme.md` | Brand, voice, visual foundations, iconography, assets, caveats |
| `styles.css`, `tokens/*.css` | The tokens, as CSS custom properties on `:root` |
| `foundations/*.card.html` | Specimen cards (colours, type, spacing, effects, brand) |
| `components/**` | React primitives: `.jsx`, `.d.ts` props contract, `.prompt.md` usage |
| `ui_kits/website/` | An interactive site built from the components |
| `assets/` | Logos for dark and light surfaces, signature line, mood photograph |
| `_ds_bundle.js` | The components compiled for the cards (`window.MDeckCoDesignSystem_4d3c0a`) |
| `_ds_manifest.json` | Claude's index of components, cards and tokens (raw CSS values) |
| `_adherence.oxlintrc.json` | Lint rules that flag raw hex, raw px and foreign fonts |

What matters for MDeck:

- Everything a slide theme needs is in `tokens/*.css` and `readme.md`, but as
  CSS (`var()` chains, `rgba()`, font shorthands) and prose, not as typed
  values. `_ds_manifest.json` lists the tokens, with the same raw CSS values.
- There is **no slide layer**: the readme says so itself ("No slide template
  was provided"). Choosing slide sizes, which ink step is body text and the
  chart series order is left to whoever builds the theme.

Open the cards and the UI kit in a browser straight from disk; they load
React, Babel and Lucide from CDNs.
