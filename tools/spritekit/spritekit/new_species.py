"""`spritekit new-species <slug>`: scaffolds a new species (docs/CONTENT.md "Adding a species"
step 1). Appends `registry.toml` (append-only: never renumber), writes `species.toml` with the
documented defaults, empty stage files with headers, and a `BRIEF.md` stub.
"""

from __future__ import annotations

import re
import tomllib
from pathlib import Path

_SLUG_RE = re.compile(r"^[a-z][a-z0-9_]*$")

SPECIES_TOML_TEMPLATE = """\
[species]
id = {id}
slug = "{slug}"
name = "{name}"
description = "TODO: one line."
visual_traits = ["TODO", "TODO", "TODO"]
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

[[stages.child.evolve.branch]]
to = "adult"

[stages.adult]
hunger_step_secs = 360
happy_step_secs = 540
poop_interval_secs = [7200, 10800]
lifespan_secs = [432000, 864000]
battle = {{ power = 6, defense = 4, hp = 10, crit_pct = 10 }}

[assets]
egg_override  = false
food_override = false
adult_alt     = false
"""

STAGE_FILE_TEMPLATE = """\
@cell 16x16
@species {slug}
@stage {stage}

@pose idle_a
; TODO: draw. Ruler:
; col  0123456789012345
"""


def _next_id(registry: list[dict]) -> int:
    return max((e["id"] for e in registry), default=0) + 1


def new_species(assets_root: Path, slug: str, name: str | None = None) -> Path:
    if not _SLUG_RE.match(slug):
        raise ValueError(f"slug must match {_SLUG_RE.pattern!r}, got {slug!r}")

    species_dir = assets_root / "species" / slug
    if species_dir.exists():
        raise FileExistsError(f"{species_dir} already exists")

    registry_path = assets_root / "registry.toml"
    text = registry_path.read_text(encoding="utf-8")
    toml_only = "\n".join(l for l in text.split("\n") if not l.lstrip().startswith("#"))
    registry = tomllib.loads(toml_only).get("species", []) if toml_only.strip() else []
    if any(e["slug"] == slug for e in registry):
        raise ValueError(f"{slug} is already in registry.toml")
    new_id = _next_id(registry)

    species_dir.mkdir(parents=True)
    (species_dir / "species.toml").write_text(
        SPECIES_TOML_TEMPLATE.format(id=new_id, slug=slug, name=name or slug.capitalize()),
        encoding="utf-8",
    )
    (species_dir / "BRIEF.md").write_text(
        f"# {slug}\n\nTODO: two or three sentences a generator/critic (or a human) can draw from.\n",
        encoding="utf-8",
    )
    for stage in ("baby", "child", "adult"):
        (species_dir / f"{stage}.txt").write_text(
            STAGE_FILE_TEMPLATE.format(slug=slug, stage=stage), encoding="utf-8"
        )

    with open(registry_path, "a", encoding="utf-8") as f:
        f.write(f'\n[[species]]\nid = {new_id}\nslug = "{slug}"\n')

    return species_dir
