# Determinism

Given the same save blob and the same sequence of `(now_ms, buttons)` calls, every host produces
the same state and the same frames. This is the property everything else rests on: it is why the
server does not need to run the simulation, why offline play and synced play are the same code,
and why the whole game can be tested with replays.

## The rules

1. **Integer only.** State is `u8`/`u16`/`u32`/`i32`; intermediates may be `i64`/`u64`. No
   `f32`/`f64` in `vpet-core`. Rates are "seconds per one point" (`step_secs: u16`), never
   points per second, so there is no fractional arithmetic.
2. **Simulated time is `Sec = u32` epoch seconds.** `sim_now` is the last processed second. On
   `update(now_ms)`: `target = (now_ms / 1000) as u32`; if `target > sim_now`, `advance_to(target)`.
   Sub-second time is used only for the animation phase (`now_ms / 250`) and for the A+C
   long-press. Neither affects simulation state.
3. **Event-stepped, never tick-stepped.** `advance_to` jumps between scheduled timers
   ([STATE_MODEL.md](STATE_MODEL.md)). The result must be identical whether a gap is covered by
   one call or a thousand. This is a property test.
4. **Time never goes backwards.** If `target < sim_now`, no advance; input still applies at
   `sim_now`. A forward jump is just a fast-forward. Clock manipulation is accepted for a hobby
   project.
5. **The core never reads a clock, entropy, or I/O.** The wasm module has zero imports. Time
   enters only as an argument to `update`; entropy only as the `seed` argument to `reset`.
6. **RNG is xoshiro128\*\***, four `u32` words, state persisted in the save. Randomness is drawn
   only when *scheduling* an event (poop, sickness, tantrum, old age), choosing among
   equal-priority evolution branches, resolving a battle, and generating the play sequence. Never
   per tick, never in rendering. The stream length is therefore proportional to the number of
   events, so replays stay stable when unrelated code changes.
7. **Inputs apply after the advance.** A press at `now_ms` takes effect at
   `floor(now_ms / 1000)` after every timer up to that second has fired.
8. **`#![no_std]`, no `alloc`.** All state is fixed-size. Sprites are `&'static [u8]`. Saturating
   or checked arithmetic everywhere; `.get()` instead of indexing; no `unwrap` outside tests.
   `panic = "abort"`; the wasm panic handler is `unreachable`.
9. **Assets are part of determinism.** A change to any sprite or `species.toml` changes
   outcomes and frames, so it changes `content_hash`, which is embedded in the build and recorded
   in every save.

## How CI enforces them

| Rule | Enforcement |
|---|---|
| 1 | `#![deny(clippy::float_arithmetic)]` in `vpet-core`; a grep for `f32|f64` in CI |
| 3 | proptest: advance invariance (one call vs k random intermediate calls) |
| 4 | proptest: time reversal is a no-op on the blob |
| 5 | `wasm-objdump -x vpet.wasm` must list zero imports |
| 6 | code review + the golden replays (an extra RNG draw shifts every later outcome and fails goldens) |
| 7 | proptest: poll-rate invariance (same press timestamps at 5 Hz vs 60 Hz give identical blobs) |
| 8 | `cargo build --target wasm32-unknown-unknown` with `#![no_std]`; size gate 64 KB; proptest: no panics under random input and gaps up to 10 years |
| 9 | schema snapshot test compares `content_hash` and the blob bytes of a fixed state |
| all | native-vs-wasmtime parity: every `.vlog` produces identical frames and blobs on both |

## What determinism does not cover

- The host's choice of `now_ms`. Two hosts with different clocks will diverge, by design; sync
  resolves that by picking the blob that has simulated furthest ([SYNC.md](SYNC.md)).
- Animation *timing* as seen by a human. Two hosts polling at different rates show the same
  frames but may skip some on screen. Every frame the core produces is still deterministic for a
  given `now_ms`.
