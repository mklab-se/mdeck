---
name: test-visualization
description: Thoroughly test and validate a visualization type in mdeck. Use when making changes to any visualization renderer, fixing visual bugs, or adding new visualization types.
---

# Visualization Testing Skill

When testing or validating a visualization in mdeck, follow this systematic process. Do NOT declare work complete until every step passes.

## Step 1: Create or identify the test sample

Find or create the sample presentation in `samples/visualizations/` that covers the visualization being tested. The sample should include:
- A slide with ALL features visible (no progressive reveal — all items use `-` markers)
- A slide with progressive reveal (`+` and `*` markers)
- Edge cases specific to the visualization type
- At least one slide that matches a real-world use case

## Step 2: Debug export at FULL resolution

Export every reveal step at the resolution users will actually see:

```bash
cargo run -p mdeck -- export samples/visualizations/<type>.md --output-dir /tmp/viz-audit --debug --width 1920 --height 1080
```

This creates `slide-NN-step-MM.png` for every step of every slide.

## Step 3: Systematic visual audit

For EACH exported image, check these criteria as a UX designer would:

### Shapes and lines
- [ ] Are S-curves actual S-shapes (not U-shapes, diagonals, or loops)?
- [ ] Are straight lines truly straight (not slightly curved)?
- [ ] Are vertical lines perfectly vertical?
- [ ] Do lines connect to dots cleanly (no gaps, no overlap past the dot)?
- [ ] Are line thicknesses consistent?

### Dots and events
- [ ] Is there exactly ONE dot per event (no double dots)?
- [ ] Are dots centered on their lane?
- [ ] Are dots the right size (visible but not overlapping with adjacent dots)?

### Labels and text
- [ ] Are labels readable (not overlapping with dots, lines, or other labels)?
- [ ] Are labels positioned correctly (near their associated element)?
- [ ] Is there enough gap between labels and dots?
- [ ] Do any labels get cut off at the edges?

### Progressive reveal
- [ ] Does step 0 show only the static items?
- [ ] Does each subsequent step add exactly one logical element?
- [ ] Do existing elements stay in the SAME position between steps (no shifting)?
- [ ] Does the final step match the all-static version?

### Layout and spacing
- [ ] Are elements evenly spaced?
- [ ] Is there enough margin on all sides?
- [ ] Does the visualization fill the available space reasonably?
- [ ] Are inactive/background elements visible but clearly different from active ones?

## Step 4: Document ALL issues found

Create a numbered list of every issue, no matter how small. Include:
- Which slide and step the issue appears on
- What the issue is (be specific)
- What it should look like instead

## Step 5: Fix issues and re-audit

After fixing issues:
1. Rebuild: `cargo fmt --all && cargo build`
2. Re-export: `cargo run -p mdeck -- export ... --debug --width 1920 --height 1080`
3. Re-check EVERY image that was affected by the fix
4. Verify that the fix didn't break other slides/steps

## Step 6: Run CI checks

```bash
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

## Step 7: Final verification

Export one more time and check slide 1 step 0 (the most important — what users see first) and the final step of the most complex slide. Only declare done if both look correct.

## Important rules

- NEVER declare work complete based on low-resolution thumbnails. Always export at 1920x1080.
- NEVER skip checking progressive reveal steps — bugs often hide in intermediate states.
- NEVER assume a fix worked without re-exporting and re-checking.
- When the same type of bug keeps recurring, step back and rethink the approach rather than patching.
- If control points for bezier curves produce wrong shapes, draw a diagram on paper first: P0→P1 defines the start tangent, P2→P3 defines the end tangent.
