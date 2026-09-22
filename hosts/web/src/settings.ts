// The settings sheet: server URL, token, pet id, a sync status line, "Sync now" and "Start
// over on server". Opened by the gear in the bezel; works on a phone without `?dev`. Pure UI:
// it reads/writes settings through the hooks and never touches the core.

import { getSettings, isConfigured, setSettings, type SyncSettings } from "./sync";

export interface SettingsHooks {
  /** Called after settings are saved; main.ts rebuilds its client and pulls. */
  onSaved(settings: SyncSettings): void;
  onSyncNow(): void;
  /** Delete the server copy (after the user confirmed). */
  onStartOver(): void;
}

export interface SettingsSheet {
  /** Update the status line (and the dot next to the screen). */
  setStatus(text: string, level: "off" | "ok" | "warn" | "error"): void;
}

export function mountSettings(root: HTMLElement, gear: HTMLElement, dot: HTMLElement, hooks: SettingsHooks): SettingsSheet {
  root.innerHTML = `
    <div class="sheet">
      <div class="sheet-head">
        <h2>Sync</h2>
        <button type="button" data-close aria-label="Close">×</button>
      </div>
      <p class="hint">Enter the address and token of your vpet-server to keep one pet across devices.
        Leave the address empty to play offline in this browser only.</p>
      <label>Server URL <input data-url type="url" placeholder="https://vpet.example.com" autocomplete="off" spellcheck="false" /></label>
      <label>Token <input data-token type="password" autocomplete="off" /></label>
      <label>Pet id <input data-pet type="text" autocomplete="off" spellcheck="false" /></label>
      <div class="row">
        <button type="button" data-save>Save</button>
        <button type="button" data-sync>Sync now</button>
        <button type="button" data-reset class="danger">Start over on server</button>
      </div>
      <p class="status" data-status>sync off</p>
    </div>`;

  const q = <T extends HTMLElement>(sel: string) => root.querySelector(sel) as T;
  const url = q<HTMLInputElement>("[data-url]");
  const token = q<HTMLInputElement>("[data-token]");
  const pet = q<HTMLInputElement>("[data-pet]");
  const status = q<HTMLElement>("[data-status]");
  const syncBtn = q<HTMLButtonElement>("[data-sync]");
  const resetBtn = q<HTMLButtonElement>("[data-reset]");

  function fill(): void {
    const s = getSettings();
    url.value = s.url;
    token.value = s.token;
    pet.value = s.petId;
    const on = isConfigured(s);
    syncBtn.disabled = !on;
    resetBtn.disabled = !on;
  }

  function open(): void {
    fill();
    root.hidden = false;
  }
  function close(): void {
    root.hidden = true;
  }

  gear.addEventListener("click", () => (root.hidden ? open() : close()));
  q("[data-close]").addEventListener("click", close);
  root.addEventListener("click", (e) => {
    if (e.target === root) close();
  });

  q("[data-save]").addEventListener("click", () => {
    const s: SyncSettings = { url: url.value, token: token.value, petId: pet.value };
    setSettings(s);
    fill();
    hooks.onSaved(getSettings());
  });
  syncBtn.addEventListener("click", () => hooks.onSyncNow());
  resetBtn.addEventListener("click", () => {
    if (confirm("Delete the pet stored on the server? Your local pet stays and will be uploaded on the next sync.")) {
      hooks.onStartOver();
    }
  });

  fill();
  return {
    setStatus(text, level) {
      status.textContent = text;
      status.dataset.level = level;
      dot.dataset.level = level;
      dot.title = text;
    },
  };
}
