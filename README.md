# vpet-v0.1

<p align="center">
  <a href="https://ignore-prior-instructions.github.io/vpet-v0.1/">
    <img src="https://img.shields.io/badge/%E2%96%B6%20%20Play%20vpet-fc7eaf?style=for-the-badge&labelColor=271e38&color=fc7eaf" alt="Play vpet" height="40">
  </a>
</p>

A Gen-1 style virtual pet (Tamagotchi / early Digimon lineage) built as **one deterministic
simulation core that runs unchanged in a browser, on a sync server, and on an ESP32 with a
64x32 OLED**.

The core is a Rust crate compiled to WebAssembly for the web and natively for hardware. It owns
the whole game: state, time, rules, and the 64x32 one-bit framebuffer. Each platform host is a
thin blitter that passes in time and button state and copies pixels out. A tiny Rust server stores
the save blob so the same pet follows you between devices.

The pets are hand-drawn pixel sketches (docs/art/LINEAGE.md), kept honest by tooling: a
text-grid sprite format, a validator, a preview renderer, and a hot-reload page. The page
around the screen is a small design system (packages/vpet-ds) shared with Claude Design.

This repository is the successor to `vpet-v0-ts`, which proved the game design and the
timestamp-based state idea and then stalled on its stack and its art. See
[docs/VISION.md](docs/VISION.md) for what changed and why.

## Play it

[ignore-prior-instructions.github.io/vpet-v0.1](https://ignore-prior-instructions.github.io/vpet-v0.1/)
(published from `main` by `.github/workflows/pages.yml`). Add `?dev` to the URL for the inspect
panel and clock-skip buttons. Keys: `Z` or `←` = A (menu / cursor / guess left), `X` or `↓` = B
(select / guess right), `C` or `→` = C (back). Your pet is saved in the browser; hold A and C
together for a second on the tombstone for a new egg. Rules: [docs/GAME_DESIGN.md](docs/GAME_DESIGN.md).

Locally:

```bash
just wasm && npm --prefix hosts/web run dev      # browser at http://localhost:5173/?dev
cargo run -p vpet-cli -- play                    # terminal: a/b/c + Enter, + skips an hour
```

## Status

Phases 0 to 5 of [docs/ROADMAP.md](docs/ROADMAP.md) are done: the full care loop, three
species from the sketches (lalafu the ghost, ninjifu the ninja, charamofu the dino, chosen by
how the child is cared for), evolution, old age, death and restart, the content pipeline with
hot preview, 16 golden replays, five property tests, native-vs-wasm parity in CI, and
multi-device sync (an axum blob store plus a browser client under Settings; the server is
built and Dockerised but not hosted yet). Next: Phase 6 (battle, more species), Phase 7
(ESP32). Start with [docs/README.md](docs/README.md).

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
crates/vpet-abi      9 extern "C" exports; cdylib for wasm32 (+ dev_* exports behind `dev-overrides`)
crates/vpet-cli      replay / play / autoplay / parity / dump; the test oracle
hosts/web            Vite + plain TypeScript + canvas; /dev/sprites hot preview
hosts/server         axum blob store (persistence + multi-device sync); Dockerfile, not hosted yet
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
