// Host flow: docs/HOST_ABI.md "Host flow", docs/hosts/web.md. This file wires the modules
// together and owns policy (when to save, when to log, when to show the dev panel); it never
// draws a sprite or decides what the screen shows (CLAUDE.md "Hosts are dumb blitters" —
// everything about *what* to draw comes from `core.frame()`/`core.inspect()`).

import { Core, Flags, loadErrorName, LoadError } from "./core";
import { blit, type Theme } from "./blit";
import { Input } from "./input";
import * as persist from "./persist";
import { mountDevPanel, type DevPanel } from "./devpanel";
import { Recorder } from "./vlog";

const TICK_MS = 100;
const SAVE_DEBOUNCE_MS = 500;
const CHECKPOINT_MS = 30_000;

function randomSeed(): bigint {
  const buf = new Uint32Array(2);
  crypto.getRandomValues(buf);
  return (BigInt(buf[0]) << 32n) | BigInt(buf[1]);
}

function now(): number {
  return Date.now() + persist.getDevOffsetMs();
}

function isDevMode(): boolean {
  return location.hostname === "localhost" || location.hostname === "127.0.0.1" || new URLSearchParams(location.search).has("dev");
}

function beep(): void {
  try {
    const ctx = new (window.AudioContext || (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext)();
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    osc.frequency.value = 880;
    gain.gain.setValueAtTime(0.2, ctx.currentTime);
    gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + 0.12);
    osc.connect(gain).connect(ctx.destination);
    osc.start();
    osc.stop(ctx.currentTime + 0.12);
  } catch {
    // No user gesture yet, or no AudioContext support: silently skip. Not essential.
  }
}

async function main(): Promise<void> {
  const canvas = document.getElementById("screen") as HTMLCanvasElement;
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("2D canvas context unavailable");

  const banner = document.getElementById("banner") as HTMLElement;
  const attentionBadge = document.getElementById("attention") as HTMLElement;
  const buttonA = document.getElementById("btn-a") as HTMLElement;
  const buttonB = document.getElementById("btn-b") as HTMLElement;
  const buttonC = document.getElementById("btn-c") as HTMLElement;
  const devPanelEl = document.getElementById("devpanel") as HTMLElement;

  let theme: Theme = persist.getTheme();
  document.body.dataset.theme = theme;

  const core = await Core.load(`${import.meta.env.BASE_URL}vpet.wasm`);
  const input = new Input({ a: buttonA, b: buttonB, c: buttonC });
  const recorder = new Recorder();

  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  let lastLoggedMask = -1; // force the first `t` line to be logged

  function saveNow(): void {
    const blob = core.save();
    if (blob) {
      persist.putBlob(blob).catch((e) => console.error("persist.putBlob failed", e));
      recorder.save();
    }
  }

  function scheduleSave(): void {
    if (saveTimer !== null) return;
    saveTimer = setTimeout(() => {
      saveTimer = null;
      saveNow();
    }, SAVE_DEBOUNCE_MS);
  }

  // Boot: load a stored blob if one exists; otherwise start a fresh egg.
  // A recording is only a complete, replayable .vlog when the session began with `reset` (no
  // stored blob) — the recorder has no way to represent "start from an arbitrary loaded state"
  // in the .vlog grammar, which always opens with `reset`.
  const stored = await persist.getBlob().catch(() => null);
  let booted = false;
  if (stored) {
    const rc = core.load(stored);
    if (rc === 0) {
      booted = true;
    } else if (rc === LoadError.VERSION_TOO_NEW) {
      banner.textContent = "This save is from a newer version of vpet. Not overwriting it; update the app to continue.";
      banner.hidden = false;
      return; // stop here: do not reset, do not touch storage
    } else {
      console.warn(`stored save failed to load (${loadErrorName(rc)}); starting a new egg`);
    }
  }
  if (!booted) {
    const seed = randomSeed();
    const n = now();
    core.reset(n, seed);
    recorder.reset(n, seed);
  }

  let devPanel: DevPanel | null = null;
  if (isDevMode()) {
    devPanelEl.hidden = false;
    devPanel = mountDevPanel(devPanelEl, {
      core,
      recorder,
      getTheme: () => theme,
      setTheme: (t) => {
        theme = t;
        persist.setTheme(t);
        document.body.dataset.theme = t;
      },
      getDevOffsetMs: persist.getDevOffsetMs,
      setDevOffsetMs: persist.setDevOffsetMs,
      onExternalChange: () => tick(),
    });
  }

  function tick(): void {
    const n = now();
    const mask = input.mask();
    const flags = core.update(n, mask);

    if (mask !== lastLoggedMask || flags !== 0) {
      recorder.t(n, mask);
      lastLoggedMask = mask;
    }

    if (flags & Flags.FRAME_CHANGED) {
      blit(ctx!, core.frame(), theme);
    }
    if (flags & Flags.SAVE_NEEDED) {
      scheduleSave();
    }
    if (flags & Flags.BEEP) {
      beep();
    }
    attentionBadge.classList.toggle("active", (flags & Flags.ATTENTION) !== 0);

    devPanel?.update();
    canvas.dataset.ready = "true";
  }

  // `reset`/`load` already render a first frame internally (crates/vpet-core/src/lib.rs), but
  // `update`'s FRAME_CHANGED flag only reflects a *change* from that frame — an `update` call
  // landing in the same 250ms animation tick as `reset` can be byte-identical, so FRAME_CHANGED
  // may not be set on the very first call (confirmed: `vpet-cli replay` shows flags=0 for an
  // update 10ms after reset). Paint once unconditionally so the egg is never left blank.
  blit(ctx, core.frame(), theme);
  tick();
  setInterval(tick, TICK_MS);
  setInterval(saveNow, CHECKPOINT_MS);

  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "visible") tick();
  });
  window.addEventListener("pagehide", saveNow);
}

main().catch((e) => {
  console.error(e);
  const banner = document.getElementById("banner");
  if (banner) {
    banner.textContent = `Failed to start: ${(e as Error).message}`;
    banner.hidden = false;
  }
});
