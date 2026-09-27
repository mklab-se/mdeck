/* @ds-bundle: {"format":4,"namespace":"MDeckCoDesignSystem_4d3c0a","components":[{"name":"Badge","sourcePath":"components/core/Badge.jsx"},{"name":"Button","sourcePath":"components/core/Button.jsx"},{"name":"Card","sourcePath":"components/core/Card.jsx"},{"name":"Divider","sourcePath":"components/core/Divider.jsx"},{"name":"Icon","sourcePath":"components/core/Icon.jsx"},{"name":"IconButton","sourcePath":"components/core/IconButton.jsx"},{"name":"Tag","sourcePath":"components/core/Tag.jsx"},{"name":"Dialog","sourcePath":"components/feedback/Dialog.jsx"},{"name":"Toast","sourcePath":"components/feedback/Toast.jsx"},{"name":"Tooltip","sourcePath":"components/feedback/Tooltip.jsx"},{"name":"Checkbox","sourcePath":"components/forms/Checkbox.jsx"},{"name":"Input","sourcePath":"components/forms/Input.jsx"},{"name":"Radio","sourcePath":"components/forms/Radio.jsx"},{"name":"Select","sourcePath":"components/forms/Select.jsx"},{"name":"Switch","sourcePath":"components/forms/Switch.jsx"},{"name":"Textarea","sourcePath":"components/forms/Textarea.jsx"},{"name":"Tabs","sourcePath":"components/navigation/Tabs.jsx"}],"sourceHashes":{"components/core/Badge.jsx":"a8bd2093a730","components/core/Button.jsx":"8bbd4783e1d1","components/core/Card.jsx":"b9b5f890c311","components/core/Divider.jsx":"0a80a9d5c0db","components/core/Icon.jsx":"35bc2ffdd48b","components/core/IconButton.jsx":"02ee25b6b230","components/core/Tag.jsx":"1c199501e683","components/feedback/Dialog.jsx":"870ca3631926","components/feedback/Toast.jsx":"6aede64eed47","components/feedback/Tooltip.jsx":"4582052db0a8","components/forms/Checkbox.jsx":"b8ab4043aa6f","components/forms/Input.jsx":"663b58427f9b","components/forms/Radio.jsx":"976774a275ce","components/forms/Select.jsx":"b07e25910347","components/forms/Switch.jsx":"ca1122bb7bb4","components/forms/Textarea.jsx":"e1d56cc4fb6c","components/navigation/Tabs.jsx":"9de0f225d418","ui_kits/website/Apply.jsx":"5814c487b2f9","ui_kits/website/Chrome.jsx":"22d1c5551dd4","ui_kits/website/Home.jsx":"ecdc2739a454"},"inlinedExternals":[],"unexposedExports":[]} */

(() => {

const __ds_ns = (window.MDeckCoDesignSystem_4d3c0a = window.MDeckCoDesignSystem_4d3c0a || {});

const __ds_scope = {};

(__ds_ns.__errors = __ds_ns.__errors || []);

// components/core/Badge.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/**
 * MDeck Co Badge: small status pill. Mono, tracked, quiet.
 */
function Badge({
  children,
  tone = "neutral",
  variant = "soft",
  dot = false,
  style = {},
  ...rest
}) {
  const tones = {
    neutral: {
      fg: "var(--ink-200)",
      bg: "rgba(255,255,255,0.06)",
      bd: "var(--border-default)"
    },
    accent: {
      fg: "var(--ember-300)",
      bg: "var(--accent-quiet)",
      bd: "rgba(255,77,28,0.3)"
    },
    positive: {
      fg: "var(--status-positive)",
      bg: "rgba(79,180,119,0.12)",
      bd: "rgba(79,180,119,0.3)"
    },
    warning: {
      fg: "var(--status-warning)",
      bg: "rgba(245,166,35,0.12)",
      bd: "rgba(245,166,35,0.3)"
    },
    critical: {
      fg: "var(--status-critical)",
      bg: "rgba(232,52,28,0.12)",
      bd: "rgba(232,52,28,0.3)"
    }
  };
  const t = tones[tone] || tones.neutral;
  const solid = variant === "solid";
  return /*#__PURE__*/React.createElement("span", _extends({
    style: {
      display: "inline-flex",
      alignItems: "center",
      gap: 6,
      height: 22,
      padding: "0 10px",
      borderRadius: "var(--radius-pill)",
      font: "var(--type-label)",
      fontFamily: "var(--font-mono)",
      fontSize: "var(--text-2xs)",
      fontWeight: "var(--weight-medium)",
      letterSpacing: "var(--tracking-wide)",
      textTransform: "uppercase",
      color: solid ? "var(--text-on-accent)" : t.fg,
      background: solid ? t.fg : t.bg,
      border: `1px solid ${solid ? "transparent" : t.bd}`,
      whiteSpace: "nowrap",
      ...style
    }
  }, rest), dot && /*#__PURE__*/React.createElement("span", {
    style: {
      width: 6,
      height: 6,
      borderRadius: "50%",
      background: solid ? "var(--text-on-accent)" : t.fg
    }
  }), children);
}
Object.assign(__ds_scope, { Badge });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Badge.jsx", error: String((e && e.message) || e) }); }

// components/core/Button.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/**
 * MDeck Co Button: the signature control.
 * Primary carries the ember glow; ghost/secondary stay quiet
 * until hover. Everything animates on the "expensive" easing.
 */
function Button({
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
    sm: {
      padding: "0 14px",
      height: 34,
      font: "var(--text-xs)",
      gap: 8
    },
    md: {
      padding: "0 20px",
      height: 42,
      font: "var(--text-sm)",
      gap: 10
    },
    lg: {
      padding: "0 28px",
      height: 52,
      font: "var(--text-base)",
      gap: 12
    }
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
    position: "relative"
  };
  const variants = {
    primary: {
      background: "var(--accent)",
      color: "var(--text-on-accent)",
      boxShadow: "var(--glow-ember-sm)"
    },
    secondary: {
      background: "var(--surface-card)",
      color: "var(--text-primary)",
      border: "1px solid var(--border-strong)",
      boxShadow: "var(--shadow-inset-top)"
    },
    ghost: {
      background: "transparent",
      color: "var(--text-secondary)"
    },
    outline: {
      background: "transparent",
      color: "var(--text-primary)",
      border: "1px solid var(--border-default)"
    }
  };
  const [hover, setHover] = React.useState(false);
  const [active, setActive] = React.useState(false);
  const hoverStyles = {
    primary: {
      background: "var(--accent-hover)",
      boxShadow: "var(--glow-ember-md)"
    },
    secondary: {
      background: "var(--surface-hover)",
      borderColor: "var(--border-strong)"
    },
    ghost: {
      background: "var(--surface-hover)",
      color: "var(--text-primary)"
    },
    outline: {
      borderColor: "var(--border-strong)",
      background: "var(--surface-hover)"
    }
  };
  const composed = {
    ...base,
    ...variants[variant],
    ...(hover && !disabled && !loading ? hoverStyles[variant] : {}),
    ...(active && !disabled && !loading ? {
      transform: "translateY(0.5px) scale(0.99)"
    } : {}),
    ...style
  };
  return /*#__PURE__*/React.createElement("button", _extends({
    type: type,
    disabled: disabled || loading,
    onClick: onClick,
    onMouseEnter: () => setHover(true),
    onMouseLeave: () => {
      setHover(false);
      setActive(false);
    },
    onMouseDown: () => setActive(true),
    onMouseUp: () => setActive(false),
    onFocus: e => e.target.style.boxShadow = "var(--focus-ring)",
    onBlur: e => e.target.style.boxShadow = composed.boxShadow || "none",
    style: composed
  }, rest), loading && /*#__PURE__*/React.createElement(Spinner, null), !loading && iconLeft, children && /*#__PURE__*/React.createElement("span", null, children), !loading && iconRight);
}
function Spinner() {
  return /*#__PURE__*/React.createElement("span", {
    style: {
      width: 14,
      height: 14,
      border: "1.5px solid currentColor",
      borderTopColor: "transparent",
      borderRadius: "50%",
      display: "inline-block",
      animation: "mdc-spin 0.7s linear infinite"
    }
  }, /*#__PURE__*/React.createElement("style", null, "@keyframes mdc-spin{to{transform:rotate(360deg)}}"));
}
Object.assign(__ds_scope, { Button });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Button.jsx", error: String((e && e.message) || e) }); }

// components/core/Card.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/**
 * MDeck Co Card: graphite panel. Quiet by default; can lift and
 * gain an ember hairline on hover when `interactive`.
 */
function Card({
  children,
  variant = "default",
  interactive = false,
  padding = "lg",
  onClick,
  style = {},
  ...rest
}) {
  const [hover, setHover] = React.useState(false);
  const pads = {
    none: 0,
    sm: "var(--space-4)",
    md: "var(--space-6)",
    lg: "var(--space-8)"
  };
  const variants = {
    default: {
      background: "var(--surface-card)",
      border: "1px solid var(--border-subtle)",
      boxShadow: "var(--shadow-md)"
    },
    inset: {
      background: "var(--surface-inset)",
      border: "1px solid var(--border-subtle)",
      boxShadow: "var(--shadow-inset-well)"
    },
    outline: {
      background: "transparent",
      border: "1px solid var(--border-default)"
    },
    glow: {
      background: "var(--surface-card)",
      border: "1px solid transparent",
      boxShadow: "var(--glow-ember-md)"
    }
  };
  return /*#__PURE__*/React.createElement("div", _extends({
    onClick: onClick,
    onMouseEnter: () => setHover(true),
    onMouseLeave: () => setHover(false),
    style: {
      borderRadius: "var(--radius-lg)",
      padding: pads[padding],
      transition: "var(--transition-control)",
      cursor: interactive ? "pointer" : "default",
      ...variants[variant],
      ...(interactive && hover ? {
        borderColor: "var(--border-strong)",
        boxShadow: "var(--shadow-lg)",
        transform: "translateY(-2px)"
      } : {}),
      ...style
    }
  }, rest), children);
}
Object.assign(__ds_scope, { Card });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Card.jsx", error: String((e && e.message) || e) }); }

// components/core/Divider.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/**
 * MDeck Co Divider: hairline separator. Optional centered label
 * for the tracked, editorial "· section ·" treatment.
 */
function Divider({
  label,
  orientation = "horizontal",
  style = {},
  ...rest
}) {
  if (orientation === "vertical") {
    return /*#__PURE__*/React.createElement("span", _extends({
      style: {
        display: "inline-block",
        width: 1,
        alignSelf: "stretch",
        background: "var(--border-default)",
        ...style
      }
    }, rest));
  }
  if (label) {
    return /*#__PURE__*/React.createElement("div", _extends({
      style: {
        display: "flex",
        alignItems: "center",
        gap: "var(--space-4)",
        ...style
      }
    }, rest), /*#__PURE__*/React.createElement("span", {
      style: {
        flex: 1,
        height: 1,
        background: "var(--border-default)"
      }
    }), /*#__PURE__*/React.createElement("span", {
      style: {
        font: "var(--type-label)",
        fontFamily: "var(--font-mono)",
        fontSize: "var(--text-2xs)",
        letterSpacing: "var(--tracking-widest)",
        textTransform: "uppercase",
        color: "var(--text-tertiary)"
      }
    }, label), /*#__PURE__*/React.createElement("span", {
      style: {
        flex: 1,
        height: 1,
        background: "var(--border-default)"
      }
    }));
  }
  return /*#__PURE__*/React.createElement("div", _extends({
    style: {
      height: 1,
      width: "100%",
      background: "var(--border-default)",
      ...style
    }
  }, rest));
}
Object.assign(__ds_scope, { Divider });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Divider.jsx", error: String((e && e.message) || e) }); }

// components/core/Icon.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/**
 * MDeck Co Icon: thin wrapper over Lucide (loaded from CDN).
 * Lucide's 1.5px stroke matches the brand's precise, quiet line.
 * Requires the Lucide UMD script to be present on the page:
 *   <script src="https://unpkg.com/lucide@latest"></script>
 */
function Icon({
  name,
  size = 18,
  strokeWidth = 1.5,
  color = "currentColor",
  style = {},
  ...rest
}) {
  const ref = React.useRef(null);
  React.useEffect(() => {
    if (window.lucide && ref.current) {
      ref.current.innerHTML = "";
      const el = document.createElement("i");
      el.setAttribute("data-lucide", name);
      ref.current.appendChild(el);
      try {
        window.lucide.createIcons({
          attrs: {
            width: size,
            height: size,
            "stroke-width": strokeWidth
          },
          nameAttr: "data-lucide"
        });
      } catch (e) {/* noop */}
    }
  }, [name, size, strokeWidth]);
  return /*#__PURE__*/React.createElement("span", _extends({
    ref: ref,
    "aria-hidden": "true",
    style: {
      display: "inline-flex",
      alignItems: "center",
      justifyContent: "center",
      width: size,
      height: size,
      color,
      flexShrink: 0,
      ...style
    }
  }, rest));
}
Object.assign(__ds_scope, { Icon });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Icon.jsx", error: String((e && e.message) || e) }); }

// components/core/IconButton.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/**
 * MDeck Co IconButton: square control for a single glyph action.
 */
function IconButton({
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
  const dims = {
    sm: 32,
    md: 40,
    lg: 48
  };
  const iconSizes = {
    sm: 16,
    md: 18,
    lg: 20
  };
  const d = dims[size] || dims.md;
  const [hover, setHover] = React.useState(false);
  const [active, setActive] = React.useState(false);
  const variants = {
    ghost: {
      background: "transparent",
      color: "var(--text-secondary)",
      border: "1px solid transparent"
    },
    secondary: {
      background: "var(--surface-card)",
      color: "var(--text-primary)",
      border: "1px solid var(--border-default)"
    },
    primary: {
      background: "var(--accent)",
      color: "var(--text-on-accent)",
      border: "1px solid transparent",
      boxShadow: "var(--glow-ember-sm)"
    }
  };
  const hoverStyles = {
    ghost: {
      background: "var(--surface-hover)",
      color: "var(--text-primary)"
    },
    secondary: {
      background: "var(--surface-hover)",
      borderColor: "var(--border-strong)"
    },
    primary: {
      background: "var(--accent-hover)",
      boxShadow: "var(--glow-ember-md)"
    }
  };
  return /*#__PURE__*/React.createElement("button", _extends({
    "aria-label": ariaLabel,
    disabled: disabled,
    onClick: onClick,
    onMouseEnter: () => setHover(true),
    onMouseLeave: () => {
      setHover(false);
      setActive(false);
    },
    onMouseDown: () => setActive(true),
    onMouseUp: () => setActive(false),
    style: {
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
      ...(active && !disabled ? {
        transform: "scale(0.94)"
      } : {}),
      ...style
    }
  }, rest), icon || name && /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: name,
    size: iconSizes[size]
  }));
}
Object.assign(__ds_scope, { IconButton });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/IconButton.jsx", error: String((e && e.message) || e) }); }

// components/core/Tag.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/**
 * MDeck Co Tag: removable/selectable chip. Sentence-case, softer
 * than Badge. Used for filters, skills, categories.
 */
function Tag({
  children,
  selected = false,
  onRemove,
  onClick,
  style = {},
  ...rest
}) {
  const [hover, setHover] = React.useState(false);
  const interactive = !!onClick;
  return /*#__PURE__*/React.createElement("span", _extends({
    onClick: onClick,
    onMouseEnter: () => setHover(true),
    onMouseLeave: () => setHover(false),
    style: {
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
      ...(interactive && hover && !selected ? {
        borderColor: "var(--border-strong)",
        color: "var(--text-primary)"
      } : {}),
      ...style
    }
  }, rest), children, onRemove && /*#__PURE__*/React.createElement("span", {
    onClick: e => {
      e.stopPropagation();
      onRemove(e);
    },
    style: {
      display: "inline-flex",
      cursor: "pointer",
      opacity: 0.6,
      marginLeft: 2
    },
    onMouseEnter: e => e.currentTarget.style.opacity = 1,
    onMouseLeave: e => e.currentTarget.style.opacity = 0.6
  }, /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: "x",
    size: 13
  })));
}
Object.assign(__ds_scope, { Tag });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Tag.jsx", error: String((e && e.message) || e) }); }

// components/feedback/Dialog.jsx
try { (() => {
/**
 * MDeck Co Dialog: overlay panel. Backdrop blurs the room behind it.
 * Controlled via `open` / `onClose`.
 */
function Dialog({
  open,
  onClose,
  title,
  description,
  children,
  footer,
  width = 480,
  style = {}
}) {
  React.useEffect(() => {
    if (!open) return;
    const onKey = e => e.key === "Escape" && onClose && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);
  if (!open) return null;
  return /*#__PURE__*/React.createElement("div", {
    onClick: onClose,
    style: {
      position: "fixed",
      inset: 0,
      zIndex: 1000,
      display: "flex",
      alignItems: "center",
      justifyContent: "center",
      padding: "var(--space-6)",
      background: "rgba(5,5,5,0.68)",
      backdropFilter: "var(--blur-overlay)",
      WebkitBackdropFilter: "var(--blur-overlay)",
      animation: "mdc-fade var(--dur-base) var(--ease-out)"
    }
  }, /*#__PURE__*/React.createElement("style", null, "@keyframes mdc-fade{from{opacity:0}to{opacity:1}}@keyframes mdc-rise{from{opacity:0;transform:translateY(8px) scale(0.99)}to{opacity:1;transform:none}}"), /*#__PURE__*/React.createElement("div", {
    role: "dialog",
    "aria-modal": "true",
    onClick: e => e.stopPropagation(),
    style: {
      width,
      maxWidth: "100%",
      background: "var(--surface-overlay)",
      border: "1px solid var(--border-default)",
      borderRadius: "var(--radius-xl)",
      boxShadow: "var(--shadow-xl)",
      padding: "var(--space-8)",
      animation: "mdc-rise var(--dur-slow) var(--ease-out)",
      ...style
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      justifyContent: "space-between",
      alignItems: "flex-start",
      gap: 16,
      marginBottom: description ? 6 : 20
    }
  }, title && /*#__PURE__*/React.createElement("h2", {
    style: {
      margin: 0,
      font: "var(--type-title)",
      fontFamily: "var(--font-display)",
      fontWeight: "var(--weight-regular)",
      fontSize: "var(--text-2xl)",
      color: "var(--text-primary)",
      letterSpacing: "var(--tracking-tight)"
    }
  }, title), onClose && /*#__PURE__*/React.createElement(__ds_scope.IconButton, {
    name: "x",
    "aria-label": "Close",
    onClick: onClose,
    size: "sm"
  })), description && /*#__PURE__*/React.createElement("p", {
    style: {
      margin: "0 0 20px",
      color: "var(--text-secondary)",
      fontSize: "var(--text-base)",
      lineHeight: "var(--leading-relaxed)"
    }
  }, description), children, footer && /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      justifyContent: "flex-end",
      gap: 10,
      marginTop: "var(--space-8)"
    }
  }, footer)));
}
Object.assign(__ds_scope, { Dialog });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/feedback/Dialog.jsx", error: String((e && e.message) || e) }); }

// components/feedback/Toast.jsx
try { (() => {
/** MDeck Co Toast: quiet notification slab. */
function Toast({
  title,
  message,
  tone = "neutral",
  onClose,
  icon,
  style = {}
}) {
  const tones = {
    neutral: {
      accent: "var(--ink-300)",
      glyph: "info"
    },
    accent: {
      accent: "var(--ember-500)",
      glyph: "sparkles"
    },
    positive: {
      accent: "var(--status-positive)",
      glyph: "check-circle"
    },
    warning: {
      accent: "var(--status-warning)",
      glyph: "alert-triangle"
    },
    critical: {
      accent: "var(--status-critical)",
      glyph: "alert-octagon"
    }
  };
  const t = tones[tone] || tones.neutral;
  return /*#__PURE__*/React.createElement("div", {
    role: "status",
    style: {
      display: "flex",
      gap: 12,
      alignItems: "flex-start",
      width: 360,
      maxWidth: "100%",
      padding: "14px 16px",
      background: "var(--surface-overlay)",
      border: "1px solid var(--border-default)",
      borderLeft: `2px solid ${t.accent}`,
      borderRadius: "var(--radius-md)",
      boxShadow: "var(--shadow-lg)",
      ...style
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: t.accent,
      display: "flex",
      marginTop: 1
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: icon || t.glyph,
    size: 18
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      minWidth: 0
    }
  }, title && /*#__PURE__*/React.createElement("div", {
    style: {
      fontSize: "var(--text-sm)",
      fontWeight: "var(--weight-semibold)",
      color: "var(--text-primary)",
      fontFamily: "var(--font-sans)"
    }
  }, title), message && /*#__PURE__*/React.createElement("div", {
    style: {
      fontSize: "var(--text-sm)",
      color: "var(--text-secondary)",
      marginTop: title ? 2 : 0,
      lineHeight: "var(--leading-normal)"
    }
  }, message)), onClose && /*#__PURE__*/React.createElement("span", {
    onClick: onClose,
    style: {
      cursor: "pointer",
      color: "var(--text-tertiary)",
      display: "flex"
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: "x",
    size: 15
  })));
}
Object.assign(__ds_scope, { Toast });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/feedback/Toast.jsx", error: String((e && e.message) || e) }); }

// components/feedback/Tooltip.jsx
try { (() => {
/** MDeck Co Tooltip: hover label on a dark slab. Wraps a trigger. */
function Tooltip({
  label,
  placement = "top",
  children,
  style = {}
}) {
  const [show, setShow] = React.useState(false);
  const pos = {
    top: {
      bottom: "calc(100% + 8px)",
      left: "50%",
      transform: "translateX(-50%)"
    },
    bottom: {
      top: "calc(100% + 8px)",
      left: "50%",
      transform: "translateX(-50%)"
    },
    left: {
      right: "calc(100% + 8px)",
      top: "50%",
      transform: "translateY(-50%)"
    },
    right: {
      left: "calc(100% + 8px)",
      top: "50%",
      transform: "translateY(-50%)"
    }
  };
  return /*#__PURE__*/React.createElement("span", {
    style: {
      position: "relative",
      display: "inline-flex"
    },
    onMouseEnter: () => setShow(true),
    onMouseLeave: () => setShow(false),
    onFocus: () => setShow(true),
    onBlur: () => setShow(false)
  }, children, show && /*#__PURE__*/React.createElement("span", {
    role: "tooltip",
    style: {
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
      ...style
    }
  }, /*#__PURE__*/React.createElement("style", null, "@keyframes mdc-tip{from{opacity:0}to{opacity:1}}"), label));
}
Object.assign(__ds_scope, { Tooltip });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/feedback/Tooltip.jsx", error: String((e && e.message) || e) }); }

// components/forms/Checkbox.jsx
try { (() => {
/** MDeck Co Checkbox: square, ember-filled when checked. */
function Checkbox({
  label,
  description,
  checked = false,
  disabled = false,
  onChange,
  id,
  style = {}
}) {
  const inputId = id || React.useId();
  return /*#__PURE__*/React.createElement("label", {
    htmlFor: inputId,
    style: {
      display: "flex",
      gap: 12,
      alignItems: description ? "flex-start" : "center",
      cursor: disabled ? "not-allowed" : "pointer",
      opacity: disabled ? 0.5 : 1,
      ...style
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      position: "relative",
      flexShrink: 0,
      width: 20,
      height: 20,
      borderRadius: "var(--radius-xs)",
      background: checked ? "var(--accent)" : "var(--surface-inset)",
      border: `1px solid ${checked ? "var(--accent)" : "var(--border-strong)"}`,
      boxShadow: checked ? "var(--glow-ember-sm)" : "var(--shadow-inset-well)",
      transition: "var(--transition-control)",
      display: "flex",
      alignItems: "center",
      justifyContent: "center",
      marginTop: description ? 2 : 0
    }
  }, checked && /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: "check",
    size: 14,
    color: "var(--text-on-accent)",
    strokeWidth: 2.5
  }), /*#__PURE__*/React.createElement("input", {
    id: inputId,
    type: "checkbox",
    checked: checked,
    disabled: disabled,
    onChange: onChange,
    style: {
      position: "absolute",
      opacity: 0,
      inset: 0,
      margin: 0,
      cursor: "inherit"
    }
  })), (label || description) && /*#__PURE__*/React.createElement("span", {
    style: {
      display: "flex",
      flexDirection: "column",
      gap: 2
    }
  }, label && /*#__PURE__*/React.createElement("span", {
    style: {
      fontSize: "var(--text-base)",
      color: "var(--text-primary)",
      fontFamily: "var(--font-sans)"
    }
  }, label), description && /*#__PURE__*/React.createElement("span", {
    style: {
      fontSize: "var(--text-sm)",
      color: "var(--text-tertiary)"
    }
  }, description)));
}
Object.assign(__ds_scope, { Checkbox });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/forms/Checkbox.jsx", error: String((e && e.message) || e) }); }

// components/forms/Input.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/**
 * MDeck Co Input: inset well that lights an ember hairline on focus.
 */
function Input({
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
  const heights = {
    sm: 36,
    md: 44,
    lg: 52
  };
  const h = heights[size] || heights.md;
  const borderColor = error ? "var(--status-critical)" : focus ? "var(--accent)" : "var(--border-default)";
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      flexDirection: "column",
      gap: 8,
      width: "100%",
      ...style
    }
  }, label && /*#__PURE__*/React.createElement("label", {
    htmlFor: inputId,
    style: labelStyle
  }, label), /*#__PURE__*/React.createElement("div", {
    style: {
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
      opacity: disabled ? 0.5 : 1
    }
  }, iconLeft && /*#__PURE__*/React.createElement("span", {
    style: {
      color: "var(--text-tertiary)",
      display: "flex"
    }
  }, iconLeft), /*#__PURE__*/React.createElement("input", _extends({
    id: inputId,
    disabled: disabled,
    onFocus: () => setFocus(true),
    onBlur: () => setFocus(false),
    style: {
      flex: 1,
      minWidth: 0,
      background: "transparent",
      border: "none",
      outline: "none",
      color: "var(--text-primary)",
      fontFamily: "var(--font-sans)",
      fontSize: "var(--text-base)"
    }
  }, rest)), iconRight && /*#__PURE__*/React.createElement("span", {
    style: {
      color: "var(--text-tertiary)",
      display: "flex"
    }
  }, iconRight)), (hint || error) && /*#__PURE__*/React.createElement("span", {
    style: {
      fontSize: "var(--text-xs)",
      color: error ? "var(--status-critical)" : "var(--text-tertiary)",
      fontFamily: "var(--font-sans)"
    }
  }, error || hint));
}
const labelStyle = {
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-2xs)",
  fontWeight: "var(--weight-medium)",
  letterSpacing: "var(--tracking-wider)",
  textTransform: "uppercase",
  color: "var(--text-secondary)"
};
Object.assign(__ds_scope, { Input });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/forms/Input.jsx", error: String((e && e.message) || e) }); }

// components/forms/Radio.jsx
try { (() => {
/** MDeck Co Radio: single-choice control with ember center dot. */
function Radio({
  label,
  description,
  checked = false,
  disabled = false,
  onChange,
  name,
  value,
  id,
  style = {}
}) {
  const inputId = id || React.useId();
  return /*#__PURE__*/React.createElement("label", {
    htmlFor: inputId,
    style: {
      display: "flex",
      gap: 12,
      alignItems: description ? "flex-start" : "center",
      cursor: disabled ? "not-allowed" : "pointer",
      opacity: disabled ? 0.5 : 1,
      ...style
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      position: "relative",
      flexShrink: 0,
      width: 20,
      height: 20,
      borderRadius: "50%",
      background: "var(--surface-inset)",
      border: `1px solid ${checked ? "var(--accent)" : "var(--border-strong)"}`,
      boxShadow: checked ? "var(--glow-ember-sm)" : "var(--shadow-inset-well)",
      transition: "var(--transition-control)",
      display: "flex",
      alignItems: "center",
      justifyContent: "center",
      marginTop: description ? 2 : 0
    }
  }, checked && /*#__PURE__*/React.createElement("span", {
    style: {
      width: 9,
      height: 9,
      borderRadius: "50%",
      background: "var(--accent)"
    }
  }), /*#__PURE__*/React.createElement("input", {
    id: inputId,
    type: "radio",
    name: name,
    value: value,
    checked: checked,
    disabled: disabled,
    onChange: onChange,
    style: {
      position: "absolute",
      opacity: 0,
      inset: 0,
      margin: 0,
      cursor: "inherit"
    }
  })), (label || description) && /*#__PURE__*/React.createElement("span", {
    style: {
      display: "flex",
      flexDirection: "column",
      gap: 2
    }
  }, label && /*#__PURE__*/React.createElement("span", {
    style: {
      fontSize: "var(--text-base)",
      color: "var(--text-primary)",
      fontFamily: "var(--font-sans)"
    }
  }, label), description && /*#__PURE__*/React.createElement("span", {
    style: {
      fontSize: "var(--text-sm)",
      color: "var(--text-tertiary)"
    }
  }, description)));
}
Object.assign(__ds_scope, { Radio });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/forms/Radio.jsx", error: String((e && e.message) || e) }); }

// components/forms/Select.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/** MDeck Co Select: native select styled as an inset well. */
function Select({
  label,
  hint,
  error,
  options = [],
  placeholder,
  size = "md",
  disabled = false,
  id,
  value,
  onChange,
  style = {},
  ...rest
}) {
  const [focus, setFocus] = React.useState(false);
  const inputId = id || React.useId();
  const heights = {
    sm: 36,
    md: 44,
    lg: 52
  };
  const h = heights[size] || heights.md;
  const borderColor = error ? "var(--status-critical)" : focus ? "var(--accent)" : "var(--border-default)";
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      flexDirection: "column",
      gap: 8,
      width: "100%",
      ...style
    }
  }, label && /*#__PURE__*/React.createElement("label", {
    htmlFor: inputId,
    style: {
      fontFamily: "var(--font-mono)",
      fontSize: "var(--text-2xs)",
      fontWeight: 500,
      letterSpacing: "var(--tracking-wider)",
      textTransform: "uppercase",
      color: "var(--text-secondary)"
    }
  }, label), /*#__PURE__*/React.createElement("div", {
    style: {
      position: "relative",
      height: h,
      borderRadius: "var(--radius-md)",
      background: "var(--surface-inset)",
      border: `1px solid ${borderColor}`,
      boxShadow: focus && !error ? "var(--glow-ember-sm)" : "var(--shadow-inset-well)",
      transition: "var(--transition-control)",
      opacity: disabled ? 0.5 : 1
    }
  }, /*#__PURE__*/React.createElement("select", _extends({
    id: inputId,
    disabled: disabled,
    value: value,
    onChange: onChange,
    onFocus: () => setFocus(true),
    onBlur: () => setFocus(false),
    style: {
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
      cursor: disabled ? "not-allowed" : "pointer"
    }
  }, rest), placeholder && /*#__PURE__*/React.createElement("option", {
    value: "",
    disabled: true
  }, placeholder), options.map(o => {
    const val = typeof o === "string" ? o : o.value;
    const lbl = typeof o === "string" ? o : o.label;
    return /*#__PURE__*/React.createElement("option", {
      key: val,
      value: val,
      style: {
        background: "var(--ink-800)",
        color: "var(--text-primary)"
      }
    }, lbl);
  })), /*#__PURE__*/React.createElement("span", {
    style: {
      position: "absolute",
      right: 12,
      top: "50%",
      transform: "translateY(-50%)",
      pointerEvents: "none",
      color: "var(--text-tertiary)",
      display: "flex"
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: "chevron-down",
    size: 16
  }))), (hint || error) && /*#__PURE__*/React.createElement("span", {
    style: {
      fontSize: "var(--text-xs)",
      color: error ? "var(--status-critical)" : "var(--text-tertiary)"
    }
  }, error || hint));
}
Object.assign(__ds_scope, { Select });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/forms/Select.jsx", error: String((e && e.message) || e) }); }

// components/forms/Switch.jsx
try { (() => {
/** MDeck Co Switch: sliding toggle; ember track + glow when on. */
function Switch({
  label,
  description,
  checked = false,
  disabled = false,
  onChange,
  id,
  style = {}
}) {
  const inputId = id || React.useId();
  return /*#__PURE__*/React.createElement("label", {
    htmlFor: inputId,
    style: {
      display: "flex",
      gap: 12,
      alignItems: description ? "flex-start" : "center",
      cursor: disabled ? "not-allowed" : "pointer",
      opacity: disabled ? 0.5 : 1,
      ...style
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      position: "relative",
      flexShrink: 0,
      width: 42,
      height: 24,
      borderRadius: "var(--radius-pill)",
      background: checked ? "var(--accent)" : "var(--surface-inset)",
      border: `1px solid ${checked ? "var(--accent)" : "var(--border-strong)"}`,
      boxShadow: checked ? "var(--glow-ember-sm)" : "var(--shadow-inset-well)",
      transition: "var(--transition-control)",
      marginTop: description ? 1 : 0
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      position: "absolute",
      top: "50%",
      left: checked ? 21 : 3,
      transform: "translateY(-50%)",
      width: 16,
      height: 16,
      borderRadius: "50%",
      background: checked ? "var(--text-on-accent)" : "var(--ink-200)",
      boxShadow: "0 1px 2px rgba(0,0,0,0.5)",
      transition: "left var(--dur-base) var(--ease-out), background var(--dur-fast)"
    }
  }), /*#__PURE__*/React.createElement("input", {
    id: inputId,
    type: "checkbox",
    checked: checked,
    disabled: disabled,
    onChange: onChange,
    style: {
      position: "absolute",
      opacity: 0,
      inset: 0,
      margin: 0,
      cursor: "inherit"
    }
  })), (label || description) && /*#__PURE__*/React.createElement("span", {
    style: {
      display: "flex",
      flexDirection: "column",
      gap: 2
    }
  }, label && /*#__PURE__*/React.createElement("span", {
    style: {
      fontSize: "var(--text-base)",
      color: "var(--text-primary)",
      fontFamily: "var(--font-sans)"
    }
  }, label), description && /*#__PURE__*/React.createElement("span", {
    style: {
      fontSize: "var(--text-sm)",
      color: "var(--text-tertiary)"
    }
  }, description)));
}
Object.assign(__ds_scope, { Switch });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/forms/Switch.jsx", error: String((e && e.message) || e) }); }

// components/forms/Textarea.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/** MDeck Co Textarea: multi-line variant of Input. */
function Textarea({
  label,
  hint,
  error,
  rows = 4,
  disabled = false,
  id,
  style = {},
  ...rest
}) {
  const [focus, setFocus] = React.useState(false);
  const inputId = id || React.useId();
  const borderColor = error ? "var(--status-critical)" : focus ? "var(--accent)" : "var(--border-default)";
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      flexDirection: "column",
      gap: 8,
      width: "100%",
      ...style
    }
  }, label && /*#__PURE__*/React.createElement("label", {
    htmlFor: inputId,
    style: {
      fontFamily: "var(--font-mono)",
      fontSize: "var(--text-2xs)",
      fontWeight: 500,
      letterSpacing: "var(--tracking-wider)",
      textTransform: "uppercase",
      color: "var(--text-secondary)"
    }
  }, label), /*#__PURE__*/React.createElement("textarea", _extends({
    id: inputId,
    rows: rows,
    disabled: disabled,
    onFocus: () => setFocus(true),
    onBlur: () => setFocus(false),
    style: {
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
      opacity: disabled ? 0.5 : 1
    }
  }, rest)), (hint || error) && /*#__PURE__*/React.createElement("span", {
    style: {
      fontSize: "var(--text-xs)",
      color: error ? "var(--status-critical)" : "var(--text-tertiary)"
    }
  }, error || hint));
}
Object.assign(__ds_scope, { Textarea });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/forms/Textarea.jsx", error: String((e && e.message) || e) }); }

// components/navigation/Tabs.jsx
try { (() => {
/**
 * MDeck Co Tabs: underline navigation. Active tab lights an ember bar.
 * Controlled: pass `value` + `onChange`, tabs as [{value,label,icon?}].
 */
function Tabs({
  tabs = [],
  value,
  onChange,
  style = {}
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      gap: 4,
      borderBottom: "1px solid var(--border-default)",
      ...style
    }
  }, tabs.map(t => {
    const active = t.value === value;
    return /*#__PURE__*/React.createElement("button", {
      key: t.value,
      onClick: () => onChange && onChange(t.value),
      style: {
        position: "relative",
        display: "inline-flex",
        alignItems: "center",
        gap: 8,
        padding: "10px 14px 12px",
        background: "transparent",
        border: "none",
        cursor: "pointer",
        fontFamily: "var(--font-sans)",
        fontSize: "var(--text-sm)",
        fontWeight: active ? "var(--weight-semibold)" : "var(--weight-regular)",
        color: active ? "var(--text-primary)" : "var(--text-tertiary)",
        transition: "color var(--dur-fast) var(--ease-standard)",
        outline: "none"
      },
      onMouseEnter: e => {
        if (!active) e.currentTarget.style.color = "var(--text-secondary)";
      },
      onMouseLeave: e => {
        if (!active) e.currentTarget.style.color = "var(--text-tertiary)";
      }
    }, t.label, t.count != null && /*#__PURE__*/React.createElement("span", {
      style: {
        fontFamily: "var(--font-mono)",
        fontSize: "var(--text-2xs)",
        color: active ? "var(--ember-300)" : "var(--text-tertiary)"
      }
    }, t.count), /*#__PURE__*/React.createElement("span", {
      style: {
        position: "absolute",
        left: 8,
        right: 8,
        bottom: -1,
        height: 2,
        borderRadius: 2,
        background: active ? "var(--accent)" : "transparent",
        boxShadow: active ? "var(--glow-ember-sm)" : "none",
        transition: "var(--transition-control)"
      }
    }));
  }));
}
Object.assign(__ds_scope, { Tabs });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/navigation/Tabs.jsx", error: String((e && e.message) || e) }); }

// ui_kits/website/Apply.jsx
try { (() => {
/* MDeck Co website: the gated access / request flow */

function Apply({
  onBack,
  onSubmit
}) {
  const {
    Button,
    Input,
    Textarea,
    Select,
    Checkbox,
    Icon,
    Badge
  } = window.MDeckCoDesignSystem_4d3c0a;
  const [ok, setOk] = React.useState(false);
  return /*#__PURE__*/React.createElement("section", {
    style: {
      minHeight: 720,
      display: "flex"
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      width: 420,
      flexShrink: 0,
      position: "relative",
      overflow: "hidden",
      background: "var(--ink-900)",
      borderRight: "1px solid var(--border-subtle)",
      padding: "56px 48px",
      display: "flex",
      flexDirection: "column",
      justifyContent: "space-between"
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      position: "absolute",
      inset: 0,
      background: "radial-gradient(120% 80% at 20% 0%, rgba(255,77,28,0.10), transparent 60%)"
    }
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      position: "relative"
    }
  }, /*#__PURE__*/React.createElement("img", {
    src: "../../assets/logos/mdeckco-white.svg",
    alt: "MDeck Co",
    style: {
      height: 40,
      opacity: 0.9,
      marginBottom: 48
    }
  }), /*#__PURE__*/React.createElement("div", {
    className: "mdc-eyebrow",
    style: {
      marginBottom: 18
    }
  }, "Bring us your challenge"), /*#__PURE__*/React.createElement("h1", {
    style: {
      fontFamily: "var(--font-display)",
      fontWeight: 300,
      fontSize: 44,
      lineHeight: 1.05,
      letterSpacing: "-0.02em",
      color: "var(--text-primary)",
      margin: "0 0 20px"
    }
  }, "Tell us", /*#__PURE__*/React.createElement("br", null), "what's hard."), /*#__PURE__*/React.createElement("p", {
    style: {
      fontFamily: "var(--font-sans)",
      fontSize: 15,
      color: "var(--text-secondary)",
      lineHeight: 1.65,
      margin: 0,
      maxWidth: 300
    }
  }, "A partner reads every enquiry themselves and replies within two business days. No gatekeeping. Just the right senior people, fast.")), /*#__PURE__*/React.createElement("div", {
    style: {
      position: "relative",
      display: "flex",
      flexDirection: "column",
      gap: 14
    }
  }, ["Read by a partner", "Two business days", "Discretion, always"].map(t => /*#__PURE__*/React.createElement("div", {
    key: t,
    style: {
      display: "flex",
      alignItems: "center",
      gap: 12,
      color: "var(--text-tertiary)",
      fontFamily: "var(--font-mono)",
      fontSize: 11,
      letterSpacing: "0.06em",
      textTransform: "uppercase"
    }
  }, /*#__PURE__*/React.createElement(Icon, {
    name: "check",
    size: 14,
    color: "var(--ember-500)"
  }), t)))), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      display: "flex",
      alignItems: "center",
      justifyContent: "center",
      padding: "56px 40px"
    }
  }, ok ? /*#__PURE__*/React.createElement("div", {
    style: {
      textAlign: "center",
      maxWidth: 400
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      width: 64,
      height: 64,
      borderRadius: "50%",
      margin: "0 auto 28px",
      display: "flex",
      alignItems: "center",
      justifyContent: "center",
      background: "var(--accent-quiet)",
      boxShadow: "var(--glow-ember-md)"
    }
  }, /*#__PURE__*/React.createElement(Icon, {
    name: "check",
    size: 28,
    color: "var(--ember-500)",
    strokeWidth: 2
  })), /*#__PURE__*/React.createElement("h2", {
    style: {
      fontFamily: "var(--font-display)",
      fontWeight: 300,
      fontSize: 36,
      color: "var(--text-primary)",
      margin: "0 0 14px"
    }
  }, "We've got it."), /*#__PURE__*/React.createElement("p", {
    style: {
      fontFamily: "var(--font-sans)",
      fontSize: 15,
      color: "var(--text-secondary)",
      lineHeight: 1.65,
      margin: "0 0 28px"
    }
  }, "Your challenge is with a partner now. Expect a reply within two business days, whatever the problem."), /*#__PURE__*/React.createElement(Button, {
    variant: "ghost",
    onClick: onBack,
    iconLeft: /*#__PURE__*/React.createElement(Icon, {
      name: "arrow-left",
      size: 16
    })
  }, "Back to site")) : /*#__PURE__*/React.createElement("div", {
    style: {
      width: "100%",
      maxWidth: 460
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      justifyContent: "space-between",
      alignItems: "center",
      marginBottom: 28
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: "var(--font-mono)",
      fontSize: 11,
      letterSpacing: "0.14em",
      textTransform: "uppercase",
      color: "var(--text-tertiary)"
    }
  }, "Enquiry \xB7 01 / 01"), /*#__PURE__*/React.createElement(Badge, {
    tone: "accent",
    dot: true
  }, "Private")), /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      flexDirection: "column",
      gap: 18
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      gap: 14
    }
  }, /*#__PURE__*/React.createElement(Input, {
    label: "Name",
    placeholder: "Full name"
  }), /*#__PURE__*/React.createElement(Input, {
    label: "Firm",
    placeholder: "Where you sit"
  })), /*#__PURE__*/React.createElement(Input, {
    label: "Work email",
    placeholder: "you@firm.com",
    iconLeft: /*#__PURE__*/React.createElement(Icon, {
      name: "mail",
      size: 16
    })
  }), /*#__PURE__*/React.createElement(Select, {
    label: "What brings you",
    placeholder: "Choose one",
    options: ["A live decision", "An interim mandate", "A problem others passed on", "Something else"]
  }), /*#__PURE__*/React.createElement(Textarea, {
    label: "In one line",
    rows: 3,
    placeholder: "The challenge you're facing."
  }), /*#__PURE__*/React.createElement(Checkbox, {
    label: "This one's time-sensitive",
    description: "We'll fast-track your first reply."
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      gap: 12,
      marginTop: 28
    }
  }, /*#__PURE__*/React.createElement(Button, {
    variant: "ghost",
    onClick: onBack
  }, "Cancel"), /*#__PURE__*/React.createElement(Button, {
    variant: "primary",
    fullWidth: true,
    onClick: () => {
      setOk(true);
      onSubmit && onSubmit();
    },
    iconRight: /*#__PURE__*/React.createElement(Icon, {
      name: "arrow-right",
      size: 16
    })
  }, "Send it over")))));
}
Object.assign(window, {
  Apply
});
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/website/Apply.jsx", error: String((e && e.message) || e) }); }

// ui_kits/website/Chrome.jsx
try { (() => {
/* MDeck Co website: top nav + footer chrome */

function Nav({
  onApply,
  onNav,
  active
}) {
  const {
    Button
  } = window.MDeckCoDesignSystem_4d3c0a;
  const links = ["Work", "Practice", "People", "Thinking"];
  return /*#__PURE__*/React.createElement("header", {
    style: {
      position: "sticky",
      top: 0,
      zIndex: 50,
      display: "flex",
      alignItems: "center",
      justifyContent: "space-between",
      padding: "0 40px",
      height: 72,
      background: "rgba(5,5,5,0.72)",
      backdropFilter: "blur(12px) saturate(1.1)",
      WebkitBackdropFilter: "blur(12px) saturate(1.1)",
      borderBottom: "1px solid var(--border-subtle)"
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      alignItems: "center",
      gap: 40
    }
  }, /*#__PURE__*/React.createElement("img", {
    src: "../../assets/logos/mdeckco-white.svg",
    alt: "MDeck Co",
    onClick: () => onNav("home"),
    style: {
      height: 30,
      opacity: 0.9,
      cursor: "pointer"
    }
  }), /*#__PURE__*/React.createElement("nav", {
    style: {
      display: "flex",
      gap: 28
    }
  }, links.map(l => /*#__PURE__*/React.createElement("a", {
    key: l,
    onClick: () => onNav("home"),
    style: {
      fontFamily: "var(--font-sans)",
      fontSize: 14,
      color: "var(--text-secondary)",
      cursor: "pointer",
      transition: "color var(--dur-fast)"
    },
    onMouseEnter: e => e.target.style.color = "var(--text-primary)",
    onMouseLeave: e => e.target.style.color = "var(--text-secondary)"
  }, l)))), /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      alignItems: "center",
      gap: 14
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: "var(--font-mono)",
      fontSize: 11,
      letterSpacing: "0.16em",
      textTransform: "uppercase",
      color: "var(--text-tertiary)"
    }
  }, "Est. MMXIX"), /*#__PURE__*/React.createElement(Button, {
    size: "sm",
    variant: "primary",
    onClick: onApply
  }, "Get in touch")));
}
function Footer() {
  return /*#__PURE__*/React.createElement("footer", {
    style: {
      padding: "64px 40px 40px",
      borderTop: "1px solid var(--border-subtle)",
      background: "var(--ink-900)"
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      justifyContent: "space-between",
      alignItems: "flex-start",
      flexWrap: "wrap",
      gap: 40
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      maxWidth: 280
    }
  }, /*#__PURE__*/React.createElement("img", {
    src: "../../assets/logos/mdeckco-white.svg",
    alt: "MDeck Co",
    style: {
      height: 34,
      opacity: 0.5,
      marginBottom: 18
    }
  }), /*#__PURE__*/React.createElement("p", {
    style: {
      fontFamily: "var(--font-sans)",
      fontSize: 13,
      color: "var(--text-tertiary)",
      lineHeight: 1.6,
      margin: 0
    }
  }, "Senior advisory for decisions that don't get a second attempt.")), /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      gap: 64
    }
  }, [["Firm", ["Practice", "People", "Thinking", "Access"]], ["Contact", ["Offices", "Enquiries", "Press", "Careers"]]].map(([h, items]) => /*#__PURE__*/React.createElement("div", {
    key: h
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      fontFamily: "var(--font-mono)",
      fontSize: 10,
      letterSpacing: "0.18em",
      textTransform: "uppercase",
      color: "var(--text-tertiary)",
      marginBottom: 16
    }
  }, h), items.map(i => /*#__PURE__*/React.createElement("div", {
    key: i,
    style: {
      fontSize: 13,
      color: "var(--text-secondary)",
      marginBottom: 10,
      cursor: "pointer"
    }
  }, i)))))), /*#__PURE__*/React.createElement("div", {
    style: {
      marginTop: 48,
      display: "flex",
      justifyContent: "space-between",
      alignItems: "center"
    }
  }, /*#__PURE__*/React.createElement("img", {
    src: "../../assets/brand/signature-line.svg",
    alt: "",
    style: {
      height: 26,
      filter: "invert(1)",
      opacity: 0.22
    }
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: "var(--font-mono)",
      fontSize: 10,
      letterSpacing: "0.16em",
      color: "var(--text-tertiary)"
    }
  }, "\xA9 MMXXVI MDeck Co \xB7 EST. MMXIX")));
}
Object.assign(window, {
  Nav,
  Footer
});
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/website/Chrome.jsx", error: String((e && e.message) || e) }); }

// ui_kits/website/Home.jsx
try { (() => {
/* MDeck Co website: home page sections */

function Hero({
  onApply
}) {
  const {
    Button,
    Icon
  } = window.MDeckCoDesignSystem_4d3c0a;
  return /*#__PURE__*/React.createElement("section", {
    style: {
      position: "relative",
      overflow: "hidden",
      minHeight: 620,
      display: "flex",
      alignItems: "center"
    }
  }, /*#__PURE__*/React.createElement("img", {
    src: "../../assets/brand/mood.jpg",
    alt: "",
    style: {
      position: "absolute",
      inset: 0,
      width: "100%",
      height: "100%",
      objectFit: "cover",
      opacity: 0.5
    }
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      position: "absolute",
      inset: 0,
      background: "linear-gradient(90deg, rgba(5,5,5,0.94) 30%, rgba(5,5,5,0.55) 100%)"
    }
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      position: "relative",
      padding: "0 40px",
      maxWidth: 820
    }
  }, /*#__PURE__*/React.createElement("div", {
    className: "mdc-eyebrow",
    style: {
      marginBottom: 24
    }
  }, "Senior advisory"), /*#__PURE__*/React.createElement("h1", {
    style: {
      fontFamily: "var(--font-display)",
      fontWeight: 300,
      fontSize: 76,
      lineHeight: 1.02,
      letterSpacing: "-0.02em",
      color: "var(--text-primary)",
      margin: "0 0 24px"
    }
  }, "Bring us the", /*#__PURE__*/React.createElement("br", null), "hard problems."), /*#__PURE__*/React.createElement("p", {
    style: {
      fontFamily: "var(--font-sans)",
      fontSize: 18,
      lineHeight: 1.6,
      color: "var(--text-secondary)",
      maxWidth: 540,
      margin: "0 0 36px"
    }
  }, "Whatever the challenge, MDeck Co can help. We're built for the ones that demand the most senior people in the room: the projects others pass on. Need experience? We have what it takes."), /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      gap: 14,
      alignItems: "center"
    }
  }, /*#__PURE__*/React.createElement(Button, {
    size: "lg",
    variant: "primary",
    onClick: onApply,
    iconRight: /*#__PURE__*/React.createElement(Icon, {
      name: "arrow-right",
      size: 18
    })
  }, "Bring us your challenge"), /*#__PURE__*/React.createElement(Button, {
    size: "lg",
    variant: "ghost"
  }, "See our work"))));
}
const WORK = [{
  tag: "Private Equity",
  title: "A carve-out, closed in 11 weeks",
  meta: "Interim leadership · Nordics"
}, {
  tag: "Industrials",
  title: "€2.4B cost programme, held",
  meta: "Embedded · DACH"
}, {
  tag: "Financial Services",
  title: "A regulator won over, quietly",
  meta: "Advisory · UK"
}];
function Work() {
  const {
    Card,
    Badge,
    Icon
  } = window.MDeckCoDesignSystem_4d3c0a;
  return /*#__PURE__*/React.createElement("section", {
    style: {
      padding: "112px 40px"
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      alignItems: "flex-end",
      justifyContent: "space-between",
      marginBottom: 48
    }
  }, /*#__PURE__*/React.createElement("div", null, /*#__PURE__*/React.createElement("div", {
    className: "mdc-eyebrow",
    style: {
      marginBottom: 14
    }
  }, "Selected work"), /*#__PURE__*/React.createElement("h2", {
    style: {
      fontFamily: "var(--font-display)",
      fontWeight: 300,
      fontSize: 42,
      letterSpacing: "-0.01em",
      color: "var(--text-primary)",
      margin: 0
    }
  }, "The outcome, not the theatre.")), /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: "var(--font-mono)",
      fontSize: 12,
      color: "var(--text-tertiary)"
    }
  }, "40+ engagements \xB7 since MMXIX")), /*#__PURE__*/React.createElement("div", {
    style: {
      display: "grid",
      gridTemplateColumns: "repeat(3, 1fr)",
      gap: 20
    }
  }, WORK.map((w, i) => /*#__PURE__*/React.createElement(Card, {
    key: i,
    interactive: true,
    padding: "lg",
    style: {
      minHeight: 260,
      display: "flex",
      flexDirection: "column",
      justifyContent: "space-between"
    }
  }, /*#__PURE__*/React.createElement(Badge, {
    tone: "neutral"
  }, w.tag), /*#__PURE__*/React.createElement("div", null, /*#__PURE__*/React.createElement("h3", {
    style: {
      fontFamily: "var(--font-display)",
      fontWeight: 400,
      fontSize: 24,
      lineHeight: 1.15,
      color: "var(--text-primary)",
      margin: "0 0 12px"
    }
  }, w.title), /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      alignItems: "center",
      gap: 8,
      fontFamily: "var(--font-mono)",
      fontSize: 11,
      letterSpacing: "0.08em",
      textTransform: "uppercase",
      color: "var(--text-tertiary)"
    }
  }, /*#__PURE__*/React.createElement(Icon, {
    name: "arrow-up-right",
    size: 13,
    color: "var(--ember-500)"
  }), w.meta))))));
}
const STATS = [["06", "Partners"], ["40+", "Engagements"], ["11", "Weeks, median"], ["2d", "First reply"]];
function Ethos() {
  return /*#__PURE__*/React.createElement("section", {
    style: {
      padding: "112px 40px",
      background: "var(--ink-900)",
      borderTop: "1px solid var(--border-subtle)",
      borderBottom: "1px solid var(--border-subtle)"
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      maxWidth: 720,
      margin: "0 auto 72px",
      textAlign: "center"
    }
  }, /*#__PURE__*/React.createElement("div", {
    className: "mdc-eyebrow",
    style: {
      marginBottom: 20
    }
  }, "The standard"), /*#__PURE__*/React.createElement("p", {
    style: {
      fontFamily: "var(--font-display)",
      fontWeight: 300,
      fontSize: 34,
      lineHeight: 1.3,
      color: "var(--text-primary)",
      margin: 0
    }
  }, "Every engagement is led by someone who has ", /*#__PURE__*/React.createElement("span", {
    style: {
      color: "var(--ember-500)"
    }
  }, "done it before"), ".")), /*#__PURE__*/React.createElement("div", {
    style: {
      display: "grid",
      gridTemplateColumns: "repeat(4, 1fr)",
      gap: 1,
      background: "var(--border-subtle)",
      borderRadius: "var(--radius-lg)",
      overflow: "hidden",
      border: "1px solid var(--border-subtle)"
    }
  }, STATS.map(([n, l]) => /*#__PURE__*/React.createElement("div", {
    key: l,
    style: {
      background: "var(--surface-base)",
      padding: "40px 28px",
      textAlign: "center"
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      fontFamily: "var(--font-display)",
      fontWeight: 300,
      fontSize: 52,
      color: "var(--text-primary)",
      lineHeight: 1
    }
  }, n), /*#__PURE__*/React.createElement("div", {
    style: {
      fontFamily: "var(--font-mono)",
      fontSize: 11,
      letterSpacing: "0.14em",
      textTransform: "uppercase",
      color: "var(--text-tertiary)",
      marginTop: 12
    }
  }, l)))));
}
function AccessBand({
  onApply
}) {
  const {
    Button,
    Icon
  } = window.MDeckCoDesignSystem_4d3c0a;
  return /*#__PURE__*/React.createElement("section", {
    style: {
      padding: "120px 40px",
      textAlign: "center"
    }
  }, /*#__PURE__*/React.createElement("div", {
    className: "mdc-eyebrow",
    style: {
      marginBottom: 22
    }
  }, "However hard, we can help"), /*#__PURE__*/React.createElement("h2", {
    style: {
      fontFamily: "var(--font-display)",
      fontWeight: 300,
      fontSize: 56,
      letterSpacing: "-0.02em",
      color: "var(--text-primary)",
      margin: "0 0 20px"
    }
  }, "Tell us what's hard."), /*#__PURE__*/React.createElement("p", {
    style: {
      fontFamily: "var(--font-sans)",
      fontSize: 17,
      color: "var(--text-secondary)",
      maxWidth: 500,
      margin: "0 auto 36px",
      lineHeight: 1.6
    }
  }, "Every challenge gets a senior pair of eyes. Bring us the problem others found too hard. We reply within two business days."), /*#__PURE__*/React.createElement(Button, {
    size: "lg",
    variant: "primary",
    onClick: onApply,
    iconRight: /*#__PURE__*/React.createElement(Icon, {
      name: "arrow-right",
      size: 18
    })
  }, "Bring us your challenge"));
}
Object.assign(window, {
  Hero,
  Work,
  Ethos,
  AccessBand
});
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/website/Home.jsx", error: String((e && e.message) || e) }); }

__ds_ns.Badge = __ds_scope.Badge;

__ds_ns.Button = __ds_scope.Button;

__ds_ns.Card = __ds_scope.Card;

__ds_ns.Divider = __ds_scope.Divider;

__ds_ns.Icon = __ds_scope.Icon;

__ds_ns.IconButton = __ds_scope.IconButton;

__ds_ns.Tag = __ds_scope.Tag;

__ds_ns.Dialog = __ds_scope.Dialog;

__ds_ns.Toast = __ds_scope.Toast;

__ds_ns.Tooltip = __ds_scope.Tooltip;

__ds_ns.Checkbox = __ds_scope.Checkbox;

__ds_ns.Input = __ds_scope.Input;

__ds_ns.Radio = __ds_scope.Radio;

__ds_ns.Select = __ds_scope.Select;

__ds_ns.Switch = __ds_scope.Switch;

__ds_ns.Textarea = __ds_scope.Textarea;

__ds_ns.Tabs = __ds_scope.Tabs;

})();
