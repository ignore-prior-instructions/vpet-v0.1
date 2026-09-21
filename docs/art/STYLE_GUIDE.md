# Style guide: one-bit creatures at 16x16

This file is included verbatim in the generator prompt. Keep it short and concrete.

## The look

Gen-1 handheld pets: bold, solid silhouettes with a few large interior features. No shading, no
dither, no outlines around the whole body. Think "a shape you could cut from black paper", then
punch out eyes and a mouth. The screen is a 128x64 OLED at 4x, so every pixel is a 4x4 block of
light on black; single stray pixels glow and look like dust.

## Rules

1. **Silhouette first.** Decide the outline before any detail. The creature should be
   recognisable with eyes and mouth removed.
2. **Minimum feature size is 2 px.** Nothing 1 px wide except deliberate whiskers or antennae,
   and never a lone 1 px dot.
3. **No dither, no checkerboard.** One-bit means on or off. Texture is silhouette, not pattern.
4. **Feet on the ground.** Row 15 (the bottom row) has at least 2 on-pixels for every standing
   pose. Sleeping poses too.
5. **Centred.** The bounding box is horizontally centred in the cell within 1 px.
6. **Symmetric unless the pose says otherwise.** idle, eat, sleep are mirror-symmetric to at
   least 85 %. happy and sad to 70 %. attack is free.
7. **Eyes are marked, not drawn.** Use `-` for an eye hole in a silhouette, `o` for an eye pixel
   in an outlined face. Eyes come in mirrored pairs in the top 45 % of the body. This is what
   makes blinking free.
8. **Density.** Between 45 % and 85 % of the bounding box is on. Below that reads as wire; above
   that reads as a blob.
9. **Stage progression.** Baby fits in about 8x8 inside the cell. Child about 12x12. Adult fills
   most of 16x16. Same creature, growing: keep the ears, the eye style, the feet.
10. **Poses are edits of idle_a.** idle_b changes 4 to 24 pixels. eat changes only the mouth.
    happy and sad move ears, arms, or the mouth, at most 40 pixels. sleep may change the
    silhouette but keeps the width and the feet row.

## Good and bad

```
good idle_a (adult)          bad: dither and strays        bad: outline only, hollow
......######......           ..#.#.#.#.#.#.#..             ..############..
....##########....           .#.#.#.#.#.#.#.#.             .#............#.
...############...           ..#.#.#.#.#.#.#..             #..............#
..###-######-###..           .#.#.#-#.#.#-#.#.             #..#........#..#
..##############..           ..#.#.#.#.#.#.#..             #..............#
..##############..           .#.#.#.#.#.#.#.#.             #......##......#
...#####..#####...           ..#.#.#.#.#.#.#..             #..............#
...############...           .#.#.#.#.#.#.#.#.             .#............#.
....##########....           ..#.#.#.#.#.#.#..             ..############..
....###....###....           ....#.#....#.#...             ...#......#.....
...####....####...           ...#.#.#..#.#.#..             ..##......##....
```

## Icons (8x8)

Draw inside the top-left 7x7; leave row 7 and column 7 empty. Density 30 % to 70 %. One idea per
icon: a bowl, a bulb, a ball, a syringe, a broom, a bar chart, a pointing hand, two crossed
swords.
