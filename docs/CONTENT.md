# Content: species, assets, and the build step

Everything under `assets/` is source. `spritekit compile` turns it into
`crates/vpet-core/src/assets/generated.rs`, which is checked in. `cargo build` never runs Python.

## Layout

```
assets/
  asset-spec.toml          cells, required poses, validator thresholds, global game rules
  approved.lock            sha256 per approved pose; CI fails if an approved pose changes
  registry.toml            append-only species id -> slug
  global/
    icons.txt              8 menu icons, 12x12
    items.txt              food_a/b/c, snack_a/b/c, poop_a/b, 16x16
    effects.txt            heart, heart_small, zz_a, zz_b, skull, sweat, sparkle_a, sparkle_b,
                           hit_star, attention, note, cross, ghost, 16x16
    font5x7.txt            A-Z 0-9 ! ? - . : space, 5x7 (advance 6)
    hearts.txt             heart_full, heart_empty, 14x14 (status pages)
    egg.txt                egg_a, egg_b, 32x32
    tombstone.txt          32x32
    hatch.txt              burst, 32x32 (the hatch flash)
    screens.txt            boot, vs, 64x32
  species/
    lalafu/
      species.toml
      [baby.txt]  [child.txt]  [adult.txt]  [adult_alt.txt]   any stage may be shared, see below
      concept/             optional image-model concept PNGs (git-ignored)
      preview/             spritekit render output (git-ignored)
```

A stage file is optional if `species.toml` shares it from another species (`[assets.share]`,
docs/adr/0017): there is one baby (lalafu's blob) and one child (ninjifu's hooded kid) for
every route, and a species only ever reached as an adult never has its own baby or child
(docs/art/LINEAGE.md). Compile still requires *some*
baby/child/adult art per species (`SpeciesDef` has no optional stages), sharing just points at
where to read it from instead of drawing a copy.

Sprite text format: [art/SPRITE_FORMAT.md](art/SPRITE_FORMAT.md).

## `registry.toml`

```toml
# Append only. Never renumber; saves reference species by id.
[[species]]
id = 1
slug = "lalafu"
```

## `species.toml`

All durations are integer seconds. `spritekit render --species` prints the human equivalents.

```toml
[species]
id = 1                                  # must match registry.toml
slug = "lalafu"
name = "Lalafu"
description = "A small ghost that hatches from a spotted egg."
visual_traits = ["soft rounded blob body", "small bow-shaped peak on top", "two dot eyes"]
hatch_secs = 300

[schedule]
awake_secs = 50400
sleep_secs = 36000

[stages.baby]
stage_secs = 3600
hunger_step_secs = 180
happy_step_secs = 240
poop_interval_secs = [7200, 10800]

[stages.child]
stage_secs = 172800
hunger_step_secs = 300
happy_step_secs = 450
poop_interval_secs = [7200, 10800]

# First match in file order, strictest first (docs/GAME_DESIGN.md "Evolution"). `to_species`
# jumps lineages outright; a branch without it stays in this species. `to = "adult_alt"` (an
# alternate adult within the *same* species, unused by lalafu) is the other option; see
# docs/adr/0017 for why these differ.
[[stages.child.evolve.branch]]
to = "adult"
to_species = "ninjifu"          # fed and disciplined
max_care_mistakes = 4
min_discipline = 40

[[stages.child.evolve.branch]]
to = "adult"                    # fed but undisciplined: lalafu's own adult
max_care_mistakes = 4

[[stages.child.evolve.branch]]
to = "adult"
to_species = "charamofu"        # neglected

[stages.adult]
hunger_step_secs = 360
happy_step_secs = 540
poop_interval_secs = [7200, 10800]
lifespan_secs = [432000, 864000]

[assets]
egg_override  = false        # ships the global egg_a/egg_b poses
food_override = false
adult_alt     = false        # no adult_alt.txt; see the [[stages.child.evolve.branch]] comment

[assets.share]
child = "ninjifu"             # <stage> = "<slug>": read this stage's art from that species
```

Branch fields: `to` (`adult` | `adult_alt`), optional `to_species` (slug), optional
`max_care_mistakes`, `min_discipline`, `max_weight`, `weight` (RNG priority, default 0).

`[assets.share]` fields: `baby`, `child`, and/or `adult` mapped to another species' slug
(docs/adr/0017). Compile reads that species' `<stage>.txt` for the art; this species' own
`[stages.<stage>]` table still supplies the rules (hunger/happy step, `stage_secs`, etc.) if
that stage is ever reached. `adult_alt` cannot be shared -- ship `adult_alt.txt` or don't set
`[assets] adult_alt = true`.
Branches are evaluated in file order; among the first matching set with equal `weight`, one is
drawn from the RNG.

Sprites are found by convention (`<slug>/<stage>.txt`), so the file holds no paths; `[assets]`
declares what optional things exist so `compile` errors on a missing file instead of silently
falling back.

## `asset-spec.toml`

The one file that drives `new-species`, `gen` ordering, `validate` thresholds, `compile`
completeness checks, and the global game constants. See the file itself; keys are commented.
The `[rules.game]` table holds every "global" default from the tuning table in
[GAME_DESIGN.md](GAME_DESIGN.md).

## Generated Rust

`spritekit compile --out crates/vpet-core/src/assets/generated.rs` emits:

```rust
// GENERATED by spritekit 0.1.0 from assets/ (sha256 ...). Do not edit.
pub const CONTENT_HASH: u32 = 0x....;               // FNV-1a over every generated table
pub static SPECIES: &[SpeciesDef] = &[ ... ];
pub static ICONS: [Sprite; 8] = [ ... ];
pub static ITEMS: [Sprite; 8] = [ ... ];
pub static EFFECTS: [Sprite; 12] = [ ... ];
pub static FONT_5X7: [Sprite; 42] = [ ... ];
pub static HEARTS: [Sprite; 2] = [ ... ];
pub static EGG: [Sprite; 2] = [ ... ];
pub static TOMBSTONE: Sprite = ...;
pub static HATCH: Sprite = ...;                       // the hatch burst
pub static SCREENS: [Sprite; 2] = [ ... ];
pub static GAME: GameRules = GameRules { care_miss_secs: 900, ... };
pub fn glyph_index(c: u8) -> Option<u8> { ... }
```

Types live in the hand-written `assets/mod.rs`:

```rust
pub struct Sprite { pub w: u8, pub h: u8, pub rows: &'static [u8] }
pub struct Pose { pub img: Sprite, pub blink: Option<Sprite>, pub mouth: (i8, i8), pub bbox: (u8, u8, u8, u8) }
pub struct StageSet { pub idle_a: Pose, pub idle_b: Pose, pub happy: Pose, pub eat: Pose, pub sleep: Pose,
                      pub sad: Option<Pose>, pub attack: Option<Pose>, pub rules: StageRules }
pub struct SpeciesDef { pub id: u8, pub slug: &'static str, pub name: &'static str, pub hatch_secs: u32,
                        pub schedule: (u32, u32), pub egg: Option<[Sprite; 2]>,
                        pub baby: StageSet, pub child: StageSet, pub adult: StageSet, pub adult_alt: Option<StageSet> }
```

`compile` refuses to emit if any validator error remains and, by default, if any pose is not in
`approved.lock` (`--allow-unapproved` for dev builds). CI runs `spritekit compile --check`,
which regenerates to a string and diffs against the checked-in file.

Size: a species is up to 4 stages x 7 poses x 64 bytes (image + mask) ~ 1.8 KB worst case, ~1 KB
typical. Globals ~0.6 KB. Ten species plus globals stay under 20 KB.

## Hot preview without rebuilding the core

The compositor exists once, in Rust. The dev path feeds it different pixels rather than
reimplementing it in JS:

- `vpet-core` has a cargo feature `dev-overrides` (std, alloc). With it, pose lookup consults an
  overlay map before the static tables, and the ABI gains `dev_set_pose`, `dev_set_global`,
  `dev_clear_overrides`, and `dev_render_clip(species, stage, activity, tick)` which renders any
  tick of any clip without touching simulation state.
- `hosts/web` has a dev-only route `/dev/sprites` that imports `assets/**/*.txt` as raw strings
  via Vite glob imports. On HMR of a text file it re-parses (a ~40-line TS port of the grid
  parser, fuzz-tested against the Python one), calls `dev_set_pose`, and redraws. The page shows
  a pose gallery, a clip selector with a tick scrubber, an 8x OLED-look canvas identical to
  `spritekit render --style oled`, and live validator badges.

Saving a `.txt` therefore updates the running Rust animation in under a second, with no Python
and no cargo.

## Adding a species

1. `spritekit new-species <slug>`: appends to `registry.toml`, writes `species.toml` with
   defaults, stage files with headers, and a `BRIEF.md` stub for the visual description.
2. Fill in `visual_traits` and tune the numbers.
3. `spritekit gen <slug> baby`, then `child`, then `adult` ([art/ART_PIPELINE.md](art/ART_PIPELINE.md)).
4. `spritekit render --species <slug> --sheet --gif`; look; edit text if needed.
5. `spritekit approve <slug>`.
6. `spritekit compile`; commit `generated.rs` with the assets.
