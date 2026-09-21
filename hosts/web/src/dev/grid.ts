// A TS port of tools/spritekit/spritekit/grid.py's parser and bit-packer (see
// docs/art/SPRITE_FORMAT.md). Deliberately permissive, matching the Python module's own
// docstring: this is the shared parser `/dev/sprites` uses to redraw on every keystroke, not
// the strict validator (that stays Python-only; see docs/CONTENT.md "Hot preview without
// rebuilding the core"). Parity with the Python packer is checked by grid.test.ts against
// tests/fixtures/packed.json (regenerate with `spritekit dev-fixture` after changing either
// side).

export const ALPHABET = ".#o-";
const ON_CHARS = new Set(["#", "o"]);
const MASK_CHARS = new Set(["o", "-"]);

export interface Sprite {
  name: string;
  kind: "pose" | "sprite";
  mouth: [number, number] | null;
  rows: string[]; // `h` strings, each `w` characters from ALPHABET
}

export interface GridFile {
  cellW: number;
  cellH: number;
  meta: Record<string, string>;
  sprites: Sprite[];
}

export class GridError extends Error {
  constructor(
    public path: string,
    public lineNo: number,
    message: string,
  ) {
    super(`${path}:${lineNo}: ${message}`);
  }
}

function stripComment(line: string): string {
  const idx = line.indexOf(";");
  return idx === -1 ? line : line.slice(0, idx);
}

const CELL_RE = /^@cell\s+(\d+)x(\d+)\s*$/;
const META_RE = /^@(species|stage|style)\s+(\S+)\s*$/;
const POSE_RE = /^@(pose|sprite)\s+(\S+)\s*$/;
const MOUTH_RE = /^@mouth\s+(-?\d+)\s*,\s*(-?\d+)\s*$/;

/** Mirrors `grid.py`'s `parse`: same error conditions, same line numbers (1-based). */
export function parse(path: string, text: string): GridFile {
  const rawLines = text.split("\n");
  if (rawLines.length > 0 && rawLines[rawLines.length - 1] === "") {
    rawLines.pop();
  }
  const stripped = rawLines.map(stripComment);

  const isBlank = (i: number) => i >= stripped.length || stripped[i].trim() === "";

  let i = 0;
  while (i < stripped.length && isBlank(i)) i++;
  if (i >= stripped.length) {
    throw new GridError(path, 1, "empty file: expected '@cell WxH' as the first line");
  }

  const cellMatch = CELL_RE.exec(stripped[i]);
  if (!cellMatch) {
    throw new GridError(path, i + 1, `expected '@cell WxH' as the first line, got: ${JSON.stringify(rawLines[i])}`);
  }
  const cellW = parseInt(cellMatch[1], 10);
  const cellH = parseInt(cellMatch[2], 10);
  const gf: GridFile = { cellW, cellH, meta: {}, sprites: [] };
  i++;

  while (!isBlank(i)) {
    const m = META_RE.exec(stripped[i]);
    if (!m) break;
    gf.meta[m[1]] = m[2];
    i++;
  }

  for (;;) {
    while (i < stripped.length && isBlank(i)) i++;
    if (i >= stripped.length) break;

    const poseMatch = POSE_RE.exec(stripped[i]);
    if (!poseMatch) {
      throw new GridError(path, i + 1, `expected '@pose <name>' or '@sprite <name>', got: ${JSON.stringify(rawLines[i])}`);
    }
    const kind = poseMatch[1] as "pose" | "sprite";
    const name = poseMatch[2];
    const headerLine = i + 1;
    i++;

    let mouth: [number, number] | null = null;
    if (i < stripped.length) {
      const mm = MOUTH_RE.exec(stripped[i]);
      if (mm) {
        mouth = [parseInt(mm[1], 10), parseInt(mm[2], 10)];
        i++;
      }
    }

    const rows: string[] = [];
    for (let r = 0; r < cellH; r++) {
      if (i >= stripped.length) {
        throw new GridError(path, headerLine, `pose '${name}': expected ${cellH} rows, found ${r} before EOF`);
      }
      const row = stripped[i];
      if (row.length !== cellW) {
        throw new GridError(path, i + 1, `pose '${name}' row ${r}: expected ${cellW} characters, got ${row.length}: ${JSON.stringify(row)}`);
      }
      const bad = [...new Set(row)].filter((c) => !ALPHABET.includes(c));
      if (bad.length > 0) {
        throw new GridError(path, i + 1, `pose '${name}' row ${r}: invalid character(s) ${JSON.stringify(bad.sort())}, alphabet is '${ALPHABET}'`);
      }
      rows.push(row);
      i++;
    }

    gf.sprites.push({ name, kind, mouth, rows });
  }

  return gf;
}

/** Row-major bit-pack, MSB is the leftmost pixel, stride `ceil(w/8)` bytes. */
export function packBits(bits: number[], w: number): Uint8Array {
  const stride = Math.ceil(w / 8);
  const out = new Uint8Array(stride);
  for (let x = 0; x < bits.length; x++) {
    if (bits[x]) {
      out[x >> 3] |= 1 << (7 - (x % 8));
    }
  }
  return out;
}

export function packImage(rows: string[], w: number): Uint8Array {
  const out: number[] = [];
  for (const row of rows) {
    const bits = [...row].map((c) => (ON_CHARS.has(c) ? 1 : 0));
    out.push(...packBits(bits, w));
  }
  return new Uint8Array(out);
}

/** `null` if the pose has no blink-participating pixels (no mask should be sent). */
export function packMask(rows: string[], w: number): Uint8Array | null {
  const hasMask = rows.some((row) => [...row].some((c) => MASK_CHARS.has(c)));
  if (!hasMask) return null;
  const out: number[] = [];
  for (const row of rows) {
    const bits = [...row].map((c) => (MASK_CHARS.has(c) ? 1 : 0));
    out.push(...packBits(bits, w));
  }
  return new Uint8Array(out);
}

/** `[x0, y0, x1, y1]` inclusive bounds of lit pixels, or `null` if the pose is entirely blank. */
export function bbox(rows: string[]): [number, number, number, number] | null {
  let x0 = Infinity;
  let y0 = Infinity;
  let x1 = -Infinity;
  let y1 = -Infinity;
  for (let y = 0; y < rows.length; y++) {
    const row = rows[y];
    for (let x = 0; x < row.length; x++) {
      if (ON_CHARS.has(row[x])) {
        x0 = Math.min(x0, x);
        x1 = Math.max(x1, x);
        y0 = Math.min(y0, y);
        y1 = Math.max(y1, y);
      }
    }
  }
  if (!Number.isFinite(x0)) return null;
  return [x0, y0, x1, y1];
}

export function hex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}
