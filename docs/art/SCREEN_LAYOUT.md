# Screen layout

The 32x16 screen is composed from cells by the core. Nothing is ever authored as a full-screen
pet frame. Zone maps below use letters for zones (not pixels); pixel examples follow where it
helps.

## Main scene (Idle, Dirty, Sick, Sleeping)

```
col   0       8       16      24     31
r0    ........PPPPPPPPPPPPPPPPAAAAAAAA   A: status effect 8x8 at (24,0): skull when sick,
r1    ........PPPPPPPPPPPPPPPPAAAAAAAA      attention "!" when calling, zz when asleep
..    ........PPPPPPPPPPPPPPPPAAAAAAAA
r7    ........PPPPPPPPPPPPPPPPAAAAAAAA   P: pet 16x16, home x = 8; walks x in [1, 15]
r8    ........PPPPPPPPPPPPPPPPQQQQQQQQ      (x in [1, 7] while a poop is present)
..    ........PPPPPPPPPPPPPPPPQQQQQQQQ   Q: poop pile 8x8 at (24, 8); see stacking below
r15   ........PPPPPPPPPPPPPPPPQQQQQQQQ
```

Poop stacking: the first poop at (24, 8); a second at (25, 7) drawn over; a third at (26, 6).
The visual is "a pile". While any poop is present the walk range shrinks so the pet never
overlaps it.

Draw order: clear, pet (with its blink mask and offset), items, effects, HUD glyph. Later draws
OR over earlier ones except `invert`, which XORs the whole buffer last.

Example pixels: child pet standing, dirty, attention on.

```
...........................##...
...........................##...
...........................##...
..............####.........##...
............########.......##...
...........##########...........
...........##.####.##......##...
..........############.....##...
..........############......#...
...........####..####......##...
...........##########.....###...
...........##########....#####..
............########.....######.
............##....##....########
............##....##....########
...........###....###....######.
```

## Eat scene

Food at (0, 8), pet pinned at x = 8 so its mouth anchor faces the food. Poses `eat` and `idle_a`
alternate per bite; the food frame advances `food_a` -> `food_b` -> `food_c` -> gone over three
bites. Snacks use `snack_a/b/c`.

## Refuse

Pet at its current x, pose `sad` (or `idle_a` with `shake` if the species has no `sad`),
`cross` effect at head_right for 4 ticks.

## Menu

The eight icons tile the screen as a 4x2 grid of 8x8 cells. The selected icon is drawn inverted
(XOR its 8x8 block). Cursor order: Feed, Lights, Play, Medicine, Clean, Status, Discipline,
Battle.

```
FFFFFFFFLLLLLLLLPPPPPPPPMMMMMMMM   rows 0-7
CCCCCCCCSSSSSSSSDDDDDDDDBBBBBBBB   rows 8-15
```

Feed submenu: two items drawn large, `food_a` at (4, 4) and `snack_a` at (20, 4), the selected
one inverted in its 8x8 block, `MEAL` / `SNACK` in 3x5 under each.

## Status

Paged with A, exit with C. Row 0 to 4: the label in 3x5 (`HUNGER`, `HAPPY`, `DISCPL`, `AGE 12D`,
`WT 25G`, `HEALTH`). Rows 6 to 12: four 7x7 hearts (filled or empty) at x = 0, 8, 16, 24 for
hunger (inverted: full hearts = well fed), happiness, and health; a 4-segment bar for
discipline. Row 15: page dots at x = 12, 14, 16, 18, 20 with the current page's dot doubled.

```
#.#.#.###.###.###.###.##........   HUNGER (3x5 font, advance 4)
................................
###.###..#.....#......#.........   (row 6..12) hearts
#####.####.####.#####..#.#.#.#..
...
............#.##.#.#.#..........   page dots
```

## Play (left/right game)

Pet at x = 8 facing left or right (`flip_h`). Row 0: `L?R` hint in 3x5; row 15: five dots, one
per round, filled as rounds resolve; heart or sweat at head_top per outcome for 2 ticks.

## Battle

BattleReady: pet at x = 0 flipped to face right, opponent's `idle_a` at x = 16; `vs` splash
for 4 ticks first. Battling: attacker lunges 4 px toward the target for 2 ticks, `hit_star` at
the target on a hit, whole-screen `invert` for 1 tick on a hit; hp as small bars on row 0
(mine, x 0..13) and row 15 (theirs, x 18..31). Result: own pet at x = 8, `WIN` / `LOSE` /
`DRAW` on row 0, heart or sweat effect.

## Evolution

Pet at x = 8, whole buffer inverted every tick for 8 ticks, `sparkle_a/b` alternating at
(0, 0) and (24, 8), then the new stage's `idle_a` with `shake` for 4 ticks.

## Egg and tombstone

Egg: `egg_a` at x = 8; `egg_b` alternates every 8 ticks in the last minute before hatch; hop on
the final 4 ticks. Tombstone at x = 8, static, `cross` at head_top; nothing else on screen.

## Sleeping

Pet `sleep` pose at x = 8, `zz_a`/`zz_b` alternating every 4 ticks at head_right. With lights
off the *entire frame is inverted* so the OLED is mostly dark (on an LCD theme this reads as
"lights out"), and the attention glyph still shows when sleepy is pending.

## Walk ranges and limits

- Idle walk: x in [1, 15], step every 2 ticks, direction flips at the bound or on a random
  pause (15 % per step) that idles for 4 to 12 ticks.
- Dirty walk: x in [1, 7].
- Sick, Sleeping, Eating, Busy, Battle: no walk; x pinned as described.
- The HUD glyph at (24, 0) is drawn last and is never covered by the pet.
