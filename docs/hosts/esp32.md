# ESP32 host

The core compiles natively into the firmware. There is no WebAssembly interpreter on the device
([ADR 0008](../adr/0008-esp32-native-not-interpreter.md)).

## Board and parts

- **ESP32-C3** preferred (RISC-V, upstream Rust toolchain, no `espup` Xtensa fork). Classic
  ESP32 (Xtensa) works with `espup`.
- 128x64 SSD1306 OLED, I2C, 0.96". The 64x32 frame is drawn at 2x, filling it exactly.
- Three tactile buttons to GPIO with internal pull-ups.
- Optional: piezo on a PWM pin for the beep; LED for attention.

## Toolchain

`hosts/esp32` is its own Cargo workspace (excluded from the root workspace because it needs a
different target and the esp-idf build). It depends on `vpet-core` by path. Stack:
`esp-idf-svc` / `esp-idf-hal` (std) rather than bare-metal `esp-hal`, because WiFi, SNTP, the
HTTP client, and NVS are mature there and are exactly what sync needs. Bare-metal is an optional
later optimisation.

## Display

`ssd1306` crate in buffered graphics mode. On `FRAME_CHANGED`, expand each core bit into a 2x2
block into the 1024-byte page buffer and flush. Frames change at most 4 times a second, so I2C
bandwidth is irrelevant.

## Buttons

Poll at 20 Hz with 20 ms software debounce; build the held mask; call `vpet_update`. The core
handles edges and the A+C long-press.

## Time

The core needs epoch milliseconds and must never see time go backwards.

- Boot: read `last_known_epoch_s` and the blob from NVS. Run on the monotonic timer from that
  base: `now_ms = last_known_epoch_s * 1000 + uptime_ms`.
- When SNTP succeeds: if the real epoch is ahead, jump forward (the next `update` fast-forwards).
  If it is behind (should not happen), ignore it. The pet effectively pauses while the device
  was unpowered without a time source, which is the honest behaviour.
- Persist `last_known_epoch_s` alongside the blob.

## Persistence

NVS namespace `vpet`, keys `pet` (blob) and `epoch`. Write on `SAVE_NEEDED` debounced 10 s and a
checkpoint every 5 min. NVS wear-levels across its partition (use >= 24 KB), which is decades of
headroom at this rate.

## Sync

When WiFi is up: GET on boot and every 10 min, adopt if newer; PUT on the same debounce as the
NVS write. Failures are ignored; the local blob is authoritative until the next attempt.

## Power

Optional light sleep between polls when `attention == 0`. `next_event_at` from `inspect` bounds
how long the device can sleep before something visible happens; wake on any button.

## Parity over serial

`just parity-esp32 <vlog>` streams a `.vlog` over USB serial; the firmware, in a test mode
entered by holding B at boot, replays it and echoes a 32-bit hash of each frame; the tool
compares with the native run.

## Bring-up order

1. Blink the OLED with a test pattern.
2. Link the core, `reset`, blit the egg.
3. Buttons and the menu.
4. NVS save and load; power-cycle survives.
5. SNTP; check a long gap fast-forwards.
6. Sync client.
