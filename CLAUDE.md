# vpet-v0.1: conventions for coding agents

Read `docs/README.md` first. Decisions live in `docs/adr/`; do not re-litigate them in code
comments or PR descriptions. If a decision must change, add a new ADR that supersedes the old one.

## Hard rules for `crates/vpet-core`

- `#![no_std]`, no `alloc`. All state is fixed-size.
- No floating point anywhere. `#![deny(clippy::float_arithmetic)]` is on; do not `allow` it.
- The core never reads a clock, RNG, or I/O. Time enters only through `vpet_update(now_ms, ..)`
  and entropy only through `vpet_reset(.., seed)`. The wasm module has zero imports.
- Fast-forward is event-stepped. Never write a loop that advances one second at a time.
- Every change to a persisted struct bumps the save version and adds a forward conversion.
  The schema snapshot test will fail if you forget.
- `crates/vpet-core/src/assets/generated.rs` is produced by `spritekit compile`. Never hand-edit
  it; edit the text grids or `species.toml` under `assets/` and regenerate.
- Keep `vpet.wasm` under 64 KB. CI enforces it.

## Hosts

Hosts are dumb blitters. If you find yourself drawing a sprite, rendering text, or deciding what
the screen shows in a host, stop: that belongs in the core. Hosts do time, input, persistence, and
copying the 64-byte frame to a display.

## Art

Sprites are text grids under `assets/`. Use `spritekit render` to look at them and
`spritekit validate` before committing. Approved sprites are hashed in `assets/approved.lock`; do
not change an approved sprite without re-approval.

## Git

This repo is pinned to the `personal` gitswitch identity. Use `gh-here` for GitHub API calls.
