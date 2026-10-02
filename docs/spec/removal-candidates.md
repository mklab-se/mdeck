# Removal candidates

Features that do not earn their place in v2. Nothing has been
removed from the code yet; this records what v2 drops. The test for every feature is the vision ([00](00-vision.md)): does it make
presentations more effortless or more breathtaking, in proportion to what it costs to explain,
maintain and keep consistent?

## Decided to remove

| Feature | Why | Instead |
|---|---|---|
| `@` prefix on frontmatter keys | Makes the frontmatter invalid YAML; buys nothing. | Plain keys (LANG-06). |
| Visible `@key: value` slide directives | Visible on every other renderer; extraction rules cannot be explained simply. | Settings in HTML comments (LANG-08). |
| Trailing-directive migration | Settings jumping to the next slide is the root of the "only works with `---`" confusion, and of D1. | A setting applies to the slide it is in (MD-07). |
| Three blank lines as a slide break | Invisible syntax. | Headings, or an explicit break. |
| `*` as "reveal with previous" | A trap for imported markdown, where `*` is just a bullet. | Nest items under a `+` item (MD-17). |
| The obsolete `@scene` fence | Dead feature with live parsing, model and warning code. | Nothing; `--check` flags it as v1 syntax (LANG-07). |
| Reserved but unimplemented names: `@aspect`, `@code-theme`, `@class`, frontmatter `date`; diagram `sequence`, `label`, `style` | They look supported and do nothing. | Add each when it works (LANG-05). |
| Capabilities `numbers_slides`, `cold_open`, `heat_trace` | One engine each: names in disguise. | Engine internals behind generic hooks (ENG-04). |
| The `editorial` capability | Ties a text design to the engine. | The `editorial` design set, chosen by the theme (DES-10). |
| Stories | A large feature (schema, AI generator, sidecar, beats, capability) for one engine and four layouts; "most decks will not carry any". | Nothing for now. Staged pictures may return later as a picture source every picture engine can draw (PIC-06). |
| `???` notes separator | Notes that contain markdown headings would split the slide. | A ```` ```@notes ```` fenced block (MD-14). |
| Prefix matching, `chart` suffixes and aliases on visual tags | False positives, silent typos, two spellings for one kind. | One exact name per kind, no aliases (VIZ-02). |
| The `laser` engine and its `etch` theme | Settled stills are a faint outline; overlaps with the line-drawing engines (engine review, Q15). | Nothing; the `line`, `sketch` and particle engines cover drawn pictures. |
| Separate `blueprint` and `chalkboard` engines | Same line-art style and the same pictures; they differ mainly in the ground and the tool. | One `line` engine with `surface: sheet` or `surface: slate`; both themes stay (ENG-17a). |
| Separate `@illustration` and `@art` | Two names for the stage picture. | One `picture` setting (PIC-01). |
| Placeholders rewritten in place by `mdeck ai generate` | The deck changes under the author; no regeneration. | A manifest and stable placeholders (GEN-04). |

## Decided to keep

- Autumn and winter (Ember recoloured) and spring and summer (Light recoloured), as **variants**
  of their theme (THM-14), not as top-level themes.
- `---` as an optional explicit slide break (MD-04).
- The `+++` column separator (LANG-14).
