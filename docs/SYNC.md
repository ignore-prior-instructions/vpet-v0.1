# Sync

The server stores one blob per pet. It does not simulate. Clients pull, compare, and push.

## Rule: last writer wins, where "last" is the highest `sim_now` in the blob

Two snapshots of one pet cannot be merged; a pet has one timeline. Among candidates, the one
that has simulated furthest wins. Using `sim_now` from inside the blob, rather than a wall-clock
`saved_at`, means the decision is derived from the pet itself and no host clock is trusted
separately. Ties break on server sequence number.

Documented loss scenario: play offline on two devices in overlapping windows; the device that
simulated furthest wins and the other's actions in that window are lost. Acceptable for one
person with one pet.

## Client protocol (browser and ESP32 alike)

```
on open / visible / boot:
    local  = storage.get()
    remote = GET /v1/pets/{id}            (skip if no server configured or offline)
    best   = argmax(sim_now) over {local, remote}
    vpet_load(best); vpet_update(now, 0)
    if best != local: storage.put(best)
    if best != remote: PUT

on SAVE_NEEDED (debounced: web 2 s, ESP32 10 s) and on hide/close/checkpoint:
    blob = vpet_save()
    storage.put(blob)
    PUT /v1/pets/{id}  X-Sim-Now: <sim_now>
    on 409: body has the server's blob; if its sim_now > ours: vpet_load(it), storage.put(it)
```

The client never deletes the server blob except through an explicit "start over" that calls
`DELETE`.

## Server API

Single Rust binary, `axum`. Bearer token from `VPET_TOKEN`; every request carries
`Authorization: Bearer <token>`. `{id}` is a client-chosen identifier (`default` is fine for one
pet; a UUID if you want several).

| Method | Path | Request | Response |
|---|---|---|---|
| GET | `/v1/pets/{id}` | | 200 `{ "sim_now": u32, "seq": u64, "saved_at": rfc3339, "blob_b64": str }` or 404 |
| PUT | `/v1/pets/{id}` | body `application/octet-stream`; header `X-Sim-Now: u32` | 200 `{ "seq" }`; 409 with the GET body if stored `sim_now` > header; 422 if the blob fails `Cart::load` |
| DELETE | `/v1/pets/{id}` | | 204 |
| GET | `/v1/health` | | 200 |

The server links `vpet-core` natively and calls `Cart::load` on every PUT to reject corrupt or
too-new blobs (a too-new blob from a newer client is accepted only if the server has been
rebuilt; otherwise 422 with a clear message, so the client keeps playing locally).

## Storage

Files on disk: `data/<id>/pet.bin` and `data/<id>/meta.json`, written to a temp file and renamed.
One user, one blob: a database would be ceremony. See [hosts/server.md](hosts/server.md).

## Transport details

- Web pushes on `pagehide` with `fetch(..., { keepalive: true })`.
- The ESP32 pushes only when WiFi is up; if the push fails it retries on the next
  `SAVE_NEEDED` or checkpoint. Its blob is authoritative locally in the meantime.
- Blobs are ~150 bytes; there is no reason to compress or batch.
