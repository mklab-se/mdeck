# MDeck Gallery

A visual showcase of what you can create with MDeck — layouts, charts, diagrams, and more. Everything below was generated from a single markdown file using `mdeck export`.

> **Source:** [`samples/gallery.md`](samples/gallery.md)
>
> **Try it yourself:** `mdeck samples/gallery.md`

---

## Ember

The Ember theme puts a living field of glowing particles behind every slide.
These stills are from `samples/ember.md`,
`samples/ember/illustrations.md` and `samples/ember/visualizations.md`; the
field moves in the presentation.

### Title

The particles gather into a constellation on the flanks of the title.

<img src="media/gallery/ember-title.png" width="720">

### Bullets

One cluster per item, each lighting with its reveal step, beside the copy column.

<img src="media/gallery/ember-bullets.png" width="720">

### A diagram

On charts and diagrams the field serves the content: here runners travel the
routed edges in their direction while dust keeps to the margins.

<img src="media/gallery/ember-diagram.png" width="720">

### An illustration

`@illustration: robot` under the slide's heading, and the particles settle into a
point cloud beside the copy: a hint of the thing, never a picture of it. Thirty-eight
are built in; `mdeck ai point-cloud` makes more from a description.

<img src="media/gallery/ember-illustration.png" width="720">

---

## Engines

The same slides on other engines (spec section 9.6); any deck switches with
`@engine` in its frontmatter. These stills are from `samples/engines/`.

### LED wall

The `led` engine and its `marquee` theme: a wall of RGB LEDs behind every
slide. Title slides get a marquee border of chasing bulbs, with the
illustration dim behind the copy.

<img src="media/gallery/marquee-title.jpg" width="720">

An illustration powers on from its centre, LED by LED, and shimmers between
the theme's colours. Brightness follows the point cloud's density, so the
filament stays brighter than the glass.

<img src="media/gallery/marquee-illustration.jpg" width="720">

On a chart, a peak marker floats over every bar like a level meter's.

<img src="media/gallery/marquee-chart.jpg" width="720">

The countdown, lit digit by digit.

<img src="media/gallery/marquee-countdown.jpg" width="720">

### Departure board

The `splitflap` engine and its `departures` theme: every slide on a
split-flap board. A table becomes a timetable.

<img src="media/gallery/departures-schedule.jpg" width="720">

Going to the next slide, every flap turns through its wheel until it shows
its new character.

<img src="media/gallery/departures-turning.jpg" width="720">

Progress bars in solid flaps, and a slide's first image in a panel on the
board.

<img src="media/gallery/departures-progress.jpg" width="720">

<img src="media/gallery/departures-picture.jpg" width="720">

The countdown in solid flaps.

<img src="media/gallery/departures-countdown.jpg" width="720">

### Laser

The `laser` engine and its `etch` theme: a beam from in front of the screen
etches each illustration. Mid-etch, the fresh marks are still hot:

<img src="media/gallery/etch-beam.jpg" width="720">

and the finished engraving:

<img src="media/gallery/etch-illustration.jpg" width="720">

### Falling blocks

The `blocks` engine and its `stack` theme: illustrations built from falling
blocks. The pieces drop bottom row first:

<img src="media/gallery/stack-falling.jpg" width="720">

and settle into the picture:

<img src="media/gallery/stack-illustration.jpg" width="720">

On a title slide the stack stands dim behind the copy.

<img src="media/gallery/stack-title.jpg" width="720">

### Blueprint

The first art engine, `blueprint`, and its theme: each slide gets line art
generated for it (`mdeck ai pictures`), inked onto a Prussian blue sheet.
Mid-drawing, construction lines run ahead of the ink under the drafting
machine's crosshair:

<img src="media/gallery/blueprint-drawing.jpg" width="720">

and the finished sheet, with its dimension lines and title block:

<img src="media/gallery/blueprint-sheet.jpg" width="720">

On a title slide the drawing stands dim behind the copy.

<img src="media/gallery/blueprint-title.jpg" width="720">

### Sketchbook

The `sketch` engine and its `sketchbook` theme: each slide gets a graphite
and ink drawing in the MKLab house style, drawn in by a pencil. The outlines
come first, then the shading sweeps in stroke by stroke:

<img src="media/gallery/sketch-drawing.jpg" width="720">

and the finished page:

<img src="media/gallery/sketch-page.jpg" width="720">

The title page, its drawing faint behind the copy.

<img src="media/gallery/sketch-title.jpg" width="720">

### Chalkboard

The `chalkboard` engine and theme: a slate in a wooden frame, generated line
art drawn in chalk with a stick of chalk at the point:

<img src="media/gallery/chalkboard-drawing.jpg" width="720">

and the finished board:

<img src="media/gallery/chalkboard-board.jpg" width="720">

The title, the drawing faint behind it.

<img src="media/gallery/chalkboard-title.jpg" width="720">

### Watercolour

The `watercolour` engine and theme: each slide's generated watercolour
blooms onto cold-press paper, a pale wash first and then the colour
spreading from where the paint is heaviest:

<img src="media/gallery/watercolour-blooming.jpg" width="720">

and dry:

<img src="media/gallery/watercolour-page.jpg" width="720">

<img src="media/gallery/watercolour-title.jpg" width="720">

### Darkroom

The `darkroom` engine and theme: each slide's generated photograph develops
as a print under the red safelight:

<img src="media/gallery/darkroom-developing.jpg" width="720">

then the white light comes on:

<img src="media/gallery/darkroom-print.jpg" width="720">

<img src="media/gallery/darkroom-title.jpg" width="720">

### Thermal

The `thermal` engine and theme: the deck seen through a thermal instrument.
Title and section headings form in heat and settle with a faint contour halo:

<img src="media/gallery/thermal-title.jpg" width="720">

A `@thermal` block finds the problem: a lens over the ordinary photo,

<img src="media/gallery/thermal-lens.jpg" width="720">

a threshold that colours only the hottest metal,

<img src="media/gallery/thermal-threshold.jpg" width="720">

an illustration as a heat signature,

<img src="media/gallery/thermal-signature.jpg" width="720">

and before and after on one temperature scale (the pictures are synthetic
examples):

<img src="media/gallery/thermal-compare.jpg" width="720">

---

## Themes

Eighteen built-in themes, all written as theme files, and your own in a few lines
of YAML (spec section 9.4). These stills are from `samples/themes/`.

### The four seasons

`spring` and `summer` on the plain engine, `autumn` and `winter` on the
particles engine in their own colours. The same slide in each:

<img src="media/gallery/theme-spring.png" width="360"> <img src="media/gallery/theme-summer.png" width="360">
<img src="media/gallery/theme-autumn.png" width="360"> <img src="media/gallery/theme-winter.png" width="360">

<img src="media/gallery/theme-winter-title.png" width="720">

### From a design system

`mdeck ai theme mdeck-co --from samples/design-systems/mdeck-co` read a Claude
Design export and wrote this theme: the brand's palette, type and particle
field, and its logo quiet in the corner.

<img src="media/gallery/theme-from-design-system.png" width="720">

---

## Layouts

MDeck automatically infers the right layout from your content structure. No configuration needed.

### Title Slide

A heading with a short subtitle — detected automatically.

<img src="media/gallery/slide-01.png" width="720">

### Section Divider

A lone heading becomes a section divider between topics.

<img src="media/gallery/slide-03.png" width="720">

### Bullet Points

A heading followed by a list renders as a bullet slide.

<img src="media/gallery/slide-04.png" width="720">

### Code Highlight

Fenced code blocks get automatic syntax highlighting with language detection.

<img src="media/gallery/slide-05.png" width="720">

### Blockquote

Blockquotes with attribution render as elegant quote slides.

<img src="media/gallery/slide-06.png" width="720">

### Data Table

Standard markdown tables render with clean formatting.

<img src="media/gallery/slide-07.png" width="720">

### Bullet Slide with Image

Add a single image to a bullet slide and it automatically renders as a split layout — content on the left, image on the right.

<img src="media/gallery/slide-27.png" width="720">

### Full-Screen Image

A slide with just an image fills the entire slide area.

<img src="media/gallery/slide-28.png" width="720">

### Two-Column Layout

Split content into two columns using the `+++` separator.

<img src="media/gallery/slide-29.png" width="720">

### Math

LaTeX between dollar signs: `$...$` inline on the text baseline, `$$...$$` centred on its own line.

<img src="media/gallery/slide-30.png" width="720">

---

## Diagrams

Architecture and flow diagrams rendered from simple text descriptions. Supports grid positioning, icons, labeled arrows, and multiple arrow types.

### Architecture Diagram

Grid-positioned nodes with icons and labeled connections.

<img src="media/gallery/slide-08.png" width="720">

### Flow Diagram

Auto-layout pipeline showing process flow.

<img src="media/gallery/slide-09.png" width="720">

---

## Charts & Visualizations

All visualizations are written as fenced code blocks with `@` language tags. Data is specified as simple `- Label: value` lines.

### Bar Chart

Vertical bar chart with axis labels.

<img src="media/gallery/slide-10.png" width="720">

### Horizontal Bar Chart

<img src="media/gallery/slide-11.png" width="720">

### Line Chart

Multi-series line chart with shared X-axis categories.

<img src="media/gallery/slide-12.png" width="720">

### Pie Chart

Proportional segments with automatic percentage labels.

<img src="media/gallery/slide-13.png" width="720">

### Donut Chart

Pie chart variant with a center label.

<img src="media/gallery/slide-14.png" width="720">

### Stacked Bar Chart

Multiple series stacked per category.

<img src="media/gallery/slide-15.png" width="720">

### Scatter Plot

2D scatter plot with labeled data points and axis descriptions.

<img src="media/gallery/slide-16.png" width="720">

### Radar Chart

Multi-axis comparison between data series.

<img src="media/gallery/slide-17.png" width="720">

### Funnel Chart

Progressive narrowing stages — great for conversion metrics.

<img src="media/gallery/slide-18.png" width="720">

### KPI Dashboard

Key metrics with trend indicators.

<img src="media/gallery/slide-19.png" width="720">

### Progress Bars

Horizontal progress indicators for project status.

<img src="media/gallery/slide-20.png" width="720">

### Timeline

Chronological events along a visual timeline.

<img src="media/gallery/slide-21.png" width="720">

### Word Cloud

Words sized proportionally to importance with automatic layout.

<img src="media/gallery/slide-22.png" width="720">

### Venn Diagram

Set intersections with automatic overlap detection.

<img src="media/gallery/slide-23.png" width="720">

### Organization Chart

Hierarchical tree with parent-child relationships.

<img src="media/gallery/slide-24.png" width="720">

### Gantt Chart

Project timeline with task dependencies and automatic time scaling.

<img src="media/gallery/slide-25.png" width="720">

### Git Graph

Branches as lanes, commits as dots, forks and merges as S-curves, with tags and progressive reveal.

<img src="media/gallery/slide-26.png" width="720">

### Flower

A platform in the middle and the teams around it, each petal flowing in and back out.

<img src="media/gallery/slide-31.png" width="720">

### Artifact Flow

Artifacts from the teams that produce them, through shared infrastructure, to the teams that consume them.

<img src="media/gallery/slide-32.png" width="720">

---

## AI-Generated Images

MDeck integrates with AI image generation. Add `![prompt](generate:)` to your slides, then run `mdeck ai images` to create images automatically.

The images below were generated using `mdeck ai images` with the style: *"Cinematic landscape photography style. Vivid colors, dramatic lighting, sweeping vistas."*

<img src="media/gallery/africa.png" width="720">

*African savanna at golden hour — generated from a text prompt and automatically placed in the slide.*

<img src="media/gallery/antarctica.png" width="720">

*Antarctic ice shelf with aurora — another AI-generated image used in the [continents presentation](samples/continents.md).*

---

## Getting Started

```bash
# Install
brew install mklab-se/tap/mdeck

# Present any markdown file
mdeck your-talk.md

# Export slides as PNG
mdeck export your-talk.md

# See the full format specification
mdeck spec
```

See the [README](README.md) for full installation and usage instructions.
