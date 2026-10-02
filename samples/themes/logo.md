---
title: "A logo without a custom theme"
theme: summer
logo: ../design-systems/mdeck-co/assets/logos/mdeckco-black.svg
logo-position: bottom-right
logo-opacity: 70%
---

# A logo on a built-in theme

`logo` in the frontmatter, no custom theme needed

---

## Four keys

- `logo`: a PNG or SVG, relative to the deck
- `logo-position`: any corner (top-right by default)
- `logo-opacity`: `0.6` by default, `70%` here
- `logo-height`: `56` px on a 1920x1080 slide

---

## A photo without the logo
<!-- logo: none -->

`<!-- logo: none -->` in a slide hides the logo on that slide only

---

## A partner's logo on one slide
<!-- logo: ../../media/MDeck-logo.png -->

A file under the heading shows that logo on this slide,
in the deck's corner, size and opacity
