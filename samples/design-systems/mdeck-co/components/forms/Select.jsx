import React from "react";
import { Icon } from "../core/Icon.jsx";

/** MDeck Co Select: native select styled as an inset well. */
export function Select({ label, hint, error, options = [], placeholder, size = "md", disabled = false, id, value, onChange, style = {}, ...rest }) {
  const [focus, setFocus] = React.useState(false);
  const inputId = id || React.useId();
  const heights = { sm: 36, md: 44, lg: 52 };
  const h = heights[size] || heights.md;
  const borderColor = error ? "var(--status-critical)" : focus ? "var(--accent)" : "var(--border-default)";

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 8, width: "100%", ...style }}>
      {label && (
        <label htmlFor={inputId} style={{ fontFamily: "var(--font-mono)", fontSize: "var(--text-2xs)", fontWeight: 500, letterSpacing: "var(--tracking-wider)", textTransform: "uppercase", color: "var(--text-secondary)" }}>{label}</label>
      )}
      <div
        style={{
          position: "relative",
          height: h,
          borderRadius: "var(--radius-md)",
          background: "var(--surface-inset)",
          border: `1px solid ${borderColor}`,
          boxShadow: focus && !error ? "var(--glow-ember-sm)" : "var(--shadow-inset-well)",
          transition: "var(--transition-control)",
          opacity: disabled ? 0.5 : 1,
        }}
      >
        <select
          id={inputId}
          disabled={disabled}
          value={value}
          onChange={onChange}
          onFocus={() => setFocus(true)}
          onBlur={() => setFocus(false)}
          style={{
            appearance: "none",
            width: "100%",
            height: "100%",
            padding: "0 40px 0 14px",
            background: "transparent",
            border: "none",
            outline: "none",
            color: value ? "var(--text-primary)" : "var(--text-tertiary)",
            fontFamily: "var(--font-sans)",
            fontSize: "var(--text-base)",
            cursor: disabled ? "not-allowed" : "pointer",
          }}
          {...rest}
        >
          {placeholder && <option value="" disabled>{placeholder}</option>}
          {options.map((o) => {
            const val = typeof o === "string" ? o : o.value;
            const lbl = typeof o === "string" ? o : o.label;
            return <option key={val} value={val} style={{ background: "var(--ink-800)", color: "var(--text-primary)" }}>{lbl}</option>;
          })}
        </select>
        <span style={{ position: "absolute", right: 12, top: "50%", transform: "translateY(-50%)", pointerEvents: "none", color: "var(--text-tertiary)", display: "flex" }}>
          <Icon name="chevron-down" size={16} />
        </span>
      </div>
      {(hint || error) && (
        <span style={{ fontSize: "var(--text-xs)", color: error ? "var(--status-critical)" : "var(--text-tertiary)" }}>{error || hint}</span>
      )}
    </div>
  );
}
