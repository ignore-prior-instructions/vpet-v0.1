# Sprite format

Text grids are the source of truth. They are diffable, readable and writable by a model, and
they carry metadata a PNG cannot (blink mask, anchors). PNG is an import path only.

## Cell sizes

The screen is 64x32 ([ADR 0016](../adr/0016-screen-64x32.md)). Packed rows are
`ceil(w / 8)` bytes; cells that are multiples of 8 wide have no partial bytes.

| Class | Cell | Count | Notes |
|---|---|---|---|
| Pet pose | 32x32 | 5 to 7 per stage | bottom-anchored: feet on row 31. Baby art is small inside the cell (bbox 12 to 20 px), child 18 to 28, adult 22 to 32 |
| Egg | 32x32 | 2 global (`egg_a` whole, `egg_b` cracked) | species may override |
| Tombstone | 32x32 | 1 global | |
| Menu icon | 12x12 | 8 global | art in the inner 11x11; row 11 and column 11 blank so inverted selection shows a border |
| Item | 16x16 | food x3, snack x3, poop x2 | species may override food |
| Effect | 16x16 | 13 global | heart, heart_small, zz_a, zz_b, skull, sweat, sparkle_a, sparkle_b, hit_star, attention, note, cross, ghost |
| Font glyph | 5x7, advance 6 | 42 | A-Z, 0-9, `! ? - . :`, space (`font5x7.txt`); plus heart / empty-heart at 14x14 for status bars (`hearts.txt`). Hand-drawn, not generated |
| Screen | 64x32 | boot, vs | the one class where image-model quantization is the primary path |

## Grammar

```
@cell 16x16            ; required, first line
@species lalafu        ; species files only
@stage child           ; species files only: baby | child | adult | adult_alt
@style silhouette      ; silhouette (default) | outline; see docs/adr/0017

@pose idle_a           ; species files: one of idle_a idle_b happy eat sleep sad attack egg_a egg_b
@mouth 5,9             ; optional anchor (x,y) in cell coords; default: leftmost on-pixel at 60 % bbox height
......####......
....########....
...##########...
...##-####-##...
..############..
..############..
...####..####...
...##########...
...##########...
....########....
....##....##....
....##....##....
...###....###...
...###....###...
..####....####..
..####....####..

@pose idle_b
...
```

- Lines starting with `@` are headers. `;` starts a comment to end of line.
- A pose is exactly `h` lines of exactly `w` characters from the alphabet, immediately after its
  `@pose` header and optional anchors. Blank lines separate poses.
- Global files use `@sprite <name>` instead of `@pose` and their own `@cell`.
- Files are UTF-8, LF line endings, no trailing whitespace. The validator rejects anything else.

## Alphabet

| Char | Base state | In blink mask |
|---|---|---|
| `.` | off | no |
| `#` | on | no |
| `o` | on | yes: an eye pixel that goes off during a blink (outline-style eyes) |
| `-` | off | yes: an eye hole or eyelid that goes on during a blink (silhouette-style eyes) |

Blink compiles to one XOR mask: `frame ^ blink_mask`. No second "eyes closed" frame is ever
drawn ([ADR 0015](../adr/0015-blink-as-xor-mask.md)). Poses without `o`/`-` have no mask and
never blink.

## Compiled form

Row-major, 1 bit per pixel, **MSB is the leftmost pixel**, row stride `ceil(w / 8)` bytes.

| Cell | Bytes |
|---|---|
| 32x32 | 128 (+128 for the mask if present) |
| 16x16 | 32 |
| 14x14 | 28 (two bytes per row, 14 bits used) |
| 12x12 | 24 (two bytes per row, 12 bits used) |
| 5x7 | 7 (one byte per row, 5 bits used) |
| 64x32 | 256 |

Worked example, row 3 of the pose above, `...##-####-##...`:

```
chars   . . . # # - # # # # - # # . . .
bits    0 0 0 1 1 0 1 1   1 1 0 1 1 0 0 0
bytes   0x1B              0xD8
mask    0 0 0 0 0 1 0 0   0 0 1 0 0 0 0 0
bytes   0x04              0x20
```

The framebuffer uses the identical packing at 64 wide, so a blit is: for each of the sprite's
rows, shift the row's bits by `x` and OR (or XOR, or AND-NOT) them into the target row.

```rust
pub struct Sprite { pub w: u8, pub h: u8, pub rows: &'static [u8] }
```

`compile` computes and stores each pose's bounding box `(x0, y0, x1, y1)` so the compositor
can centre, measure, and check ground contact without scanning at runtime.

## Import from PNG

`spritekit quantize <png> --cell 32x32 --pose idle_a --out species/x/adult.txt` produces a text
grid from an image ([ART_PIPELINE.md](ART_PIPELINE.md)). `spritekit import-png` does a
1:1 conversion for a PNG that is already at cell size (black = on). Eye marks are never
recovered automatically; `spritekit mark-eyes` proposes them.
