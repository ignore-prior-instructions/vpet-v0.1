// A small TS port of tools/spritekit/spritekit/validate.py's dims/ground/centred/density
// checks, for `/dev/sprites`'s live badges (docs/CONTENT.md "Hot preview without rebuilding
// the core"). The full validator (connectivity, symmetry, coherence, eyes, dither, ...) stays
// Python-only, run via `spritekit validate` / `just art-check` -- these four are just enough to
// catch the most common slip while drawing (off the ground row, mis-sized, badly off-centre,
// or way too sparse/solid) without duplicating the whole pipeline in the browser.
//
// Thresholds below mirror assets/asset-spec.toml's `[rules.pet]` table exactly; that file is
// the source of truth; keep these in sync by hand if it changes (no TOML parser in this
// dev-only page, to avoid a new dependency for four numbers).
import { bbox } from "./grid";

export const GROUND_ROW = 31;
export const CENTER_TOLERANCE = 1.0;
export const DENSITY_RANGE: [number, number] = [0.45, 0.85];
export const BBOX_RANGE: Record<"baby" | "child" | "adult", { w: [number, number]; h: [number, number] }> = {
  baby: { w: [12, 20], h: [12, 20] },
  child: { w: [18, 30], h: [20, 30] }, // 30: the hand-drawn ghost's happy pose (ADR 0017)
  adult: { w: [22, 32], h: [24, 32] },
};

export interface Badge {
  label: string;
  ok: boolean;
  /** true for a check that is out of band but only a warning here (ADR 0017's `@style
   * outline` downgrade) -- distinct from `ok`, since it should render neither green nor red. */
  warn?: boolean;
  detail: string;
}

function groundBadge(rows: string[], h: number): Badge {
  const bb = bbox(rows);
  if (!bb) return { label: "ground", ok: false, detail: "pose is entirely blank" };
  const [, , , y1] = bb;
  const groundRow = Math.min(GROUND_ROW, h - 1);
  const onInGroundRow = [...rows[groundRow]].filter((c) => c === "#" || c === "o").length;
  const ok = y1 === groundRow && onInGroundRow >= 2;
  return {
    label: "ground",
    ok,
    detail: ok ? `feet on row ${groundRow}` : `bbox bottom row ${y1}, expected ${groundRow}; ${onInGroundRow} on-pixel(s) there`,
  };
}

function centredBadge(rows: string[], w: number): Badge {
  const bb = bbox(rows);
  if (!bb) return { label: "centred", ok: true, detail: "blank" };
  const [x0, , x1] = bb;
  const centre = (x0 + x1) / 2;
  const ideal = (w - 1) / 2;
  const ok = Math.abs(centre - ideal) <= CENTER_TOLERANCE;
  return { label: "centred", ok, detail: `centre ${centre.toFixed(1)}px, ideal ${ideal.toFixed(1)}px` };
}

function dimsBadge(rows: string[], stage: "baby" | "child" | "adult"): Badge {
  const bb = bbox(rows);
  const range = BBOX_RANGE[stage];
  if (!bb) return { label: "dims", ok: false, detail: "pose is entirely blank" };
  const [x0, y0, x1, y1] = bb;
  const bw = x1 - x0 + 1;
  const bh = y1 - y0 + 1;
  const ok = bw >= range.w[0] && bw <= range.w[1] && bh >= range.h[0] && bh <= range.h[1];
  return {
    label: "dims",
    ok,
    detail: `${bw}x${bh}, want ${range.w[0]}-${range.w[1]}x${range.h[0]}-${range.h[1]}`,
  };
}

/** `soft`: true for `@style outline` files (ADR 0017) -- density is one of the checks the
 * Python validator downgrades to a warning for hand-drawn art, so an out-of-band density here
 * is shown amber ("soft"), not red, to match. */
function densityBadge(rows: string[], soft: boolean): Badge {
  const bb = bbox(rows);
  if (!bb) return { label: "density", ok: true, detail: "blank" };
  const [x0, y0, x1, y1] = bb;
  const area = (x1 - x0 + 1) * (y1 - y0 + 1);
  let on = 0;
  for (let y = y0; y <= y1; y++) {
    for (let x = x0; x <= x1; x++) {
      const c = rows[y][x];
      if (c === "#" || c === "o") on++;
    }
  }
  const density = area > 0 ? on / area : 0;
  const [lo, hi] = DENSITY_RANGE;
  const ok = density >= lo && density <= hi;
  return {
    label: "density",
    ok,
    warn: !ok && soft,
    detail: `${(density * 100).toFixed(0)}% within bbox, want ${lo * 100}-${hi * 100}%${!ok && soft ? " (warning only: @style outline)" : ""}`,
  };
}

/** `stage` only matters for the `dims` check's per-stage bbox range; pass `null` to skip it
 * (globals, or a stage this table doesn't cover). `style` softens the density badge the same
 * way `tools/spritekit/spritekit/validate.py` does for hand-drawn `@style outline` art. */
export function badgesFor(
  rows: string[],
  w: number,
  h: number,
  stage: "baby" | "child" | "adult" | null,
  style?: string,
): Badge[] {
  const badges = [groundBadge(rows, h), centredBadge(rows, w), densityBadge(rows, style === "outline")];
  if (stage) badges.splice(1, 0, dimsBadge(rows, stage));
  return badges;
}
