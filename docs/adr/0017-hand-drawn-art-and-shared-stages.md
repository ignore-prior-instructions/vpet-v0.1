# 0017: Hand-drawn art keeps its own style; species may share a stage's art

Status: accepted
Date: 2026-09-22

## Context

[ADR 0014](0014-text-grid-art-with-critique-loop.md) built the validator as a ruler for
*generated* art: a model draws each pose once, and the validator's density band, symmetry
threshold, checkerboard limit and mirrored-eye placement are the same checks a critique-loop
prompt self-corrects against. `docs/art/LINEAGE.md` records real hand-drawn source art for this
game's three pet lines (v0's `lalafu`/`ninjifu`/`charamofu` and further Procreate sketches),
predating the validator and the style guide both. It does not fit the generated-art mould: it
is line art, not filled silhouettes; poses are edited freehand, not diffed pixel-for-pixel from
`idle_a`; symmetry is often deliberately broken (a ghost's wave has one arm up at a time). Refit
by the letter of `docs/art/STYLE_GUIDE.md`, most of it fails density, symmetry or checkerboard
checks that exist to keep a *generator* converging, not because the art reads badly at 8x.

Separately, this pass makes the three lines three full species (`lalafu`, `ninjifu`,
`charamofu`), each an adult a child can evolve into. `SpeciesDef` requires a baby, a child, and
an adult per species, but the sketches give exactly one baby (the blob) and one child (the
hooded kid) for all of them.

## Decision

**`@style outline` marks hand-drawn art.** `docs/art/SPRITE_FORMAT.md` already had the header
(silhouette | outline) doing nothing; `spritekit validate` now reads it. For a pet pose under
`@style outline`, the checks that exist to keep a *generator* converging on the style guide's
look -- density, dither/checkerboard, connectivity, symmetry, eye *placement* (top-band and
mirrored-pair), and coherence-vs-`idle_a` -- become warnings instead of errors. The checks that
keep the game itself working stay hard errors regardless of style: file format, dims and
alphabet, ground row, centring, the per-stage bbox band, stray pixels and unintentional holes,
`sleep`'s shape rule, and *presence* of at least one marked eye on `idle_a`/`idle_b` (a pose
with no mask never blinks). `@style silhouette` (the default, unset) keeps every check as an
error, unchanged from before this ADR -- for art the generator loop produces, once a model key
exists in this environment.

**A stage's art may be shared from another species.** `species.toml` gets an `[assets.share]`
table: `baby`/`child`/`adult` mapped to another species' slug means "read `<slug>/<stage>.txt`
for this stage's poses" instead of requiring `<this-species>/<stage>.txt` to exist. The rules in
this species' own `[stages.<stage>]` table are unaffected -- sharing is art only. This is how
`ninjifu` and `charamofu` get a baby (shared from `lalafu`) and `lalafu` and `charamofu` get a
child (shared from `ninjifu`, the only child sketch) without hand-copying files. `adult_alt` is not shareable: ship `adult_alt.txt` or don't declare it.

Ids follow v0's (`lalafu`=1, `charamofu`=2, `ninjifu`=3; `docs/art/LINEAGE.md`), and `compile`
now cross-checks `species.toml`'s own `id` against `registry.toml`'s, since a mismatch would
silently break a `to_species` branch aimed at that slug.

## Consequences

- The style guide (`docs/art/STYLE_GUIDE.md`) describes the look generated art should converge
  on; it is not a gate hand-drawn art must pass pixel-for-pixel. A human still approves every
  pose (`spritekit approve`), same as before.
- `spritekit validate --all`'s warning count is expected to be non-zero for hand-drawn species
  from now on; CI (`just art-check`) only fails on errors, unchanged.
- `hosts/web/src/dev/validators.ts` (the `/dev/sprites` page's four-check TS port) reads the
  same `@style` header and shows an out-of-band density check amber, not red, to match.
- `[assets.share]` means "no `<stage>.txt` in this species' own directory" is not automatically
  a compile error; the error text says which slug to add `[assets.share]` for, or to draw the
  file instead.
- If `spritekit gen`'s critique loop is ever exercised (no model key in this environment yet),
  it should keep targeting `@style silhouette` output; this ADR does not change the generator
  prompt or `docs/art/STYLE_GUIDE.md` itself.
