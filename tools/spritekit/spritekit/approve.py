"""`spritekit approve`: records a sha256 per pose into `assets/approved.lock`. `compile` refuses
unapproved poses by default (docs/art/ART_PIPELINE.md); `validate --all` fails if an approved
pose's hash no longer matches what's on disk (the CI gate in docs/CONTENT.md), so an edit
requires re-approval, not just a passing validator.
"""

from __future__ import annotations

import hashlib
import tomllib
from pathlib import Path

from .grid import parse

LOCK_HEADER = (
    "# sha256 per approved pose, written by `spritekit approve`. CI fails if an approved pose\n"
    "# changes without re-approval (docs/art/ART_PIPELINE.md).\n"
)


def pose_key(slug: str, stage: str, pose: str) -> str:
    return f"{slug}/{stage}/{pose}"


def sha256_of_pose(rows: list[str]) -> str:
    h = hashlib.sha256()
    for line in rows:
        h.update(line.encode("utf-8"))
        h.update(b"\n")
    return "sha256:" + h.hexdigest()


def load_lock(assets_root: Path) -> dict[str, str]:
    path = assets_root / "approved.lock"
    if not path.is_file():
        return {}
    text = path.read_text(encoding="utf-8")
    # Strip full-line comments before parsing as TOML (the header above isn't valid TOML
    # otherwise, and keeping the file human-readable matters more than a strict format).
    toml_text = "\n".join(l for l in text.split("\n") if not l.lstrip().startswith("#"))
    if not toml_text.strip():
        return {}
    return tomllib.loads(toml_text)


def write_lock(assets_root: Path, entries: dict[str, str]) -> None:
    path = assets_root / "approved.lock"
    lines = [LOCK_HEADER.rstrip("\n")]
    for key in sorted(entries):
        lines.append(f'"{key}" = "{entries[key]}"')
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def approve_species(assets_root: Path, slug: str, stage: str | None = None) -> list[str]:
    """Approves every pose of `slug` (or just `stage` if given). Returns the list of pose keys
    that were newly approved or re-approved (hash changed)."""
    species_dir = assets_root / "species" / slug
    if not species_dir.is_dir():
        raise FileNotFoundError(f"no such species: {species_dir}")

    entries = load_lock(assets_root)
    changed = []
    stages = [stage] if stage else [p.stem for p in sorted(species_dir.glob("*.txt"))]
    for st in stages:
        f = species_dir / f"{st}.txt"
        if not f.is_file():
            continue
        gf = parse(str(f), f.read_text(encoding="utf-8"))
        for sprite in gf.sprites:
            key = pose_key(slug, st, sprite.name)
            digest = sha256_of_pose(sprite.rows)
            if entries.get(key) != digest:
                changed.append(key)
            entries[key] = digest

    write_lock(assets_root, entries)
    return changed


def unapproved_or_changed_poses(assets_root: Path, slug: str) -> list[str]:
    """Poses that are missing from approved.lock, or whose on-disk hash no longer matches the
    locked one. Used by `compile` (refuses unapproved poses) and `validate --all` (fails on a
    changed approved pose)."""
    species_dir = assets_root / "species" / slug
    entries = load_lock(assets_root)
    problems = []
    for f in sorted(species_dir.glob("*.txt")):
        stage = f.stem
        gf = parse(str(f), f.read_text(encoding="utf-8"))
        for sprite in gf.sprites:
            key = pose_key(slug, stage, sprite.name)
            digest = sha256_of_pose(sprite.rows)
            locked = entries.get(key)
            if locked is None:
                problems.append(f"{key}: not approved")
            elif locked != digest:
                problems.append(f"{key}: changed since approval (re-run `spritekit approve`)")
    return problems
