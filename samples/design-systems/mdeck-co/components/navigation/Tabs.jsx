import React from "react";

/**
 * MDeck Co Tabs: underline navigation. Active tab lights an ember bar.
 * Controlled: pass `value` + `onChange`, tabs as [{value,label,icon?}].
 */
export function Tabs({ tabs = [], value, onChange, style = {} }) {
  return (
    <div style={{ display: "flex", gap: 4, borderBottom: "1px solid var(--border-default)", ...style }}>
      {tabs.map((t) => {
        const active = t.value === value;
        return (
          <button
            key={t.value}
            onClick={() => onChange && onChange(t.value)}
            style={{
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
              outline: "none",
            }}
            onMouseEnter={(e) => { if (!active) e.currentTarget.style.color = "var(--text-secondary)"; }}
            onMouseLeave={(e) => { if (!active) e.currentTarget.style.color = "var(--text-tertiary)"; }}
          >
            {t.label}
            {t.count != null && (
              <span style={{ fontFamily: "var(--font-mono)", fontSize: "var(--text-2xs)", color: active ? "var(--ember-300)" : "var(--text-tertiary)" }}>{t.count}</span>
            )}
            <span
              style={{
                position: "absolute",
                left: 8,
                right: 8,
                bottom: -1,
                height: 2,
                borderRadius: 2,
                background: active ? "var(--accent)" : "transparent",
                boxShadow: active ? "var(--glow-ember-sm)" : "none",
                transition: "var(--transition-control)",
              }}
            />
          </button>
        );
      })}
    </div>
  );
}
