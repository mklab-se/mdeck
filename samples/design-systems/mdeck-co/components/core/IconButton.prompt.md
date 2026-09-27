Square button for a single glyph action (toolbar, close, overflow menu). Always pass `aria-label`.

```jsx
<IconButton name="x" aria-label="Close" onClick={close} />
<IconButton name="arrow-right" variant="primary" aria-label="Continue" />
```

Variants: `ghost` (default), `secondary`, `primary` (glows). Sizes `sm | md | lg`.
