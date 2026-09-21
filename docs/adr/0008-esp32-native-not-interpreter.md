# 0008: ESP32 compiles the core natively

Status: accepted
Date: 2026-09-20

## Context

"One artifact" could mean shipping `vpet.wasm` to the device and running it under wasm3 or WAMR,
both of which run on ESP32. Or the device could compile the same Rust source natively.

## Decision

Native. `hosts/esp32` depends on `vpet-core` by path and links it into the firmware on the
esp-idf std stack. "One core" is a source-level guarantee enforced by the cross-host parity test.

## Consequences

- No interpreter RAM (30 to 60 KB plus a 64 KB linear memory page), no C glue, no opaque traps;
  normal Rust panics and symbols.
- Parity risk exists in theory (compiler differences) and is covered by the parity harness and
  the integer-only rule.
- The `.wasm` is a browser artifact only.
