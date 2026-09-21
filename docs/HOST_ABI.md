# Host ABI

The contract between `vpet-core` and every host. Eight exported functions, two static buffers,
no imports. The native Rust API (`Cart`) mirrors it one-to-one; `vpet-abi` is a 60-line wrapper
around a `static mut CART`.

## Exports

```c
uint32_t       vpet_abi_version(void);
const uint8_t* vpet_frame_ptr(void);
uint8_t*       vpet_io_ptr(void);
void           vpet_reset(uint64_t now_ms, uint64_t seed);
int32_t        vpet_load(uint32_t len);
uint32_t       vpet_save(void);
uint32_t       vpet_update(uint64_t now_ms, uint32_t buttons);
uint32_t       vpet_inspect(void);
```

| Export | Purpose |
|---|---|
| `vpet_abi_version` | Returns `ABI_VERSION` (starts at 1). A host refuses to run on mismatch. |
| `vpet_frame_ptr` | Pointer to the 64-byte framebuffer. Read-only for the host. |
| `vpet_io_ptr` | Pointer to the `VPET_IO_CAP` = 2048 byte scratch buffer shared for save/load/inspect. |
| `vpet_reset(now_ms, seed)` | Start a new egg at `now_ms`. `seed` is host entropy (any 64 bits). Species is the registry default; a host may set another via the reset-with-species variant added when needed. |
| `vpet_load(len)` | Host has copied a blob of `len` bytes into IO. Returns 0 or a negative error. |
| `vpet_save()` | Writes the current blob into IO and returns its length. 0 means Uninit. |
| `vpet_update(now_ms, buttons)` | Advance to `now_ms`, apply input, render. Returns flags. |
| `vpet_inspect()` | Writes the `Inspect` struct into IO and returns its length. |

## Memory convention

- The core exports its linear `memory`. The host never allocates inside it. There is no
  allocator, so memory never grows; hosts may still re-create typed-array views after
  instantiation as a habit.
- `FRAME: [u8; 64]` and `IO: [u8; 2048]` are statics. Their addresses are stable for the life of
  the instance.
- Save: `len = vpet_save(); blob = IO[0..len]` (copy it out before the next call).
- Load: `IO[0..len] = blob; rc = vpet_load(len)`.
- Inspect: `len = vpet_inspect(); view IO[0..len] as Inspect`.
- `now_ms` is `u64`. In JS it is passed as a `BigInt`; wasm `i64` parameters are supported in all
  current browsers. Chosen over a lo/hi pair so the native and wasm signatures are identical.

## Frame format

32x16, one bit per pixel, row-major, 4 bytes per row, **MSB is the leftmost pixel**, 1 = pixel on
(lit on the OLED, dark on an LCD theme). Pixel `(x, y)` is bit `7 - (x % 8)` of byte
`y * 4 + x / 8`.

Row-major was chosen over SSD1306 page order because every host has to expand to 4x anyway, and
row-major frames are readable in test output.

## Buttons

`buttons` is a **held-state mask**: bit 0 = A, bit 1 = B, bit 2 = C. The host reports what is
currently down; the core detects rising edges and long presses (A+C held for 1000 ms on the
tombstone restarts). Held-state was chosen over press events because it is the natural GPIO
model, it is trivially logged, and it survives dropped events.

Inputs are applied after the fast-forward, at sim second `floor(now_ms / 1000)`. Menu cursor
movement is UI state and is immediate.

## Update flags

| Bit | Name | Host should |
|---|---|---|
| 0 | `FRAME_CHANGED` | copy the frame to the display |
| 1 | `SAVE_NEEDED` | persist (debounced: web 500 ms, ESP32 10 s) and sync |
| 2 | `BEEP` | make a short sound if it can |
| 3 | `ATTENTION` | the pet is calling; may blink an LED or badge |

`SAVE_NEEDED` is set when state changed materially (an event fired, an action applied, a stage
changed), not on every animation tick. Hosts also checkpoint periodically (web 30 s, ESP32
5 min) and on hide/close.

## Load errors

| Code | Name | Meaning |
|---|---|---|
| -1 | `BAD_MAGIC` | not a VPET blob |
| -2 | `BAD_CRC` | corrupt |
| -3 | `VERSION_TOO_NEW` | written by a newer core; **do not overwrite the stored blob** |
| -4 | `TOO_LONG` | `len` > `VPET_IO_CAP` or > declared payload |
| -5 | `DECODE` | postcard failed |
| -6 | `BAD_CONTENT_REF` | species id not in this build's registry |

On any error the core stays in its previous state (Uninit after instantiation).

## Inspect layout

`#[repr(C)]`, little-endian, fixed. For dev panels, tests, and the ESP32 sleep hint only; the
status screen is rendered by the core.

```rust
#[repr(C)]
pub struct Inspect {
    pub abi_version: u32,
    pub save_version: u16,
    pub content_hash: u32,
    pub state: u8,          // 0 Uninit, 1 Alive, 2 Dead
    pub species: u8,
    pub stage: u8,          // 0 Egg, 1 Baby, 2 Child, 3 Adult, 4 AdultAlt
    pub hunger: u8,
    pub happiness: u8,
    pub discipline: u8,
    pub health: u8,
    pub weight: u16,
    pub age_secs: u32,
    pub flags: u8,          // bit0 sick, bit1 sleeping, bit2 lights_off, bit3 attention_any
    pub attention: u8,      // hungry, sad, poop, sick, sleepy, tantrum
    pub poops: u8,
    pub care_mistakes: u8,
    pub sim_now: u32,
    pub next_event_at: u32, // Sec; NEVER (u32::MAX) if none. ESP32 may sleep until then.
}
```

## Versioning

- `ABI_VERSION` bumps on any change to exports, buffer sizes, frame format, flag bits, error
  codes, or `Inspect`. Hosts check it once at startup.
- Save format versioning is separate ([SAVE_FORMAT.md](SAVE_FORMAT.md)).
- `content_hash` identifies the compiled assets and is recorded in every save.

## Host flow

```
instantiate
  assert vpet_abi_version() == EXPECTED
  if stored blob: copy to IO; rc = vpet_load(len); if rc == VERSION_TOO_NEW: show message, stop
  else: vpet_reset(now_ms, entropy)
loop every 50..100 ms:
  flags = vpet_update(now_ms, buttons_mask)
  if flags & FRAME_CHANGED: blit(frame_ptr, 64)
  if flags & SAVE_NEEDED: schedule_save()
  if flags & BEEP: beep()
on hide/close: save now
```
