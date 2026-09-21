# 0004: Anchored meters and a timer table instead of ticks

Status: accepted
Date: 2026-09-20

## Context

v0 stored stats as value-at-timestamp plus rate and derived current values on demand, but had to
be told when to check for events. A tick loop over a multi-day gap is slow on a microcontroller
and hard to make identical across call patterns.

## Decision

Every time-varying stat is a closed-form `Meter { v0, t0, step_secs, dir }`. Every scheduled or
threshold event is an entry in a fixed `Timers[EventKind]` array. `advance_to(target)` fires
timers in time order (enum order breaks ties) and never iterates seconds. Cost is O(events in
the gap); a dead pet is O(1).

## Consequences

- Advance invariance (one call vs many) is a tested property.
- Fifteen timer slots and about 200 bytes of state; no heap.
- Every code path that changes a meter must recompute that meter's threshold timers; the
  invariants section of STATE_MODEL.md lists the debug assertions that catch a miss.
