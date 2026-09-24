"""Validator checks. Implements the whole table in docs/art/ART_PIPELINE.md's "Validator"
section, with thresholds from `assets/asset-spec.toml`'s `[rules.pet]`/`[rules.icon]`. The
validator is the ruler and mirror the art pipeline is built around (ADR 0014): most bad output
should be rejected here before a critic (model) call is ever spent, and a human drawing poses
by hand uses exactly the same checks.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path

from .grid import MASK_CHARS, ON_CHARS, GridError, bbox as compute_bbox, parse
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


BBox = tuple[int, int, int, int]  # (x0, y0, x1, y1), inclusive


def _at(rows: list[str], x: int, y: int, w: int, h: int) -> str | None:
    if 0 <= x < w and 0 <= y < h:
        return rows[y][x]
    return None


# --- generic structural checks (any cell class) ------------------------------------------------


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


# --- pet-pose checks (32x32 species poses, and the global egg/tombstone at the same cell) ------


def ground_errors(rows: list[str], h: int, bbox: BBox | None, ground_row: int) -> list[str]:
    if bbox is None:
        return ["pose is entirely blank"]
    _, _, _, y1 = bbox
    errors = []
    if y1 != ground_row:
        errors.append(f"bbox bottom is row {y1}, expected {ground_row} (feet must touch the ground row)")
    count = sum(1 for c in rows[ground_row] if c in ON_CHARS) if ground_row < h else 0
    if count < 2:
        errors.append(f"ground row {ground_row} has {count} on-pixel(s), need at least 2")
    return errors


def centred_errors(bbox: BBox | None, w: int, tolerance: float) -> list[str]:
    if bbox is None:
        return []
    x0, _, x1, _ = bbox
    centre = (x0 + x1) / 2
    ideal = (w - 1) / 2
    if abs(centre - ideal) > tolerance:
        return [f"bbox horizontal centre {centre} is more than {tolerance}px from {ideal}"]
    return []


def bbox_range_errors(bbox: BBox | None, w_range: list[int], h_range: list[int]) -> list[str]:
    if bbox is None:
        return ["pose is entirely blank"]
    x0, y0, x1, y1 = bbox
    bw, bh = x1 - x0 + 1, y1 - y0 + 1
    errors = []
    if not (w_range[0] <= bw <= w_range[1]):
        errors.append(f"bbox width {bw} outside [{w_range[0]},{w_range[1]}]")
    if not (h_range[0] <= bh <= h_range[1]):
        errors.append(f"bbox height {bh} outside [{h_range[0]},{h_range[1]}]")
    return errors


def density_errors(rows: list[str], bbox: BBox | None, lo: float, hi: float) -> tuple[list[str], dict]:
    if bbox is None:
        return [], {"density": 0.0}
    x0, y0, x1, y1 = bbox
    area = (x1 - x0 + 1) * (y1 - y0 + 1)
    on = sum(1 for y in range(y0, y1 + 1) for x in range(x0, x1 + 1) if rows[y][x] in ON_CHARS)
    density = on / area if area else 0.0
    if not (lo <= density <= hi):
        return [f"density {density:.2f} within bbox outside [{lo},{hi}]"], {"density": density}
    return [], {"density": density}


def dither_errors(rows: list[str], w: int, h: int, max_checkerboards: int) -> list[str]:
    count = 0
    for y in range(h - 1):
        for x in range(w - 1):
            a = rows[y][x] in ON_CHARS
            b = rows[y][x + 1] in ON_CHARS
            c = rows[y + 1][x] in ON_CHARS
            d = rows[y + 1][x + 1] in ON_CHARS
            if a == d and b == c and a != b:
                count += 1
    if count > max_checkerboards:
        return [f"{count} 2x2 checkerboard pattern(s) found, max {max_checkerboards}"]
    return []


def connected_components(rows: list[str], w: int, h: int) -> list[list[tuple[int, int]]]:
    visited = [[False] * w for _ in range(h)]
    comps: list[list[tuple[int, int]]] = []
    for y in range(h):
        for x in range(w):
            if rows[y][x] in ON_CHARS and not visited[y][x]:
                stack = [(x, y)]
                visited[y][x] = True
                comp: list[tuple[int, int]] = []
                while stack:
                    cx, cy = stack.pop()
                    comp.append((cx, cy))
                    for dy in (-1, 0, 1):
                        for dx in (-1, 0, 1):
                            if dx == 0 and dy == 0:
                                continue
                            nx, ny = cx + dx, cy + dy
                            if 0 <= nx < w and 0 <= ny < h and rows[ny][nx] in ON_CHARS and not visited[ny][nx]:
                                visited[ny][nx] = True
                                stack.append((nx, ny))
                comps.append(comp)
    return comps


def connectivity_errors(rows: list[str], w: int, h: int, max_components: int, min_component_px: int) -> list[str]:
    comps = connected_components(rows, w, h)
    if not comps:
        return []
    comps.sort(key=len, reverse=True)
    main = comps[0]
    errors = []
    if len(comps) > max_components:
        errors.append(f"{len(comps)} disconnected components, max {max_components}")
    for comp in comps[1:]:
        if len(comp) < min_component_px:
            errors.append(f"component of {len(comp)}px at {comp[0]} is smaller than the minimum {min_component_px}px")
        min_dist = min(max(abs(mx - cx), abs(my - cy)) for (cx, cy) in comp for (mx, my) in main)
        if min_dist > 2:
            errors.append(f"component near {comp[0]} is {min_dist - 1}px from the main body (must be within 1px)")
    return errors


def mirror_about_bbox(rows: list[str], w: int, h: int, bbox: BBox) -> list[str]:
    x0, _, x1, _ = bbox
    axis_sum = x0 + x1
    out = []
    for y in range(h):
        row = []
        for x in range(w):
            mx = axis_sum - x
            row.append(rows[y][mx] if 0 <= mx < w else ".")
        out.append("".join(row))
    return out


def symmetry_score(rows: list[str], w: int, h: int, bbox: BBox | None) -> float:
    if bbox is None:
        return 1.0
    mirrored = mirror_about_bbox(rows, w, h, bbox)
    on_count = sum(1 for row in rows for c in row if c in ON_CHARS)
    if on_count == 0:
        return 1.0
    diff = sum(1 for y in range(h) for x in range(w) if (rows[y][x] in ON_CHARS) != (mirrored[y][x] in ON_CHARS))
    return 1.0 - diff / on_count


def symmetry_errors(rows: list[str], w: int, h: int, bbox: BBox | None, threshold: float, pose_name: str) -> tuple[list[str], dict]:
    score = symmetry_score(rows, w, h, bbox)
    if score < threshold:
        return [f"{pose_name} symmetry {score:.2f} below required {threshold:.2f}"], {"symmetry": score}
    return [], {"symmetry": score}


def eyes_presence_errors(rows: list[str], w: int, h: int) -> list[str]:
    """Every idle pose must have at least one marked eye pixel, generated or hand-drawn (ADR
    0017): a pose with no mask never blinks, which is always a mistake for idle_a/idle_b."""
    mask_pixels = [(x, y) for y in range(h) for x in range(w) if rows[y][x] in MASK_CHARS]
    if not mask_pixels:
        return ["no blink-mask pixels ('-'/'o'): idle_a/idle_b require marked eyes"]
    return []


def eyes_placement_errors(rows: list[str], w: int, h: int, bbox: BBox | None, top_fraction: float) -> list[str]:
    """Top-band and mirrored-pair placement, split out from presence (ADR 0017) so a caller can
    route placement into warnings for hand-drawn `@style outline` art while presence stays a
    hard error for every style."""
    mask_pixels = [(x, y) for y in range(h) for x in range(w) if rows[y][x] in MASK_CHARS]
    if not mask_pixels:
        return []
    errors = []
    if bbox is not None:
        x0, y0, x1, y1 = bbox
        top_limit = y0 + top_fraction * (y1 - y0)
        axis_sum = x0 + x1
        mask_set = set(mask_pixels)
        for x, y in mask_pixels:
            if y > top_limit + 1e-9:
                errors.append(f"eye mask pixel at ({x},{y}) is below the top {top_fraction * 100:.0f}% of the bbox")
            mx = axis_sum - x
            if (mx, y) not in mask_set:
                errors.append(f"eye mask pixel at ({x},{y}) has no mirrored partner at ({mx},{y})")
    return errors


def eyes_errors(rows: list[str], w: int, h: int, bbox: BBox | None, top_fraction: float) -> list[str]:
    return eyes_presence_errors(rows, w, h) + eyes_placement_errors(rows, w, h, bbox, top_fraction)


def _eye_centroid(rows: list[str], w: int, h: int) -> tuple[float, float] | None:
    pts = [(x, y) for y in range(h) for x in range(w) if rows[y][x] in MASK_CHARS]
    if not pts:
        return None
    return (sum(p[0] for p in pts) / len(pts), sum(p[1] for p in pts) / len(pts))


def coherence_errors(
    rows: list[str],
    idle_a_rows: list[str],
    w: int,
    h: int,
    lo: int,
    hi: int,
    pose_name: str,
    max_eye_drift_px: float,
) -> tuple[list[str], dict]:
    diff = sum(1 for y in range(h) for x in range(w) if (rows[y][x] in ON_CHARS) != (idle_a_rows[y][x] in ON_CHARS))
    errors = []
    if not (lo <= diff <= hi):
        errors.append(f"{pose_name} differs from idle_a by {diff}px, expected [{lo},{hi}]")
    a_centroid = _eye_centroid(idle_a_rows, w, h)
    b_centroid = _eye_centroid(rows, w, h)
    if a_centroid and b_centroid:
        drift = ((a_centroid[0] - b_centroid[0]) ** 2 + (a_centroid[1] - b_centroid[1]) ** 2) ** 0.5
        if drift > max_eye_drift_px + 1e-9:
            errors.append(f"{pose_name} eye centroid drifted {drift:.2f}px from idle_a, max {max_eye_drift_px}")
    return errors, {"diff_vs_idle_a": diff}


def sleep_errors(rows: list[str], idle_a_rows: list[str], w: int, h: int) -> list[str]:
    sleep_bbox = compute_bbox(rows, w, h)
    idle_bbox = compute_bbox(idle_a_rows, w, h)
    if sleep_bbox is None or idle_bbox is None:
        return []
    errors = []
    if sleep_bbox[3] != idle_bbox[3]:
        errors.append(f"sleep bbox bottom (row {sleep_bbox[3]}) must match idle_a's (row {idle_bbox[3]})")
    sleep_h = sleep_bbox[3] - sleep_bbox[1] + 1
    idle_h = idle_bbox[3] - idle_bbox[1] + 1
    if sleep_h > idle_h:
        errors.append(f"sleep bbox height {sleep_h} must be <= idle_a's height {idle_h}")
    return errors


# --- cell-class dispatch -------------------------------------------------------------------

_GLOBAL_CLASS_BY_STEM = {
    "icons": "icon",
    "items": "item",
    "effects": "effect",
    "egg": "pet",
    "tombstone": "pet",
    # The hatch flash: a 32x32 burst that floats over the egg's spot, no ground contact.
    "hatch": "effect",
    "screens": "screen",
    "font5x7": "glyph",
    # The two 14x14 status-bar hearts live in their own file because the text-grid format is
    # one @cell size per file (docs/art/SPRITE_FORMAT.md) and font5x7.txt's glyphs are 5x7.
    "hearts": "effect",
}


def infer_cell_class(path: Path) -> str | None:
    if path.parent.name == "global":
        return _GLOBAL_CLASS_BY_STEM.get(path.stem)
    if path.parent.parent.name == "species":
        return "pet"
    return None


def _stage_bbox_range(spec: Spec, stage: str) -> tuple[list[int], list[int]]:
    table = spec.rules_pet.get("bbox", {})
    entry = table.get(stage) or table.get("adult")  # adult_alt falls back to adult's range
    return entry["w"], entry["h"]


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
    stage = gf.meta.get("stage")
    style = gf.meta.get("style", "silhouette")
    if style not in ("silhouette", "outline"):
        file_errors.append(f"@style {style!r} is not 'silhouette' or 'outline'")
    by_name = {s.name: s for s in gf.sprites}
    idle_a = by_name.get("idle_a")
    rp = spec.rules_pet

    reports = []
    for sprite in gf.sprites:
        r = Report(file=rel, sprite=sprite.name)
        r.errors.extend(file_errors)
        # Glyphs are exempt from the stray/hole check: a lone dot is a legitimate period or a
        # colon's dot, and a fully-enclosed off-pixel is a legitimate hollow letterform (0, 8,
        # O, Q, D...) at 5x7 resolution, not a mistake to catch.
        if cell_class != "glyph":
            r.errors.extend(stray_and_hole_errors(sprite.rows, gf.cell_w, gf.cell_h))

        if cell_class == "icon":
            errs, metrics = icon_errors(sprite.rows, gf.cell_w, gf.cell_h, spec)
            r.errors.extend(errs)
            r.metrics.update(metrics)

        elif cell_class == "pet":
            # ADR 0017: `@style outline` marks hand-drawn art, checked structurally (dims,
            # ground row, centring, per-stage bbox, stray/hole, sleep shape, eye *presence*)
            # but not for the aesthetic bands the generator/validator loop was built around
            # (density, dither, connectivity, symmetry, eye placement, coherence-vs-idle_a),
            # which become warnings instead of errors. `@style silhouette` (the default) keeps
            # the full strict validator, for generated art.
            sink = r.warnings if style == "outline" else r.errors
            bb = compute_bbox(sprite.rows, gf.cell_w, gf.cell_h)
            r.errors.extend(ground_errors(sprite.rows, gf.cell_h, bb, rp.get("ground_row", 15)))
            r.errors.extend(centred_errors(bb, gf.cell_w, rp.get("center_tolerance", 1.0)))
            sink.extend(dither_errors(sprite.rows, gf.cell_w, gf.cell_h, rp.get("max_checkerboards", 1)))
            sink.extend(
                connectivity_errors(
                    sprite.rows, gf.cell_w, gf.cell_h, rp.get("max_components", 3), rp.get("min_component_px", 3)
                )
            )
            errs, metrics = density_errors(sprite.rows, bb, *rp.get("density", [0.45, 0.85]))
            sink.extend(errs)
            r.metrics.update(metrics)

            if stage in ("baby", "child", "adult", "adult_alt"):
                # `sleep` has its own shape rule (sleep_errors: same bottom, height <= idle_a's)
                # instead of the per-stage bbox band — it is explicitly meant to be shorter/
                # rounder than a standing pose (docs/art/STYLE_GUIDE.md rule 10, docs/art/
                # ART_PIPELINE.md's coherence row for sleep). Bbox band stays a hard error even
                # for outline art: it is the one shape check that keeps stages visibly growing.
                if sprite.name != "sleep":
                    w_range, h_range = _stage_bbox_range(spec, stage)
                    r.errors.extend(bbox_range_errors(bb, w_range, h_range))

                sym_table = rp.get("symmetry", {})
                if sprite.name in sym_table:
                    errs, metrics = symmetry_errors(
                        sprite.rows, gf.cell_w, gf.cell_h, bb, sym_table[sprite.name], sprite.name
                    )
                    if sprite.name in ("happy", "sad") or style == "outline":
                        r.warnings.extend(errs)
                    else:
                        r.errors.extend(errs)
                    r.metrics.update(metrics)

                if sprite.name in rp.get("eyes_required", []):
                    r.errors.extend(eyes_presence_errors(sprite.rows, gf.cell_w, gf.cell_h))
                    sink.extend(
                        eyes_placement_errors(sprite.rows, gf.cell_w, gf.cell_h, bb, rp.get("eyes_top_fraction", 0.45))
                    )

                coh_table = rp.get("coherence", {})
                if sprite.name in coh_table and idle_a is not None and sprite.name != "idle_a":
                    lo, hi = coh_table[sprite.name]
                    errs, metrics = coherence_errors(
                        sprite.rows,
                        idle_a.rows,
                        gf.cell_w,
                        gf.cell_h,
                        lo,
                        hi,
                        sprite.name,
                        coh_table.get("max_eye_drift_px", 1),
                    )
                    sink.extend(errs)
                    r.metrics.update(metrics)

                if sprite.name == "sleep" and idle_a is not None:
                    r.errors.extend(sleep_errors(sprite.rows, idle_a.rows, gf.cell_w, gf.cell_h))

        reports.append(r)

    if not gf.sprites:
        r = Report(file=rel, sprite=None)
        r.errors.append("file has @cell but no @pose/@sprite blocks")
        r.errors.extend(file_errors)
        reports.append(r)

    return reports


def _species_progression_report(species_dir: Path) -> Report | None:
    """Warn (not error) if adult's idle_a bbox area isn't > child's > baby's
    (docs/art/ART_PIPELINE.md validator table, "progression")."""
    areas = {}
    for stage in ("baby", "child", "adult"):
        f = species_dir / f"{stage}.txt"
        if not f.is_file():
            return None
        gf = parse(str(f), f.read_text(encoding="utf-8"))
        idle_a = next((s for s in gf.sprites if s.name == "idle_a"), None)
        if idle_a is None:
            return None
        bb = compute_bbox(idle_a.rows, gf.cell_w, gf.cell_h)
        if bb is None:
            return None
        areas[stage] = (bb[2] - bb[0] + 1) * (bb[3] - bb[1] + 1)

    r = Report(file=str(species_dir), sprite=None)
    if not (areas["adult"] > areas["child"] > areas["baby"]):
        r.warnings.append(
            f"idle_a bbox area should increase baby < child < adult; got "
            f"baby={areas['baby']} child={areas['child']} adult={areas['adult']}"
        )
    return r


def validate_all(assets_root: Path, spec: Spec) -> list[Report]:
    reports: list[Report] = []
    global_dir = assets_root / "global"
    if global_dir.is_dir():
        for path in sorted(global_dir.glob("*.txt")):
            reports.extend(validate_file(path, spec))
    species_dir = assets_root / "species"
    if species_dir.is_dir():
        for slug_dir in sorted(p for p in species_dir.iterdir() if p.is_dir()):
            for stage_file in sorted(slug_dir.glob("*.txt")):
                reports.extend(validate_file(stage_file, spec))
            prog = _species_progression_report(slug_dir)
            if prog is not None:
                reports.append(prog)
    return reports
