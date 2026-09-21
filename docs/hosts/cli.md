# CLI host

`vpet-cli` is the test oracle and the fastest way to poke the core.

| Command | What it does |
|---|---|
| `vpet-cli replay <file.vlog> [--bless] [--verbose]` | run a log; compare (or write) `.expected` at each `snap`; `--verbose` prints every frame |
| `vpet-cli play [--speed N] [--blob file]` | interactive terminal pet: `a`/`b`/`c` keys, ASCII frame redrawn on change, `+` skips an hour |
| `vpet-cli dump <blob>` | print `Inspect` and the decoded save for a blob (hex or file) |
| `vpet-cli parity [--wasm path] <vlog...>` | run each log natively and through `wasmi` on `vpet.wasm`; diff flags, frames, blobs and `Inspect` (cargo feature `parity`) |
| `vpet-cli autoplay --policy caretaker\|sloppy\|neglect --seed N --out file.vlog [--until-secs S] [--every-secs E] [--stop-at-adult]` | play the pet by policy and write the `.vlog` it produced (how the lifecycle goldens are made; docs/TESTING.md) |
| `vpet-cli vlog-from-json` | convert a dev-panel recording to `.vlog` |

Frames print as 16 rows of 32 `#`/`.` characters. Because the core is `no_std`, the CLI links
it as a normal crate with std around it; nothing in the CLI touches simulation logic.
