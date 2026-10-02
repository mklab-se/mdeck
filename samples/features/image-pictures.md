---
title: "Image pictures"
theme: ember
# Export with --theme dark to see the standard set open a stage for each
# picture, and the plain engine stipple the point cloud.
---

# Poker Night
<!-- picture: ../images/poker-1.png -->

A picture can be an image file

---

## The table
<!-- picture: ../images/poker-2.png -->

- `picture:` names an image file, relative to the deck
- mdeck draws it on the slide's stage, on every engine
- The design decides where the stage is

---

## Or a point cloud
<!-- picture: rocket -->

- `picture: rocket` names a built-in point cloud
- A picture engine draws it in its own medium
- On `plain`, mdeck stipples it in the theme's accent

---

<!-- picture: ../showcase/station.jpg -->

> "Show the thing, then say the thing."

The stage sits beside the quote

---

## Code gives its stage up

```rust
// a picture on this slide would be ignored,
// and `mdeck --check` would say so
fn main() {}
```
