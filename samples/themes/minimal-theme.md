---
title: "Custom themes: ten lines"
theme: acme
---

# Ten lines of YAML

`themes/acme.yaml` extends `light` and changes the rest

---

## A brand in a few keys

- `colors`: background, text, heading and **one accent**
- `series`: the chart palette, accent first
- `fonts.display`: a bundled serif for headings
- Everything else comes from `extends: light`

---

## Charts follow the series

```@bar
- North: 42
- South: 68
- East: 55
- West: 81
```
