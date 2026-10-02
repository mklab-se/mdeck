# mdeck gallery

Every picture on this page is a slide exported with `mdeck export` from a markdown file in
[`samples/`](samples/). Nothing was touched up afterwards: what you see is what the window shows.
Stills of motion (particles, LEDs, drawings that ink themselves in) are taken with
`mdeck export --at <seconds>`.

- [Designs](#designs): the thirteen kinds of slide, in both design sets
- [Engines](#engines): ten ways to bring a deck to life
- [Visuals](#visuals): charts, diagrams and thermal images from fenced blocks
- [Themes](#themes): colours, variants and your own
- [Presenter view](#presenter-view)

---

## Designs

mdeck recognises what each slide is from its content and gives it one of thirteen designs. A
theme decides how the designs look by picking a design set: **standard** (the classic centred
slide, here the default `dark` theme) or **editorial** (a magazine spread with an eyebrow, display
type and a stage for the picture, here `ember`). The markdown is the same in both columns.

Source: [`samples/features/designs.md`](samples/features/designs.md), exported with
`--theme dark` and `--theme ember`.

| Design | Standard | Editorial |
|---|---|---|
| `title`: an H1 and one short line | <img src="media/gallery/design-title-standard.jpg" width="400" alt="Title slide, standard"> | <img src="media/gallery/design-title-editorial.jpg" width="400" alt="Title slide, editorial"> |
| `section`: a lone heading, or a heading and a deeper one | <img src="media/gallery/design-section-standard.jpg" width="400" alt="Section slide, standard"> | <img src="media/gallery/design-section-editorial.jpg" width="400" alt="Section slide, editorial"> |
| `statement`: a heading and a sentence or two, said big | <img src="media/gallery/design-statement-standard.jpg" width="400" alt="Statement slide, standard"> | <img src="media/gallery/design-statement-editorial.jpg" width="400" alt="Statement slide, editorial"> |
| `points`: a heading and a list | <img src="media/gallery/design-points-standard.jpg" width="400" alt="Points slide, standard"> | <img src="media/gallery/design-points-editorial.jpg" width="400" alt="Points slide, editorial"> |
| `split`: text beside one image | <img src="media/gallery/design-split-standard.jpg" width="400" alt="Split slide, standard"> | <img src="media/gallery/design-split-editorial.jpg" width="400" alt="Split slide, editorial"> |
| `media`: one image, large, with a lead and a caption | <img src="media/gallery/design-media-standard.jpg" width="400" alt="Media slide, standard"> | <img src="media/gallery/design-media-editorial.jpg" width="400" alt="Media slide, editorial"> |
| `gallery`: two or more images, captioned from their alt text | <img src="media/gallery/design-gallery-standard.jpg" width="400" alt="Gallery slide, standard"> | <img src="media/gallery/design-gallery-editorial.jpg" width="400" alt="Gallery slide, editorial"> |
| `quote`: a quotation and its attribution | <img src="media/gallery/design-quote-standard.jpg" width="400" alt="Quote slide, standard"> | <img src="media/gallery/design-quote-editorial.jpg" width="400" alt="Quote slide, editorial"> |
| `code`: a code block with a heading and a line of context | <img src="media/gallery/design-code-standard.jpg" width="400" alt="Code slide, standard"> | <img src="media/gallery/design-code-editorial.jpg" width="400" alt="Code slide, editorial"> |
| `visual`: a chart or diagram | <img src="media/gallery/design-visual-standard.jpg" width="400" alt="Visual slide, standard"> | <img src="media/gallery/design-visual-editorial.jpg" width="400" alt="Visual slide, editorial"> |
| `columns`: side by side, split with `+++` | <img src="media/gallery/design-columns-standard.jpg" width="400" alt="Columns slide, standard"> | <img src="media/gallery/design-columns-editorial.jpg" width="400" alt="Columns slide, editorial"> |
| `table`: a heading and a table | <img src="media/gallery/design-table-standard.jpg" width="400" alt="Table slide, standard"> | <img src="media/gallery/design-table-editorial.jpg" width="400" alt="Table slide, editorial"> |
| `content`: anything else, in reading order | <img src="media/gallery/design-content-standard.jpg" width="400" alt="Content slide, standard"> | <img src="media/gallery/design-content-editorial.jpg" width="400" alt="Content slide, editorial"> |

### Arrangements

A theme can change any design's arrangement with a few lines of YAML. The deck-local `magazine`
theme next to [`designs.md`](samples/features/designs.md) uses the editorial set on a still
screen, with diamond bullets everywhere and a centred quote with marks instead of a bar.

| Points with `bullet: "◆"` | A quote, centred, with marks |
|---|---|
| <img src="media/gallery/arrangement-points.jpg" width="400" alt="A points slide with diamond bullets"> | <img src="media/gallery/arrangement-quote.jpg" width="400" alt="A centred quote with quote marks"> |

---

## Engines

An engine is what moves on the screen. Every built-in engine comes with a showcase theme; any deck
switches with one line (`theme: ember`), or try one without editing with
`mdeck talk.md --theme ember`.

### plain: `dark`, `light`, `nord`

The default. A still, clean page that shows that mdeck is simple.

<img src="media/gallery/engine-plain.jpg" width="720" alt="A points slide on the plain dark theme">

*Source: [`samples/showcase/launch.md`](samples/showcase/launch.md), theme `dark`.*

### particles: `ember` (variants `autumn`, `winter`)

A living field of glowing particles. A slide's `picture` (a point cloud such as `lightbulb` or
`gear`) gathers on the stage beside the copy.

| | |
|---|---|
| <img src="media/gallery/engine-particles.jpg" width="400" alt="Particles forming a light bulb beside a list"> | <img src="media/gallery/engine-particles-gear.jpg" width="400" alt="Particles forming a gear"> |

*Source: [`samples/showcase/launch.md`](samples/showcase/launch.md), theme `ember`, `--at 8`.*

### led: `marquee`

A wall of RGB LEDs. Pictures power on from their centre, title slides get a border of chasing
bulbs, and charts get peak markers.

| | | |
|---|---|---|
| <img src="media/gallery/engine-led-title.jpg" width="260" alt="LED title slide with a marquee border"> | <img src="media/gallery/engine-led.jpg" width="260" alt="An LED light bulb beside a list"> | <img src="media/gallery/engine-led-chart.jpg" width="260" alt="A bar chart with LED peak markers"> |

*Source: [`samples/engines/led.md`](samples/engines/led.md).*

### splitflap: `departures`

Every slide on a split-flap departure board, flap by flap. A board engine: it draws the whole
slide itself.

| | | |
|---|---|---|
| <img src="media/gallery/engine-splitflap-schedule.jpg" width="260" alt="A schedule table on a departure board"> | <img src="media/gallery/engine-splitflap-progress.jpg" width="260" alt="Progress bars made of flaps"> | <img src="media/gallery/engine-splitflap.jpg" width="260" alt="A departure board with a photograph"> |

*Source: [`samples/engines/splitflap.md`](samples/engines/splitflap.md).*

### blocks: `stack`

Pictures built from falling blocks, bottom row first, with a small bounce.

| | |
|---|---|
| <img src="media/gallery/engine-blocks-title.jpg" width="400" alt="A title over a rocket of blocks"> | <img src="media/gallery/engine-blocks.jpg" width="400" alt="A light bulb built from blocks"> |

*Source: [`samples/engines/blocks.md`](samples/engines/blocks.md).*

### thermal: `thermal`

A heat field: headings form in heat, pictures show as heat signatures, and `@thermal` images get
palettes, lenses, threshold reveals and measured spots.

| | |
|---|---|
| <img src="media/gallery/engine-thermal-title.jpg" width="400" alt="A title formed in heat"> | <img src="media/gallery/engine-thermal.jpg" width="400" alt="A thermal image of a cabinet with two spots"> |
| <img src="media/gallery/engine-thermal-signature.jpg" width="400" alt="A heat signature beside a list"> | <img src="media/gallery/engine-thermal-compare.jpg" width="400" alt="Two thermal images, before and after a repair"> |

*Source: [`samples/engines/thermal.md`](samples/engines/thermal.md).*

### line: `blueprint` (surface `sheet`) and `chalkboard` (surface `slate`)

Line art drawn stroke by stroke, construction lines first. Each slide's picture is made once with
`mdeck ai pictures` and kept next to the deck; without one, the slide's point cloud is drawn with
the same pen.

| | |
|---|---|
| <img src="media/gallery/engine-line-sheet-title.jpg" width="400" alt="A blueprint title sheet with a dim drawing"> | <img src="media/gallery/engine-line-sheet.jpg" width="400" alt="A blueprint drawing beside a list"> |
| <img src="media/gallery/engine-line-slate-title.jpg" width="400" alt="A chalkboard title with a lighthouse"> | <img src="media/gallery/engine-line-slate.jpg" width="400" alt="A chalk drawing of a lighthouse lens"> |
| <img src="media/gallery/engine-line-sheet-fallback.jpg" width="400" alt="A gear point cloud drawn with a technical pen"> | |

*Sources: [`samples/engines/blueprint.md`](samples/engines/blueprint.md),
[`samples/engines/chalkboard.md`](samples/engines/chalkboard.md).*

### sketch: `sketchbook`

Graphite drawings on paper, pencilled in as the slide opens: outlines first, then the shading.

| | |
|---|---|
| <img src="media/gallery/engine-sketch-title.jpg" width="400" alt="A sketchbook title page"> | <img src="media/gallery/engine-sketch.jpg" width="400" alt="A pencil drawing of a workshop beside a list"> |

*Source: [`samples/engines/sketch.md`](samples/engines/sketch.md).*

### watercolour: `watercolour`

Paintings that bloom onto the page, ink first, then the washes.

| | |
|---|---|
| <img src="media/gallery/engine-watercolour-title.jpg" width="400" alt="A watercolour title page"> | <img src="media/gallery/engine-watercolour.jpg" width="400" alt="A watercolour of a gardener resting"> |

*Source: [`samples/engines/watercolour.md`](samples/engines/watercolour.md).*

### darkroom: `darkroom`

Black-and-white photographs that develop under a safelight, shadows first.

| | | |
|---|---|---|
| <img src="media/gallery/engine-darkroom-title.jpg" width="260" alt="A darkroom title over a dim photograph"> | <img src="media/gallery/engine-darkroom-print.jpg" width="260" alt="A photograph of a print in a developer tray"> | <img src="media/gallery/engine-darkroom.jpg" width="260" alt="A photograph of a fishing boat"> |

*Source: [`samples/engines/darkroom.md`](samples/engines/darkroom.md).*

### Countdowns and endings

Engines with a countdown open the deck with 3, 2, 1, and some close it with an end act. Export a
moment with `mdeck export deck.md --moment countdown` (or `3`, `2`, `1`, `burst`, `end`).

| particles | led | splitflap | thermal |
|---|---|---|---|
| <img src="media/gallery/moment-countdown-particles.jpg" width="200" alt="A 3 made of particles"> | <img src="media/gallery/moment-countdown-led.jpg" width="200" alt="A 3 on an LED wall"> | <img src="media/gallery/moment-countdown-splitflap.jpg" width="200" alt="A 3 made of yellow flaps"> | <img src="media/gallery/moment-end-thermal.jpg" width="200" alt="THE END glowing in heat"> |

---

## Visuals

Charts and diagrams are fenced blocks: ```` ```@bar ````, ```` ```@architecture ```` and so on.
They take the theme's colours and animate in. These are on `ember`.

Source: [`samples/visualizations/all.md`](samples/visualizations/all.md) (one file per kind in
[`samples/visualizations/`](samples/visualizations/)).

| | | |
|---|---|---|
| <img src="media/gallery/visual-bar.jpg" width="260" alt="Bar chart"><br>`@bar` | <img src="media/gallery/visual-bar-horizontal.jpg" width="260" alt="Horizontal bar chart"><br>`@bar` with `orientation: horizontal` | <img src="media/gallery/visual-line.jpg" width="260" alt="Line chart"><br>`@line` |
| <img src="media/gallery/visual-pie.jpg" width="260" alt="Pie chart"><br>`@pie` | <img src="media/gallery/visual-donut.jpg" width="260" alt="Donut chart"><br>`@donut` | <img src="media/gallery/visual-scatter.jpg" width="260" alt="Scatter plot"><br>`@scatter` |
| <img src="media/gallery/visual-stackedbar.jpg" width="260" alt="Stacked bar chart"><br>`@stackedbar` | <img src="media/gallery/visual-funnel.jpg" width="260" alt="Funnel chart"><br>`@funnel` | <img src="media/gallery/visual-radar.jpg" width="260" alt="Radar chart"><br>`@radar` |
| <img src="media/gallery/visual-progress.jpg" width="260" alt="Progress bars"><br>`@progress` | <img src="media/gallery/visual-kpi.jpg" width="260" alt="KPI cards"><br>`@kpi` | <img src="media/gallery/visual-wordcloud.jpg" width="260" alt="Word cloud"><br>`@wordcloud` (on `dark`) |
| <img src="media/gallery/visual-timeline.jpg" width="260" alt="Timeline"><br>`@timeline` | <img src="media/gallery/visual-gantt.jpg" width="260" alt="Gantt chart"><br>`@gantt` | <img src="media/gallery/visual-gitgraph.jpg" width="260" alt="Git graph"><br>`@gitgraph` |
| <img src="media/gallery/visual-architecture.jpg" width="260" alt="Architecture diagram"><br>`@architecture` | <img src="media/gallery/visual-orgchart.jpg" width="260" alt="Org chart"><br>`@orgchart` | <img src="media/gallery/visual-venn.jpg" width="260" alt="Venn diagram"><br>`@venn` |
| <img src="media/gallery/visual-flower.jpg" width="260" alt="Flower diagram"><br>`@flower` | <img src="media/gallery/visual-artifactflow.jpg" width="260" alt="Artifact flow"><br>`@artifactflow` | <img src="media/gallery/visual-thermal.jpg" width="260" alt="Thermal image"><br>`@thermal` (on `thermal`) |

---

## Themes

A theme is colours, fonts, a design set and an engine, in one YAML file. Variants recolour a theme
and are listed after it (`mdeck theme list`, `Shift+T` while presenting). These slides are from
[`samples/showcase/launch.md`](samples/showcase/launch.md), exported with `--theme <name>`.

| | |
|---|---|
| <img src="media/gallery/theme-light.jpg" width="400" alt="An architecture diagram on the light theme"><br>`light` | <img src="media/gallery/theme-nord.jpg" width="400" alt="A points slide on the nord theme"><br>`nord` |
| <img src="media/gallery/theme-spring.jpg" width="400" alt="A stacked bar chart on the spring theme"><br>`spring`, a variant of `light` | <img src="media/gallery/theme-summer.jpg" width="400" alt="A stacked bar chart on the summer theme"><br>`summer`, a variant of `light` |
| <img src="media/gallery/theme-autumn.jpg" width="400" alt="A particle light bulb on the autumn theme"><br>`autumn`, a variant of `ember` | <img src="media/gallery/theme-winter.jpg" width="400" alt="KPI cards on the winter theme"><br>`winter`, a variant of `ember` |

### Your own

`mdeck theme new acme` writes a commented starter, and `mdeck ai theme acme --from <folder>`
converts a design system. This one was converted from
[`samples/design-systems/mdeck-co`](samples/design-systems/mdeck-co); the deck is
[`samples/themes/custom-theme.md`](samples/themes/custom-theme.md).

<img src="media/gallery/theme-from-design-system.jpg" width="720" alt="A slide in a theme converted from a design system">

---

## Presenter view

`V` (or `mdeck talk.md --presenter`) opens the presenter view on a second display: the current
slide, the next slide or step, your notes rendered as markdown, and the time.

<img src="media/showcase/presenter.jpg" width="720" alt="The presenter view: current slide, next slide, notes and timer">

*Source: [`samples/features/notes.md`](samples/features/notes.md).*
