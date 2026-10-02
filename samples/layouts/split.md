---
title: "Design: split"
theme: dark
transition: fade
---

# Design: split
Text beside one picture


# Points and a picture

- First point about this topic
- Second point with more detail
- Third point wrapping up
- Fourth point for good measure

![Poker scene](../images/poker-1.png)


# Paragraphs and a picture

The copy sits in one column and the picture in the other, whichever comes
first in the markdown.

A second paragraph keeps the slide a split rather than a media slide.

![Poker table](../images/poker-3.png)


# An ordered list and a tall picture

1. Step one of the process
2. Step two continues
3. Step three follows naturally
4. Final step completes

![Saloon](../images/saloon-vertical.png)


# A filling image stays in its panel

![Saloon @fill](../images/saloon-horizontal.png)

- `@fill` covers the picture's panel
- and is cut at its edge, never over the text


# Code and a picture are content

```rust
fn main() {
    println!("Hello, world!");
}
```

![Poker scene](../images/poker-2.png)

Code with an image is not a split: the `content` design shows both, in
reading order.
