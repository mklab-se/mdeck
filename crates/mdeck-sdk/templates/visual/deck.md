---
title: The @{{name}} visual
theme: {{name}}
---

# The @{{name}} visual

Labelled values as horizontal bars

---

## Revenue by region

```@{{name}}
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

```@{{name}}
- Done: 8
- Open: 3 (color: 4)
```
