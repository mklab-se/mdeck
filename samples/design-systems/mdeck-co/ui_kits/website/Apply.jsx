/* MDeck Co website: the gated access / request flow */

function Apply({ onBack, onSubmit }) {
  const { Button, Input, Textarea, Select, Checkbox, Icon, Badge } = window.MDeckCoDesignSystem_4d3c0a;
  const [ok, setOk] = React.useState(false);
  return (
    <section style={{ minHeight: 720, display: "flex" }}>
      {/* Left rail: the "black card" panel */}
      <div style={{ width: 420, flexShrink: 0, position: "relative", overflow: "hidden", background: "var(--ink-900)", borderRight: "1px solid var(--border-subtle)", padding: "56px 48px", display: "flex", flexDirection: "column", justifyContent: "space-between" }}>
        <div style={{ position: "absolute", inset: 0, background: "radial-gradient(120% 80% at 20% 0%, rgba(255,77,28,0.10), transparent 60%)" }} />
        <div style={{ position: "relative" }}>
          <img src="../../assets/logos/mdeckco-white.svg" alt="MDeck Co" style={{ height: 40, opacity: 0.9, marginBottom: 48 }} />
          <div className="mdc-eyebrow" style={{ marginBottom: 18 }}>Bring us your challenge</div>
          <h1 style={{ fontFamily: "var(--font-display)", fontWeight: 300, fontSize: 44, lineHeight: 1.05, letterSpacing: "-0.02em", color: "var(--text-primary)", margin: "0 0 20px" }}>
            Tell us<br />what's hard.
          </h1>
          <p style={{ fontFamily: "var(--font-sans)", fontSize: 15, color: "var(--text-secondary)", lineHeight: 1.65, margin: 0, maxWidth: 300 }}>
            A partner reads every enquiry themselves and replies within two business days. No gatekeeping. Just the right senior people, fast.
          </p>
        </div>
        <div style={{ position: "relative", display: "flex", flexDirection: "column", gap: 14 }}>
          {["Read by a partner", "Two business days", "Discretion, always"].map((t) => (
            <div key={t} style={{ display: "flex", alignItems: "center", gap: 12, color: "var(--text-tertiary)", fontFamily: "var(--font-mono)", fontSize: 11, letterSpacing: "0.06em", textTransform: "uppercase" }}>
              <Icon name="check" size={14} color="var(--ember-500)" />{t}
            </div>
          ))}
        </div>
      </div>

      {/* Right: the form / confirmation */}
      <div style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center", padding: "56px 40px" }}>
        {ok ? (
          <div style={{ textAlign: "center", maxWidth: 400 }}>
            <div style={{ width: 64, height: 64, borderRadius: "50%", margin: "0 auto 28px", display: "flex", alignItems: "center", justifyContent: "center", background: "var(--accent-quiet)", boxShadow: "var(--glow-ember-md)" }}>
              <Icon name="check" size={28} color="var(--ember-500)" strokeWidth={2} />
            </div>
            <h2 style={{ fontFamily: "var(--font-display)", fontWeight: 300, fontSize: 36, color: "var(--text-primary)", margin: "0 0 14px" }}>We've got it.</h2>
            <p style={{ fontFamily: "var(--font-sans)", fontSize: 15, color: "var(--text-secondary)", lineHeight: 1.65, margin: "0 0 28px" }}>
              Your challenge is with a partner now. Expect a reply within two business days, whatever the problem.
            </p>
            <Button variant="ghost" onClick={onBack} iconLeft={<Icon name="arrow-left" size={16} />}>Back to site</Button>
          </div>
        ) : (
          <div style={{ width: "100%", maxWidth: 460 }}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 28 }}>
              <span style={{ fontFamily: "var(--font-mono)", fontSize: 11, letterSpacing: "0.14em", textTransform: "uppercase", color: "var(--text-tertiary)" }}>Enquiry · 01 / 01</span>
              <Badge tone="accent" dot>Private</Badge>
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: 18 }}>
              <div style={{ display: "flex", gap: 14 }}>
                <Input label="Name" placeholder="Full name" />
                <Input label="Firm" placeholder="Where you sit" />
              </div>
              <Input label="Work email" placeholder="you@firm.com" iconLeft={<Icon name="mail" size={16} />} />
              <Select label="What brings you" placeholder="Choose one" options={["A live decision", "An interim mandate", "A problem others passed on", "Something else"]} />
              <Textarea label="In one line" rows={3} placeholder="The challenge you're facing." />
              <Checkbox label="This one's time-sensitive" description="We'll fast-track your first reply." />
            </div>
            <div style={{ display: "flex", gap: 12, marginTop: 28 }}>
              <Button variant="ghost" onClick={onBack}>Cancel</Button>
              <Button variant="primary" fullWidth onClick={() => { setOk(true); onSubmit && onSubmit(); }} iconRight={<Icon name="arrow-right" size={16} />}>Send it over</Button>
            </div>
          </div>
        )}
      </div>
    </section>
  );
}

Object.assign(window, { Apply });
