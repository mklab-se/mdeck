import React from "react";
import { Icon } from "../core/Icon.jsx";

/** MDeck Co Toast: quiet notification slab. */
export function Toast({ title, message, tone = "neutral", onClose, icon, style = {} }) {
  const tones = {
    neutral: { accent: "var(--ink-300)", glyph: "info" },
    accent: { accent: "var(--ember-500)", glyph: "sparkles" },
    positive: { accent: "var(--status-positive)", glyph: "check-circle" },
    warning: { accent: "var(--status-warning)", glyph: "alert-triangle" },
    critical: { accent: "var(--status-critical)", glyph: "alert-octagon" },
  };
  const t = tones[tone] || tones.neutral;
  return (
    <div
      role="status"
      style={{
        display: "flex",
        gap: 12,
        alignItems: "flex-start",
        width: 360,
        maxWidth: "100%",
        padding: "14px 16px",
        background: "var(--surface-overlay)",
        border: "1px solid var(--border-default)",
        borderLeft: `2px solid ${t.accent}`,
        borderRadius: "var(--radius-md)",
        boxShadow: "var(--shadow-lg)",
        ...style,
      }}
    >
      <span style={{ color: t.accent, display: "flex", marginTop: 1 }}><Icon name={icon || t.glyph} size={18} /></span>
      <div style={{ flex: 1, minWidth: 0 }}>
        {title && <div style={{ fontSize: "var(--text-sm)", fontWeight: "var(--weight-semibold)", color: "var(--text-primary)", fontFamily: "var(--font-sans)" }}>{title}</div>}
        {message && <div style={{ fontSize: "var(--text-sm)", color: "var(--text-secondary)", marginTop: title ? 2 : 0, lineHeight: "var(--leading-normal)" }}>{message}</div>}
      </div>
      {onClose && (
        <span onClick={onClose} style={{ cursor: "pointer", color: "var(--text-tertiary)", display: "flex" }}><Icon name="x" size={15} /></span>
      )}
    </div>
  );
}
