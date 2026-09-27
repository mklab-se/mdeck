import React from "react";

/**
 * MDeck Co Button: the signature control.
 * Primary carries the ember glow; ghost/secondary stay quiet
 * until hover. Everything animates on the "expensive" easing.
 */
export function Button({
  children,
  variant = "primary",
  size = "md",
  disabled = false,
  loading = false,
  iconLeft = null,
  iconRight = null,
  fullWidth = false,
  type = "button",
  onClick,
  style = {},
  ...rest
}) {
  const sizes = {
    sm: { padding: "0 14px", height: 34, font: "var(--text-xs)", gap: 8 },
    md: { padding: "0 20px", height: 42, font: "var(--text-sm)", gap: 10 },
    lg: { padding: "0 28px", height: 52, font: "var(--text-base)", gap: 12 },
  };
  const s = sizes[size] || sizes.md;

  const base = {
    display: "inline-flex",
    alignItems: "center",
    justifyContent: "center",
    gap: s.gap,
    height: s.height,
    padding: s.padding,
    width: fullWidth ? "100%" : "auto",
    font: "var(--type-body-sm)",
    fontFamily: "var(--font-sans)",
    fontSize: s.font,
    fontWeight: "var(--weight-semibold)",
    letterSpacing: "var(--tracking-wide)",
    borderRadius: "var(--radius-md)",
    border: "1px solid transparent",
    cursor: disabled || loading ? "not-allowed" : "pointer",
    opacity: disabled ? 0.4 : 1,
    transition: "var(--transition-control)",
    whiteSpace: "nowrap",
    userSelect: "none",
    outline: "none",
    position: "relative",
  };

  const variants = {
    primary: {
      background: "var(--accent)",
      color: "var(--text-on-accent)",
      boxShadow: "var(--glow-ember-sm)",
    },
    secondary: {
      background: "var(--surface-card)",
      color: "var(--text-primary)",
      border: "1px solid var(--border-strong)",
      boxShadow: "var(--shadow-inset-top)",
    },
    ghost: {
      background: "transparent",
      color: "var(--text-secondary)",
    },
    outline: {
      background: "transparent",
      color: "var(--text-primary)",
      border: "1px solid var(--border-default)",
    },
  };

  const [hover, setHover] = React.useState(false);
  const [active, setActive] = React.useState(false);

  const hoverStyles = {
    primary: { background: "var(--accent-hover)", boxShadow: "var(--glow-ember-md)" },
    secondary: { background: "var(--surface-hover)", borderColor: "var(--border-strong)" },
    ghost: { background: "var(--surface-hover)", color: "var(--text-primary)" },
    outline: { borderColor: "var(--border-strong)", background: "var(--surface-hover)" },
  };

  const composed = {
    ...base,
    ...variants[variant],
    ...(hover && !disabled && !loading ? hoverStyles[variant] : {}),
    ...(active && !disabled && !loading ? { transform: "translateY(0.5px) scale(0.99)" } : {}),
    ...style,
  };

  return (
    <button
      type={type}
      disabled={disabled || loading}
      onClick={onClick}
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => { setHover(false); setActive(false); }}
      onMouseDown={() => setActive(true)}
      onMouseUp={() => setActive(false)}
      onFocus={(e) => (e.target.style.boxShadow = "var(--focus-ring)")}
      onBlur={(e) => (e.target.style.boxShadow = composed.boxShadow || "none")}
      style={composed}
      {...rest}
    >
      {loading && <Spinner />}
      {!loading && iconLeft}
      {children && <span>{children}</span>}
      {!loading && iconRight}
    </button>
  );
}

function Spinner() {
  return (
    <span
      style={{
        width: 14,
        height: 14,
        border: "1.5px solid currentColor",
        borderTopColor: "transparent",
        borderRadius: "50%",
        display: "inline-block",
        animation: "mdc-spin 0.7s linear infinite",
      }}
    >
      <style>{"@keyframes mdc-spin{to{transform:rotate(360deg)}}"}</style>
    </span>
  );
}
