# lalafu

A small ghost: a soft rounded body with a small bow-shaped peak on top, two dot eyes, tiny nub
arms and feet, a scalloped bottom. This is an **adult**: the one a well-fed but undisciplined
child grows into (`docs/GAME_DESIGN.md` "Evolution", `docs/art/LINEAGE.md`).

- **idle_a / idle_b**: idle_b drops the bow-peak by one row (a small bob).
- **happy**: the bow-peak raises higher.
- **eat**: a small mouth notch, otherwise idle_a.
- **sleep**: compact, the bow-peak and head tucked away, same ground contact as idle_a but
  noticeably shorter.
- **sad**: the bow wilts to one side. **attack**: a whole-body lunge.

Eyes: the eye row reads wall, dot, dot, wall; the two *interior dots* are the blink marks,
never the wall (a blink is an XOR, so a wall pixel in the mask disappears for a tick).

Every egg hatches as lalafu. `baby.txt` here is the blob every route starts from; the child is
the hooded kid, the only child sketch there is, shared from `ninjifu/child.txt` via
`[assets.share]` in `species.toml`.

Drawn by hand from `~/Desktop/vpet-sketches/` (`docs/art/LINEAGE.md`), not generated.
`@style outline` (`docs/adr/0017-hand-drawn-art-and-shared-stages.md`).
