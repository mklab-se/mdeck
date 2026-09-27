import React from "react";
import { IconButton } from "../core/IconButton.jsx";

/**
 * MDeck Co Dialog: overlay panel. Backdrop blurs the room behind it.
 * Controlled via `open` / `onClose`.
 */
export function Dialog({ open, onClose, title, description, children, footer, width = 480, style = {} }) {
  React.useEffect(() => {
    if (!open) return;
    const onKey = (e) => e.key === "Escape" && onClose && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      onClick={onClose}
      style={{
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
        animation: "mdc-fade var(--dur-base) var(--ease-out)",
      }}
    >
      <style>{"@keyframes mdc-fade{from{opacity:0}to{opacity:1}}@keyframes mdc-rise{from{opacity:0;transform:translateY(8px) scale(0.99)}to{opacity:1;transform:none}}"}</style>
      <div
        role="dialog"
        aria-modal="true"
        onClick={(e) => e.stopPropagation()}
        style={{
          width,
          maxWidth: "100%",
          background: "var(--surface-overlay)",
          border: "1px solid var(--border-default)",
          borderRadius: "var(--radius-xl)",
          boxShadow: "var(--shadow-xl)",
          padding: "var(--space-8)",
          animation: "mdc-rise var(--dur-slow) var(--ease-out)",
          ...style,
        }}
      >
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 16, marginBottom: description ? 6 : 20 }}>
          {title && <h2 style={{ margin: 0, font: "var(--type-title)", fontFamily: "var(--font-display)", fontWeight: "var(--weight-regular)", fontSize: "var(--text-2xl)", color: "var(--text-primary)", letterSpacing: "var(--tracking-tight)" }}>{title}</h2>}
          {onClose && <IconButton name="x" aria-label="Close" onClick={onClose} size="sm" />}
        </div>
        {description && <p style={{ margin: "0 0 20px", color: "var(--text-secondary)", fontSize: "var(--text-base)", lineHeight: "var(--leading-relaxed)" }}>{description}</p>}
        {children}
        {footer && <div style={{ display: "flex", justifyContent: "flex-end", gap: 10, marginTop: "var(--space-8)" }}>{footer}</div>}
      </div>
    </div>
  );
}
