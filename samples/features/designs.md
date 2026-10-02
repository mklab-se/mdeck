---
title: Designs
author: Ada Lovelace
---

# Every slide has a design

One deck, thirteen designs

---

## Part two

### Where designs come from

---

## Why designs

Slides should look designed without the author having to design them.

---

## Recognised from the content

The first rule of the table that matches gives the design.

- A heading and a list is `points`
- A heading and a sentence is a `statement`
  - nested items keep their own level
  - and their own bullet
- One image and some text is a `split`

1. Ordered lists count
2. In both design sets

---

## Text beside a picture

![Poker night](../images/poker-1.png)

The copy sits in one column and the picture in the other.

- No content is dropped
- The image never covers the text

---

## One image, large

Above the picture, a short lead.

![A saloon](../images/saloon-horizontal.png)

A caption under the image.

---

## Several images

![Cards](../images/poker-1.png)

![Chips](../images/poker-2.png)

![Table](../images/poker-3.png)

---

## A quotation

> Simplicity is prerequisite for reliability.

-- Edsger W. Dijkstra

---

## Code with context

The renderer draws whatever the arrangement says.

```rust
fn design(slide: &Slide) -> Design {
    RULES.iter().find(|r| (r.test)(&shape)).design
}
```

---

## A chart

Revenue by quarter, in millions.

```@bar
- Q1: 12
- Q2: 18
- Q3: 15
- Q4: 24
```

---

## Side by side

### Standard

Centred copy, no ornament, no entry motion.

+++

### Editorial

A copy column, an eyebrow, display type and a stage.

---

## Tabular data

| Design | Holds |
|---|---|
| `points` | a heading and a list |
| `media` | one image, large |
| `table` | a heading and a table |

---

## Anything else

Content holds what no other design takes, in reading order.

- A list

```rust
let x = 1;
```

> And a quote at the end.
