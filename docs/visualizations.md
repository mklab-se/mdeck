<p align="center"><a href="../README.md">MDeck</a> &middot; <a href="tutorial.md">Tutorial</a> &middot; <a href="install.md">Install</a> &middot; <a href="writing-slides.md">Writing slides</a> &middot; <a href="visualizations.md">Visualizations</a> &middot; <a href="themes.md">Themes</a> &middot; <a href="engines.md">Engines</a> &middot; <a href="presenting.md">Presenting</a> &middot; <a href="export.md">Export</a> &middot; <a href="ai.md">AI</a> &middot; <a href="commands.md">Commands</a></p>

# Visualizations

Seventeen charts and diagrams from plain text. Every one animates in and supports
step-by-step reveal with the same `+` markers as lists. See all of them in the
[Gallery](../GALLERY.md).

## Charts

Fenced code blocks with an `@` tag become charts:

| Type | Tag | Example line |
|------|-----|--------------|
| Bar chart | `@barchart` | `- Python: 48` |
| Line chart | `@linechart` | `- Revenue: 100, 150, 200` |
| Pie chart | `@piechart` | `- Frontend: 35%` |
| Donut chart | `@donutchart` | `- Complete: 78` |
| Stacked bar | `@stackedbar` | `- Product A: 40, 45, 50` |
| Scatter plot | `@scatter` | `- Alice: 80, 90` |
| Radar chart | `@radar` | `- Speed: 9, 7, 5, 3` |
| Funnel | `@funnel` | `- Visitors: 10000` |
| KPI cards | `@kpi` | `- Revenue: $4.2M (trend: +12%)` |
| Progress bars | `@progress` | `- Design: 100%` |
| Timeline | `@timeline` | `- 2024: Project launch` |
| Word cloud | `@wordcloud` | `- AI (size: 50)` |
| Venn diagram | `@venn` | `- Design & Business: Product` |
| Org chart | `@orgchart` | `- CEO -> CTO` |
| Gantt chart | `@gantt` | `- Design: 8d, after Research` |
| Git graph | `@gitgraph` | `- branch main -> develop` |
| Architecture | `@architecture` | `- Client -> Server: requests` |

Values may carry units and separators (`$4,200`, `12%`, `40 users`). Charts
pick round axis limits, size their labels to fit, and share one colour palette
per theme. Options such as `# x-label:`, `# orientation: horizontal`, or
`# axes:` go on comment lines inside the block; the
[format spec](../crates/mdeck/doc/mdeck-spec.md) lists them all.

## Architecture diagrams

```markdown
​```@architecture
- Browser   (icon: browser,  pos: 1,1)
- API       (icon: api,      pos: 2,1)
- Database  (icon: database, pos: 2,2)

- Browser -> API: requests
- API -> Database: queries
​```
```

Grid or automatic placement, 20+ built-in icons, five arrow types
(`->`, `<-`, `<->`, `--`, `-->`), colour-coded labels, and A* routed edges
that avoid nodes and each other. Node icons can also be AI-generated.
