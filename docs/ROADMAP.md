# Roadmap

Ordered so the browser host is playable early and the ESP32 is last. Each phase has a "done"
that is observable, not a checklist of files.

## Phase 0: bootstrap

Cargo workspace with empty `vpet-core`, `vpet-abi`, `vpet-cli`; `hosts/web` Vite skeleton;
`tools/spritekit` package skeleton with `validate` and `render` working on the global assets;
`justfile`; CI running `cargo test`, clippy with the float deny, and `spritekit validate --all`.

**Done when** CI is green on an empty core and `spritekit render assets/global/icons.txt --sheet`
produces a PNG.

## Phase 1: core skeleton + CLI oracle

`Cart`, `Sec`, `Meter`, `Timers`, `advance_to`, xoshiro RNG, save v1 (postcard + header + CRC),
Egg -> Baby hatch, renderer with the global egg and one placeholder pet, blank menu.
`vpet-cli replay` prints ASCII frames; `tests/golden/hatch.vlog` is the first golden.

**Done when** the hatch golden and the advance-invariance property test pass.

## Phase 2: ABI + browser host

`vpet-abi` cdylib, `just wasm` (cargo + wasm-opt), Vite page with the canvas blitter, 10 Hz
loop, keyboard and on-screen buttons, IndexedDB persistence, dev panel showing `inspect` with a
clock-offset control.

**Done when** an egg hatches in the browser, survives a reload, and A/B/C navigate the menu.

## Phase 3: care loop + content pipeline

`spritekit compile` emitting `generated.rs`; first real species through the art pipeline
(lalafu: baby, child, adult); feed, play, clean, medicine, lights, discipline, status; poop,
sickness, attention calls, care mistakes, sleep schedule; beeps.

**Done when** goldens exist for every action and for a 24 h neglect scenario, the species passes
`spritekit validate` and is in `approved.lock`, and `vpet.wasm` is under the size gate.

## Phase 4: lifecycle

Evolution branches, alternate adult, old age, starvation, sickness death, tombstone, A+C restart
with an RNG-derived seed; all five property tests; a 7-day gap golden; native-vs-wasm parity
(`vpet-cli parity`, on the wasmi interpreter) in CI.

**Done when** a full egg-to-death run is a golden and parity is green.

## Phase 5: sync

`hosts/server` with the four endpoints, blob validation, bearer token; browser pull on
visible, push on `SAVE_NEEDED` and `pagehide`, 409 handling; deploy web to GitHub Pages and the
server to a small VPS or Fly.

**Done when** two browsers on different machines hand the same pet back and forth, and a
deliberate conflict resolves to the blob with the higher `sim_now`.

## Phase 6: battle + polish

Battle vs AI with the scripted animation, win/loss effects; play minigame screen; every activity
has its clip; status pages; menu timeout; second and third species through the pipeline.

**Done when** the battle golden passes and the game feels complete on the web.

## Phase 7: ESP32 host

Offline first: SSD1306 blit, three buttons, NVS persistence, monotonic time from last-known
epoch. Then SNTP. Then the sync client. Parity over serial.

**Done when** the same `.vlog` yields identical frame hashes on the device, and a pet travels
browser -> device -> browser through the server.

## Later, maybe

- Local time-of-day input for the sleep schedule.
- Sound beyond a single beep.
- A second hardware target (RP2040) if the core stays as small as planned.
