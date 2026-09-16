---
name: fix-issue
description: Use when Kristofer asks to look at, evaluate, triage, reproduce, answer, fix or close a GitHub issue on mklab-se/mdeck (an issue number like #12, an issue URL, or "the bug report about ..."), including feature requests and follow-ups on already answered issues.
argument-hint: "<issue-number> [evaluate|fix]"
---

# Handle a reported issue

Someone took the time to report a problem. The reply they get is the product's face:
it must be true (reproduced, not guessed), it must show its evidence, and it must thank
them. Every claim in a comment is one you have verified in this session.

What to do depends on what Kristofer asked for:

| Asked | Scope |
|---|---|
| "evaluate", "look at", "triage" | Steps 1-4, then report to Kristofer. No code changes. |
| "fix", "handle", "close" | Steps 1-8. If the issue turns out not to be doable (Step 2), stop after Step 4 and say why. |

If both words appear ("evaluate it and handle it"), the wider one wins: evaluate, then fix.

## Step 1: Read everything

```bash
gh issue view <n> --json title,author,body,labels,comments,state
```

- Note the reporter's `@handle`, their mdeck version and platform, and every attachment
  (images under `user-attachments`; fetch them with `curl -sL` and Read the file).
- If they did not give a version, assume the latest release (`git tag --sort=-v:refname | head -1`)
  and say so in your reply.
- Check whether main already fixed it since their version: `git log v<their>..HEAD --oneline`
  and `CHANGELOG.md` `[Unreleased]`.
- Read the earlier comments: an issue with a "Fixed in vX" reply and a new comment is a
  follow-up, and the question is whether the fix missed their case.

## Step 2: Classify

| It is | Then |
|---|---|
| A bug you can reproduce (Step 3) and fix in under a day without a product decision | Doable. Label `bug`, fix it (Steps 5-8). |
| A feature request, or a bug whose fix changes behaviour Kristofer must decide on | Not yours to decide. Add an entry to `BACKLOG.md` (size, recommendation, decision needed) and reply that it is recorded, label `enhancement`. |
| Already fixed on main or in a release | Reply naming the version, close as completed. |
| Not a bug: works as documented | Reply with the doc pointer and a working example, close as not planned. |
| A duplicate | Reply linking the original, label `duplicate`, close as not planned. |
| Not reproducible from what was given | Ask (Step 4). Do not fix a guess. |

Labels: `gh issue edit <n> --add-label bug|enhancement|question|duplicate`. Use the ones
that exist (`gh label list`).

## Step 3: Reproduce a bug before believing it

A report describes a symptom; the cause is often elsewhere (a "circled numbers" report
turned out to be every symbol glyph in every theme). Reproduce with the smallest deck that
shows it, in the scratchpad, at the resolution users see:

```bash
cargo run -q -p mdeck -- <deck>.md --check                       # parser and structure warnings
cargo run -q -p mdeck -- export <deck>.md --slide 2 --output-dir <dir>   # add --debug for reveal steps
cargo run -q -p mdeck -- <deck>.md                                # runtime incidents; logs under ~/Library/Application Support/mdeck/logs/
```

Read the exported PNG. Keep the failing export: it is the "before" picture for Step 8.
Then widen: other themes, other layouts, the neighbouring visualization types, the
bundled `samples/` for the same feature. Write down exactly what reproduces and what does not.

Reproduced means you saw it yourself, not that the code looks like it could do that.
A detail that differs from the report (they saw a warning, you see silence) does not send
you to Step 4 when the substance reproduces; note the difference for the comment (Step 8).

## Step 4: Ask when you cannot reproduce

If the report lacks what you need, ask for it now instead of fixing what you imagine.
Post one comment, label `question`, leave the issue open, and stop:

- Thank them and say what you tried and that it rendered correctly for you (name the
  version you tested).
- Ask for the specific things that would let you reproduce: the markdown block or a minimal
  deck, a screenshot, `mdeck version`, the OS. Ask only for what you need.
- Say values can be anonymised if the deck is private.
- Include your own render of the closest case (upload it as in Step 8) so they can compare.

An issue where you found *another* bug while looking is still a question to the reporter.
The other bug gets its own issue (`gh issue create --label bug`, with the reproduction and
a screenshot) when it is reproducible and needs no product decision, and a `BACKLOG.md`
entry otherwise. Never a fix under their number.

## Step 5: Fix it properly

Fixes land on main directly; there is no PR review step. Do the full job the project
requires (CLAUDE.md, Quality Requirements): regression test that fails before and passes
after (when the test targets new code and cannot compile against the old, the Step 3
export or transcript is the "before" evidence), a sample deck or slide under `samples/` that shows the case, `CHANGELOG.md`
`[Unreleased]` entry linking the issue, and the spec/README/reference docs when behaviour
or syntax changed. Check that the fix did not move anything on the existing samples for
the same feature.

## Step 6: Verify like the reporter will

- Re-export the reproduction deck; Read the PNG; keep it as the "after" picture.
- Export is the check for anything rendered. Run the app (on the reproduction deck and the
  relevant `samples/` file, then read the incident log) when the change touches `app/`:
  input, transitions, scrolling, the Ember field. It needs a display and a hand on the
  keyboard; if you cannot run it, say so in the report instead of skipping silently.
- Run the full check: `cargo fmt --all -- --check && cargo clippy --workspace -- -D warnings && cargo test --workspace`.

## Step 7: Commit and push

Subject: `<area>: <what changed> (#<n>)`, body explaining cause and fix, standard trailers.
Never write `Fixes #n` / `Closes #n`: GitHub would close the issue the moment main is
pushed, before the reporter sees the evidence. Closing happens in Step 8, by hand.

Do not release. Kristofer runs `/release`; your comment says which state the fix is in.

## Step 8: Report on the issue, then close

Every closing comment has these parts, in this order:

1. Thanks, addressed to `@handle`, naming what in their report helped (the precise code
   points, the screenshot, the exact expected behaviour).
2. What was actually wrong, in one or two sentences, including when it was wider or
   different from what they described.
3. What changed and where it is: "Fixed in v1.2.2" if released, otherwise "Fixed on main
   in `<sha>`; it will be in the next release". Name the sample that covers their case.
4. Evidence: for anything visible, a before and an after screenshot; for CLI behaviour,
   the terminal transcript in a code block. The screenshots go in the comment itself:

   ```bash
   .claude/skills/fix-issue/upload-screenshot.sh <n> <dir>/before.png "before"   # prints the ![..](..) line
   .claude/skills/fix-issue/upload-screenshot.sh <n> <dir>/after.png "after"
   ```

   Rename exports to `before.png` / `after.png` (or `after-dark.png` etc.) first; the
   file name becomes the path. Put the two image lines under a **Before** / **After** pair
   of bold labels. The script creates the `issue-screenshots` branch on first use and
   prints the raw URL; `curl -sI` it if in doubt.
5. An invitation to reply on the issue if their case still looks wrong. Do not tell them
   to reopen; reporters usually cannot.

Then:

```bash
gh issue comment <n> --body-file <comment.md>
gh issue close <n> --reason completed        # or --reason "not planned"
```

Post the comment first, look at it (`gh issue view <n> --comments`) to make sure the
images render, then close.

## Red flags

| You are thinking | Do instead |
|---|---|
| "The report is vague but I found a bug that could be it" | Step 4. Ask; file the found bug separately. |
| "Reading the code, this is clearly the cause" | Step 3. Export it and look. |
| "No time for a regression test / sample / changelog" | Those are the fix. Kristofer's deadline is not a reason to ship less; say what is not done. |
| "I'll put `Fixes #n` in the commit, it's cleaner" | It closes the issue before the evidence is posted. |
| "A description of the fix is enough, they can try it" | Screenshots or a transcript, in the comment. |
| "It works now, close it" | The comment comes first, and it thanks them. |
| "This needs a bigger change, I'll just do it" | Behaviour and scope decisions go to `BACKLOG.md` and to Kristofer. |
