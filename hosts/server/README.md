# vpet-server

The blob store from [docs/SYNC.md](../../docs/SYNC.md) and
[docs/hosts/server.md](../../docs/hosts/server.md): one save per pet, last-writer-wins by the
blob's own simulated time, four endpoints, bearer token. It links `vpet-core` to validate
blobs and never runs the simulation.

## Run locally

```bash
VPET_TOKEN=dev cargo run -p vpet-server        # or: just server
curl http://127.0.0.1:8080/v1/health
```

| Env | Default | Meaning |
|---|---|---|
| `VPET_TOKEN` | required | bearer token every `/v1/pets/*` request must carry |
| `VPET_DATA` | `./data` | storage directory (`<id>/pet.bin` + `meta.json`) |
| `VPET_BIND` | `127.0.0.1:8080` | listen address |
| `VPET_CORS_ORIGINS` | the Pages origin and `http://localhost:5173` | comma-separated allowed origins |

## Deploy

Not deployed yet (Phase 5 built it; hosting is a later step). The image is ready:

```bash
docker build -f hosts/server/Dockerfile -t vpet-server .     # from the repo root
docker run -d -e VPET_TOKEN=<secret> -p 8080:8080 -v vpet-data:/data vpet-server
```

Put it behind TLS (Caddy, or a platform edge such as Fly). Back up the `/data` volume; blobs
are ~150 bytes each. Then enter the URL and token in the web app's settings sheet (gear icon).
