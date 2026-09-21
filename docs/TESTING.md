# Testing

A deterministic core is easy to test exhaustively. The strategy: unit tests for the pieces,
golden replays for behaviour, property tests for the invariants that make determinism real, and
a parity harness to prove every host is the same core.

## `.vlog` input logs

A text format, one command per line, used by goldens, the parity harness, and the dev panel
recorder.

```
# hatch.vlog
reset 1700000000000 seed=42
t 1700000001000 -        # update at ms with no buttons
t 1700000001100 A        # A held
t 1700000001200 -
skip 300000              # advance the clock 300 000 ms with no calls (a closed tab)
t 1700000301200 -
snap hatched             # compare frame + inspect against hatch.expected#hatched
save                     # capture blob
load                     # reload the last captured blob
```

Buttons are any subset of `ABC` or `-`. `snap` names are unique within a file.

## Golden replays

`tests/golden/<name>.vlog` + `<name>.expected`. `vpet-cli replay <vlog>` runs the log and at each
`snap` compares the ASCII frame (32x16 of `#`/`.`) and the `Inspect` fields with the expected
file. `vpet-cli replay --bless` regenerates. Review a blessed diff like any code change: the
ASCII frames are readable in a PR.

Required scenarios by Phase 4: hatch, each action (applied and refused), 24 h neglect to death,
7-day gap, sleep cycle with and without lights, each evolution branch, old age, a battle, a play
minigame.

## Property tests (proptest)

1. **Advance invariance**: advancing to T in one `update` equals advancing through k random
   intermediate `update`s.
2. **Poll-rate invariance**: the same press timestamps delivered at 5 Hz and at 60 Hz produce
   identical blobs.
3. **Save/load transparency**: `save(load(save(s))) == save(s)`, and future behaviour matches
   with or without the round trip.
4. **No panics**: random button mashing and gaps up to 10 years never trap.
5. **Time reversal is a no-op**: an `update` with `now_ms` earlier than `sim_now` changes nothing
   but the animation phase.

## Cross-host parity

`vpet-cli parity` loads `vpet.wasm` through `wasmtime` and runs every `.vlog` through both the
native `Cart` and the wasm exports, asserting identical frame bytes after every `t` line and
identical blobs at the end. Runs in CI.

ESP32 parity is manual: `just parity-esp32 <vlog>` streams the log over serial; the firmware
echoes a hash of each frame; the tool compares against native.

## Other gates

- **Schema snapshot**: a fixed `Cart` serialises to fixed bytes and a fixed `content_hash`.
- **Size**: CI fails if `vpet.wasm` exceeds 64 KB after `wasm-opt -Oz`.
- **Imports**: CI fails if `vpet.wasm` has any import.
- **Assets**: `spritekit validate --all` and `spritekit compile --check` in CI.
- **Web smoke**: Playwright loads the page, presses A/B/C, and screenshots the canvas.

## Unit tests

Ordinary `#[cfg(test)]` tests in `vpet-core` (std is enabled under test): `Meter` math at
boundaries, timer tie-breaking, each action's effect table, branch selection, save round trip,
each load error code.
