# 0003: Deterministic integer simulation with host-supplied time and entropy

Status: accepted
Date: 2026-09-20

## Context

For a save blob to mean the same pet everywhere, every host must compute identical results from
identical inputs. Floats differ across targets in subtle ways; clocks and entropy sources are
host-specific.

## Decision

`vpet-core` uses integer arithmetic only (`deny(clippy::float_arithmetic)`), simulated time is
`u32` epoch seconds passed in by the host, entropy enters once as a `u64` seed at reset and is
thereafter xoshiro128** state inside the save, and randomness is drawn only when scheduling
events, breaking evolution ties, resolving battles, and generating the play sequence. The wasm
module has zero imports.

## Consequences

- Rates are "seconds per point", never "points per second".
- Golden replays and property tests cover the whole game.
- Clock manipulation is not defended against; time backwards is clamped.
- Adding a random draw anywhere shifts every later outcome and fails goldens, which is the
  intended tripwire.
