---
title: "Layout Test: Content Slides"
@theme: dark
@transition: fade
---

# Layout Test: Content Slides
Focused tests for the content (fallback) layout


# Mixed Content

This slide has a paragraph followed by a list and more text.

- Item one
- Item two

And then another paragraph after the list.


# Paragraphs and Code

Here is some introductory text explaining the concept.

```bash
echo "Hello from the terminal"
```

And a follow-up paragraph with **bold** and *italic* formatting.


# Table with Context

Here are the current benchmarks:

| Format | Parse Time | Render Time |
|--------|-----------|-------------|
| Markdown | 2ms | 8ms |
| HTML | 5ms | 12ms |
| LaTeX | 15ms | 45ms |

All times measured on an M1 MacBook Pro.


# Inline Styles

This paragraph mixes **bold text**, *italic text*, ~~struck text~~, `inline code`, and a [link to mdeck](https://github.com/mklab-se/mdeck) in one line.

- **Bold** at the start of a bullet
- A bullet with `code` and a [link](https://example.com)


# Wide Table

| Region | Q1 | Q2 | Q3 | Q4 | Total | Growth | Notes |
|--------|----|----|----|----|-------|--------|-------|
| North | 120 | 135 | 150 | 170 | 575 | +12% | Strong finish |
| South | 98 | 102 | 110 | 115 | 425 | +5% | Steady |
| East | 140 | 138 | 145 | 160 | 583 | +8% | New office opened |
| West | 75 | 80 | 92 | 105 | 352 | +15% | Fastest growing |


# Table with Long Cells

| Feature | Description |
|---------|-------------|
| Overflow | Long slides scroll smoothly, with a fade at the bottom edge to hint that more content is available |
| Layout | Layouts are inferred from the markdown structure, so any file is presentable without tailoring |
| Images | Images never upscale beyond their reference size, so exports look the same at every resolution |
