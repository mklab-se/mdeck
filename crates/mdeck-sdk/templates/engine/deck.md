---
title: The {{name}} engine
theme: {{name}}
---

# The {{name}} engine

A sample deck that shows the engine on the common designs

```@notes
Present it with your custom build: `./target/release/mdeck deck.md`.
```

---

## Points

- The engine paints under every slide
- It scales with the window
- It takes its colours from the theme

---

## A statement
<!-- design: statement -->

Calm motion keeps the room's attention on the words.

---

## A chart

```@bar
- North: 42
- South: 31
- East: 55
- West: 27
```

---

> An engine is the room the words are spoken in.

---

## Code

```rust
fn update(&mut self, frame: &Frame, stage: &Stage) {
    self.clock += frame.dt;
}
```
