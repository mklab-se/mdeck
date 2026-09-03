---
title: "Layout Test: Bullet Slides"
@theme: dark
@transition: fade
---

# Layout Test: Bullet Slides
Focused tests for the bullet layout


# Simple Unordered List

- First item
- Second item
- Third item
- Fourth item


# Ordered List

1. Clone the repository
2. Install dependencies
3. Run the test suite
4. Submit a pull request


# Nested Ordered List

1. Set up the environment
   1. Install Rust toolchain
   2. Clone the repository
2. Build the project
   1. Run cargo build
   2. Verify no errors
3. Deploy


# Nested List with Reveals

+ Top-level item one
  - Sub-item alpha
  - Sub-item beta
+ Top-level item two
  - Sub-item gamma
    - Deep nested item
+ Top-level item three
* Also part of item three


# Deeply Nested

- Level one
  - Level two
    - Level three
      - Level four
        - Level five
          - Level six keeps a readable width even this deep, so long text still wraps sensibly


# Long List That Overflows

- The first point is long enough to wrap onto a second row at the bullet column width
- The second point is also long enough to wrap onto a second row at the bullet column width
- A third point, likewise long enough to wrap onto a second row at the bullet column width
- A fourth point that continues the pattern and wraps onto a second row as well
- A fifth point to make sure the slide clearly overflows the available height
- A sixth point so the scroll indicator has something to reveal below the fold
- And a seventh point that lives entirely below the fold until scrolled into view
