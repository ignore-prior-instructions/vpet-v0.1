# Style guide: one-bit creatures at 32x32

This file is included verbatim in the generator prompt. Keep it short and concrete. It describes
the look generated (`@style silhouette`) art converges on; hand-drawn `@style outline` art
(`docs/art/LINEAGE.md`) is checked structurally against `docs/art/SPRITE_FORMAT.md` and
`spritekit validate`, not against every rule below (`docs/adr/0017-hand-drawn-art-and-shared-stages.md`).

## The look

Gen-1 handheld pets (the Tamagotchi Connection is the reference): bold, solid silhouettes with a
few large interior features. No shading, no dither, no outlines around the whole body. Think "a
shape you could cut from black paper", then punch out eyes and a mouth. The screen is a 128x64
OLED at 2x, so every pixel is a 2x2 block of light on black; single stray pixels glow and look
like dust.

## Rules

1. **Silhouette first.** Decide the outline before any detail. The creature should be
   recognisable with eyes and mouth removed.
2. **Minimum feature size is 2 px; lines that should read as bold are 3 px.** Nothing 1 px wide
   except deliberate whiskers or antennae, and never a lone 1 px dot.
3. **No dither, no checkerboard.** One-bit means on or off. Texture is silhouette, not pattern.
4. **Feet on the ground.** Row 31 (the bottom row) has at least 2 on-pixels for every standing
   pose. Sleeping poses too.
5. **Centred.** The bounding box is horizontally centred in the cell within 1 px.
6. **Symmetric unless the pose says otherwise.** idle, eat, sleep are mirror-symmetric to at
   least 85 %. happy and sad to 70 %. attack is free.
7. **Eyes are marked, not drawn.** Use `-` for an eye hole in a silhouette, `o` for an eye pixel
   in an outlined face. Eyes come in mirrored pairs in the top 45 % of the body, 2x2 or larger.
   This is what makes blinking free.
8. **Density.** Between 45 % and 85 % of the bounding box is on. Below that reads as wire; above
   that reads as a blob.
9. **Stage progression.** Baby bbox 12 to 20 px inside the cell. Child 18 to 28 wide, 20 to 30
   tall. Adult 22 to 32 wide, 24 to 32 tall. Same creature, growing: keep the ears, the eye
   style, the feet.
10. **Poses are edits of idle_a.** idle_b changes 16 to 96 pixels. eat changes only the mouth.
    happy and sad move ears, arms, or the mouth, at most 160 pixels. sleep may change the
    silhouette but keeps the width and the feet row.

## Good and bad

```
good idle_a (adult, centre rows)        bad: dither and strays          bad: outline only, hollow
........############........            ..#.#.#.#.#.#.#.#.#.#.#.        ....################....
......################......            .#.#.#.#.#.#.#.#.#.#.#.#        ...##..............##...
.....##################.....            ..#.#.#.#.#.#.#.#.#.#.#.        ..##................##..
....####--########--####....            .#.#.#--#.#.#.#--#.#.#.#        .##....--......--....##.
....####--########--####....            ..#.#.#--#.#.#.#--#.#.#.        .##....--......--....##.
....####################....            .#.#.#.#.#.#.#.#.#.#.#.#        .##..................##.
.....########....########...            ..#.#.#.#.#.#.#.#.#.#.#.        .##......######......##.
```

## Icons (12x12)

Draw inside the top-left 11x11; leave row 11 and column 11 empty so the inverted selection
shows a border. Density 30 % to 70 %. One idea per icon, readable at 2x2 blocks with no label:
a bowl with steam (feed), a light bulb (lights), a ball (play), a syringe (medicine), a broom
(clean), a bar chart (status), a pointing hand (discipline), two crossed swords (battle).

## Items and effects (16x16), hearts (14x14), font (5x7)

Items and effects follow the creature rules minus symmetry. The font is a plain 5x7 sans: every
glyph 5 wide, 7 tall, 2 px strokes where they fit, advance 6.
