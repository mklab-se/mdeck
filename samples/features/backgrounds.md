---
title: Background images
background: ../images/saloon-horizontal.png
background-opacity: 30%
---

# Background images

A deck-wide image behind every slide, set once in the frontmatter

# Every slide gets the deck's image

- `background` in the frontmatter puts an image behind every slide
- `background-opacity` tones it down: 0 to 1, or a percentage
- The image covers the slide: scaled, centred and cropped, never stretched
- Text stays readable: the image blends toward the theme's own colour

# A slide can use its own image
<!--
background: ../images/poker-1.png
background-opacity: 40%
-->

- `<!-- background: ... -->` in a slide replaces the deck's image on that slide
- It keeps the deck's opacity unless the slide sets its own

# Or turn it off
<!-- background: none -->

```rust
fn main() {
    // A clean slate for code: `<!-- background: none -->`
    println!("Hello from a slide without a background");
}
```

# Opacity alone
<!-- background-opacity: 60% -->

- A slide that sets only `background-opacity` reuses the deck's image
- Higher values let the picture lead; keep the text short

# SVG works too
<!--
background: background-grid.svg
background-opacity: 35%
-->

- PNG, JPEG, WebP and SVG
- Paths are relative to the deck file
