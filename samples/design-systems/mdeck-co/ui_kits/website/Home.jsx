/* MDeck Co website: home page sections */

function Hero({ onApply }) {
  const { Button, Icon } = window.MDeckCoDesignSystem_4d3c0a;
  return (
    <section style={{ position: "relative", overflow: "hidden", minHeight: 620, display: "flex", alignItems: "center" }}>
      <img src="../../assets/brand/mood.jpg" alt="" style={{ position: "absolute", inset: 0, width: "100%", height: "100%", objectFit: "cover", opacity: 0.5 }} />
      <div style={{ position: "absolute", inset: 0, background: "linear-gradient(90deg, rgba(5,5,5,0.94) 30%, rgba(5,5,5,0.55) 100%)" }} />
      <div style={{ position: "relative", padding: "0 40px", maxWidth: 820 }}>
        <div className="mdc-eyebrow" style={{ marginBottom: 24 }}>Senior advisory</div>
        <h1 style={{ fontFamily: "var(--font-display)", fontWeight: 300, fontSize: 76, lineHeight: 1.02, letterSpacing: "-0.02em", color: "var(--text-primary)", margin: "0 0 24px" }}>
          Bring us the<br />hard problems.
        </h1>
        <p style={{ fontFamily: "var(--font-sans)", fontSize: 18, lineHeight: 1.6, color: "var(--text-secondary)", maxWidth: 540, margin: "0 0 36px" }}>
          Whatever the challenge, MDeck Co can help. We're built for the ones that demand the most senior people in the room: the projects others pass on. Need experience? We have what it takes.
        </p>
        <div style={{ display: "flex", gap: 14, alignItems: "center" }}>
          <Button size="lg" variant="primary" onClick={onApply} iconRight={<Icon name="arrow-right" size={18} />}>Bring us your challenge</Button>
          <Button size="lg" variant="ghost">See our work</Button>
        </div>
      </div>
    </section>
  );
}

const WORK = [
  { tag: "Private Equity", title: "A carve-out, closed in 11 weeks", meta: "Interim leadership · Nordics" },
  { tag: "Industrials", title: "€2.4B cost programme, held", meta: "Embedded · DACH" },
  { tag: "Financial Services", title: "A regulator won over, quietly", meta: "Advisory · UK" },
];

function Work() {
  const { Card, Badge, Icon } = window.MDeckCoDesignSystem_4d3c0a;
  return (
    <section style={{ padding: "112px 40px" }}>
      <div style={{ display: "flex", alignItems: "flex-end", justifyContent: "space-between", marginBottom: 48 }}>
        <div>
          <div className="mdc-eyebrow" style={{ marginBottom: 14 }}>Selected work</div>
          <h2 style={{ fontFamily: "var(--font-display)", fontWeight: 300, fontSize: 42, letterSpacing: "-0.01em", color: "var(--text-primary)", margin: 0 }}>The outcome, not the theatre.</h2>
        </div>
        <span style={{ fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--text-tertiary)" }}>40+ engagements · since MMXIX</span>
      </div>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 20 }}>
        {WORK.map((w, i) => (
          <Card key={i} interactive padding="lg" style={{ minHeight: 260, display: "flex", flexDirection: "column", justifyContent: "space-between" }}>
            <Badge tone="neutral">{w.tag}</Badge>
            <div>
              <h3 style={{ fontFamily: "var(--font-display)", fontWeight: 400, fontSize: 24, lineHeight: 1.15, color: "var(--text-primary)", margin: "0 0 12px" }}>{w.title}</h3>
              <div style={{ display: "flex", alignItems: "center", gap: 8, fontFamily: "var(--font-mono)", fontSize: 11, letterSpacing: "0.08em", textTransform: "uppercase", color: "var(--text-tertiary)" }}>
                <Icon name="arrow-up-right" size={13} color="var(--ember-500)" />{w.meta}
              </div>
            </div>
          </Card>
        ))}
      </div>
    </section>
  );
}

const STATS = [["06", "Partners"], ["40+", "Engagements"], ["11", "Weeks, median"], ["2d", "First reply"]];

function Ethos() {
  return (
    <section style={{ padding: "112px 40px", background: "var(--ink-900)", borderTop: "1px solid var(--border-subtle)", borderBottom: "1px solid var(--border-subtle)" }}>
      <div style={{ maxWidth: 720, margin: "0 auto 72px", textAlign: "center" }}>
        <div className="mdc-eyebrow" style={{ marginBottom: 20 }}>The standard</div>
        <p style={{ fontFamily: "var(--font-display)", fontWeight: 300, fontSize: 34, lineHeight: 1.3, color: "var(--text-primary)", margin: 0 }}>
          Every engagement is led by someone who has <span style={{ color: "var(--ember-500)" }}>done it before</span>.
        </p>
      </div>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: 1, background: "var(--border-subtle)", borderRadius: "var(--radius-lg)", overflow: "hidden", border: "1px solid var(--border-subtle)" }}>
        {STATS.map(([n, l]) => (
          <div key={l} style={{ background: "var(--surface-base)", padding: "40px 28px", textAlign: "center" }}>
            <div style={{ fontFamily: "var(--font-display)", fontWeight: 300, fontSize: 52, color: "var(--text-primary)", lineHeight: 1 }}>{n}</div>
            <div style={{ fontFamily: "var(--font-mono)", fontSize: 11, letterSpacing: "0.14em", textTransform: "uppercase", color: "var(--text-tertiary)", marginTop: 12 }}>{l}</div>
          </div>
        ))}
      </div>
    </section>
  );
}

function AccessBand({ onApply }) {
  const { Button, Icon } = window.MDeckCoDesignSystem_4d3c0a;
  return (
    <section style={{ padding: "120px 40px", textAlign: "center" }}>
      <div className="mdc-eyebrow" style={{ marginBottom: 22 }}>However hard, we can help</div>
      <h2 style={{ fontFamily: "var(--font-display)", fontWeight: 300, fontSize: 56, letterSpacing: "-0.02em", color: "var(--text-primary)", margin: "0 0 20px" }}>Tell us what's hard.</h2>
      <p style={{ fontFamily: "var(--font-sans)", fontSize: 17, color: "var(--text-secondary)", maxWidth: 500, margin: "0 auto 36px", lineHeight: 1.6 }}>
        Every challenge gets a senior pair of eyes. Bring us the problem others found too hard. We reply within two business days.
      </p>
      <Button size="lg" variant="primary" onClick={onApply} iconRight={<Icon name="arrow-right" size={18} />}>Bring us your challenge</Button>
    </section>
  );
}

Object.assign(window, { Hero, Work, Ethos, AccessBand });
