# Server host

A single Rust binary, `axum`, that stores one blob per pet. It links `vpet-core` natively to
validate blobs and could later render a frame for a status page, but it never runs the
simulation for play ([ADR 0013](../adr/0013-server-axum-file-store.md)).

## Running

```
VPET_TOKEN=<random string> VPET_DATA=./data VPET_BIND=0.0.0.0:8080 vpet-server
```

| Env | Default | Meaning |
|---|---|---|
| `VPET_TOKEN` | required | bearer token every request must carry |
| `VPET_DATA` | `./data` | storage directory |
| `VPET_BIND` | `127.0.0.1:8080` | listen address |

## Storage

```
data/
  default/
    pet.bin      the blob
    meta.json    { "seq": 17, "sim_now": 1700000123, "saved_at": "2026-...", "content_hash": 123 }
```

Writes go to `pet.bin.tmp` then `rename`. A `Mutex` per id serialises PUTs. There is no
database because there is one user and one blob per pet.

## Endpoints

See [SYNC.md](../SYNC.md) for the contract. Implementation notes:

- PUT decodes the body with `Cart::load` before writing; `VERSION_TOO_NEW` is returned as 422
  with a message telling the client the server needs rebuilding, so the client keeps playing
  locally.
- 409 responses carry the full GET body so the client can adopt without a second round trip.
- CORS: allow the Pages origin and `localhost:5173`; the token header must be allowed.
- `/v1/health` is unauthenticated for uptime checks.

## Deployment

Any small box: a Fly machine, a VPS, a Raspberry Pi at home. Put it behind TLS (Caddy or Fly's
edge). Back up `data/` with a cron `tar`; the blobs are tiny.
