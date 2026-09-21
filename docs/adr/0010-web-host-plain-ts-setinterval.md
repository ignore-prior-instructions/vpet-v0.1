# 0010: Web host: plain TypeScript, no bindgen, 10 Hz interval

Status: accepted
Date: 2026-09-20

## Context

The browser host needs to load a wasm module with eight exports, draw 64 bytes, and store a
blob. Frameworks and wasm-bindgen add build steps and weight for no benefit at this size.

## Decision

Plain TypeScript with Vite. `WebAssembly.instantiateStreaming` with an empty import object.
`setInterval` at 100 ms rather than `requestAnimationFrame`. IndexedDB for the blob,
localStorage for settings.

## Consequences

- The host is a few hundred lines and the wasm is loaded without glue code.
- `now_ms` crosses as a `BigInt`; all current browsers support i64 parameters.
- Background tabs keep ticking at 10 Hz (browsers throttle to about 1 Hz), which is fine because
  the core fast-forwards on the next call anyway.
