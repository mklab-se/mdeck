import React from "react";
import { Icon } from "./Icon.jsx";

/**
 * MDeck Co Tag: removable/selectable chip. Sentence-case, softer
 * than Badge. Used for filters, skills, categories.
 */
export function Tag({ children, selected = false, onRemove, onClick, style = {}, ...rest }) {
  const [hover, setHover] = React.useState(false);
  const interactive = !!onClick;

  return (
    <span
      onClick={onClick}
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: 8,
        height: 28,
        padding: onRemove ? "0 8px 0 12px" : "0 12px",
        borderRadius: "var(--radius-sm)",
        font: "var(--type-body-sm)",
        fontFamily: "var(--font-sans)",
        fontSize: "var(--text-sm)",
        cursor: interactive ? "pointer" : "default",
        transition: "var(--transition-control)",
        color: selected ? "var(--ember-300)" : "var(--text-secondary)",
        background: selected ? "var(--accent-quiet)" : "rgba(255,255,255,0.04)",
        border: `1px solid ${selected ? "rgba(255,77,28,0.35)" : "var(--border-default)"}`,
        ...(interactive && hover && !selected ? { borderColor: "var(--border-strong)", color: "var(--text-primary)" } : {}),
        ...style,
      }}
      {...rest}
    >
      {children}
      {onRemove && (
        <span
          onClick={(e) => { e.stopPropagation(); onRemove(e); }}
          style={{ display: "inline-flex", cursor: "pointer", opacity: 0.6, marginLeft: 2 }}
          onMouseEnter={(e) => (e.currentTarget.style.opacity = 1)}
          onMouseLeave={(e) => (e.currentTarget.style.opacity = 0.6)}
        >
          <Icon name="x" size={13} />
        </span>
      )}
    </span>
  );
}
