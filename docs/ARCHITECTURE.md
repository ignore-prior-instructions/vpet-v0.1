# Architecture

## The cartridge model

The whole game is one `no_std` Rust crate, `vpet-core`. It owns the simulation, the rules, the
assets, and the 32x16 framebuffer. It has no dependencies on a clock, a display, storage, or a
network. Think of it as a cartridge: the hosts are consoles.

```
                 +------------------------------------------------+
                 |                  vpet-core                     |
   now_ms  --->  |  advance_to(now)  -> timers fire, meters move  |
   buttons --->  |  apply input      -> actions, menu, battle     |  ---> frame[64]   (32x16 bits)
   seed    --->  |  render()         -> compositor, font, HUD     |  ---> flags       (changed/save/beep/attention)
   blob    --->  |  load/save        -> postcard + header + crc   |  ---> blob
                 +------------------------------------------------+
```

The interface is eight C-ABI functions ([HOST_ABI.md](HOST_ABI.md)). The same source compiles
three ways:

| Target | Built as | Used by |
|---|---|---|
| `wasm32-unknown-unknown` | `vpet-abi` cdylib, `vpet.wasm`, no imports | browser host |
| native (x86_64 / aarch64) | `vpet-core` rlib | CLI oracle, server (blob validation), tests |
| `riscv32imc-esp-espidf` (or xtensa) | `vpet-core` rlib linked into the firmware | ESP32 host |

There is no interpreter on the device. "One core" is a source-level guarantee enforced by the
cross-host parity test ([TESTING.md](TESTING.md)), not by shipping the same bytes.

## Hosts are dumb blitters

A host does exactly four things:

1. **Time**: call `vpet_update(now_ms, buttons)` at some rate (web 10 Hz, ESP32 20 Hz). The rate
   does not matter to the core; it fast-forwards to `now_ms` however far that is.
2. **Input**: maintain a held-state mask for A/B/C. The core does edge detection and long-press.
3. **Output**: when the `FRAME_CHANGED` flag comes back, copy 64 bytes to the display, upscaled.
   Optionally beep on `BEEP` and blink an LED on `ATTENTION`.
4. **Persistence**: when `SAVE_NEEDED` comes back (debounced), call `vpet_save` and store the
   blob locally; optionally push it to the server ([SYNC.md](SYNC.md)).

A host never draws a sprite, renders text, or decides what the screen shows. If it needs to know
something about the pet (a dev panel, a sleep-until hint), it calls `vpet_inspect`.

## Data flow

**Open the app after a night away (browser):**

```
page load
  -> IndexedDB.get("pet:default")              local blob
  -> GET /v1/pets/default                      server blob (if configured)
  -> pick the blob with the higher sim_now
  -> vpet_load(blob)
  -> vpet_update(Date.now(), 0)                core fast-forwards: Sleep, Wake, Poop, HungerEmpty...
  -> FRAME_CHANGED: blit                       pet is awake, hungry, standing next to a poop
  -> SAVE_NEEDED: save locally, PUT to server
```

**Press Feed:**

```
pointerdown on A     -> buttons |= 1
vpet_update(now, 1)  -> rising edge on A: open menu, cursor on Feed; FRAME_CHANGED
pointerup            -> buttons = 0
...B...              -> select Feed: submenu Meal/Snack
...B...              -> apply meal at sim second floor(now/1000): hunger -30, weight +2,
                        UiBusyEnd = now + 18 anim ticks; FRAME_CHANGED | SAVE_NEEDED | BEEP
next 18 ticks        -> eat/idle toggle with the food sprite shrinking; then back to Idle
```

**Same pet on the ESP32:**

```
boot -> NVS.get("pet") + NVS.get("epoch")      blob + last known epoch seconds
     -> vpet_load; vpet_update(epoch*1000 + uptime_ms, buttons)
     -> WiFi + SNTP later: epoch jumps forward only; next update fast-forwards
     -> GET /v1/pets/default; adopt if newer; PUT on SAVE_NEEDED (debounced 10 s)
```

## Repo map

```
vpet-v0.1/
  Cargo.toml                workspace: crates/*, hosts/server (hosts/esp32 excluded: own toolchain)
  rust-toolchain.toml       stable + wasm32-unknown-unknown
  justfile                  just wasm | web | test | golden-bless | parity | size | art-check
  .github/workflows/ci.yml  cargo test, clippy (float deny), wasm size gate, spritekit --check
  crates/
    vpet-core/              the cartridge: sim, render, assets (no_std, no alloc)
      src/lib.rs            Cart: reset/load/save/update/frame/inspect
      src/time.rs           Sec, NEVER, ms->Sec
      src/meter.rs          Meter closed-form stats
      src/timers.rs         Timers[EventKind], earliest(), fire()
      src/pet.rs            Pet struct, attention bits, care mistakes
      src/actions.rs        feed/play/clean/medicine/lights/discipline
      src/evolve.rs         branch evaluation, stage change
      src/battle.rs         resolution + turn list
      src/rng.rs            xoshiro128**
      src/save/{mod,v1}.rs  header, crc, versioned structs
      src/render/           compositor, font, screens, HUD
      src/anim.rs           clip table, procedural ops, AnimState
      src/assets/mod.rs     Sprite/Pose/StageSet/SpeciesDef types
      src/assets/generated.rs   CHECKED-IN output of `spritekit compile`
    vpet-abi/               8 extern "C" exports around a static Cart; cdylib for wasm32
    vpet-cli/               replay .vlog, play in terminal, dump frames, bless goldens
  hosts/
    web/                    Vite + plain TS + canvas; dev panel; /dev/sprites hot preview
    server/                 axum; file blob store; links vpet-core to validate PUTs
    esp32/                  separate Cargo workspace; esp-idf std; ssd1306; NVS; SNTP
  assets/
    asset-spec.toml         cells, required poses, validator thresholds, global rules
    approved.lock           sha256 per approved sprite
    registry.toml           append-only species id -> slug
    global/                 icons.txt items.txt effects.txt font3x5.txt egg.txt tombstone.txt screens.txt
    species/<slug>/         species.toml baby.txt child.txt adult.txt [adult_alt.txt] [concept/]
  tools/spritekit/          Python (uv): validate render quantize gen critique approve compile
  tests/golden/             *.vlog input logs + *.expected frame/inspect snapshots
  docs/
```

## Why these boundaries

- **Core owns the framebuffer** rather than emitting a scene description: three hosts would
  otherwise each reimplement compositing, the font, and menu layout, and drift. With pixels as
  the output, parity is "compare 64 bytes". ([ADR 0002](adr/0002-core-owns-framebuffer.md))
- **No allocator in the core**: fixed-size state (~200 bytes), sprites as `&'static [u8]`, a 2 KB
  scratch buffer for save/inspect. This is what makes the ESP32 port a dependency rather than a
  project. ([ADR 0011](adr/0011-no_std-no-alloc-core.md))
- **Assets compiled in**: the device needs them in flash anyway, runtime loading would need a
  parser and allocation, and a content change is a rules change so it belongs in the build hash.
  ([ADR 0009](adr/0009-assets-compiled-in-checked-in-codegen.md))
- **Server does not run the sim** for play; it links the core only to reject corrupt blobs.
  ([ADR 0007](adr/0007-lww-sync-by-sim-now.md))
