"""Parser and bit-packer for the text-grid sprite format.

See docs/art/SPRITE_FORMAT.md. This module is deliberately permissive about formatting details
(trailing whitespace, CRLF) that `validate.py` treats as errors — `grid.py` is the shared parser
used by both `validate` (which must see and report those problems) and `render`/`pack` (which
just need the pixels).
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field

ALPHABET = ".#o-"
# '#' and 'o' are lit in the base image; 'o' and '-' participate in the blink mask.
ON_CHARS = {"#", "o"}
MASK_CHARS = {"o", "-"}

_CELL_RE = re.compile(r"^@cell\s+(\d+)x(\d+)\s*$")
_META_RE = re.compile(r"^@(species|stage|style)\s+(\S+)\s*$")
_POSE_RE = re.compile(r"^@(pose|sprite)\s+(\S+)\s*$")
_MOUTH_RE = re.compile(r"^@mouth\s+(-?\d+)\s*,\s*(-?\d+)\s*$")


class GridError(ValueError):
    def __init__(self, path: str, line_no: int, message: str):
        self.path = path
        self.line_no = line_no
        super().__init__(f"{path}:{line_no}: {message}")


@dataclass
class Sprite:
    name: str
    kind: str  # "pose" | "sprite"
    mouth: tuple[int, int] | None
    rows: list[str]  # `h` strings, each `w` characters from ALPHABET
    line_no: int  # line of the @pose/@sprite header, for error messages


@dataclass
class GridFile:
    path: str
    cell_w: int
    cell_h: int
    meta: dict[str, str] = field(default_factory=dict)
    sprites: list[Sprite] = field(default_factory=list)
    raw_lines: list[str] = field(default_factory=list)  # for validate.py's formatting checks


def _strip_comment(line: str) -> str:
    idx = line.find(";")
    return line if idx == -1 else line[:idx]


def parse(path: str, text: str) -> GridFile:
    raw_lines = text.split("\n")
    # A trailing newline produces one empty element at the end; drop it so line numbers below
    # correspond to 1-based source lines without an off-by-one.
    if raw_lines and raw_lines[-1] == "":
        raw_lines.pop()

    stripped = [_strip_comment(l) for l in raw_lines]

    def is_blank(i: int) -> bool:
        return i >= len(stripped) or stripped[i].strip() == ""

    i = 0
    while is_blank(i) and i < len(stripped):
        i += 1
    if i >= len(stripped):
        raise GridError(path, 1, "empty file: expected '@cell WxH' as the first line")

    m = _CELL_RE.match(stripped[i])
    if not m:
        raise GridError(path, i + 1, f"expected '@cell WxH' as the first line, got: {raw_lines[i]!r}")
    cell_w, cell_h = int(m.group(1)), int(m.group(2))
    gf = GridFile(path=path, cell_w=cell_w, cell_h=cell_h, raw_lines=raw_lines)
    i += 1

    # File-level metadata headers (@species/@stage/@style), zero or more, in any order.
    while not is_blank(i):
        m = _META_RE.match(stripped[i])
        if not m:
            break
        gf.meta[m.group(1)] = m.group(2)
        i += 1

    # Sprite/pose blocks.
    while True:
        while is_blank(i) and i < len(stripped):
            i += 1
        if i >= len(stripped):
            break

        m = _POSE_RE.match(stripped[i])
        if not m:
            raise GridError(
                path, i + 1, f"expected '@pose <name>' or '@sprite <name>', got: {raw_lines[i]!r}"
            )
        kind, name = m.group(1), m.group(2)
        header_line = i + 1
        i += 1

        mouth = None
        if i < len(stripped):
            mm = _MOUTH_RE.match(stripped[i])
            if mm:
                mouth = (int(mm.group(1)), int(mm.group(2)))
                i += 1

        rows: list[str] = []
        for r in range(cell_h):
            if i >= len(stripped):
                raise GridError(
                    path,
                    header_line,
                    f"pose '{name}': expected {cell_h} rows, found {r} before EOF",
                )
            row = stripped[i]
            if len(row) != cell_w:
                raise GridError(
                    path,
                    i + 1,
                    f"pose '{name}' row {r}: expected {cell_w} characters, got {len(row)}: {row!r}",
                )
            bad = set(row) - set(ALPHABET)
            if bad:
                raise GridError(
                    path,
                    i + 1,
                    f"pose '{name}' row {r}: invalid character(s) {sorted(bad)!r}, alphabet is '{ALPHABET}'",
                )
            rows.append(row)
            i += 1

        gf.sprites.append(Sprite(name=name, kind=kind, mouth=mouth, rows=rows, line_no=header_line))

    return gf


def pack_bits(bits: list[int], w: int) -> bytes:
    """Row-major bit-pack, MSB is the leftmost pixel, stride `ceil(w/8)` bytes."""
    stride = (w + 7) // 8
    out = bytearray(stride)
    for x, b in enumerate(bits):
        if b:
            out[x // 8] |= 1 << (7 - (x % 8))
    return bytes(out)


def pack_image(rows: list[str], w: int) -> bytes:
    out = bytearray()
    for row in rows:
        bits = [1 if c in ON_CHARS else 0 for c in row]
        out += pack_bits(bits, w)
    return bytes(out)


def pack_mask(rows: list[str], w: int) -> bytes | None:
    """`None` if the pose has no blink-participating pixels (no mask should be emitted)."""
    has_mask = any(c in MASK_CHARS for row in rows for c in row)
    if not has_mask:
        return None
    out = bytearray()
    for row in rows:
        bits = [1 if c in MASK_CHARS else 0 for c in row]
        out += pack_bits(bits, w)
    return bytes(out)


def bbox(rows: list[str], w: int, h: int) -> tuple[int, int, int, int] | None:
    """`(x0, y0, x1, y1)` inclusive bounds of lit ('#'/'o') pixels, or `None` if the pose is
    entirely blank."""
    x0 = y0 = None
    x1 = y1 = None
    for y, row in enumerate(rows):
        for x, c in enumerate(row):
            if c in ON_CHARS:
                x0 = x if x0 is None else min(x0, x)
                x1 = x if x1 is None else max(x1, x)
                y0 = y if y0 is None else min(y0, y)
                y1 = y if y1 is None else max(y1, y)
    if x0 is None:
        return None
    return (x0, y0, x1, y1)
