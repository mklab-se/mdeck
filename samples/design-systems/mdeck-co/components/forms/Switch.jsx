import React from "react";

/** MDeck Co Switch: sliding toggle; ember track + glow when on. */
export function Switch({ label, description, checked = false, disabled = false, onChange, id, style = {} }) {
  const inputId = id || React.useId();
  return (
    <label htmlFor={inputId} style={{ display: "flex", gap: 12, alignItems: description ? "flex-start" : "center", cursor: disabled ? "not-allowed" : "pointer", opacity: disabled ? 0.5 : 1, ...style }}>
      <span
        style={{
          position: "relative",
          flexShrink: 0,
          width: 42,
          height: 24,
          borderRadius: "var(--radius-pill)",
          background: checked ? "var(--accent)" : "var(--surface-inset)",
          border: `1px solid ${checked ? "var(--accent)" : "var(--border-strong)"}`,
          boxShadow: checked ? "var(--glow-ember-sm)" : "var(--shadow-inset-well)",
          transition: "var(--transition-control)",
          marginTop: description ? 1 : 0,
        }}
      >
        <span
          style={{
            position: "absolute",
            top: "50%",
            left: checked ? 21 : 3,
            transform: "translateY(-50%)",
            width: 16,
            height: 16,
            borderRadius: "50%",
            background: checked ? "var(--text-on-accent)" : "var(--ink-200)",
            boxShadow: "0 1px 2px rgba(0,0,0,0.5)",
            transition: "left var(--dur-base) var(--ease-out), background var(--dur-fast)",
          }}
        />
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
