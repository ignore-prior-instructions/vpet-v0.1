# State model

How the pet is represented so that fast-forwarding any gap is correct and cheap. This
generalises v0's "value at timestamp plus rate": every time-varying quantity is an anchored
closed form, and every threshold crossing or scheduled occurrence is a precomputed timer in a
fixed table. Fast-forward walks the table in time order; cost is O(events in the gap).

## Types

```rust
pub type Sec = u32;
pub const NEVER: Sec = u32::MAX;

/// A 0..=100 stat that moves one point every `step_secs`, in direction `dir`.
pub struct Meter { v0: u8, t0: Sec, step_secs: u16, dir: i8, paused: bool }

impl Meter {
    pub fn value_at(&self, t: Sec) -> u8 {
        if self.paused { return self.v0; }
        let steps = (t.saturating_sub(self.t0) / self.step_secs as u32) as i32;
        (self.v0 as i32 + self.dir as i32 * steps).clamp(0, 100) as u8
    }
    /// When will the meter reach `limit` moving in its direction? NEVER if it will not.
    pub fn reaches(&self, limit: u8) -> Sec { /* t0 + |limit - v0| * step_secs, or NEVER */ }
    pub fn reanchor(&mut self, t: Sec, v: u8) { self.v0 = v; self.t0 = t; }
    pub fn pause(&mut self, t: Sec)  { let v = self.value_at(t); self.reanchor(t, v); self.paused = true; }
    pub fn resume(&mut self, t: Sec) { self.t0 = t; self.paused = false; }
}
```

Hunger is `dir = +1` (rises toward starving). Happiness is `dir = -1`. Sleep pauses both; a
stage change re-anchors both with the new stage's `step_secs`.

## Timers

```rust
#[repr(u8)]
pub enum EventKind {
    // firing order at equal timestamps is this order
    Wake, Sleep,                 // schedule, relative to hatch
    Hatch, Evolve, OldAge,       // lifecycle
    Poop, Tantrum, SickOnset,    // environment
    HungerEmpty, HappyEmpty,     // meter thresholds (hunger reaches 80, happiness reaches 20)
    CareMiss,                    // an attention call unanswered for care_miss_secs
    SickDamage,                  // health -10 every sick_damage_secs while sick
    Starve,                      // hunger at 100 for starve_secs
    UiBusyEnd, MenuTimeout,      // UI timers
}
pub const EVENT_COUNT: usize = 15;
pub struct Timers([Sec; EVENT_COUNT]);   // NEVER = unset

impl Timers {
    pub fn set(&mut self, k: EventKind, at: Sec);
    pub fn clear(&mut self, k: EventKind);
    /// Lowest time; ties broken by enum order. None if all NEVER.
    pub fn earliest(&self) -> Option<(Sec, EventKind)>;
}
```

Why a fixed array and not a heap: there is at most one pending instance of each kind, fifteen
entries is a trivial linear scan, and it costs 60 bytes with no allocation.

## Fast-forward

```
advance_to(target):
    if state != Alive { sim_now = target; return }         // Egg is Alive too; Dead/Uninit is O(1)
    loop:
        match timers.earliest():
            None => break
            Some((t, kind)) if t > target => break
            Some((t, kind)) => { sim_now = t; fire(kind) }  // may reanchor meters, set/clear timers, draw RNG
    sim_now = target
```

`fire` handlers must be written so that the state after firing does not depend on `target`.
Every meter threshold timer is recomputed whenever its meter is re-anchored, paused, or resumed
(`timers.set(HungerEmpty, hunger.reaches(80))`).

Ordering guarantees from the enum: schedule before lifecycle, lifecycle before environment,
environment before needs, needs before consequences, consequences before UI. So a `HungerEmpty`
and a `CareMiss` scheduled for the same second resolve the same way everywhere.

Cost bound: an unattended pet dies within about two days, so the number of events in any gap is
bounded by content (a few hundred). A year-long gap after death is one comparison.

## Full state

```rust
pub struct Cart {
    pub state: CartState,            // Uninit | Alive | Dead
    pub sim_now: Sec,
    pub rng: [u32; 4],
    pub pet: Pet,
    pub timers: Timers,
    pub ui: Ui,                      // persisted, small
    // not persisted:
    prev_buttons: u8,
    hold_since_ms: u64,
    anim: AnimState,                 // derived from sim state + now_ms; see art/ANIMATION.md
}

pub struct Pet {
    pub species: u8, pub stage: Stage,
    pub hatched_at: Sec, pub stage_since: Sec,
    pub hunger: Meter, pub happy: Meter,
    pub discipline: u8, pub health: u8, pub weight: u16,
    pub poops: u8,                   // 0..=3
    pub poop_since: Sec,             // when the oldest poop appeared; NEVER if none
    pub sick: bool, pub sleeping: bool, pub lights_off: bool,
    pub attention: u8,               // bit0 hungry, 1 sad, 2 poop, 3 sick, 4 sleepy, 5 tantrum
    pub care_mistakes: u8, pub sick_count: u8,
    pub snacks_since_wake: u8,
    pub wins: u8, pub losses: u8, pub last_battle_at: Sec,
    pub death_cause: u8,             // 0 none, 1 starve, 2 sickness, 3 old age
}

pub enum Ui {
    Idle,
    Menu { cursor: u8 },             // 0..8; MenuTimeout pending
    FeedSub { snack: bool },
    Status { page: u8 },
    Busy { kind: BusyKind },         // Eating, Refuse, Playing{seq,round,correct}, Discipline, Evolving, Result
    Battle { phase: u8, turns: [u8; 6], hp_me: u8, hp_them: u8 },
}
```

`Ui` is persisted so a save mid-animation reloads invisibly and parity tests can snapshot at any
point. `AnimState` is not persisted; it is recomputed from `Ui`, `Pet`, and `now_ms`.

## Event handlers (what `fire` does)

| Event | Effect | Reschedules |
|---|---|---|
| Hatch | stage = Baby, hatched_at = now, meters anchored to baby steps | Evolve at +stage_secs; Sleep at +awake_secs; Poop; thresholds |
| Evolve | evaluate branches; stage change; re-anchor meters; Ui = Busy(Evolving) | Evolve (Child) or OldAge (Adult); UiBusyEnd |
| OldAge | die(old age) | clears all |
| Sleep | sleeping = true; if lights_off: pause meters; else set sleepy + CareMiss | Wake at +sleep_secs |
| Wake | sleeping = false, lights_off = false, resume meters, snacks_since_wake = 0, health +10 if not sick | Sleep at +awake_secs; thresholds |
| Poop | poops = min(poops+1, 3); poop_since if first; set poop attention + CareMiss; maybe schedule SickOnset (Child+: 3 poops, or 1 h from now if not cleaned) | Poop at +rand(interval) if awake |
| Tantrum | set tantrum attention + CareMiss | Tantrum at +rand(interval) |
| SickOnset | sick = true; set sick attention + CareMiss | SickDamage at +sick_damage_secs |
| SickDamage | health -= 10; if 0: die(sickness) | SickDamage again while sick |
| HungerEmpty | set hungry attention + CareMiss; if hunger will reach 100: Starve at reaches(100) + starve_secs; schedule SickOnset when at 100 | |
| HappyEmpty | set sad attention + CareMiss | |
| CareMiss | care_mistakes += 1; clear the attention bit(s) that triggered it | next CareMiss if other bits still set |
| Starve | die(starvation) | clears all |
| UiBusyEnd | Ui = Idle (or next phase for multi-step busy states) | |
| MenuTimeout | Ui = Idle | |

`CareMiss` is a single timer; it is set to `now + care_miss_secs` when a bit is set and no timer
is pending, and when it fires it clears every bit that has been set for at least
`care_miss_secs` (tracked with a small `attention_since: [Sec; 6]`). Only one care mistake is
counted per firing.

## Actions

Applied at `sim_now` after `advance_to`. See [GAME_DESIGN.md](GAME_DESIGN.md) for the rules; the
implementation shape is:

```rust
pub fn feed_meal(cart: &mut Cart) -> Outcome {
    let now = cart.sim_now;
    let h = cart.pet.hunger.value_at(now);
    if cart.pet.sleeping || h < 10 { return cart.refuse(); }
    cart.pet.hunger.reanchor(now, h.saturating_sub(30));
    cart.pet.weight = cart.pet.weight.saturating_add(2);
    cart.clear_attention(HUNGRY);
    cart.timers.set(HungerEmpty, cart.pet.hunger.reaches(80));
    cart.timers.clear(Starve);
    cart.ui = Ui::Busy { kind: BusyKind::Eating };
    cart.timers.set(UiBusyEnd, now + 5);   // 18 anim ticks ~ 4.5 s, rounded up
    Outcome::Applied
}
```

Every action ends by recomputing the threshold timers for any meter it touched and returning
whether it was applied or refused, which decides the animation.

## Invariants (asserted in debug builds and tested)

- `sim_now` is monotonic.
- No timer is set in the past relative to `sim_now` after `advance_to` returns.
- `poops == 0` implies `poop_since == NEVER`.
- `sleeping` implies both meters are paused iff `lights_off`.
- Dead implies every timer is `NEVER`.
