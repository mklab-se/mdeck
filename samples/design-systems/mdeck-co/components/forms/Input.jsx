import React from "react";

/**
 * MDeck Co Input: inset well that lights an ember hairline on focus.
 */
export function Input({
  label,
  hint,
  error,
  iconLeft = null,
  iconRight = null,
  size = "md",
  disabled = false,
  id,
  style = {},
  ...rest
}) {
  const [focus, setFocus] = React.useState(false);
  const inputId = id || React.useId();
  const heights = { sm: 36, md: 44, lg: 52 };
  const h = heights[size] || heights.md;

  const borderColor = error
    ? "var(--status-critical)"
    : focus
    ? "var(--accent)"
    : "var(--border-default)";

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 8, width: "100%", ...style }}>
      {label && (
        <label htmlFor={inputId} style={labelStyle}>{label}</label>
      )}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 10,
          height: h,
          padding: "0 14px",
          borderRadius: "var(--radius-md)",
          background: "var(--surface-inset)",
          border: `1px solid ${borderColor}`,
          boxShadow: focus && !error ? "var(--glow-ember-sm)" : "var(--shadow-inset-well)",
          transition: "var(--transition-control)",
          opacity: disabled ? 0.5 : 1,
        }}
      >
        {iconLeft && <span style={{ color: "var(--text-tertiary)", display: "flex" }}>{iconLeft}</span>}
        <input
          id={inputId}
          disabled={disabled}
          onFocus={() => setFocus(true)}
          onBlur={() => setFocus(false)}
          style={{
            flex: 1,
            minWidth: 0,
            background: "transparent",
            border: "none",
            outline: "none",
            color: "var(--text-primary)",
            fontFamily: "var(--font-sans)",
            fontSize: "var(--text-base)",
          }}
          {...rest}
        />
        {iconRight && <span style={{ color: "var(--text-tertiary)", display: "flex" }}>{iconRight}</span>}
      </div>
      {(hint || error) && (
        <span style={{ fontSize: "var(--text-xs)", color: error ? "var(--status-critical)" : "var(--text-tertiary)", fontFamily: "var(--font-sans)" }}>
          {error || hint}
        </span>
      )}
    </div>
  );
}

const labelStyle = {
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-2xs)",
  fontWeight: "var(--weight-medium)",
  letterSpacing: "var(--tracking-wider)",
  textTransform: "uppercase",
  color: "var(--text-secondary)",
};
