import React from "react";
import { Icon } from "./Icon.jsx";

/**
 * MDeck Co IconButton: square control for a single glyph action.
 */
export function IconButton({
  icon,
  name,
  variant = "ghost",
  size = "md",
  disabled = false,
  "aria-label": ariaLabel,
  onClick,
  style = {},
  ...rest
}) {
  const dims = { sm: 32, md: 40, lg: 48 };
  const iconSizes = { sm: 16, md: 18, lg: 20 };
  const d = dims[size] || dims.md;

  const [hover, setHover] = React.useState(false);
  const [active, setActive] = React.useState(false);

  const variants = {
    ghost: { background: "transparent", color: "var(--text-secondary)", border: "1px solid transparent" },
    secondary: { background: "var(--surface-card)", color: "var(--text-primary)", border: "1px solid var(--border-default)" },
    primary: { background: "var(--accent)", color: "var(--text-on-accent)", border: "1px solid transparent", boxShadow: "var(--glow-ember-sm)" },
  };
  const hoverStyles = {
    ghost: { background: "var(--surface-hover)", color: "var(--text-primary)" },
    secondary: { background: "var(--surface-hover)", borderColor: "var(--border-strong)" },
    primary: { background: "var(--accent-hover)", boxShadow: "var(--glow-ember-md)" },
  };

  return (
    <button
      aria-label={ariaLabel}
      disabled={disabled}
      onClick={onClick}
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => { setHover(false); setActive(false); }}
      onMouseDown={() => setActive(true)}
      onMouseUp={() => setActive(false)}
      style={{
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        width: d,
        height: d,
        borderRadius: "var(--radius-md)",
        cursor: disabled ? "not-allowed" : "pointer",
        opacity: disabled ? 0.4 : 1,
        transition: "var(--transition-control)",
        outline: "none",
        ...variants[variant],
        ...(hover && !disabled ? hoverStyles[variant] : {}),
        ...(active && !disabled ? { transform: "scale(0.94)" } : {}),
        ...style,
      }}
      {...rest}
    >
      {icon || (name && <Icon name={name} size={iconSizes[size]} />)}
    </button>
  );
}
