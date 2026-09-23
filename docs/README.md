# vpet-v0.1 documentation

## Reading order

1. [VISION.md](VISION.md): what this is, what failed before, the four principles.
2. [GAME_DESIGN.md](GAME_DESIGN.md): the game itself. Lifecycle, needs, actions, evolution, battle.
3. [ARCHITECTURE.md](ARCHITECTURE.md): the cartridge model, the hosts, the repo map.
4. [ROADMAP.md](ROADMAP.md): the phases and what "done" means for each.
5. [NEXT.md](NEXT.md): the open threads right now, each with a starting point. Read this
   before picking up new work.

Then the technical specs, in dependency order:

| Doc | What it pins down |
|---|---|
| [HOST_ABI.md](HOST_ABI.md) | The eight exports, memory convention, frame format, flags, errors |
| [DETERMINISM.md](DETERMINISM.md) | The rules that make every host compute the same pet |
| [STATE_MODEL.md](STATE_MODEL.md) | Meters, timers, fast-forward, action semantics |
| [SAVE_FORMAT.md](SAVE_FORMAT.md) | Blob layout, versioning, migration |
| [SYNC.md](SYNC.md) | Last-writer-wins by simulated time, server API |
| [CONTENT.md](CONTENT.md) | `assets/` layout, `species.toml`, `asset-spec.toml`, codegen, hot preview |
| [TESTING.md](TESTING.md) | Goldens, property tests, cross-host parity |

Art and rendering:

| Doc | What it pins down |
|---|---|
| [art/SPRITE_FORMAT.md](art/SPRITE_FORMAT.md) | Text-grid grammar, blink mask, cell sizes, bit packing |
| [art/SCREEN_LAYOUT.md](art/SCREEN_LAYOUT.md) | How the 64x32 screen is composed, scene by scene |
| [art/ANIMATION.md](art/ANIMATION.md) | Pose set, procedural motion, clip table, state machine |
| [art/STYLE_GUIDE.md](art/STYLE_GUIDE.md) | Visual rules for one-bit 32x32 creatures |
| [art/ART_PIPELINE.md](art/ART_PIPELINE.md) | `spritekit`: generate, validate, render, critique, approve |
| [art/LINEAGE.md](art/LINEAGE.md) | Where the pet lines came from: v0 species and the Procreate sketches |

Hosts:

| Doc | Host |
|---|---|
| [hosts/web.md](hosts/web.md) | Browser: Vite, canvas, IndexedDB, dev panel |
| [hosts/server.md](hosts/server.md) | axum blob store |
| [hosts/esp32.md](hosts/esp32.md) | ESP32 + SSD1306 |
| [hosts/cli.md](hosts/cli.md) | Replay and play in a terminal |

Decisions: [adr/README.md](adr/README.md). Each ADR is one page: context, decision, consequences.

## Conventions used in these docs

- Sizes are `width x height`. The screen is 64x32. Pixel `(x, y)` has origin top-left.
- Time in the core is `Sec = u32` epoch seconds. Hosts pass milliseconds; the core truncates.
- "Tick" without qualification means an animation tick, 250 ms. The simulation has no tick; it
  is event-stepped.
- Code snippets are illustrative Rust unless marked otherwise. Names in snippets are the names
  the implementation should use.
