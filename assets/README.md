# assets

Everything the pet looks like and everything that tunes it, as text. Species live under
`species/<slug>/` with a `species.toml` and one text-grid file per stage. Shared art (icons,
items, effects, font, egg, tombstone, splash screens) lives under `global/`. `asset-spec.toml`
says what a species must ship and how strict the validator is; `registry.toml` assigns stable
ids; `approved.lock` records which poses a human has signed off.

To look at anything: `uv run spritekit render <file> --sheet` (or `--species <slug>`). To check
everything: `uv run spritekit validate --all`. To rebuild the Rust tables:
`uv run spritekit compile`. See [docs/CONTENT.md](../docs/CONTENT.md) and
[docs/art/ART_PIPELINE.md](../docs/art/ART_PIPELINE.md).
