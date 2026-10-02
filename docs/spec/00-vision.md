# 00. Vision and principles

## What mdeck is

mdeck turns a markdown file into a presentation that makes people say *"I want to present like
that."*

It is two promises at once, and both matter equally:

1. **Effortless.** If you can write a markdown file, you can present with mdeck. Nothing else to
   learn, nothing to install in your document, no slide-shaped thinking required.
2. **Breathtaking.** The result is not a tidy PDF of your notes. It is a presentation with motion,
   typography and atmosphere that looks better than what PowerPoint or Keynote produce, and that
   people remember.

mdeck is also an experiment: how far can we push presentation design when the *only* input is
a markdown file? The answer so far is "much further than expected". This specification exists so
that the experiment stays understandable and keeps its direction.

## Principles

Every requirement in this folder is judged against these. When two principles conflict, the
earlier one wins.

### P1. Markdown is the only prerequisite

- **VIS-01** MUST `keep`: A user who knows only ordinary markdown can present any markdown file
  with mdeck, with no mdeck-specific syntax in the file.
- **VIS-02** MUST `change`: Everything mdeck adds to markdown is optional. A deck without any mdeck
  syntax is a first-class deck, not a degraded one.
- **VIS-03** MUST `change`: A deck file stays a good markdown document. Rendered on GitHub, in an
  editor preview or by another markdown tool, an mdeck deck reads cleanly: mdeck syntax is either
  invisible or reads as natural markdown. (Today the `@` frontmatter keys are invalid YAML, which
  breaks this. See [07](07-authoring-language.md).)
- **VIS-04** MUST `keep`: Authors write for *meaning* (a heading, some points, a quote, a chart);
  mdeck decides *presentation*. When the author wants control, they can take it, but they never
  have to.

### P2. The wow effect is a requirement, not polish

- **VIS-05** MUST `keep`: Every rendered element (text, code, charts, transitions, scrolling) looks
  polished and professional at any resolution, in every built-in theme.
- **VIS-06** MUST `keep`: Motion is part of the design. State changes animate smoothly (reveal,
  slide change, scroll, overview zoom), and motion always has a purpose: it guides the eye, it
  never distracts from the content.
- **VIS-07** MUST `change`: The default experience shows that mdeck is **simple**. A plain
  markdown file opened with no options lands in a plain dark theme with a bright foreground, the
  `standard` design set, no engine animation and fade transitions. The wow effect is one setting
  away (`theme: ember`), never forced on someone who just wants to present a file. (Today the
  default is `light` with slide transitions.)
- **VIS-08** MUST `new`: Every built-in theme earns its place by being genuinely distinctive and
  showcase-quality. A theme that is merely "the same with other colours" is a variant of another
  theme, not a separate offering.
- **VIS-09** MUST `keep`: The audience never sees mdeck's machinery: no visible markup, no broken
  layouts, no content clipped without a cue, no debug output.

### P3. Simplicity over complexity

- **VIS-10** MUST `keep`: Fewer controls, fewer options, fewer edge cases. When in doubt, leave it
  out.
- **VIS-11** MUST `new`: Every concept has one name and one meaning, used consistently in the code,
  the docs and the UI (see [01](01-concepts.md)). If a concept is hard to name, it is probably two
  concepts or none.
- **VIS-12** MUST `new`: A feature that only works in one corner (one engine, one layout, one
  theme) is either made general or documented as belonging to that corner. It is never presented
  as a general feature.
- **VIS-13** MUST `new`: mdeck never silently drops content or silently ignores author intent. If
  something the author wrote will not be shown, or a setting has no effect, `mdeck --check` says so.

### P4. Any markdown file is presentable

- **VIS-14** MUST `keep`: Overflow handling, design inference and sensible defaults mean users do
  not tailor their markdown to the tool.
- **VIS-15** MUST `change`: Common markdown found in the wild (READMEs, notes, docs) presents
  without visible junk. Constructs mdeck cannot present are hidden gracefully or reported, never
  shown as raw syntax. (Today raw HTML, task lists, footnotes and reference links show literally.)

### P5. Extensible by anyone, privately

- **VIS-16** MUST `new`: mdeck can be extended with data (themes, designs, illustrations, fonts)
  and with code (engines, visuals, designs). See [08](08-extensibility.md).
- **VIS-17** MUST `new`: Extensions do not have to be shared. A company can build themes, engines
  or visuals that only it uses, without forking mdeck and without publishing anything.
- **VIS-18** MUST `new`: mdeck's own built-ins use the same extension interfaces that third parties
  use. There is no privileged internal path.
- **VIS-19** MUST `new`: The extension trust model is the owner's machine. Whoever installs mdeck
  and its extensions owns what runs, so extensions are trusted like any installed program and may
  do anything mdeck itself can do. mdeck is not a hosted service and does not sandbox its owner.
- **VIS-20** MUST `new`: Opening a deck never installs or fetches code. A deck can only use
  extensions the user has already installed. A markdown file received from someone else is as
  safe to open as any document.

### P6. What you see is what you export

- **VIS-21** MUST `keep`: Export (PNG, PDF) shows exactly what the presenting window shows, at any
  output size, reproducibly.

### P7. Presenting never depends on the network

- **VIS-22** MUST `keep`: Presenting and exporting never call an AI service or the network.
  Generated assets are produced ahead of time, stored next to the deck and versioned with it.

## Non-goals

mdeck deliberately does **not**:

- **NG-01** offer a WYSIWYG editor or drag-and-drop layout. The markdown file is the source of
  truth; your editor is the editor.
- **NG-02** offer pixel positioning of elements. Authors choose *what* and, at most, *which design*;
  never coordinates. (Diagram grid positions are the one exception, because a diagram's topology
  is content.)
- **NG-03** import or export PowerPoint/Keynote files as editable documents.
- **NG-04** run in a browser or as a hosted service. mdeck is a native application on the user's
  machine. (Export produces files that can be shared anywhere.)
- **NG-05** support arbitrary HTML, CSS or JavaScript inside decks.
- **NG-06** sandbox or police installed extensions (see VIS-19).
- **NG-07** require AI. Every AI feature is optional, and a deck without generated assets is
  complete.

## Who uses mdeck

| Persona | Needs |
|---|---|
| **Author** | Writes markdown, presents it. Should never need to read this specification. |
| **Power author** | Wants control over a specific slide: choose a design, add a chart, set a picture. Uses the documented authoring language ([07](07-authoring-language.md)). |
| **AI agent** | Writes decks on someone's behalf. Needs a precise, complete, machine-readable description of the language and of the slide designs (`mdeck spec`). |
| **Theme designer** | Creates a look and restyles slide designs with data files only. |
| **Extension developer** | Writes code (an engine, a visual kind, a design set), possibly privately for a company. |
| **mdeck maintainer** | Keeps the core small and the concepts clean. |
