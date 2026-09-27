import React from "react";

/**
 * MDeck Co Badge: small status pill. Mono, tracked, quiet.
 */
export function Badge({ children, tone = "neutral", variant = "soft", dot = false, style = {}, ...rest }) {
  const tones = {
    neutral: { fg: "var(--ink-200)", bg: "rgba(255,255,255,0.06)", bd: "var(--border-default)" },
    accent: { fg: "var(--ember-300)", bg: "var(--accent-quiet)", bd: "rgba(255,77,28,0.3)" },
    positive: { fg: "var(--status-positive)", bg: "rgba(79,180,119,0.12)", bd: "rgba(79,180,119,0.3)" },
    warning: { fg: "var(--status-warning)", bg: "rgba(245,166,35,0.12)", bd: "rgba(245,166,35,0.3)" },
    critical: { fg: "var(--status-critical)", bg: "rgba(232,52,28,0.12)", bd: "rgba(232,52,28,0.3)" },
  };
  const t = tones[tone] || tones.neutral;
  const solid = variant === "solid";

  return (
    <span
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: 6,
        height: 22,
        padding: "0 10px",
        borderRadius: "var(--radius-pill)",
        font: "var(--type-label)",
        fontFamily: "var(--font-mono)",
        fontSize: "var(--text-2xs)",
        fontWeight: "var(--weight-medium)",
        letterSpacing: "var(--tracking-wide)",
        textTransform: "uppercase",
        color: solid ? "var(--text-on-accent)" : t.fg,
        background: solid ? t.fg : t.bg,
        border: `1px solid ${solid ? "transparent" : t.bd}`,
        whiteSpace: "nowrap",
        ...style,
      }}
      {...rest}
    >
      {dot && (
        <span style={{ width: 6, height: 6, borderRadius: "50%", background: solid ? "var(--text-on-accent)" : t.fg }} />
      )}
      {children}
    </span>
  );
}
