# Point cloud illustrations

Date: 2026-09-15. Status: approved.

Point clouds become the one way the particle engine draws a thing. A point
cloud is a named file of importance-ordered points that an AI-generated image
was reduced to. MDeck ships a built-in library, a user keeps their own, a deck
carries its own, and a slide asks for one with `@illustration: server`. The
story cast draws from the same library, replacing the hand-drawn silhouettes.

Out of scope for this spec: contributing clouds back to MDeck (a later spec
that builds on the metadata this format already carries).

## 1. File format

An `.mdpc` file is JSON:

```json
{
  "version": 1,
  "name": "server",
  "description": "A server, in a rack, in a datacenter",
  "prompt": "the full prompt sent to the image model, or null for imports",
  "generated": "2026-09-15 09:10",
  "aspect": 1.35,
  "points": [[0.512, 0.031], [0.104, 0.980]]
}
```

- Points are in the unit square (x and y both 0..1); `aspect` is height over
  width of the bounding box. This is the shape `Home::Mask` already takes.
- Points are stored in **importance order**: the first 60 sketch the whole
  subject, the first 600 fill it in. Consumers take the first n.
- At most 1500 points (about 25 KB). Names match `[a-z0-9-]+`.

## 2. Conversion

`illustration::convert(image) -> Cloud` serves both `generate` and `import`:

1. Composite over black, downscale to 512 px on the long side.
2. Keep pixels brighter than a fraction of the peak brightness.
3. Order by greedy farthest-point sampling weighted by brightness: brightest
   pixel first, then repeatedly the candidate farthest from everything chosen,
   favouring bright ones. Stop at 1500 points or when the next point would
   sit closer than a minimum spacing.
4. Crop to the chosen points' bounding box with a small margin; normalise.

The generation prompt asks for a sparse constellation of glowing warm
particles forming only the essential outline of a single centred subject on
pure black, no text, no ground. Conversion is deterministic.

## 3. Library and lookup

`render::illustration` owns `Cloud`, parsing and `resolve(name, deck_dir)`:

1. `<deck dir>/illustrations/<name>.mdpc`
2. `~/.config/mdeck/illustrations/<name>.mdpc`
3. built-in (`crates/mdeck/illustrations/*.mdpc`, embedded with `include_str!`)

First match wins, so a deck can shadow the user library, which can shadow a
built-in. Built-ins for this spec: the thirteen former story kinds (person,
hooded, box, orb, doc, docs, inbox, db, cloud, laptop, folder, mail, gate)
plus server, robot, phone, globe, lock, gear, rocket. Each is generated with
the new command and checked at cast size and full-frame. Loaded clouds are
cached per session and dropped when the deck reloads.

## 4. Deck directive and placement

`@illustration: server` at the top of a slide (a block directive, like
`@layout`). Ignored outside Ember. Placement by layout:

- **Title**: full-frame behind the centred copy, ~60% of the slide tall, dim,
  larger and softer sprites, slow breathing.
- **Bullet, content, quote, section**: on the right-half story stage, fitted
  to the stage box, brighter, ~70% of the pool.
- **Code, visualization, diagram, image, gallery, two-column**: not shown;
  `--check` warns. `--check` also warns on unknown names.

Lit from step 0; no part in reveals. Story and illustration on one slide: the
story wins and `--check` warns (a cast member can name the same cloud).

## 5. Stories on point clouds

- `Kind` and `silhouettes.rs` are removed. A cast member's `kind` is a name
  resolved through the library lookup; `kind: laptop` keeps working and
  `kind: server` works once `server.mdpc` resolves.
- Validation rejects unknown kinds, listing the available names. The story
  generator's allowed values come from the deck's resolved library.
- `person` and `hooded` remain the figure names (sizing, label placement).
- Fills, flows, cells and beats are unchanged.
- Field change: `Home::Mask` assigns the first n points to a group of n
  particles (cycling when the mask is smaller) instead of a random point per
  particle. Glyph masks (countdown digits, end words) are shuffled once when
  built so the same rule holds.

## 6. CLI

```
mdeck illustration generate --name server --description "A server in a rack" [--user] [--force]
mdeck illustration import <image> --name server [--user] [--force]
mdeck illustration list
mdeck illustration show <name>
```

- `generate` needs the image capability; writes `illustrations/<name>.mdpc`
  in the current directory, or the user library with `--user`.
- Existing files are overwritten only with `--force`.
- `list` prints every name with its source (deck, user, built-in) and notes
  shadowing.
- `show` plots the cloud as dots on black into a PNG with the `image` crate
  and displays it inline like image results today.

## 7. Testing and docs

Unit tests: format round-trip; conversion on synthetic images (a ring gives
points on the ring, aspect 1, first points pairwise far apart); determinism;
lookup shadowing with temp directories; directive parsing; placement per
layout; check warnings; story validation of unknown kinds; the first-n mask
rule. A new `samples/ember/illustrations.md` exercises every layout; it and
`samples/ember/stories.md` are exported before and after. Spec, quick
reference, README, changelog, AI reference supplement, story prompt and
CLAUDE.md are updated in the same change.
