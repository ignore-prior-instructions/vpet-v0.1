// Host flow: docs/HOST_ABI.md "Host flow", docs/hosts/web.md, docs/SYNC.md "Client protocol".
// This file wires the modules together and owns policy (when to save, when to sync, when to
// log, when to show the dev panel); it never draws a sprite or decides what the screen shows
// (CLAUDE.md "Hosts are dumb blitters" — everything about *what* to draw comes from
// `core.frame()`/`core.inspect()`).

import { Core, Flags, loadErrorName, LoadError } from "./core";
import { blit, type Theme } from "./blit";
import { Input } from "./input";
import * as persist from "./persist";
import { mountDevPanel, type DevPanel } from "./devpanel";
import { mountSettings, type SettingsSheet } from "./settings";
import { getSettings, isConfigured, pickNewest, SyncClient, SyncNetworkError, type SyncSettings } from "./sync";
import { Recorder } from "./vlog";

const TICK_MS = 100;
const SAVE_DEBOUNCE_MS = 500;
const PUSH_DEBOUNCE_MS = 2_000;
const CHECKPOINT_MS = 30_000;
const PULL_TIMEOUT_MS = 5_000;

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

function relative(ms: number): string {
  const s = Math.max(0, Math.round((Date.now() - ms) / 1000));
  if (s < 5) return "just now";
  if (s < 60) return `${s}s ago`;
  if (s < 3600) return `${Math.round(s / 60)} min ago`;
  return `${Math.round(s / 3600)} h ago`;
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
  const devBoxEl = document.getElementById("devbox") as HTMLElement;
  const devToggleEl = document.getElementById("dev-toggle") as HTMLElement;
  const devPanelEl = document.getElementById("devpanel") as HTMLElement;
  const settingsEl = document.getElementById("settings") as HTMLElement;
  const gearEl = document.getElementById("gear") as HTMLElement;
  const syncDotEl = document.getElementById("syncdot") as HTMLElement;
  const shellEl = document.getElementById("shell") as HTMLElement;
  const helpEl = document.getElementById("help") as HTMLElement;
  const helpBtnEl = document.getElementById("help-btn") as HTMLElement;
  const cheatEl = document.getElementById("cheatsheet") as HTMLElement;

  let theme: Theme = persist.getTheme();
  let sound = persist.getSound();
  shellEl.dataset.tone = persist.getTone();

  // Help sheet: static markup in index.html, just shown/hidden here.
  helpBtnEl.addEventListener("click", () => (helpEl.hidden = !helpEl.hidden));
  helpEl.querySelector("[data-close]")?.addEventListener("click", () => (helpEl.hidden = true));
  helpEl.addEventListener("click", (e) => {
    if (e.target === helpEl) helpEl.hidden = true;
  });
  // Cheat sheet: which adult you get and why. Reachable from the help sheet, or straight
  // from the URL with `?cheatsheet`.
  const openCheat = () => {
    helpEl.hidden = true;
    cheatEl.hidden = false;
  };
  helpEl.querySelector("[data-open-cheatsheet]")?.addEventListener("click", openCheat);
  cheatEl.querySelector("[data-close]")?.addEventListener("click", () => (cheatEl.hidden = true));
  cheatEl.addEventListener("click", (e) => {
    if (e.target === cheatEl) cheatEl.hidden = true;
  });
  if (new URLSearchParams(location.search).has("cheatsheet")) openCheat();
  // The footer status is a button too: it opens Settings, where the full sync status lives.
  syncDotEl.addEventListener("click", () => gearEl.click());

  const core = await Core.load(`${import.meta.env.BASE_URL}vpet.wasm`);
  const input = new Input({ a: buttonA, b: buttonB, c: buttonC });
  const recorder = new Recorder();

  // --- sync state ----------------------------------------------------------------------------
  let settings: SyncSettings = getSettings();
  let client: SyncClient | null = isConfigured(settings) ? new SyncClient(settings) : null;
  let lastSyncedAt: number | null = null;
  let sheet: SettingsSheet;

  function status(text: string, level: "off" | "ok" | "warn" | "error"): void {
    sheet?.setStatus(text, level);
  }
  function statusSynced(): void {
    lastSyncedAt = Date.now();
    status(`synced ${relative(lastSyncedAt)}`, "ok");
  }

  // --- local save ----------------------------------------------------------------------------
  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  let pushTimer: ReturnType<typeof setTimeout> | null = null;

  function saveNow(): Uint8Array | null {
    const blob = core.save();
    if (blob) {
      persist.putBlob(blob).catch((e) => console.error("persist.putBlob failed", e));
      recorder.save();
    }
    return blob;
  }

  function scheduleSave(): void {
    if (saveTimer !== null) return;
    saveTimer = setTimeout(() => {
      saveTimer = null;
      saveNow();
    }, SAVE_DEBOUNCE_MS);
  }

  // --- sync: adopt / push / pull -------------------------------------------------------------

  /** Load `blob` as the live pet (it won the sim_now comparison) and persist it locally. */
  function adopt(blob: Uint8Array, why: string): boolean {
    const rc = core.load(blob);
    if (rc !== 0) {
      console.warn(`adopting the server save failed (${loadErrorName(rc)})`);
      status(`server save could not be loaded (${loadErrorName(rc)})`, "error");
      return false;
    }
    persist.putBlob(blob).catch((e) => console.error("persist.putBlob failed", e));
    tick();
    status(why, "warn");
    return true;
  }

  async function pushNow(keepalive = false): Promise<void> {
    if (!client) return;
    const blob = core.save();
    if (!blob) return;
    const simNow = core.peekSimNow(blob);
    if (simNow === null) return;
    try {
      const result = await client.push(blob, simNow, keepalive);
      if (result.kind === "ok") {
        statusSynced();
      } else if (result.kind === "conflict") {
        const mine = core.peekSimNow(core.save() ?? blob) ?? simNow;
        if (result.remote.simNow > mine) {
          adopt(result.remote.blob, "adopted the newer save from the server");
          lastSyncedAt = Date.now();
        } else {
          schedulePush(); // the compare was racy; try again with our now-current blob
        }
      } else {
        status(result.versionTooNew ? "server needs updating; playing locally" : `server rejected the save: ${result.message}`, "error");
      }
    } catch (e) {
      status(e instanceof SyncNetworkError ? "offline, playing locally" : `sync error: ${(e as Error).message}`, "warn");
    }
  }

  function schedulePush(): void {
    if (!client || pushTimer !== null) return;
    pushTimer = setTimeout(() => {
      pushTimer = null;
      void pushNow();
    }, PUSH_DEBOUNCE_MS);
  }

  /** docs/SYNC.md "on open / visible": compare the local pet with the server's and keep the
   * one that simulated furthest; push if ours won, adopt if theirs did. */
  async function pullAndReconcile(): Promise<void> {
    if (!client) {
      status("sync off", "off");
      return;
    }
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), PULL_TIMEOUT_MS);
    try {
      const remote = await client.pull(controller.signal);
      const localBlob = core.save();
      const local = localBlob ? { blob: localBlob, simNow: core.peekSimNow(localBlob) ?? 0 } : null;
      const best = pickNewest([local, remote]);
      if (!best) return;
      if (remote && best === remote && (!local || remote.simNow > local.simNow)) {
        adopt(remote.blob, "adopted the newer save from the server");
        lastSyncedAt = Date.now(); // the status line keeps saying "adopted" until the next push
      } else if (!remote || (local && local.simNow > remote.simNow)) {
        await pushNow();
      } else {
        statusSynced(); // equal: nothing to do
      }
    } catch (e) {
      status(e instanceof SyncNetworkError ? "offline, playing locally" : `sync error: ${(e as Error).message}`, "warn");
    } finally {
      clearTimeout(timer);
    }
  }

  // --- boot ---------------------------------------------------------------------------------
  // Load a stored blob if one exists; otherwise start a fresh egg.
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

  sheet = mountSettings(settingsEl, gearEl, syncDotEl, {
    onSaved(s) {
      settings = s;
      client = isConfigured(s) ? new SyncClient(s) : null;
      void pullAndReconcile();
    },
    onSyncNow() {
      void pullAndReconcile();
    },
    onSound(on) {
      sound = on;
    },
    onNewEgg() {
      const seed = randomSeed();
      const n = now();
      core.reset(n, seed);
      recorder.reset(n, seed);
      blit(ctx!, core.frame(), theme);
      tick();
      saveNow();
      schedulePush();
    },
    onTone(tone) {
      shellEl.dataset.tone = tone;
    },
    onStartOver() {
      if (!client) return;
      client
        .remove()
        .then(() => {
          status("server copy deleted; will upload this pet on the next change", "warn");
          schedulePush();
        })
        .catch((e) => status(`could not delete: ${(e as Error).message}`, "error"));
    },
  });

  let devPanel: DevPanel | null = null;
  if (isDevMode()) {
    devBoxEl.hidden = false;
    // Folded by default on a phone, where the panel would cover the device.
    devPanelEl.hidden = window.matchMedia("(max-width: 480px)").matches;
    devToggleEl.textContent = devPanelEl.hidden ? "Dev ▴" : "Dev ▾";
    const setDevOpen = (open: boolean) => {
      devPanelEl.hidden = !open;
      devToggleEl.textContent = open ? "Dev ▾" : "Dev ▴";
    };
    devToggleEl.addEventListener("click", () => setDevOpen(devPanelEl.hidden));
    // Click anywhere outside the panel folds it, like the Settings sheet. The device's own
    // A/B/C buttons are the exception: the panel is for watching `inspect` while you play.
    document.addEventListener("click", (e) => {
      const target = e.target as Node;
      if (devPanelEl.hidden || devBoxEl.contains(target)) return;
      if ((target as Element).closest?.(".vp-shell__buttons")) return;
      setDevOpen(false);
    });
    devPanel = mountDevPanel(devPanelEl, {
      core,
      recorder,
      getTheme: () => theme,
      setTheme: (t) => {
        theme = t;
        persist.setTheme(t);
      },
      getDevOffsetMs: persist.getDevOffsetMs,
      setDevOffsetMs: persist.setDevOffsetMs,
      onExternalChange: () => tick(),
    });
  }

  let lastLoggedMask = -1; // force the first `t` line to be logged

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
      schedulePush();
    }
    if (flags & Flags.BEEP && sound) {
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
  setInterval(() => {
    saveNow();
    if (lastSyncedAt !== null) status(`synced ${relative(lastSyncedAt)}`, "ok");
  }, CHECKPOINT_MS);

  // The pull happens after the first paint so a slow server never delays the pet appearing.
  void pullAndReconcile();

  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "visible") {
      tick();
      void pullAndReconcile();
    }
  });
  window.addEventListener("pagehide", () => {
    saveNow();
    void pushNow(true);
  });
}

main().catch((e) => {
  console.error(e);
  const banner = document.getElementById("banner");
  if (banner) {
    banner.textContent = `Failed to start: ${(e as Error).message}`;
    banner.hidden = false;
  }
});
