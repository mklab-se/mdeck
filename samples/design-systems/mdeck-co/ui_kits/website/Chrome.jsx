/* MDeck Co website: top nav + footer chrome */

function Nav({ onApply, onNav, active }) {
  const { Button } = window.MDeckCoDesignSystem_4d3c0a;
  const links = ["Work", "Practice", "People", "Thinking"];
  return (
    <header style={{
      position: "sticky", top: 0, zIndex: 50,
      display: "flex", alignItems: "center", justifyContent: "space-between",
      padding: "0 40px", height: 72,
      background: "rgba(5,5,5,0.72)",
      backdropFilter: "blur(12px) saturate(1.1)", WebkitBackdropFilter: "blur(12px) saturate(1.1)",
      borderBottom: "1px solid var(--border-subtle)",
    }}>
      <div style={{ display: "flex", alignItems: "center", gap: 40 }}>
        <img src="../../assets/logos/mdeckco-white.svg" alt="MDeck Co"
          onClick={() => onNav("home")}
          style={{ height: 30, opacity: 0.9, cursor: "pointer" }} />
        <nav style={{ display: "flex", gap: 28 }}>
          {links.map((l) => (
            <a key={l} onClick={() => onNav("home")} style={{
              fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--text-secondary)",
              cursor: "pointer", transition: "color var(--dur-fast)",
            }}
            onMouseEnter={(e)=>e.target.style.color="var(--text-primary)"}
            onMouseLeave={(e)=>e.target.style.color="var(--text-secondary)"}>{l}</a>
          ))}
        </nav>
      </div>
      <div style={{ display: "flex", alignItems: "center", gap: 14 }}>
        <span style={{ fontFamily: "var(--font-mono)", fontSize: 11, letterSpacing: "0.16em", textTransform: "uppercase", color: "var(--text-tertiary)" }}>Est. MMXIX</span>
        <Button size="sm" variant="primary" onClick={onApply}>Get in touch</Button>
      </div>
    </header>
  );
}

function Footer() {
  return (
    <footer style={{ padding: "64px 40px 40px", borderTop: "1px solid var(--border-subtle)", background: "var(--ink-900)" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", flexWrap: "wrap", gap: 40 }}>
        <div style={{ maxWidth: 280 }}>
          <img src="../../assets/logos/mdeckco-white.svg" alt="MDeck Co" style={{ height: 34, opacity: 0.5, marginBottom: 18 }} />
          <p style={{ fontFamily: "var(--font-sans)", fontSize: 13, color: "var(--text-tertiary)", lineHeight: 1.6, margin: 0 }}>
            Senior advisory for decisions that don't get a second attempt.
          </p>
        </div>
        <div style={{ display: "flex", gap: 64 }}>
          {[["Firm", ["Practice", "People", "Thinking", "Access"]], ["Contact", ["Offices", "Enquiries", "Press", "Careers"]]].map(([h, items]) => (
            <div key={h}>
              <div style={{ fontFamily: "var(--font-mono)", fontSize: 10, letterSpacing: "0.18em", textTransform: "uppercase", color: "var(--text-tertiary)", marginBottom: 16 }}>{h}</div>
              {items.map((i) => (
                <div key={i} style={{ fontSize: 13, color: "var(--text-secondary)", marginBottom: 10, cursor: "pointer" }}>{i}</div>
              ))}
            </div>
          ))}
        </div>
      </div>
      <div style={{ marginTop: 48, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <img src="../../assets/brand/signature-line.svg" alt="" style={{ height: 26, filter: "invert(1)", opacity: 0.22 }} />
        <span style={{ fontFamily: "var(--font-mono)", fontSize: 10, letterSpacing: "0.16em", color: "var(--text-tertiary)" }}>© MMXXVI MDeck Co · EST. MMXIX</span>
      </div>
    </footer>
  );
}

Object.assign(window, { Nav, Footer });
