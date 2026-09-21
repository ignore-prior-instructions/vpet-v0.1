# Animation

Goal: the model draws each pose once; motion is procedural; every frame is a deterministic
function of simulation state and the animation tick, so replays and parity tests cover animation
too.

## Tick

`ANIM_HZ = 4`: an animation tick is 250 ms, `tick = now_ms / 250`. The classic 2 fps pose toggle
is "swap every 2 ticks". Hosts call `update` at any rate; the core renders whatever tick
`now_ms` falls in. Animation lengths that matter to the game (eating takes 18 ticks) are
mirrored by simulation timers (`UiBusyEnd`) so the *game* does not depend on host timing.

## Drawn poses per stage

| Pose | Required | Brief given to the generator |
|---|---|---|
| `idle_a` | yes | neutral, front-facing, feet on row 15, eyes marked |
| `idle_b` | yes | same silhouette, 4 to 24 pixels changed: body squashed 1 px, or ears/arms moved; bottom row identical |
| `happy` | yes | arms or ears up, or open smiling mouth; used for hops and wins |
| `eat` | yes | mouth open wide, otherwise identical to idle_a |
| `sleep` | yes | eyes closed, body lower and rounder; drawn, not derived, because the silhouette changes |
| `sad` | no | eyes or ears droop; used for refuse, discipline, sick, loss. Fallback: idle_a + shake |
| `attack` | no | leaning or lunging; used in battle. Fallback: happy |

Required drawn frames per species: 15 (3 stages x 5). Typical 21. Maximum 28 with an alternate
adult. v0 had 17 hand-translated frames for one activity of one stage.

## Procedural operators

All are pure functions of `(tick, rng_state_snapshot, ui, pet)`.

| Op | Params | Effect |
|---|---|---|
| `offset` | dx, dy | translate the cell before blitting; clipped at screen edges |
| `flip_h` | | mirror the cell horizontally |
| `blink` | | draw `img ^ blink_mask` this tick |
| `invert` | | XOR the whole framebuffer after compositing |
| `toggle` | period = 2 | alternate two poses every `period` ticks |
| `hop` | | dy from `[0, -1, -2, -1]` indexed by `tick % 4` |
| `shake` | | dx from `[-1, 1, -1, 0]` |
| `walk` | range, step = 2 | every `step` ticks x += dir; at a bound, or with 15 % chance per step, flip dir or pause 4 to 12 ticks; poses toggle per step |
| `overlay` | sprite, anchor | OR-blit a 16x16 effect at `head_top`, `head_right`, `corner_tr`, `mouth`, or absolute |

Anchors derive from the pose bbox: `head_top` = (bbox centre x - 4, bbox y0 - 8, clamped),
`head_right` = (bbox x1 + 1, bbox y0), `mouth` = the pose's `@mouth`.

Blink schedule: `next_blink = tick + 12 + (rng % 16)`; a blink lasts 1 tick; only for poses with
a mask. The RNG draw for blink comes from a *separate* animation RNG seeded from `(sim_now,
rng[0])` at each state change so blinks never consume simulation randomness.

## Clip table

| Activity / reaction | Poses | Motion | Overlays | Length |
|---|---|---|---|---|
| Idle | idle_a / idle_b toggle | walk [2, 30], blink | `attention` at corner_tr if calling | loop |
| Dirty | as Idle | walk [2, 14] | poop pile; `attention` | loop |
| Sick | sad (or idle_a) | none, x = 8 | `skull` at corner_tr blinking every 4 ticks | loop |
| Sleeping | sleep | none, x = 8; `invert` if lights off | `zz_a` / `zz_b` alternate at head_right every 4 ticks | until Wake |
| Eating | eat / idle_a toggle, 3 bites of 6 ticks | x = 8 | food frame 0, 1, 2 then none at (0, 8) | 18 ticks |
| Refuse | sad | shake | `cross` at head_right | 4 ticks |
| Playing | happy / idle_a toggle; flip_h per round | hop on correct | `heart` or `sweat` at head_top | 5 rounds, player-paced, 30 s timeout |
| Discipline | sad | shake | none | 6 ticks |
| Evolving | idle_a (old) 8 ticks, idle_a (new) 4 ticks | invert each tick, then shake | `sparkle_a/b` at corners | 12 ticks |
| Dead | tombstone | none | `cross` at head_top | static |
| BattleReady | attack (or happy) at x = 0 flipped; opponent idle_a at x = 16 | shake once | none | until B or C |
| Battling | attack | scripted lunges from the turn list; invert 1 tick on hit | `hit_star` at target | 6 turns x 4 ticks |
| BattleResult | happy or sad | hop or none | `heart` or `sweat`; WIN/LOSE/DRAW text row 0 | 8 ticks |
| Egg | egg_a; egg_b in the last minute | hop in the last 4 ticks | none | until Hatch |

## State machine

```
AnimState {
  clip: ClipId, clip_start_tick: u32,
  pet_x: i8, facing: L | R, walk_pause_until: u32,
  next_blink: u32, anim_rng: u32,
  reaction: Option<(ClipId, end_tick)>,   // transient override: Refuse, Discipline, Result
}

on_state_change(ui, pet, tick):            // called by the core whenever Ui or activity changes
  clip = clip_for(ui, pet); clip_start_tick = tick; reseed anim_rng from (sim_now, rng[0])
push_reaction(clip, len, tick):            // does not clobber the base clip
  reaction = Some((clip, tick + len))
render(fb, tick):
  fb.clear()
  (clip, t) = reaction if still active else (clip, tick - clip_start_tick)
  compose per the clip table using the operators
  hud(fb)                                   // attention / skull / zz drawn last
  if lights_off && sleeping: fb.invert()
```

Walk stepping and blink scheduling happen inside `render` as pure functions of `tick` and
`anim_rng`, so rendering the same tick twice gives the same frame and rendering ticks out of
order is harmless.

The simulation decides when activities change (`UiBusyEnd` ends Eating), so animation length
is part of the deterministic simulation, not host timing.
