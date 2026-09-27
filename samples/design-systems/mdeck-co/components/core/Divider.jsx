import React from "react";

/**
 * MDeck Co Divider: hairline separator. Optional centered label
 * for the tracked, editorial "· section ·" treatment.
 */
export function Divider({ label, orientation = "horizontal", style = {}, ...rest }) {
  if (orientation === "vertical") {
    return (
      <span
        style={{ display: "inline-block", width: 1, alignSelf: "stretch", background: "var(--border-default)", ...style }}
        {...rest}
      />
    );
  }

  if (label) {
    return (
      <div style={{ display: "flex", alignItems: "center", gap: "var(--space-4)", ...style }} {...rest}>
        <span style={{ flex: 1, height: 1, background: "var(--border-default)" }} />
        <span
          style={{
            font: "var(--type-label)",
            fontFamily: "var(--font-mono)",
            fontSize: "var(--text-2xs)",
            letterSpacing: "var(--tracking-widest)",
            textTransform: "uppercase",
            color: "var(--text-tertiary)",
          }}
        >
          {label}
        </span>
        <span style={{ flex: 1, height: 1, background: "var(--border-default)" }} />
      </div>
    );
  }

  return <div style={{ height: 1, width: "100%", background: "var(--border-default)", ...style }} {...rest} />;
}
