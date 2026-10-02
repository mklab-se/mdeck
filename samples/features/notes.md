---
title: "Speaker Notes Demo"
author: "MDeck"
theme: dark
transition: slide
---

# Speaker Notes

Markdown notes in a fenced block

```@notes
Welcome the audience and explain that this presentation demonstrates
MDeck's speaker notes. Notes are never shown on the slides; they print
with `mdeck export --format pdf --notes`.
```

## Why Speaker Notes?

- Help presenters understand slide intent
- Provide delivery guidance and talking points
- Essential for AI-generated presentations
+ Enable collaboration between slide creator and presenter

```@notes
Walk through each bullet point. The key insight is the last one:
when MDeck creates a presentation with AI, every slide gets detailed
notes explaining **what** to say and **how** to present it.

Pause after revealing the last bullet, and let it sink in.
```

## How to Add Notes

Put them in a fenced block tagged `@notes`, anywhere in the slide:

````markdown
# My Slide

- Content here

```@notes
Your notes go here. They are **markdown**.
```
````

```@notes
Show the example and point out that the notes block is an ordinary fenced
block: on GitHub it reads as a code block under the slide.

## Notes are markdown

- headings, **emphasis** and lists
- code, with a longer outer fence
- a `---` or a heading in the notes never starts a new slide
```

## Notes for AI-Generated Decks

When MDeck generates a presentation, every slide includes notes:

- The core message of the slide
- Suggested talking points are included
+ The presenter can deliver effectively even without reading the source material

```@notes
This is the big selling point. Emphasize that a presenter who
has never seen the original content can still deliver the talk
effectively, because the AI-generated notes explain everything.

Ask the audience: "How many times have you been handed a slide
deck with no context about what to say?"
```

## Best Practices

- Keep notes concise but complete
- Explain the *why* behind each slide
- Note any pauses or emphasis points
- Include timing hints for long presentations[^timing]

```@notes
Wrap up by encouraging the audience to try writing notes in their
next presentation. Even brief notes make a big difference.
```

[^timing]: A footnote's text joins the slide's notes, and its marker is hidden on the slide.
