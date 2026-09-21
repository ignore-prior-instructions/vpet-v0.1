# Screen layout

The 64x32 screen ([ADR 0016](../adr/0016-screen-64x32.md)) is composed from cells by the core.
Nothing is ever authored as a full-screen pet frame. The numbers below live in
`crates/vpet-core/src/render/layout.rs`; `compose.rs` has no bare coordinates.

The reference is the Tamagotchi Connection: a plain play area, with the eight action icons
along the top and bottom edges (four each) and the selected one highlighted. On the original
the icons are LCD segments on the bezel; here they are drawn into the frame while the menu is
open and the play area is otherwise uncluttered.

## Main scene (Idle, Dirty, Sick, Sleeping)

```
col   0               16              32              48             63
r0    ................PPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPAAAAAAAAAAAAAAAA   A: 16x16 HUD at (48,0): skull when
..    ................PPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPAAAAAAAAAAAAAAAA      sick, "!" when calling
r15   ................PPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPAAAAAAAAAAAAAAAA   P: pet 32x32, home x = 16;
r16   ................PPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPQQQQQQQQQQQQQQQQ      walks x in [2, 30] ([2, 14] while
..    ................PPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPQQQQQQQQQQQQQQQQ      a poop is present)
r31   ................PPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPQQQQQQQQQQQQQQQQ   Q: poop pile 16x16 at (48,16)
```

Poop stacking: the first poop at (48, 16); a second at (50, 14) drawn over; a third at (52, 12).
The visual is "a pile". While any poop is present the walk range shrinks so the pet never
overlaps it.

Draw order: clear, pet (with its blink mask and offset), items, effects, HUD glyph. Later draws
OR over earlier ones except `invert`, which XORs the whole buffer last.

## Eat scene

Food (16x16) at (0, 16), pet pinned at x = 16 so its mouth faces the food. Poses `eat` and
`idle_a` alternate per bite; the food frame advances `food_a` -> `food_b` -> `food_c` -> gone
over three bites. Snacks use `snack_a/b/c`.

## Refuse

Pet at home, pose `sad` (or `idle_a` if the species has no `sad`) with `shake`, `cross` effect
at head_right, for 8 ticks (2 s: long enough to register as "no").

## Menu

Eight 12x12 icons in two rows of four, at x = 2, 18, 34, 50: the top row at y = 0 (Feed, Lights,
Play, Medicine), the bottom row at y = 20 (Clean, Status, Discipline, Battle), the selected icon
inverted in its 12x12 block. The selected action's name (`FEED`, `LIGHTS`, `PLAY`, `MEDICINE`,
`CLEAN`, `STATUS`, `DISCIPLINE`, `BATTLE`) is centred in 5x7 in the band between the rows
(rows 12-18). Cursor order is that same reading order.

```
r0    ..FFFFFFFFFFFF....LLLLLLLLLLLL....PPPPPPPPPPPP....MMMMMMMMMMMM..   top row, y = 0
..
r11   ..FFFFFFFFFFFF....LLLLLLLLLLLL....PPPPPPPPPPPP....MMMMMMMMMMMM..
r12   ..................... L I G H T S ..............................   label band, 5x7, centred
r18
r19   ................................................................
r20   ..CCCCCCCCCCCC....SSSSSSSSSSSS....DDDDDDDDDDDD....BBBBBBBBBBBB..   bottom row, y = 20
..
r31   ..CCCCCCCCCCCC....SSSSSSSSSSSS....DDDDDDDDDDDD....BBBBBBBBBBBB..
```

Feed submenu: `food_a` at (8, 4) and `snack_a` at (40, 4), the selected one inverted in its
16x16 block, `MEAL` / `SNACK` in 5x7 under each at y = 24.

## Status

Paged with A, exit with C. Rows 0 to 6: the label in 5x7 (`HUNGER`, `HAPPY`, `DISCPL`, `AGE`
plus the number at x = 40, `WT` plus the number, `HEALTH`). Rows 10 to 23: four 14x14 hearts
(filled or empty) at x = 0, 16, 32, 48 for hunger (inverted: full hearts = well fed), happiness,
and health; for discipline, four 14x6 bar segments at the same x, y = 14. Health shows the
skull at (48, 12) instead of the fourth heart's slot when sick. Rows 30-31: 2x2 page dots at
x = 24, 28, 32, 36, 40, the current page's dot doubled in height.

## Play (left/right game)

Pet at x = 16 facing left or right (`flip_h`). `L?R` hint in 5x7 at (24, 0); five 2x2 round
dots on rows 30-31 at x = 22, 26, 30, 34, 38, filled as rounds resolve; heart or sweat at
head_top per outcome for 2 ticks.

## Battle

BattleReady: pet at x = 0 flipped to face right, opponent's `idle_a` at x = 32; `vs` splash
for 4 ticks first. Battling: attacker lunges 8 px toward the target for 2 ticks, `hit_star` at
the target on a hit, whole-screen `invert` for 1 tick on a hit; hp as small bars on row 0
(mine, x 0..27) and row 31 (theirs, x 36..63). Result: own pet at x = 16, `WIN` / `LOSE` /
`DRAW` on row 0, heart or sweat effect.

## Evolution

Pet at x = 16, whole buffer inverted every other tick for 8 ticks, `sparkle_a/b` alternating
at (0, 0) and (48, 16), then the new stage's `idle_a` with `shake` for 4 ticks.

## Egg and tombstone

Egg: `egg_a` at x = 16; `egg_b` alternates every 8 ticks in the last minute before hatch; hop on
the final 4 ticks. Tombstone at x = 16, static, `cross` at (28, 0); nothing else on screen.

## Sleeping

Pet `sleep` pose at x = 16, `zz_a`/`zz_b` alternating every 4 ticks at head_right. With lights
off the *entire frame is inverted* so the OLED is mostly dark (on an LCD theme this reads as
"lights out"), and the attention glyph still shows when sleepy is pending.

## Lights off while awake

Every pet scene (Main, Sick, Eating, Refuse, Discipline, Result, Playing, Evolving) is likewise
inverted while `lights_off` is set, so the Lights action always has a visible effect, as on the
Connection. The Menu, Feed submenu and Status pages are never inverted (they are the player's
UI, not the room). Lights come back on automatically at Wake.

## Walk ranges and limits

- Idle walk: x in [2, 30], step every 2 ticks, direction flips at the bound or on a random
  pause (15 % per step) that idles for 4 to 12 ticks.
- Dirty walk: x in [2, 14].
- Sick, Sleeping, Eating, Busy, Battle: no walk; x pinned as described.
- The HUD glyph at (48, 0) is drawn last and is never covered by the pet.
