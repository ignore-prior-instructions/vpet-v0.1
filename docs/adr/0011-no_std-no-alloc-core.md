# 0011: `no_std`, no `alloc`, size budget

Status: accepted
Date: 2026-09-20

## Context

The core must run on an ESP32 with tens of KB of RAM and load fast in a browser.

## Decision

`#![no_std]` with no allocator. All state is fixed-size; sprites are `&'static [u8]`; a 2 KB
static scratch buffer serves save, load, and inspect. `panic = "abort"`, `opt-level = "z"`,
`lto = "fat"`, `codegen-units = 1`. `vpet.wasm` must stay under 64 KB after `wasm-opt`; CI
enforces it. Saturating or checked arithmetic and `.get()` everywhere; no `unwrap` outside tests.

## Consequences

- Fixed limits are explicit: 3 poops, 15 timers, 6 attention bits, a bounded species count.
- No strings at runtime; text is rendered from glyph indices.
- `std` is enabled under `cfg(test)` and behind the `dev-overrides` feature only.
