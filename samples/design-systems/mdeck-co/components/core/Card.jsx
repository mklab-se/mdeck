import React from "react";

/**
 * MDeck Co Card: graphite panel. Quiet by default; can lift and
 * gain an ember hairline on hover when `interactive`.
 */
export function Card({
  children,
  variant = "default",
  interactive = false,
  padding = "lg",
  onClick,
  style = {},
  ...rest
}) {
  const [hover, setHover] = React.useState(false);
  const pads = { none: 0, sm: "var(--space-4)", md: "var(--space-6)", lg: "var(--space-8)" };

  const variants = {
    default: {
      background: "var(--surface-card)",
      border: "1px solid var(--border-subtle)",
      boxShadow: "var(--shadow-md)",
    },
    inset: {
      background: "var(--surface-inset)",
      border: "1px solid var(--border-subtle)",
      boxShadow: "var(--shadow-inset-well)",
    },
    outline: {
      background: "transparent",
      border: "1px solid var(--border-default)",
    },
    glow: {
      background: "var(--surface-card)",
      border: "1px solid transparent",
      boxShadow: "var(--glow-ember-md)",
    },
  };

  return (
    <div
      onClick={onClick}
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
      style={{
        borderRadius: "var(--radius-lg)",
        padding: pads[padding],
        transition: "var(--transition-control)",
        cursor: interactive ? "pointer" : "default",
        ...variants[variant],
        ...(interactive && hover
          ? {
              borderColor: "var(--border-strong)",
              boxShadow: "var(--shadow-lg)",
              transform: "translateY(-2px)",
            }
          : {}),
        ...style,
      }}
      {...rest}
    >
      {children}
    </div>
  );
}
