---
title: Introducing mdeck
author: The mdeck team
theme: ember
slide-level: 2
---

# Introducing mdeck
<!-- picture: rocket -->

Markdown in, a presentation people remember out

```@notes
Welcome. Everything you are about to see is **one markdown file**:
`samples/introducing-mdeck.md`. Press `V` to open the presenter view
with these notes, the next slide and a timer.
```

## A deck is a markdown file

Write the talk in any editor. mdeck reads the structure and does the design.

```@notes
This slide has no settings at all. A heading with a short paragraph is
recognised as a **statement**, so the line is set large.
```

## Why markdown

+ You already know it
+ It lives in git, next to your code
+ It reads well on GitHub, in any editor, in any preview
+ Nothing locks you in

```@notes
Each `+` item is a step: press Space to reveal the next one.
Items written with `-` show at once.
```

## Sixty seconds to your first talk

```bash
brew install mklab-se/tap/mdeck     # or: cargo install mdeck
mdeck talk.md                       # present it
mdeck export talk.md --format pdf   # share it
```

## How a file becomes slides

- A heading at the slide level starts a new slide
- Three dashes on a line of their own force a break
- Each slide gets a design from what is on it
- Settings, when you want them, hide in an HTML comment

## Thirteen designs, chosen for you

| You write | mdeck shows |
|---|---|
| `#` with one short line | title |
| A lone heading | section |
| A heading and a sentence | statement |
| A heading and a list | points |
| An image beside text | split |
| A quote with an attribution | quote |
| A fence of chart data | visual |
| Two halves split by `+++` | columns |

```@notes
`mdeck --check -v talk.md` prints the design of every slide and the
rule that picked it. The other designs: media, gallery, code, table
and content.
```

## Platform 4

![The evening train](showcase/station.jpg)

- Night train to Paris
- Departs 18:42
- Boarding now

---

> Any markdown file should be presentable.
>
> mdeck design principle

## Before and after

**Before**

- Slides in a proprietary file
- An hour of nudging boxes
- A PDF nobody can diff

+++

**After**

- One markdown file in git
- Design picked from the content
- Pull requests for your talk

# Charts and diagrams from text

## Where the time goes

```@bar
x-label: Hours per week
+ Status meetings: 6
+ Updating slides: 4
+ Writing the talk: 2
```

```@notes
Chart items written with `+` are steps too: each bar grows in on its
own press.
```

## How it fits together

```@architecture
- Markdown  (icon: storage,   pos: 1,2)
- Parser    (icon: function,  pos: 2,2)
- Designs   (icon: container, pos: 3,1)
- Theme     (icon: browser,   pos: 3,3)
- Window    (icon: monitor,   pos: 4,2)

- Markdown -> Parser: reads
- Parser -> Designs: slides
- Designs -> Window: arranges
- Theme -> Window: look
```

## Launch week

```@kpi
- Sign-ups: 12.4K (trend: +38%)
- Activation: 64% (trend: +9%)
- Churn: 1.8% (trend: -0.6%)
```

## Math, as you would write it

$$\hat{f}(\xi) = \int_{-\infty}^{\infty} f(x)\, e^{-2\pi i x \xi}\, dx$$

Inline too: $E = mc^2$, and $5 stays a price.

## Pictures
<!-- picture: lightbulb -->

- `picture: lightbulb` puts a point cloud on the stage
- The engine draws it: particles here, LEDs or blocks elsewhere
- Thirty-eight are built in, and `mdeck ai point-cloud` makes more

# Make it yours

## One line changes the look

| `theme:` | What you get |
|---|---|
| `dark` | The default: plain, bright, still |
| `ember` | A living field of glowing particles |
| `marquee` | A wall of RGB LEDs |
| `departures` | Every slide on a split-flap board |
| `stack` | Pictures built from falling blocks |
| `thermal` | Headings that form in heat |
| `blueprint`, `chalkboard` | Line art on a sheet or a slate |
| `sketchbook`, `watercolour`, `darkroom` | Drawings, paintings, prints |

```@notes
`Shift+T` cycles the themes live, so you can try them on this deck now.
```

## Presenting

- `V` opens the presenter view: notes, next slide, timer
- `G` shows every slide; type a number and Enter to jump
- Draw with the mouse; `B` blacks out the screen
- Save the file and the slide on screen updates

## Share it

```bash
mdeck export talk.md                          # a PNG per slide
mdeck export talk.md --format pdf --notes     # a PDF with notes pages
```

Export shows exactly what the window shows, at any size.

## Extend it, privately

- Themes, designs, point clouds and fonts travel as **packs**
- `mdeck sdk new engine glow` starts an engine in Rust
- `mdeck build --with ./glow` builds your own mdeck with it
- Nothing has to be published

# Start presenting

`mdeck your-talk.md`
