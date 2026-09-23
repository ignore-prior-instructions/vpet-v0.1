# Architecture decision records

One page each: context, decision, consequences. Numbered, never edited after acceptance; a
change is a new ADR that supersedes the old one.

| # | Title | Status |
|---|---|---|
| [0001](0001-rust-core-as-cartridge.md) | Rust core as a cartridge with thin hosts | accepted |
| [0002](0002-core-owns-framebuffer.md) | The core owns the framebuffer | accepted |
| [0003](0003-deterministic-integer-sim.md) | Deterministic integer simulation with host-supplied time and entropy | accepted |
| [0004](0004-event-timer-fast-forward.md) | Anchored meters and a timer table instead of ticks | accepted |
| [0005](0005-postcard-versioned-save.md) | Versioned postcard save with header and CRC | accepted |
| [0006](0006-no-input-log-in-save.md) | No input log in the save | accepted |
| [0007](0007-lww-sync-by-sim-now.md) | Last-writer-wins sync by simulated time; dumb server | accepted |
| [0008](0008-esp32-native-not-interpreter.md) | ESP32 compiles the core natively | accepted |
| [0009](0009-assets-compiled-in-checked-in-codegen.md) | Assets compiled in via checked-in codegen | accepted |
| [0010](0010-web-host-plain-ts-setinterval.md) | Web host: plain TypeScript, no bindgen, 10 Hz interval | accepted |
| [0011](0011-no_std-no-alloc-core.md) | `no_std`, no `alloc`, size budget | accepted |
| [0012](0012-screen-32x16-and-cell-sizes.md) | 32x16 screen, 16x16 pets, 8x8 icons, 3x5 font | superseded by 0016 |
| [0013](0013-server-axum-file-store.md) | Rust axum server with a file store | accepted |
| [0014](0014-text-grid-art-with-critique-loop.md) | Model-generated art through text grids with a validate/render/critique loop | accepted |
| [0015](0015-blink-as-xor-mask.md) | Blink as an XOR mask in the sprite alphabet | accepted |
| [0016](0016-screen-64x32.md) | 64x32 screen, 32x32 pets, 12x12 icons, 5x7 font | accepted |
| [0017](0017-hand-drawn-art-and-shared-stages.md) | Hand-drawn art keeps its own style; species may share a stage's art | accepted |

## Template

```
# NNNN: Title

Status: proposed | accepted | superseded by NNNN
Date: YYYY-MM-DD

## Context
## Decision
## Consequences
```
