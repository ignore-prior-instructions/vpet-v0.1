// `/dev/sprites`: docs/CONTENT.md "Hot preview without rebuilding the core". Glob-imports every
// `assets/**/*.txt`, feeds parsed poses to vpet-dev.wasm's `dev_set_pose`, and renders real
// composited clips via `dev_render_clip` -- the compositor exists once, in Rust; this page
// never reimplements scene logic in TS, only the text-grid parser (grid.ts) and the wire
// encoding (wire.ts). See crates/vpet-core/src/dev.rs for what each `activity` name does.

import { blit, type Theme } from "../blit";
import { DevCore } from "./devcore";
import { bbox, packImage, packMask, parse, type GridFile } from "./grid";
import { badgesFor, type Badge } from "./validators";

const STAGE_INDEX: Record<string, number> = { baby: 0, child: 1, adult: 2 };
const POSE_NAMES = ["idle_a", "idle_b", "happy", "eat", "sleep", "sad"] as const;
const ACTIVITIES = [
  "egg_idle",
  "egg_hatching",
  "idle",
  "dirty",
  "sick",
  "sleep_lights_on",
  "sleep_lights_off",
  "eat_meal",
  "eat_snack",
  "refuse",
  "discipline",
  "result_win",
  "result_lose",
  "playing",
] as const;

// docs/art/SPRITE_FORMAT.md's default mouth anchor, mirroring
// tools/spritekit/spritekit/compile.py's `_default_mouth`: leftmost-ish, 60% down the bbox.
function defaultMouth(bb: [number, number, number, number] | null): [number, number] {
  if (!bb) return [7, 9];
  const [x0, y0, , y1] = bb;
  return [x0, Math.round(y0 + 0.6 * (y1 - y0))];
}

interface SpeciesFile {
  slug: string;
  stage: "baby" | "child" | "adult";
  gf: GridFile;
}

function loadCatalog(): SpeciesFile[] {
  // Eager + `?raw` so every file is a plain string, statically analyzable by Vite for HMR.
  const modules = import.meta.glob("../../../../assets/species/*/*.txt", {
    eager: true,
    query: "?raw",
    import: "default",
  }) as Record<string, string>;

  const out: SpeciesFile[] = [];
  for (const [path, text] of Object.entries(modules)) {
    const m = /\/species\/([^/]+)\/(baby|child|adult)\.txt$/.exec(path);
    if (!m) continue;
    const [, slug, stage] = m;
    try {
      out.push({ slug, stage: stage as "baby" | "child" | "adult", gf: parse(path, text) });
    } catch (e) {
      console.error(`parse error in ${path}:`, e);
    }
  }
  return out;
}

async function pushOverrides(core: DevCore, catalog: SpeciesFile[]): Promise<void> {
  core.clearOverrides();
  for (const { slug, stage, gf } of catalog) {
    for (const name of POSE_NAMES) {
      const sprite = gf.sprites.find((s) => s.name === name);
      if (!sprite) continue;
      const bb = bbox(sprite.rows);
      const mouth = sprite.mouth ?? defaultMouth(bb);
      core.setPose({
        slug,
        stage: STAGE_INDEX[stage],
        name,
        w: gf.cellW,
        h: gf.cellH,
        img: packImage(sprite.rows, gf.cellW),
        mask: packMask(sprite.rows, gf.cellW),
        mouth,
        bbox: bb ?? [0, 0, 0, 0],
      });
    }
  }
}

function renderGallery(container: HTMLElement, gf: GridFile, stage: "baby" | "child" | "adult"): void {
  container.innerHTML = "";
  for (const name of POSE_NAMES) {
    const sprite = gf.sprites.find((s) => s.name === name);
    const cell = document.createElement("div");
    cell.className = "pose-cell";
    const title = document.createElement("div");
    title.textContent = name;
    cell.appendChild(title);

    if (!sprite) {
      const missing = document.createElement("div");
      missing.textContent = "(not drawn)";
      missing.className = "missing";
      cell.appendChild(missing);
      container.appendChild(cell);
      continue;
    }

    const canvas = document.createElement("canvas");
    canvas.width = gf.cellW;
    canvas.height = gf.cellH;
    canvas.className = "pose-canvas";
    cell.appendChild(canvas);
    drawRawPose(canvas, sprite.rows, gf.cellW, gf.cellH);

    const badges = badgesFor(sprite.rows, gf.cellW, gf.cellH, stage);
    cell.appendChild(renderBadges(badges));
    container.appendChild(cell);
  }
}

function get2dContext(canvas: HTMLCanvasElement): CanvasRenderingContext2D {
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("2D canvas context unavailable");
  return ctx;
}

function drawRawPose(canvas: HTMLCanvasElement, rows: string[], w: number, h: number): void {
  const ctx = get2dContext(canvas);
  const image = ctx.createImageData(w, h);
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const on = rows[y][x] === "#" || rows[y][x] === "o";
      const i = (y * w + x) * 4;
      const v = on ? 255 : 0;
      image.data[i] = image.data[i + 1] = image.data[i + 2] = v;
      image.data[i + 3] = 255;
    }
  }
  ctx.putImageData(image, 0, 0);
}

function renderBadges(badges: Badge[]): HTMLElement {
  const row = document.createElement("div");
  row.className = "badges";
  for (const b of badges) {
    const span = document.createElement("span");
    span.className = `badge ${b.ok ? "ok" : "err"}`;
    span.title = b.detail;
    span.textContent = `${b.label} ${b.ok ? "✓" : "✗"}`;
    row.appendChild(span);
  }
  return row;
}

async function main(): Promise<void> {
  const statusEl = document.getElementById("status") as HTMLElement;
  const speciesSel = document.getElementById("species") as HTMLSelectElement;
  const stageSel = document.getElementById("stage") as HTMLSelectElement;
  const activitySel = document.getElementById("activity") as HTMLSelectElement;
  const tickInput = document.getElementById("tick") as HTMLInputElement;
  const tickLabel = document.getElementById("tick-label") as HTMLElement;
  const canvas = document.getElementById("clip-canvas") as HTMLCanvasElement;
  const gallery = document.getElementById("gallery") as HTMLElement;
  const ctx = get2dContext(canvas);

  for (const a of ACTIVITIES) {
    const opt = document.createElement("option");
    opt.value = a;
    opt.textContent = a;
    activitySel.appendChild(opt);
  }

  let catalog = loadCatalog();
  let core: DevCore;
  try {
    core = await DevCore.load(`${import.meta.env.BASE_URL}vpet-dev.wasm`);
  } catch (e) {
    statusEl.textContent = `Failed to load vpet-dev.wasm -- run 'just wasm-dev' first. (${e})`;
    return;
  }

  function refreshSpeciesOptions(): void {
    const slugs = [...new Set(catalog.map((f) => f.slug))].sort();
    const prev = speciesSel.value;
    speciesSel.innerHTML = "";
    for (const slug of slugs) {
      const opt = document.createElement("option");
      opt.value = slug;
      opt.textContent = slug;
      speciesSel.appendChild(opt);
    }
    if (slugs.includes(prev)) speciesSel.value = prev;
  }

  function refreshStageOptions(): void {
    const stages = catalog.filter((f) => f.slug === speciesSel.value).map((f) => f.stage);
    const prev = stageSel.value;
    stageSel.innerHTML = "";
    for (const stage of ["baby", "child", "adult"] as const) {
      if (!stages.includes(stage)) continue;
      const opt = document.createElement("option");
      opt.value = stage;
      opt.textContent = stage;
      stageSel.appendChild(opt);
    }
    if (stages.includes(prev as "baby" | "child" | "adult")) stageSel.value = prev;
  }

  const theme: Theme = "oled";

  function redraw(): void {
    const slug = speciesSel.value;
    const stage = stageSel.value as "baby" | "child" | "adult";
    const file = catalog.find((f) => f.slug === slug && f.stage === stage);
    if (!file) return;

    renderGallery(gallery, file.gf, stage);

    const tick = Number(tickInput.value);
    tickLabel.textContent = String(tick);
    const frame = core.renderClip(slug, STAGE_INDEX[stage], activitySel.value, tick);
    blit(ctx, frame, theme);
  }

  async function reloadAll(): Promise<void> {
    catalog = loadCatalog();
    refreshSpeciesOptions();
    refreshStageOptions();
    await pushOverrides(core, catalog);
    redraw();
    statusEl.textContent = `${catalog.length} stage file(s) loaded. Edit a .txt under assets/ and save to see it update.`;
  }

  speciesSel.addEventListener("change", () => {
    refreshStageOptions();
    redraw();
  });
  stageSel.addEventListener("change", redraw);
  activitySel.addEventListener("change", redraw);
  tickInput.addEventListener("input", redraw);

  await reloadAll();

  // HMR: Vite's eager glob creates static import edges to every matched .txt file, so saving
  // one triggers this module to be re-evaluated. Self-accepting and re-running `reloadAll`
  // (rather than a full page reload) keeps the current species/stage/activity/tick selection.
  if (import.meta.hot) {
    import.meta.hot.accept(() => {
      void reloadAll();
    });
  }
}

void main();
