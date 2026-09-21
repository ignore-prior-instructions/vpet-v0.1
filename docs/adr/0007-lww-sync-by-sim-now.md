# 0007: Last-writer-wins sync by simulated time; dumb server

Status: accepted
Date: 2026-09-20

## Context

The server exists for persistence and multi-device use by one person. Options were an
always-on server simulation, CRDT or log merge, or a blob store with a conflict rule.

## Decision

The server stores a blob per pet and does not simulate. Conflicts resolve to the blob with the
higher `sim_now` (taken from inside the blob, not from any host clock); ties break on server
sequence. Four endpoints: GET, PUT (409 on stale), DELETE, health. The server links the core only
to reject invalid blobs on PUT.

## Consequences

- The server is a few hundred lines and has no game logic to keep in step.
- Overlapping offline play on two devices loses the shorter timeline. Documented and accepted.
- No host clock is trusted for the decision.
