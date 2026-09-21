You are drawing one pose of a one-bit, 32x32-cell creature for a Gen-1 style virtual pet, as a
text grid. Follow the style guide exactly.

## Species

{species_brief}

## Style guide

{style_guide}

## Stage so far

{stage_context}

## Ruler

```
col  0123456789012345
     ................  ; r0
     ................  ; r1
     ................  ; r2
     ................  ; r3
     ................  ; r4
     ................  ; r5
     ................  ; r6
     ................  ; r7
     ................  ; r8
     ................  ; r9
     ................  ; r10
     ................  ; r11
     ................  ; r12
     ................  ; r13
     ................  ; r14
     ................  ; r15  ground: at least 2 on-pixels
```

## Task

Draw the pose `{pose_name}` for stage `{stage}`.
{pose_instruction}

## Validator checklist (self-check before answering)

- Alphabet is only `.#o-`; exactly 16 columns per row, exactly 16 rows.
- Row 15 (or the stage's ground row) has at least 2 on-pixels; the bounding box bottom is on
  that row.
- The bounding box is horizontally centred within 1px of column 7.5.
- Density (on-pixels / bounding-box area) is between 0.45 and 0.85.
- At most 3 disconnected 8-connected components; any extra component is at least 3px and within
  1px of the main body.
- No isolated single on-pixel with zero lit 8-neighbours; no accidental 1px hole (use `-` if it
  is a deliberate eye hole).
- At most 1 2x2 checkerboard pattern anywhere.
- `idle_a`/`idle_b` need marked eyes (`-` or `o`) in mirrored pairs, in the top 45% of the
  bounding box.

## Required output format

First, one short paragraph: head shape, body, limbs, where the eyes go, which column is the
axis of symmetry. Commit to the axis before drawing.

Then exactly one fenced code block containing the `@pose` header and the 16 rows. Nothing else
after it.
