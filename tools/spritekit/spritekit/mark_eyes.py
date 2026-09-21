"""`spritekit mark-eyes <file> <pose>`: proposes mirrored off-pixel pairs in the top 45% of the
bbox as blink-mask candidates (docs/art/ART_PIPELINE.md, docs/art/SPRITE_FORMAT.md). Used after
`quantize` (no eye marks survive image import) or as a sanity check on a hand-drawn pose. Never
mutates the file; prints/returns proposals for a human (or the text-grid model) to apply.
"""

from __future__ import annotations

from dataclasses import dataclass

from .grid import ON_CHARS, bbox as compute_bbox


@dataclass
class EyeProposal:
    x: int
    y: int
    mirror_x: int


def propose_eyes(rows: list[str], w: int, h: int, top_fraction: float = 0.45) -> list[EyeProposal]:
    """Off-pixels in the top band whose mirror position is also off, and which sit at a
    plausible eye row (has on-pixels to either side within the bbox, i.e. inside the silhouette,
    not the background) — deliberately conservative to avoid noisy proposals."""
    bb = compute_bbox(rows, w, h)
    if bb is None:
        return []
    x0, y0, x1, y1 = bb
    axis_sum = x0 + x1
    top_limit = y0 + top_fraction * (y1 - y0)

    proposals = []
    seen = set()
    for y in range(y0, min(h, int(top_limit) + 1)):
        row = rows[y]
        for x in range(x0, x1 + 1):
            if row[x] in ON_CHARS:
                continue  # only off-pixels can become eye holes
            mx = axis_sum - x
            if mx <= x or not (x0 <= mx <= x1):
                continue  # only propose each mirrored pair once, from its left half
            if row[mx] in ON_CHARS:
                continue  # mirror must also be off
            # Require on-pixels somewhere else in the row (inside the silhouette), so we don't
            # propose eyes in open background.
            if not any(c in ON_CHARS for c in row):
                continue
            key = (x, y)
            if key not in seen:
                seen.add(key)
                proposals.append(EyeProposal(x=x, y=y, mirror_x=mx))
    return proposals
