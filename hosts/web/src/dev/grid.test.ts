// Parser parity: tools/spritekit/spritekit/grid.py's `parse`/`pack_image`/`pack_mask` and this
// module's TS port must pack every real asset identically, or `/dev/sprites` (docs/CONTENT.md
// "Hot preview without rebuilding the core") would silently show something different from what
// `spritekit compile` bakes into the shipped core. `tests/fixtures/packed.json` is the Python
// side's output (`spritekit dev-fixture`), keyed the same way this test recomputes: `<path
// relative to assets/>::<sprite name>`.
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative, resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { hex, packImage, packMask, parse } from "./grid";

const ASSETS_ROOT = resolve(__dirname, "../../../../assets");
const FIXTURE_PATH = resolve(__dirname, "../../../../tests/fixtures/packed.json");

interface FixtureEntry {
  w: number;
  h: number;
  img: string;
  mask: string | null;
}

function listTxtFiles(dir: string): string[] {
  const out: string[] = [];
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    const st = statSync(full);
    if (st.isDirectory()) {
      out.push(...listTxtFiles(full));
    } else if (entry.endsWith(".txt")) {
      out.push(full);
    }
  }
  return out;
}

describe("grid.ts parser parity with spritekit/grid.py", () => {
  const fixture: Record<string, FixtureEntry> = JSON.parse(readFileSync(FIXTURE_PATH, "utf-8"));
  const files = [...listTxtFiles(join(ASSETS_ROOT, "global")), ...listTxtFiles(join(ASSETS_ROOT, "species"))];

  it("found the fixture and at least one asset file (setup sanity)", () => {
    expect(Object.keys(fixture).length).toBeGreaterThan(0);
    expect(files.length).toBeGreaterThan(0);
  });

  it("packs the same img/mask bytes as the Python side for every sprite", () => {
    const seenKeys = new Set<string>();
    for (const file of files) {
      const rel = relative(ASSETS_ROOT, file).split("\\").join("/"); // posix-style on any OS
      const text = readFileSync(file, "utf-8");
      const gf = parse(file, text);
      for (const sprite of gf.sprites) {
        const key = `${rel}::${sprite.name}`;
        seenKeys.add(key);
        const expected = fixture[key];
        expect(expected, `fixture is missing ${key}; regenerate with 'spritekit dev-fixture'`).toBeDefined();

        expect(gf.cellW).toBe(expected.w);
        expect(gf.cellH).toBe(expected.h);

        const img = packImage(sprite.rows, gf.cellW);
        expect(hex(img)).toBe(expected.img);

        const mask = packMask(sprite.rows, gf.cellW);
        expect(mask === null ? null : hex(mask)).toBe(expected.mask);
      }
    }
    // Catches the fixture going stale in the other direction too: a sprite Python packed that
    // the TS side never saw (a file the glob missed, a sprite the TS parser silently dropped).
    const missing = Object.keys(fixture).filter((k) => !seenKeys.has(k));
    expect(missing, "TS parser did not produce these fixture keys").toEqual([]);
  });
});
