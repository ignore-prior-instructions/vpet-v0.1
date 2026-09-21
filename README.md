# vpet-v0.1

A Gen-1 style virtual pet (Tamagotchi / early Digimon lineage) built as **one deterministic
simulation core that runs unchanged in a browser, on a sync server, and on an ESP32 with a
128x64 OLED**.

The core is a Rust crate compiled to WebAssembly for the web and natively for hardware. It owns
the whole game: state, time, rules, and the 32x16 one-bit framebuffer. Each platform host is a
thin blitter that passes in time and button state and copies pixels out. A tiny Rust server stores
the save blob so the same pet follows you between devices.

Art is model-generated pixel art, kept honest by tooling: a text-grid sprite format, a validator,
an OLED-look preview renderer, and a generate/validate/render/critique loop.

This repository is the successor to `vpet-v0-ts`, which proved the game design and the
timestamp-based state idea and then stalled on its stack and its art. See
[docs/VISION.md](docs/VISION.md) for what changed and why.

## Status

Phase 1 of [docs/ROADMAP.md](docs/ROADMAP.md): the core sim skeleton (egg -> hatch, the menu,
save/load, the hatch golden) and the CLI oracle exist. Start with
[docs/README.md](docs/README.md).

## Layout (planned)

```
crates/vpet-core     no_std simulation + renderer + compiled-in assets (the cartridge)
crates/vpet-abi      8 extern "C" exports; cdylib for wasm32
crates/vpet-cli      replay / play / dump; the test oracle
hosts/web            Vite + plain TypeScript + canvas
hosts/server         axum blob store (persistence + multi-device sync)
hosts/esp32          esp-idf host: SSD1306 over I2C, three buttons, NVS
assets/              species definitions and text-grid sprites
tools/spritekit      Python art tooling: validate, render, quantize, gen, compile
tests/golden         .vlog input logs and expected frames
docs/                everything below
```

## Docs

- [docs/README.md](docs/README.md): index and reading order
- [docs/VISION.md](docs/VISION.md): why this project exists and its four principles
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md): the cartridge model and the hosts
- [docs/ROADMAP.md](docs/ROADMAP.md): phases 0 to 7 with done criteria
- [docs/adr/README.md](docs/adr/README.md): architecture decision records
