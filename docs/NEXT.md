# What's next

Open threads as of 2026-09-23, after the pet-lines work (PR #6 and the loose-ends PR after it).
Written so a fresh session can pick one up cold. Each item says where to start and what "done"
looks like. Roadmap phases are in [ROADMAP.md](ROADMAP.md); this is the finer-grained list.

## Where things stand

Three species from David's sketches ([art/LINEAGE.md](art/LINEAGE.md)): every egg is Lalafu,
blob baby → hooded-kid child (shared) → ghost / ninja / dino by care. All seven poses per stage,
spotted egg, hatch flash, tombstone + ghost death scene. Hand-drawn art validates under
`@style outline` ([adr/0017](adr/0017-hand-drawn-art-and-shared-stages.md)). Walking faces its
direction of travel. `just ci` is the definition of green.

## Gameplay and animation

- **Raise one end to end.** Nobody has yet raised a pet from egg to adult through the real UI
  without the dev clock. Start: play in `?dev`, `+1 h` steps with feeds. Done: all three adults
  reached by care, and any rough edge found (meter pacing, attention calls) written up.
- **Battle scene with the real species.** Phase 6 in ROADMAP. The facing convention is now
  consistent (unflipped faces left), so the BattleReady layout should just work; nothing has
  rendered it with these sprites yet. Start: `render_playing`/battle rows in
  [art/ANIMATION.md](art/ANIMATION.md), `crates/vpet-core/src/render/compose.rs`.
- **Ninjifu's blink.** The ninja's default face is closed eyes; the blink XOR briefly "opens"
  them. Harmless, but backwards. Options: no eye marks on the ninja (then it never blinks), or a
  mask that toggles a different pixel pair. Start: `assets/species/ninjifu/adult.txt` rows 18-19.
- **Sleep overlays on tall adults.** 30-tall adults leave two rows for the `zz` effect at
  `head_right`; check it isn't clipped for the ninja. Start: `render_sleeping` in compose.rs.
- **Dev clock jumps starve the pet.** `+1 day` with no feeds kills her, correctly. A dev-only
  "pause meters" toggle would make evolution testing bearable. Start: `hosts/web/src/devpanel.ts`;
  the core must stay deterministic, so this is a host-side time trick, not a core flag.

## Tooling

- **`/dev/sprites` gaps.** It never pushes `attack` (`POSE_NAMES` in
  `hosts/web/src/dev/sprites.ts`) and its badges are a four-check subset of the validator.
- **`spritekit new-species`** scaffolds three stage files and knows nothing about
  `[assets.share]`. Start: `tools/spritekit/spritekit/new_species.py`.
- **`spritekit import-png --scale 2`.** The sketch conversion this pass used was a scratchpad
  script (crop bbox, nearest 2x, bottom-anchor, centre). Worth a real command if more sketches
  arrive. Start: `tools/spritekit/spritekit/cli.py`; docs say it exists but it doesn't.
- **`spritekit gen`** still injects the full silhouette style guide into its prompt. For a
  species marked `@style outline` it should say so. Not exercised: no model key here.
- **`egg_override` / `food_override`** are declared in `species.toml` but `compile` refuses the
  former and ignores the latter.

## Docs debt

- [art/ART_PIPELINE.md](art/ART_PIPELINE.md)'s validator table still quotes the old 16x16
  numbers (`bbox.bottom == 15`, adult w 11..16).
- `docs/art/STYLE_GUIDE.md` is written for generated silhouette art; with three hand-drawn
  species shipped, decide whether it is still the target look or a fallback.

## Roadmap

- **Phase 6, battle + polish**: see above. Battle golden is the "done".
- **Phase 7, ESP32 host**: untouched. The 64x32 frame and held-state buttons were chosen for it;
  the A+C restart hold is a two-finger gesture there, which is fine on hardware.
