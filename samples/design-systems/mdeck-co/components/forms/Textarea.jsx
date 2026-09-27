import React from "react";

/** MDeck Co Textarea: multi-line variant of Input. */
export function Textarea({ label, hint, error, rows = 4, disabled = false, id, style = {}, ...rest }) {
  const [focus, setFocus] = React.useState(false);
  const inputId = id || React.useId();
  const borderColor = error ? "var(--status-critical)" : focus ? "var(--accent)" : "var(--border-default)";

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 8, width: "100%", ...style }}>
      {label && (
        <label htmlFor={inputId} style={{ fontFamily: "var(--font-mono)", fontSize: "var(--text-2xs)", fontWeight: 500, letterSpacing: "var(--tracking-wider)", textTransform: "uppercase", color: "var(--text-secondary)" }}>{label}</label>
      )}
      <textarea
        id={inputId}
        rows={rows}
        disabled={disabled}
        onFocus={() => setFocus(true)}
        onBlur={() => setFocus(false)}
        style={{
          resize: "vertical",
          padding: "12px 14px",
          borderRadius: "var(--radius-md)",
          background: "var(--surface-inset)",
          border: `1px solid ${borderColor}`,
          boxShadow: focus && !error ? "var(--glow-ember-sm)" : "var(--shadow-inset-well)",
          transition: "var(--transition-control)",
          outline: "none",
          color: "var(--text-primary)",
          fontFamily: "var(--font-sans)",
          fontSize: "var(--text-base)",
          lineHeight: "var(--leading-normal)",
          opacity: disabled ? 0.5 : 1,
        }}
        {...rest}
      />
      {(hint || error) && (
        <span style={{ fontSize: "var(--text-xs)", color: error ? "var(--status-critical)" : "var(--text-tertiary)" }}>{error || hint}</span>
      )}
    </div>
  );
}
