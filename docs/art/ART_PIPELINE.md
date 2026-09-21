# Art pipeline: `spritekit`

Principle: the model draws each pose once, at cell size, in text. The tooling gives it a ruler, a
mirror, a validator, and a picture of its own output. Coherence between poses is enforced by
diff bounds, not hoped for. A human approves contact sheets, not text.

`tools/spritekit` is a Python 3.12 package run with `uv run spritekit ...`. It calls model APIs
for `gen`, `critique`, and `concept`; everything else is offline.

## Commands

| Command | Input | Output |
|---|---|---|
| `new-species <slug>` | | registry entry, `species.toml`, empty stage files, `BRIEF.md` |
| `validate [path \| --all] [--json]` | text files | report; non-zero exit on errors; runs in CI |
| `render <path \| --species slug> [--pose p] [--sheet] [--anim idle\|eat\|sleep --gif] [--scene main\|eat\|menu] [--style oled\|lcd] [--scale 8]` | text | PNG/GIF under `preview/` |
| `quantize <png> --cell WxH [--target-bbox WxH] [--bias n] --pose name --out file` | PNG | text grid |
| `gen <slug> <stage>[/<pose>] [--candidates 3] [--rounds 4] [--model ...]` | brief, spec, siblings, concept | text grids + `gen-log.jsonl` |
| `concept <slug> [--n 4]` | brief | `concept/*.png` |
| `mark-eyes <file> <pose>` | text | proposed `-` / `o` marks |
| `diff <a> <b>` | two poses | xor count, bbox and centroid deltas, side-by-side PNG |
| `approve <slug>[/<stage>]` | text | updates `assets/approved.lock` |
| `compile [--check] [--allow-unapproved] --out crates/vpet-core/src/assets/generated.rs` | all assets | Rust source |

## Text-grid path (primary for pets, icons, effects)

### Prompt template (`tools/spritekit/prompts/pose.md`)

1. Species brief from `species.toml` and `BRIEF.md`: name, description, `visual_traits`.
2. [STYLE_GUIDE.md](STYLE_GUIDE.md) verbatim.
3. Concept image if one exists, attached.
4. **The stage file so far**: every accepted pose of this stage. This is the coherence anchor.
   For `idle_b`, `eat`, `happy`, `sad` the instruction is "start from idle_a; change at most N
   pixels; keep the bottom row and the eye positions". For a new stage, the previous stage's
   `idle_a` is included as "this creature, younger".
5. Two or three exemplars from `tools/spritekit/examples/` (hand-approved reference sprites).
6. A ruler and an empty template:
   ```
   col  0123456789012345
        ................  ; r0
        ...
        ................  ; r15  ground: at least 2 on-pixels
   ```
7. Required output: **first** a one-paragraph plan (head shape, body, limbs, where the eyes go,
   which column is the axis of symmetry), **then** exactly one fenced block containing the
   `@pose` header and the rows. Nothing else. Committing to an axis before emitting pixels is
   what fixes symmetry.
8. The validator checklist, verbatim, so the model self-checks before emitting.

For `idle_a` the tool generates **3 candidates** at temperature 0.8, validates all, renders a
contact sheet, and asks the critic to rank them. The winner anchors the rest of the stage.
Candidate selection is the cheapest quality lever in the loop.

### Validator

Pure functions; thresholds from `asset-spec.toml`. JSON report per pose:
`{pose, errors[], warnings[], metrics{...}}`, fed back to the generator verbatim.

| Check | Rule (16x16 pet) | Severity |
|---|---|---|
| dims | exactly `w` x `h`, alphabet `.#o-` only, LF, no trailing space | error |
| ground | `bbox.bottom == 15` | error |
| centred | `abs(bbox.centre_x - 7.5) <= 1.0` | error |
| bbox | baby w 6..10 h 6..10; child w 9..14 h 10..15; adult w 11..16 h 12..16 | error |
| density | on / bbox area in [0.45, 0.85] | error |
| connectivity | 8-connected components <= 3; each extra component >= 3 px and within 1 px of the main one | error |
| strays | no on-pixel with zero 8-neighbours; no 1 px hole unless in the blink mask | error |
| dither | 2x2 checkerboards <= 1 | error |
| symmetry | `1 - popcount(f ^ mirror_about_bbox(f)) / popcount(f)`; idle_a/idle_b/eat/sleep >= 0.85; happy/sad >= 0.70; attack exempt | error (warn for happy/sad) |
| eyes | idle_a/idle_b: mask non-empty, mask pixels in mirrored pairs, all in the top 45 % of the bbox | error |
| coherence vs idle_a | idle_b diff in [4, 24]; eat <= 20; happy/sad <= 40; sleep same bottom and height <= idle_a; eye centroid drift <= 1 px (except sleep) | error |
| progression | adult bbox area > child > baby | warn |
| icon (8x8) | column 7 and row 7 empty; density [0.30, 0.70] | error |

### Preview renderer

`render` upscales x8 with a 1 px gap between pixels. `--style oled` (default): black background,
pale cyan lit pixels with a faint bloom, because that is the hardware. `--style lcd`: grey-green.
`--sheet` puts every pose of a stage side by side with names. `--anim --gif` runs the same
operator tables as the core (an 80-line Python port; the *authoritative* preview is the browser
`/dev/sprites` page, which runs the real Rust compositor). `--scene` composes the pose into the
32x16 screen with a bezel so the critic judges at true scale and in context.

### Critique loop (`gen`)

```
round = 0; feedback = none
loop:
  grid    = generate(prompt, feedback)
  report  = validate(grid)
  if report.errors and round < 3: feedback = report; round += 1; continue
  png     = render(grid, sheet_with_siblings=True, scene="main")
  verdict = critique(png, grid, report, brief)
            # separate vision call; strict JSON:
            # {readability: 1-5, character: 1-5, consistency: 1-5, silhouette: 1-5, issues: [...], accept: bool}
  if verdict.accept and min(scores) >= 4 and not report.errors: write pose; break
  if round >= 4: write as <pose>_candidate; mark NEEDS_HUMAN in gen-log; break
  feedback = verdict.issues + report; round += 1
```

The critic and the generator are distinct calls with distinct prompts. The critic never sees
the generator's reasoning: only the picture, the grid, the metrics, and the brief. It is told
what it is looking at (8x upscale, OLED look, 1-bit) and asked concrete questions: can you tell
where the eyes are at a glance; does this read as the same creature as its siblings; is the
silhouette readable at true scale. Budget: 2 to 5 model calls per pose, 60 to 100 per species.

## Image-model path (concepts; direct quantization for 32x16 screens)

Prompt style: "single creature, full body, front view, centred, flat black silhouette on pure
white, bold simple shapes, no shading, no gradients, no outline detail, no text, 1024x1024".
Do not ask for "pixel art": image models fake a grid at the wrong pitch and it aliases badly.

`quantize` pipeline, in the order that matters:

1. Grayscale. Detect polarity from the corner median; invert if the background is dark.
2. Foreground = Otsu threshold; keep the largest connected component (drops watermarks).
3. **Crop to the foreground bbox** with 3 % padding. The single most important step: resizing
   the whole canvas wastes most of the 16 px on margin.
4. Resize to fit `--target-bbox` (default 14x14 for adults) preserving aspect, with **box / area
   averaging**. Never nearest (drops thin limbs), never bicubic (rings).
5. **Threshold, never dither.** Otsu on the tile, clamped to [96, 160]; `--bias` fattens or
   thins. At 16x16 every dither dot is a stray pixel.
6. Cleanup: drop components < 3 px; fill 1 px holes; if symmetry >= 0.7, snap to symmetric
   (`f | mirror(f)`, then re-threshold density). Most of the "blobby" feel disappears here.
7. Bottom-centre into the cell; emit the grid; run the validator. Eyes: `mark-eyes` proposes
   mirrored off-pixel pairs in the top 45 % of the bbox; a human or the text-grid model confirms.

## Combining the paths, and what the human sees

1. **Concept** (optional): 2 to 4 images per species, stored in `concept/`, never compiled. They
   give the generator and the critic one consistent mental picture.
2. **Sprites**: text-grid path, `idle_a` with candidates, then the remaining poses derived from
   the winner, stage by stage: baby, child, adult.
3. **Direct quantization** only for `boot` and `vs` screens, and as a seed ("here is a quantized
   draft; fix it") when the text path stalls on a stage.
4. **Human review**: one contact sheet per species (`render --species x --sheet --gif`): every
   pose at 8x, idle/eat/sleep GIFs, the composed main scene. Edit the text directly if something
   is off (it is just characters), then `approve`, which writes sha256 per pose into
   `assets/approved.lock`. CI fails if an approved pose changes without re-approval; `compile`
   refuses unapproved poses unless `--allow-unapproved`.

The human never has to look at a text grid unless they want to.

## Cost expectations

Per species: about 21 poses, 2 to 5 calls each, one vision call per round, plus a handful of
concept images. Tens of calls, not thousands. The validator catches most problems before a
critic call is spent.
