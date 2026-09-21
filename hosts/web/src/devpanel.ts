// Dev panel: shown on localhost or with `?dev` (docs/hosts/web.md "Dev panel"). Four parts:
// an inspect table, a clock-offset control, a `.vlog` recorder/downloader, and blob
// import/export as hex (the manual transfer path to an ESP32 over serial before WiFi sync
// exists). This module only ever reads `core.inspect()` and calls back into main.ts for
// anything that changes host state — it never touches the simulation directly except via
// `core.load` for hex import, which is the same "load a blob" path the host itself uses.

import type { Core } from "./core";
import { loadErrorName } from "./core";
import { CartState, Stage, NEVER, type Inspect } from "./inspect";
import type { Recorder } from "./vlog";
import type { Theme } from "./blit";

export interface DevPanelHooks {
  core: Core;
  recorder: Recorder;
  getTheme(): Theme;
  setTheme(t: Theme): void;
  getDevOffsetMs(): number;
  setDevOffsetMs(ms: number): void;
  /** Called after anything that changes state out from under the normal tick loop (a hex
   * import, a clock-offset change) so main.ts can tick immediately and persist. */
  onExternalChange(): void;
}

function toHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

function fromHex(hex: string): Uint8Array {
  const clean = hex.replace(/\s+/g, "");
  if (clean.length === 0 || clean.length % 2 !== 0 || !/^[0-9a-fA-F]*$/.test(clean)) {
    throw new Error("not an even-length hex string");
  }
  const out = new Uint8Array(clean.length / 2);
  for (let i = 0; i < out.length; i++) {
    out[i] = parseInt(clean.slice(i * 2, i * 2 + 2), 16);
  }
  return out;
}

function fieldRow(label: string, value: string): string {
  return `<tr><td>${label}</td><td>${value}</td></tr>`;
}

function renderInspect(insp: Inspect): string {
  const rows = [
    fieldRow("state", CartState[insp.state as keyof typeof CartState] ?? String(insp.state)),
    fieldRow("stage", Stage[insp.stage as keyof typeof Stage] ?? String(insp.stage)),
    fieldRow("species", String(insp.species)),
    fieldRow("hunger", String(insp.hunger)),
    fieldRow("happiness", String(insp.happiness)),
    fieldRow("discipline", String(insp.discipline)),
    fieldRow("health", String(insp.health)),
    fieldRow("weight", String(insp.weight)),
    fieldRow("age_secs", String(insp.ageSecs)),
    fieldRow("flags", `0b${insp.flags.toString(2).padStart(4, "0")}`),
    fieldRow("attention", `0b${insp.attention.toString(2).padStart(6, "0")}`),
    fieldRow("poops", String(insp.poops)),
    fieldRow("care_mistakes", String(insp.careMistakes)),
    fieldRow("sim_now", String(insp.simNow)),
    fieldRow("next_event_at", insp.nextEventAt === NEVER ? "NEVER" : String(insp.nextEventAt)),
    fieldRow("save_version", String(insp.saveVersion)),
    fieldRow("content_hash", `0x${insp.contentHash.toString(16).padStart(8, "0")}`),
  ];
  return `<table class="inspect">${rows.join("")}</table>`;
}

export interface DevPanel {
  update(): void;
}

export function mountDevPanel(container: HTMLElement, hooks: DevPanelHooks): DevPanel {
  container.innerHTML = `
    <section data-panel="inspect"><h3>inspect</h3></section>
    <section data-panel="clock">
      <h3>clock offset</h3>
      <button data-offset="60000">+1 min</button>
      <button data-offset="3600000">+1 h</button>
      <button data-offset="86400000">+1 day</button>
      <button data-offset-reset>reset</button>
      <div><input data-offset-input type="number" placeholder="custom ms" />
        <button data-offset-apply>apply</button></div>
      <div data-offset-current></div>
    </section>
    <section data-panel="vlog">
      <h3>.vlog recorder</h3>
      <button data-snap>snap...</button>
      <button data-download>download .vlog</button>
    </section>
    <section data-panel="blob">
      <h3>blob (hex)</h3>
      <button data-export>export</button>
      <textarea data-hex rows="3" style="width: 100%"></textarea>
      <button data-import>import</button>
      <div data-import-result></div>
    </section>
    <section data-panel="theme">
      <h3>theme</h3>
      <button data-theme="oled">OLED</button>
      <button data-theme="lcd">LCD</button>
    </section>
  `;

  const inspectSection = container.querySelector<HTMLElement>('[data-panel="inspect"]')!;
  const offsetCurrent = container.querySelector<HTMLElement>("[data-offset-current]")!;

  const updateOffsetLabel = () => {
    offsetCurrent.textContent = `current offset: ${hooks.getDevOffsetMs()} ms`;
  };
  updateOffsetLabel();

  container.querySelectorAll<HTMLButtonElement>("[data-offset]").forEach((btn) => {
    btn.addEventListener("click", () => {
      const delta = Number(btn.dataset.offset);
      hooks.setDevOffsetMs(hooks.getDevOffsetMs() + delta);
      updateOffsetLabel();
      hooks.onExternalChange();
    });
  });
  container.querySelector<HTMLButtonElement>("[data-offset-reset]")!.addEventListener("click", () => {
    hooks.setDevOffsetMs(0);
    updateOffsetLabel();
    hooks.onExternalChange();
  });
  container.querySelector<HTMLButtonElement>("[data-offset-apply]")!.addEventListener("click", () => {
    const input = container.querySelector<HTMLInputElement>("[data-offset-input]")!;
    const ms = Number(input.value);
    if (Number.isFinite(ms)) {
      hooks.setDevOffsetMs(ms);
      updateOffsetLabel();
      hooks.onExternalChange();
    }
  });

  container.querySelector<HTMLButtonElement>("[data-snap]")!.addEventListener("click", () => {
    const name = window.prompt("snap name?");
    if (name) hooks.recorder.snap(name);
  });
  container.querySelector<HTMLButtonElement>("[data-download]")!.addEventListener("click", () => {
    hooks.recorder.download();
  });

  container.querySelector<HTMLButtonElement>("[data-export]")!.addEventListener("click", () => {
    const hex = container.querySelector<HTMLTextAreaElement>("[data-hex]")!;
    const blob = hooks.core.save();
    hex.value = blob ? toHex(blob) : "";
  });
  container.querySelector<HTMLButtonElement>("[data-import]")!.addEventListener("click", () => {
    const hex = container.querySelector<HTMLTextAreaElement>("[data-hex]")!;
    const result = container.querySelector<HTMLElement>("[data-import-result]")!;
    try {
      const blob = fromHex(hex.value);
      const rc = hooks.core.load(blob);
      if (rc === 0) {
        result.textContent = "loaded ok";
        hooks.recorder.load();
        hooks.onExternalChange();
      } else {
        result.textContent = `load failed: ${loadErrorName(rc)}`;
      }
    } catch (e) {
      result.textContent = `parse error: ${(e as Error).message}`;
    }
  });

  container.querySelectorAll<HTMLButtonElement>("[data-theme]").forEach((btn) => {
    btn.addEventListener("click", () => {
      hooks.setTheme(btn.dataset.theme as Theme);
      hooks.onExternalChange();
    });
  });

  return {
    update() {
      inspectSection.innerHTML = `<h3>inspect</h3>${renderInspect(hooks.core.inspect())}`;
    },
  };
}
