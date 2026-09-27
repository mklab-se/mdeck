import React from "react";
import { Icon } from "../core/Icon.jsx";

/** MDeck Co Checkbox: square, ember-filled when checked. */
export function Checkbox({ label, description, checked = false, disabled = false, onChange, id, style = {} }) {
  const inputId = id || React.useId();
  return (
    <label htmlFor={inputId} style={{ display: "flex", gap: 12, alignItems: description ? "flex-start" : "center", cursor: disabled ? "not-allowed" : "pointer", opacity: disabled ? 0.5 : 1, ...style }}>
      <span
        style={{
          position: "relative",
          flexShrink: 0,
          width: 20,
          height: 20,
          borderRadius: "var(--radius-xs)",
          background: checked ? "var(--accent)" : "var(--surface-inset)",
          border: `1px solid ${checked ? "var(--accent)" : "var(--border-strong)"}`,
          boxShadow: checked ? "var(--glow-ember-sm)" : "var(--shadow-inset-well)",
          transition: "var(--transition-control)",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          marginTop: description ? 2 : 0,
        }}
      >
        {checked && <Icon name="check" size={14} color="var(--text-on-accent)" strokeWidth={2.5} />}
        <input id={inputId} type="checkbox" checked={checked} disabled={disabled} onChange={onChange} style={{ position: "absolute", opacity: 0, inset: 0, margin: 0, cursor: "inherit" }} />
      </span>
      {(label || description) && (
        <span style={{ display: "flex", flexDirection: "column", gap: 2 }}>
          {label && <span style={{ fontSize: "var(--text-base)", color: "var(--text-primary)", fontFamily: "var(--font-sans)" }}>{label}</span>}
          {description && <span style={{ fontSize: "var(--text-sm)", color: "var(--text-tertiary)" }}>{description}</span>}
        </span>
      )}
    </label>
  );
}
