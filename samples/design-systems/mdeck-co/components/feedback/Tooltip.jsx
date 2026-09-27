import React from "react";

/** MDeck Co Tooltip: hover label on a dark slab. Wraps a trigger. */
export function Tooltip({ label, placement = "top", children, style = {} }) {
  const [show, setShow] = React.useState(false);
  const pos = {
    top: { bottom: "calc(100% + 8px)", left: "50%", transform: "translateX(-50%)" },
    bottom: { top: "calc(100% + 8px)", left: "50%", transform: "translateX(-50%)" },
    left: { right: "calc(100% + 8px)", top: "50%", transform: "translateY(-50%)" },
    right: { left: "calc(100% + 8px)", top: "50%", transform: "translateY(-50%)" },
  };
  return (
    <span
      style={{ position: "relative", display: "inline-flex" }}
      onMouseEnter={() => setShow(true)}
      onMouseLeave={() => setShow(false)}
      onFocus={() => setShow(true)}
      onBlur={() => setShow(false)}
    >
      {children}
      {show && (
        <span
          role="tooltip"
          style={{
            position: "absolute",
            zIndex: 900,
            whiteSpace: "nowrap",
            padding: "6px 10px",
            background: "var(--ink-700)",
            border: "1px solid var(--border-default)",
            borderRadius: "var(--radius-sm)",
            boxShadow: "var(--shadow-md)",
            color: "var(--text-primary)",
            fontSize: "var(--text-xs)",
            fontFamily: "var(--font-sans)",
            pointerEvents: "none",
            animation: "mdc-tip var(--dur-fast) var(--ease-out)",
            ...pos[placement],
            ...style,
          }}
        >
          <style>{"@keyframes mdc-tip{from{opacity:0}to{opacity:1}}"}</style>
          {label}
        </span>
      )}
    </span>
  );
}
