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
| `VPET_CORS_ORIGINS` | `https://ignore-prior-instructions.github.io,http://localhost:5173` | comma-separated browser origins allowed by CORS |

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
  locally. Any other `LoadError` is also 422.
- The `X-Sim-Now` header must equal the blob's own `sim_now` (`inspect()` after the throwaway
  load), else 400: a client cannot claim a position its blob does not have. The header exists
  so the compare is explicit in the request, not so it can disagree with the body.
- 409 responses carry the full GET body so the client can adopt without a second round trip.
  Equal `sim_now` is accepted (ties break on server sequence, per SYNC.md).
- Missing or wrong bearer token: 401, compared in constant time.
- CORS from `VPET_CORS_ORIGINS`; `Authorization`, `Content-Type` and `X-Sim-Now` are allowed
  request headers.
- `/v1/health` is unauthenticated for uptime checks.
- `id` must match `[A-Za-z0-9_-]{1,64}`; anything else is 400, so ids can't escape `VPET_DATA`.

## Deployment

Not hosted yet: Phase 5 built and tested the server; putting it on a box is a later step.
`hosts/server/Dockerfile` builds a small image (`docker build -f hosts/server/Dockerfile .`
from the repo root; `VPET_DATA=/data`, port 8080); see `hosts/server/README.md` for the run
line. Any small box will do: a Fly machine, a VPS, a Raspberry Pi at home. Put it behind TLS
(Caddy or Fly's edge). Back up `data/` with a cron `tar`; the blobs are tiny.

Locally, `just server` runs it on `127.0.0.1:8080` with token `dev`; `just web-e2e` builds it
and spawns a copy on port 18081 for the two-browser Playwright test.
