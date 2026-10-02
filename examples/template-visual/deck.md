---
title: The @template-visual visual
theme: template-visual
---

# The @template-visual visual

Labelled values as horizontal bars

---

## Revenue by region

```@template-visual
title: Revenue (MEUR)
max: 60
- North: 42
- South: 31 (color: 1)
+ East: 55 (color: 2)    # appears on the next step
+ West: 27 (color: 3)
```

```@notes
East and West appear one step at a time.
```

---

## Beside a point list
<!-- design: columns -->

- Short fences read well
- Every item is a list line

+++

```@template-visual
- Done: 8
- Open: 3 (color: 4)
```
