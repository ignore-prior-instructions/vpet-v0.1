# 0006: No input log in the save

Status: accepted
Date: 2026-09-20

## Context

An input log would allow replay-based merge of divergent snapshots and richer debugging, at the
cost of unbounded growth and a second source of truth.

## Decision

The save is a snapshot only. Input logs (`.vlog`) exist as test and dev artifacts, recorded by
hosts in dev mode, never persisted with the pet.

## Consequences

- Bounded flash writes on the ESP32.
- Sync cannot merge; it picks a winner (see 0007).
- Debugging a field report needs the blob plus a description, not a log.
