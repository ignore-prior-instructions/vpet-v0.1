# ninjifu

A cute, masked ninja: a hooded head with two tall pointed ears/hood-points, a closed-eye masked
face, no arms, stubby feet. The adult a fed *and* disciplined child grows into
(`docs/GAME_DESIGN.md` "Evolution", `docs/art/LINEAGE.md`).

- **idle_a / idle_b**: idle_b drops the hood peaks by a couple of rows.
- **happy**: the hood peaks raise higher.
- **eat**: a small mouth notch cut into the lower face, otherwise idle_a.
- **sleep**: full height, eyes drawn shut as a line across the mask band.
- **sad**: the ears wilt to one side. **attack**: a whole-body lunge.

`child.txt` is the hooded kid with small stick arms (sketches 7-10), the same hood family a
couple of stages earlier. It is the **only child stage in the game**: every route passes
through it, so `lalafu` and `charamofu` share it via `[assets.share]`. It also renders during
every evolving transition (`crates/vpet-core/src/lib.rs`, `BusyKind::Evolving` plays the new
species' own child as the "before" pose), which is why sharing one child keeps that consistent.

Drawn by hand from `~/Desktop/vpet-sketches/` (`docs/art/LINEAGE.md`), `@style outline`
(`docs/adr/0017-hand-drawn-art-and-shared-stages.md`).
