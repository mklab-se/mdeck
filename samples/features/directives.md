---
title: "Directives under the heading"
@theme: ember
---

# Directives under the heading

Slides split by headings, no `---` anywhere

# Where to write them
@illustration: server

- Directly under the slide's heading
- `@illustration`, `@layout` and `@logo`
- They never show on the slide

# Anywhere at the top level
@illustration: laptop

A slide directive applies to the slide it is written in.

`mdeck --check` warns about typos such as `@ilustration`,
and about directives that sit inside a list or a quote.

# Two columns, no separator
@layout: two-column

**Before**

- A `---` before every directive
- Or the line above the heading

+++

**Now**

- Right under the heading
- Where you would write it anyway
