"""`spritekit diff <a> <b>`: pixel diff between two poses (docs/art/ART_PIPELINE.md). Offline,
useful while hand-drawing: xor count, bbox and centroid deltas."""

from __future__ import annotations

from dataclasses import dataclass

from .grid import ON_CHARS, bbox as compute_bbox


@dataclass
class DiffResult:
    xor_count: int
    bbox_a: tuple[int, int, int, int] | None
    bbox_b: tuple[int, int, int, int] | None
    centroid_a: tuple[float, float] | None
    centroid_b: tuple[float, float] | None


def _centroid(rows: list[str], w: int, h: int) -> tuple[float, float] | None:
    pts = [(x, y) for y in range(h) for x in range(w) if rows[y][x] in ON_CHARS]
    if not pts:
        return None
    return (sum(p[0] for p in pts) / len(pts), sum(p[1] for p in pts) / len(pts))


def diff_poses(rows_a: list[str], rows_b: list[str], w: int, h: int) -> DiffResult:
    if len(rows_a) != h or len(rows_b) != h:
        raise ValueError("both poses must have the same height as the grid")
    xor = sum(
        1
        for y in range(h)
        for x in range(w)
        if (rows_a[y][x] in ON_CHARS) != (rows_b[y][x] in ON_CHARS)
    )
    return DiffResult(
        xor_count=xor,
        bbox_a=compute_bbox(rows_a, w, h),
        bbox_b=compute_bbox(rows_b, w, h),
        centroid_a=_centroid(rows_a, w, h),
        centroid_b=_centroid(rows_b, w, h),
    )
