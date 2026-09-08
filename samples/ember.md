---
title: "Ember"
author: "MKLab"
@theme: ember
@transition: fade
@story: >
  Calm, factual, a little dry. The people in these stories are ordinary
  colleagues in a Stockholm software company: developers, a product owner,
  a support engineer. Keep the same person across slides when one fits.
---

# Light, held quietly

A theme for MDeck drawn from the MKLab site: graphite on near-black, one ember accent, and a living field of particles behind every slide.

---

## What you are looking at

- Nine hundred particles that *migrate* between slides instead of cutting
- Additive glow drawn in a dedicated OpenGL pass, so overlapping lights **burn hotter**
- An editorial serif for headings, a light grotesque for copy, tracked mono for the fine print
- The same reveal steps you already write, now lighting one cluster per item

---

## The rules of the room

Every choice here follows three rules from the brand book.

+ **Monochrome first.** Ink on ink. The palette is a ladder of greys, and the heat comes from a single accent.
+ **One voice of colour.** Ember carries every highlight. Nothing else is allowed to be loud.
+ **Restraint in motion.** Particles are actors, traces are wakes, and nothing rushes at the viewer.

---

# Part two

---

## The tickets nobody read

Anders automated triage and stopped reading the queue. The assistant answered
faster than he ever had, and for three weeks nobody looked at what it said.

+ The queue went quiet
+ The answers went out
+ One of them was wrong in a way that mattered


???
This slide's story is hand-written: a pinned entry in ember.scenes.yaml. The
beats are released with Space, one per line.

---

## The same day, done differently

Sofia kept the assistant. She also kept reading.

+ Every answer waited in a review queue
+ A rule flagged anything that mentioned money
+ The assistant learned from what she changed

```@story
Sofia, a support engineer, sits at her laptop on the left. The support queue
feeds an AI orb, but between the orb and the outgoing mail there is a gate
that she controls. Step one: tickets flow into the orb. Step two: the orb's
answers flow to the gate and wait. Step three: Sofia's laptop connects to the
gate and it lights up as she reviews. Step four: mail leaves the gate, calm
and white, nothing hot.
```

???
This slide has an English @story hint; `mdeck ai story samples/ember.md`
turns it into a scene in the sidecar.

---

> Less deck. More decision.

-- MKLab, on what a presentation is for

---

## Code keeps its own weather

```rust
fn scene_for(slide: &Slide) -> Scene {
    match slide.layout {
        Layout::Title   => constellation(),
        Layout::Quote   => candle(),
        Layout::Code    => rain(),
        Layout::Bullet  => clusters(slide),
        _               => quiet(),
    }
}
```

---

## Where the time goes

```@barchart
# y-label: Hours
- Writing: 12
- Design: 3
- Rehearsal: 6
- Delivery: 1
```

---

## How a slide becomes a scene

```@architecture
- Markdown  (icon: storage,   pos: 1,1)
- Parser    (icon: function,  pos: 2,1)
- Scene     (icon: container, pos: 3,1)
- Field     (icon: monitor,   pos: 3,2)
- Glow      (icon: browser,   pos: 4,2)

- Markdown -> Parser: reads
- Parser -> Scene: layout and steps
- Scene -> Field: homes
- Field -> Glow: sprites, additively
```

---

## Ordinary markdown still works

Any deck you already have can switch to this theme with one line of frontmatter.

- Headings split slides, as before
- Lists, quotes and code keep their meaning
- Charts and diagrams pick up the cocktail palette
- Everything else is *inferred*

---

# Thank you

Switch the theme with `Shift+T`, or set `@theme: ember` in your frontmatter.
