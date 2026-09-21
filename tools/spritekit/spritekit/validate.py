"""Validator checks. See docs/art/ART_PIPELINE.md's validator table and
docs/CONTENT.md/asset-spec.toml for thresholds.

Phase 0 scope (global assets only; no species exist yet): dims/alphabet (enforced by the parser
itself — a `GridError` becomes a file-level error here), strays (isolated on-pixels and
unexplained 1px holes), and the icon-class blank-border/density rule. The full pet-pose
validator (bbox, symmetry, eyes, coherence, connectivity, dither) is exercised starting Phase 3
when the first species' text grids exist; the functions below are written generally so it will
extend rather than rewrite when that lands.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path

from .grid import ON_CHARS, GridError, Sprite, parse
from .spec import Spec


@dataclass
class Report:
    file: str
    sprite: str | None
    errors: list[str] = field(default_factory=list)
    warnings: list[str] = field(default_factory=list)
    metrics: dict = field(default_factory=dict)

    @property
    def ok(self) -> bool:
        return not self.errors

    def to_json(self) -> dict:
        return {
            "file": self.file,
            "sprite": self.sprite,
            "errors": self.errors,
            "warnings": self.warnings,
            "metrics": self.metrics,
        }


def _at(rows: list[str], x: int, y: int, w: int, h: int) -> str | None:
    if 0 <= x < w and 0 <= y < h:
        return rows[y][x]
    return None


def stray_and_hole_errors(rows: list[str], w: int, h: int) -> list[str]:
    errors = []
    for y in range(h):
        for x in range(w):
            c = rows[y][x]
            if c in ON_CHARS:
                lit_neighbours = sum(
                    1
                    for dy in (-1, 0, 1)
                    for dx in (-1, 0, 1)
                    if not (dx == 0 and dy == 0)
                    and (_at(rows, x + dx, y + dy, w, h) in ON_CHARS)
                )
                if lit_neighbours == 0:
                    errors.append(f"stray on-pixel at ({x},{y}): isolated, no 8-neighbours lit")
            elif c == ".":
                neigh = [
                    _at(rows, x - 1, y, w, h),
                    _at(rows, x + 1, y, w, h),
                    _at(rows, x, y - 1, w, h),
                    _at(rows, x, y + 1, w, h),
                ]
                in_bounds = [n for n in neigh if n is not None]
                if len(in_bounds) == 4 and all(n in ON_CHARS for n in in_bounds):
                    errors.append(
                        f"1px hole at ({x},{y}): off-pixel fully enclosed by on-pixels "
                        "(use '-' if this is an intentional eye hole)"
                    )
    return errors


def icon_errors(rows: list[str], w: int, h: int, spec: Spec) -> tuple[list[str], dict]:
    errors = []
    rule = spec.rules_icon
    blank_col = rule.get("blank_col", w - 1)
    blank_row = rule.get("blank_row", h - 1)
    for y in range(h):
        if rows[y][blank_col] != ".":
            errors.append(f"icon column {blank_col} must be blank; row {y} has '{rows[y][blank_col]}'")
    row = rows[blank_row]
    if any(c != "." for c in row):
        errors.append(f"icon row {blank_row} must be blank")
    on = sum(1 for row in rows for c in row if c in ON_CHARS)
    total = w * h
    density = on / total if total else 0.0
    lo, hi = rule.get("density", [0.30, 0.70])
    if not (lo <= density <= hi):
        errors.append(f"icon density {density:.2f} outside [{lo},{hi}]")
    return errors, {"density": density}


# Cell class inferred by filename convention (docs/CONTENT.md's `assets/global/` layout).
_GLOBAL_CLASS_BY_STEM = {
    "icons": "icon",
    "items": "item",
    "effects": "effect",
    "egg": "pet",
    "tombstone": "pet",
    "screens": "screen",
    "font3x5": "glyph",
}


def infer_cell_class(path: Path) -> str | None:
    if path.parent.name == "global":
        return _GLOBAL_CLASS_BY_STEM.get(path.stem)
    if path.parent.parent.name == "species":
        return "pet"
    return None


def validate_file(path: Path, spec: Spec) -> list[Report]:
    text = path.read_text(encoding="utf-8")
    rel = str(path)

    file_errors = []
    if "\r" in text:
        file_errors.append("file contains CR: sprite files must use LF line endings")
    for i, line in enumerate(text.split("\n"), start=1):
        stripped_comment = line.split(";", 1)[0]
        if stripped_comment != stripped_comment.rstrip() and stripped_comment.strip() != "":
            file_errors.append(f"line {i}: trailing whitespace")

    try:
        gf = parse(rel, text)
    except GridError as e:
        r = Report(file=rel, sprite=None)
        r.errors.append(str(e))
        r.errors.extend(file_errors)
        return [r]

    cell_class = infer_cell_class(path)
    reports = []
    for sprite in gf.sprites:
        r = Report(file=rel, sprite=sprite.name)
        r.errors.extend(file_errors)
        r.errors.extend(stray_and_hole_errors(sprite.rows, gf.cell_w, gf.cell_h))
        if cell_class == "icon":
            errs, metrics = icon_errors(sprite.rows, gf.cell_w, gf.cell_h, spec)
            r.errors.extend(errs)
            r.metrics.update(metrics)
        reports.append(r)

    if not gf.sprites:
        r = Report(file=rel, sprite=None)
        r.errors.append("file has @cell but no @pose/@sprite blocks")
        r.errors.extend(file_errors)
        reports.append(r)

    return reports


def validate_all(assets_root: Path, spec: Spec) -> list[Report]:
    reports: list[Report] = []
    global_dir = assets_root / "global"
    if global_dir.is_dir():
        for path in sorted(global_dir.glob("*.txt")):
            reports.extend(validate_file(path, spec))
    species_dir = assets_root / "species"
    if species_dir.is_dir():
        for stage_file in sorted(species_dir.glob("*/*.txt")):
            reports.extend(validate_file(stage_file, spec))
    return reports
