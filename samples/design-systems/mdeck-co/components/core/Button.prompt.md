Primary call-to-action control. Use `primary` for the single most important action on a view (it carries the ember glow: never place two on one screen); `secondary`/`outline`/`ghost` for everything else.

```jsx
<Button variant="primary" size="lg" onClick={apply}>Request access</Button>
<Button variant="ghost" iconLeft={<Icon name="arrow-right" />}>Learn more</Button>
```

Variants: `primary` (ember, glows), `secondary` (raised graphite), `outline` (hairline), `ghost` (text-only until hover). Sizes: `sm | md | lg`. Supports `loading`, `disabled`, `iconLeft/iconRight`, `fullWidth`.
