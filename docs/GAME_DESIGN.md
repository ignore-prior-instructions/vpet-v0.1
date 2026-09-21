# Game design

The game carried forward from v0, made concrete enough to implement. Every number here is a
default; species override them in `species.toml` (see [CONTENT.md](CONTENT.md)). All durations are
integer seconds because the core has no floats; the tuning table gives human units alongside.

## Lifecycle

```
Egg --(hatch, 5 min)--> Baby --(1 h)--> Child --(48 h, branch)--> Adult / Adult-alt --(old age)--> Dead
                                                                                       ^
 any stage after Baby --(starvation or sickness)---------------------------------------+
```

- **Egg**: nothing to do. Buttons are ignored. Hatches on a timer.
- **Baby**: needs attention often (fast meters), cannot get sick from poop, cannot battle.
  Evolves to Child on a timer with no conditions.
- **Child**: full rules. Evolves to Adult on a timer; *which* adult depends on care.
- **Adult**: full rules. Dies of old age inside a window, shortened by care mistakes.
- **Dead**: tombstone. Hold A+C for one second to start a new egg.

The pet's species is fixed at reset (chosen by the host from the registry, default the first
species). A species defines all three living stages plus an optional alternate adult. Evolution
may also jump to another species' next stage (`to_species` in the branch), which is how the v0
lineage lalafu -> ninjifu / charamofu is expressed.

## Needs and stats

| Stat | Range | Direction over time | Shown as |
|---|---|---|---|
| Hunger | 0..100 | rises; 0 = full, 100 = starving | 4 hearts, inverted (full hearts = well fed) |
| Happiness | 0..100 | falls | 4 hearts |
| Discipline | 0..100 | only by actions | 4-segment bar |
| Health | 0..100 | only by events | sick flag + hearts on last status page |
| Weight | 1..999 | only by actions | number, "g" |
| Age | derived | from hatch time | days |

Hunger and happiness are **meters**: a stored value at a stored time plus a fixed step
(`step_secs` = seconds per one point). Everything else changes only when something happens.
Meters pause while the pet is properly asleep (sleep window and lights off).

## Attention calls and care mistakes

The pet "calls" by showing the `!` glyph and beeping when a need is unmet. There are five call
reasons, tracked as bits in `attention`:

| Bit | Set when | Cleared by |
|---|---|---|
| hungry | hunger reaches 80 | Feed (meal) |
| sad | happiness reaches 20 | Play |
| poop | a poop appears | Clean |
| sick | sickness begins | Medicine |
| sleepy | sleep window starts and lights are on | Lights (off) |
| tantrum | a random false call (Child+ only) | Discipline |

When a call is set, a **CareMiss** timer starts (15 min). If the call is still set when it fires,
`care_mistakes += 1` and that call bit is cleared (the pet gives up asking; the underlying need
remains). Answering the call cancels the timer. Care mistakes never decrease. They drive the
evolution branch and shorten lifespan.

A tantrum is a call with no need behind it. Answering it with Discipline is correct (+10
discipline). Answering a *real* call with Discipline is wrong (happiness -5). Feeding during a
tantrum is not penalized; the tantrum simply times out as a care mistake.

## Actions

The menu has eight actions in a 4x2 icon grid. A moves the cursor, B selects, C backs out.
The menu closes on its own after 10 s (MenuTimeout).

| Action | Effect | Refused when |
|---|---|---|
| Feed: meal | hunger -30, weight +2, clears hungry | hunger < 10 (sad + shake) or asleep |
| Feed: snack | happiness +10, weight +3; 4th snack since last wake schedules SickOnset | asleep |
| Play | left/right guessing game, 5 rounds; 3+ correct: happiness +20, else +10; weight -1; clears sad | asleep, sick |
| Clean | poops = 0, clears poop, cancels poop-driven SickOnset | no poop |
| Medicine | if sick: sick = false, health +20 (cap 100), sick_count +1, clears sick; if not sick: discipline -5 | never |
| Lights | toggles lights_off; turning lights off during the sleep window starts proper sleep and clears sleepy | never |
| Discipline | if tantrum set: discipline +10, clears tantrum; else happiness -5 | asleep |
| Status | paged read-only screen | never |
| Battle | see below | Baby, asleep, sick, or within 1 h of the last battle |

Every action is applied *after* the core fast-forwards to the moment of the button press, so a
feed at 09:00 after a night away lands on the 09:00 state, not the state at bedtime.

## Sleep

The schedule is relative to hatch time and repeats: awake 14 h, asleep 10 h. The core has no
timezone; the host can shift the schedule by choosing the hatch moment, and a later ADR may add a
local-offset input if it matters. At the start of the sleep window the pet lies down and sets
`sleepy`. If the lights go off within 15 min, meters pause and the night is clean. If not, meters
keep running, the call becomes a care mistake, and the pet sleeps badly (no health bonus).
At wake: lights come on automatically, `sleeping = false`, and if the pet is not sick,
health +10 (cap 100).

## Poop and sickness

- A poop appears every 2 to 3 h (uniform, RNG) while awake, up to 3 on screen.
- Sickness starts by scheduled **SickOnset**. It is scheduled with a delay of 30 to 90 min when any
  of these become true, and cancelled if the trigger is removed before it fires:
  - a poop has been on screen for 1 h, or 3 poops are present (Child+ only)
  - hunger has reached 100
  - the 4th snack since wake
  - a lost battle (10 % chance, decided at battle time)
  - weight >= 40
- While sick, **SickDamage** fires every 30 min: health -10. Health 0 is death.
- Medicine ends sickness. A day without sickness heals +10 at each wake.

## Death

| Cause | Condition |
|---|---|
| Starvation | hunger has been at 100 for 12 h continuously (Starve timer) |
| Sickness | health reaches 0 |
| Old age | OldAge timer, scheduled at adult evolution: uniform in [5 d, 10 d] minus 6 h per care mistake, floor 3 d |

Death clears every timer, shows the tombstone, and records `death_cause`. The fast-forward path
for a dead pet is O(1) regardless of the gap.

## Evolution

- Egg -> Baby: `hatch_secs` (300) after reset.
- Baby -> Child: `stage_secs` (3600) after hatch. No conditions.
- Child -> Adult: `stage_secs` (172800) after becoming a child. Branches are evaluated in order;
  the first whose conditions all hold wins:

```toml
[[stages.child.evolve.branch]]
to = "adult"                     # this species' adult
max_care_mistakes = 4
min_discipline = 40

[[stages.child.evolve.branch]]
to = "adult_alt"                 # fallback; no conditions
```

A branch may name `to_species = "ninjifu"` to jump lineages. Randomised branches use
`weight = N` and the RNG picks among branches with equal-highest priority. Evolution plays the
flash animation and re-anchors both meters to the new stage's step values.

## Battle

Single-player, against a random adult from the registry. Available from Child. Flow:

1. **BattleReady**: your pet at left facing right, opponent at right. B to fight, C to leave.
2. **Battling**: three exchanges. Each exchange, each side attacks once. Hit chance in percent is
   `clamp(50 + 5 * (power - defense), 20, 80)`. A hit does 3 damage, or 6 on a crit
   (`crit_pct`). Both start at `hp` (default 10). All rolls come from the RNG.
3. **BattleResult**: higher remaining hp wins; equal is a draw. Win: happiness +10, wins +1.
   Lose: happiness -10, losses +1, 10 % SickOnset. Draw: nothing.

The whole battle is resolved in the core at the moment B is pressed; the animation replays the
turn list. Battles are recorded by `last_battle_at` for the 1 h cooldown.

## Play minigame

The pet faces left or right for each of 5 rounds. Press A for left, B for right. A correct guess
shows a heart; a wrong one shows sweat. Outcome is scored at the end (see Feed table). The
sequence of directions is drawn from the RNG when Play is selected, so a replay is deterministic.

## Status screen

Paged with A, exit with C. Pages: hunger hearts, happiness hearts, discipline bar, age and
weight, health (hearts plus a skull if sick). Layout in [art/SCREEN_LAYOUT.md](art/SCREEN_LAYOUT.md).

## Tuning table (defaults)

| Key | Default | Human units | Where |
|---|---|---|---|
| `hatch_secs` | 300 | 5 min | species |
| `stages.baby.stage_secs` | 3600 | 1 h | species |
| `stages.child.stage_secs` | 172800 | 48 h | species |
| `stages.*.hunger_step_secs` | baby 180, child 300, adult 360 | 1 point per 3 / 5 / 6 min | species |
| `stages.*.happy_step_secs` | baby 240, child 450, adult 540 | | species |
| `awake_secs` / `sleep_secs` | 50400 / 36000 | 14 h / 10 h | species |
| `poop_interval_secs` | [7200, 10800] | 2 to 3 h | species |
| `care_miss_secs` | 900 | 15 min | global |
| `sick_delay_secs` | [1800, 5400] | 30 to 90 min | global |
| `sick_damage_secs` | 1800 | 30 min | global |
| `starve_secs` | 43200 | 12 h | global |
| `lifespan_secs` | [432000, 864000] | 5 to 10 d | species |
| `lifespan_penalty_per_mistake_secs` | 21600 | 6 h | global |
| `tantrum_interval_secs` | [14400, 28800] | 4 to 8 h, awake, Child+ | global |
| `battle_cooldown_secs` | 3600 | 1 h | global |
| `menu_timeout_secs` | 10 | | global |
| thresholds | hungry 80, sad 20, full 10, overweight 40 | | global |

"Global" values live in `assets/asset-spec.toml` under `[rules.game]` and are compiled in with the
species tables.
