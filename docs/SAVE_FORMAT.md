# Save format

The blob is a snapshot of `Cart` minus the non-persisted fields. Any host that has the blob and
"now" can reconstruct the pet. There is no input log inside it
([ADR 0006](adr/0006-no-input-log-in-save.md)).

## Layout

```
offset  size  field
0       4     magic "VPET"
4       2     save_version  u16 LE   (first version is 1)
6       4     content_hash  u32 LE   (assets the writer was built with; informational)
10      2     payload_len   u16 LE
12      n     payload       postcard(SaveVn)      ~80..130 bytes
12+n    4     crc32         u32 LE over bytes [0, 12+n)
```

Typical total: about 100 to 150 bytes. `VPET_IO_CAP` (2048) is the hard ceiling.

## Serialization

**postcard** with serde derives: `no_std`, `to_slice` into the IO buffer, varint-compact, about
2 KB of code. JSON was rejected (needs alloc and a parser on the ESP32), bincode (larger,
std-oriented), hand-rolled (error-prone across versions).

Postcard is not self-describing. **Any edit to a persisted struct requires a version bump.** A
schema snapshot test serialises a fixed `Cart` and compares bytes; it fails CI if someone
forgets.

## Payload contents (v1)

```rust
#[derive(Serialize, Deserialize)]
pub struct SaveV1 {
    pub state: u8,
    pub sim_now: u32,
    pub rng: [u32; 4],
    pub pet: PetV1,          // every field of Pet, meters as (v0, t0, step_secs, dir, paused)
    pub timers: [u32; 15],
    pub ui: UiV1,
    pub attention_since: [u32; 6],
}
```

## Versioning and migration

- `crates/vpet-core/src/save/v1.rs`, `v2.rs`, ... each hold a frozen struct. The current version
  is a type alias. Loading decodes by the header version and converts forward with
  `impl From<SaveV1> for SaveV2`, chained.
- Loading a **newer** version than the core knows returns `VERSION_TOO_NEW`. The host must keep
  the stored blob untouched. This protects an old ESP32 firmware from clobbering a newer browser
  save.
- `content_hash` mismatch does not fail the load. The species id is resolved against this
  build's registry; if it is missing, `BAD_CONTENT_REF`. Timers and meters are taken as-is; a
  content change that alters step values takes effect at the next re-anchor.
- The registry is append-only so species ids never change meaning.

## What is not in the blob

- `prev_buttons`, `hold_since_ms`, `AnimState`: derived or transient.
- Any wall-clock timestamp of the save itself. The server records `saved_at` in its own
  metadata; the core does not trust host clocks beyond `now_ms`.
- Host settings (server URL, token, pet id).

## Tests

- Round trip: `save(load(save(s))) == save(s)` for random states, and behaviour after a round
  trip matches behaviour without one (property test).
- Snapshot: a fixed state serialises to fixed bytes (guards layout drift).
- Corruption: a flipped bit fails with `BAD_CRC`; a truncated blob with `TOO_LONG` or `DECODE`;
  a future version with `VERSION_TOO_NEW`; each leaves the core state unchanged.
