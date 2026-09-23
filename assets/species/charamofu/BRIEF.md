# charamofu

A small side-view dinosaur: a long snout, one visible eye, small back spines, stubby legs,
facing right. The adult a neglected child grows into: five or more care mistakes
(`docs/GAME_DESIGN.md` "Evolution", `docs/art/LINEAGE.md`); the "bad care" outcome, hungrier,
poops more, shorter lifespan.

- **idle_a / idle_b**: idle_b drops the head hump by one row.
- **happy**: the head hump raises as a tapered dome.
- **eat**: a small mouth notch cut into the snout, otherwise idle_a.
- **sleep**: full height, the single eye drawn shut as a dash.
- **sad**: the head tilts forward. **attack**: a whole-body lunge.

Only ever reached as the adult. Baby (lalafu's blob) and child (ninjifu's hooded kid) are
shared via `[assets.share]` in `species.toml`; no dino-child sketch exists.

Drawn by hand from `~/Desktop/vpet-sketches/` (`docs/art/LINEAGE.md`), `@style outline`
(`docs/adr/0017-hand-drawn-art-and-shared-stages.md`).
