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

## Play it

**https://ignore-prior-instructions.github.io/vpet-v0.1/** (published from `main` by
`.github/workflows/pages.yml`). Add `?dev` to the URL for the inspect panel and clock-skip
buttons. Keys: `Z` = A (menu / cursor / guess left), `X` = B (select / guess right), `C` = C
(back). Your pet is saved in the browser; hold `Z`+`C` for a second on the tombstone for a new
egg. Rules: [docs/GAME_DESIGN.md](docs/GAME_DESIGN.md).

Locally:

```bash
just wasm && npm --prefix hosts/web run dev      # browser at http://localhost:5173/?dev
cargo run -p vpet-cli -- play                    # terminal: a/b/c + Enter, + skips an hour
```

## Status

Phases 0 to 4 of [docs/ROADMAP.md](docs/ROADMAP.md) are done: the full care loop, one species
(lalafu) with both adult forms, evolution, old age, death and restart, the content pipeline
with hot preview, 16 golden replays, five property tests, and native-vs-wasm parity in CI.
Next: Phase 5 (sync server), Phase 6 (battle, more species), Phase 7 (ESP32). Start with
[docs/README.md](docs/README.md).

## Developing

```bash
just ci          # everything CI runs: fmt, clippy, tests, size and import gates, goldens, parity, art, web
just golden      # replay tests/golden/*.vlog; `just bless` to re-record after an intended change
just art-check   # spritekit validate --all && compile --check
just wasm-dev && npm --prefix hosts/web run dev   # then /dev/sprites.html: edit assets/**/*.txt live
```

Needs: Rust stable with the `wasm32-unknown-unknown` target, `wasm-opt` (binaryen),
`wasm-tools`, `just`, `uv`, Node 24. Conventions for changes are in [CLAUDE.md](CLAUDE.md).

## Layout

```
crates/vpet-core     no_std simulation + renderer + compiled-in assets (the cartridge)
crates/vpet-abi      8 extern "C" exports; cdylib for wasm32 (+ dev_* exports behind `dev-overrides`)
crates/vpet-cli      replay / play / autoplay / parity / dump; the test oracle
hosts/web            Vite + plain TypeScript + canvas; /dev/sprites hot preview
hosts/server         axum blob store (persistence + multi-device sync) -- Phase 5
hosts/esp32          esp-idf host: SSD1306 over I2C, three buttons, NVS -- Phase 7
assets/              species definitions and text-grid sprites
tools/spritekit      Python art tooling: validate, render, compile, approve, gen, dev-fixture
tests/golden         .vlog input logs and expected frames
docs/                everything below
```

## Docs

- [docs/README.md](docs/README.md): index and reading order
- [docs/VISION.md](docs/VISION.md): why this project exists and its four principles
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md): the cartridge model and the hosts
- [docs/ROADMAP.md](docs/ROADMAP.md): phases 0 to 7 with done criteria
- [docs/adr/README.md](docs/adr/README.md): architecture decision records
