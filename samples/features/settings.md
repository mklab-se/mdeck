---
title: "Slide settings"
theme: ember
---

# Slide settings

Slides split by headings, no `---` anywhere

# Where to write them
<!-- picture: server -->

- In an HTML comment in the slide: `<!-- picture: server -->`
- `design`, `picture`, `logo`, `background`, `transition`, ...
- They never show, here or on GitHub

# Anywhere in the slide
<!-- picture: laptop -->

A settings comment applies to the slide it is written in,
and never moves to another one.

`mdeck --check` warns about typos such as `pictur`, about deck
settings in a slide, and names the v2 form of v1 syntax.

# Two columns, no separator
<!--
design: columns
logo: none
-->

**Before**

- `@layout: two-column` on a line of its own
- Visible on every other renderer

+++

**Now**

- One comment, several settings
- Invisible everywhere but in mdeck
