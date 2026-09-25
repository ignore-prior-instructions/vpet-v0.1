// The settings sheet: sound, shell colour, and sync (server URL, token, pet id, a status line,
// "Sync now", "Start over on server"). Opened by the Settings button in the header; works on a
// phone without `?dev`. Pure UI: it reads/writes settings through persist/sync and the hooks,
// and never touches the core. Markup is @vpet/ds classes (packages/vpet-ds).

import * as persist from "./persist";
import { getSettings, isConfigured, setSettings, type SyncSettings } from "./sync";

export interface SettingsHooks {
  /** Called after sync settings are saved; main.ts rebuilds its client and pulls. */
  onSaved(settings: SyncSettings): void;
  onSyncNow(): void;
  /** Delete the server copy (after the user confirmed). */
  onStartOver(): void;
  onSound(on: boolean): void;
  onTone(tone: persist.Tone): void;
  /** Replace the pet with a fresh egg (after the user confirmed). The host's own `reset`
   * path, the same one boot uses; the core never learns why. */
  onNewEgg(): void;
}

export interface SettingsSheet {
  /** Update the status line (and the dot in the footer). */
  setStatus(text: string, level: "off" | "ok" | "warn" | "error"): void;
}

const TONES: [persist.Tone, string][] = [
  ["pink", "Pink"],
  ["lilac", "Lilac"],
  ["orange", "Orange"],
];

const DOT_CLASS: Record<string, string> = { off: "idle", ok: "ok", warn: "warn", error: "error" };

export function mountSettings(root: HTMLElement, gear: HTMLElement, dot: HTMLElement, hooks: SettingsHooks): SettingsSheet {
  root.innerHTML = `
    <section class="vp-sheet" aria-label="Settings">
      <header class="vp-sheet__head">
        <h2 class="vp-sheet__title">Settings</h2>
        <button type="button" class="vp-button vp-button--ghost vp-button--sm" data-close aria-label="Close">✕</button>
      </header>
      <div class="vp-sheet__body">
        <div class="settings-group">
          <span class="settings-group__label">Sound</span>
          <div class="settings-row">
            <button type="button" class="vp-button vp-button--sm" data-sound="on">On</button>
            <button type="button" class="vp-button vp-button--sm" data-sound="off">Off</button>
          </div>
        </div>
        <div class="settings-group">
          <span class="settings-group__label">Shell colour</span>
          <div class="settings-row">
            ${TONES.map(([id, label]) => `<span data-tone="${id}"><button type="button" class="vp-button vp-button--sm" data-tone-pick="${id}">${label}</button></span>`).join("")}
          </div>
        </div>
        <div class="settings-group settings-group--rule">
          <span class="settings-group__label">Pet</span>
          <p class="vp-sheet__hint">A dead pet restarts by holding A and C together on the tombstone. This
            replaces a living pet.</p>
          <div class="settings-row">
            <button type="button" class="vp-button vp-button--danger vp-button--sm" data-new-egg>New egg</button>
          </div>
        </div>
        <div class="settings-group settings-group--rule">
          <span class="settings-group__label">Sync</span>
          <p class="vp-sheet__hint">Enter the address and token of your vpet-server to keep one pet across
            devices. Leave the address empty to play offline in this browser only.</p>
          <label class="vp-field"><span class="vp-field__label">Server URL</span>
            <input class="vp-field__input" data-url type="url" placeholder="https://vpet.example.com" autocomplete="off" spellcheck="false" /></label>
          <label class="vp-field"><span class="vp-field__label">Token</span>
            <input class="vp-field__input" data-token type="password" autocomplete="off" /></label>
          <label class="vp-field"><span class="vp-field__label">Pet id</span>
            <input class="vp-field__input" data-pet type="text" autocomplete="off" spellcheck="false" /></label>
          <div class="settings-row">
            <button type="button" class="vp-button vp-button--sm" data-save>Save</button>
            <button type="button" class="vp-button vp-button--secondary vp-button--sm" data-sync>Sync now</button>
            <button type="button" class="vp-button vp-button--danger vp-button--sm" data-reset>Start over on server</button>
          </div>
          <p class="settings-status" data-status>sync off</p>
        </div>
      </div>
    </section>`;

  const q = <T extends HTMLElement>(sel: string) => root.querySelector(sel) as T;
  const url = q<HTMLInputElement>("[data-url]");
  const token = q<HTMLInputElement>("[data-token]");
  const pet = q<HTMLInputElement>("[data-pet]");
  const status = q<HTMLElement>("[data-status]");
  const syncBtn = q<HTMLButtonElement>("[data-sync]");
  const resetBtn = q<HTMLButtonElement>("[data-reset]");

  function highlight(selector: string, isOn: (el: HTMLButtonElement) => boolean): void {
    root.querySelectorAll<HTMLButtonElement>(selector).forEach((b) => {
      b.classList.toggle("vp-button--primary", isOn(b));
      b.classList.toggle("vp-button--secondary", !isOn(b));
    });
  }

  function fill(): void {
    const s = getSettings();
    url.value = s.url;
    token.value = s.token;
    pet.value = s.petId;
    const on = isConfigured(s);
    syncBtn.disabled = !on;
    resetBtn.disabled = !on;
    const sound = persist.getSound();
    highlight("[data-sound]", (b) => (b.dataset.sound === "on") === sound);
    const tone = persist.getTone();
    highlight("[data-tone-pick]", (b) => b.dataset.tonePick === tone);
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

  root.querySelectorAll<HTMLButtonElement>("[data-sound]").forEach((b) =>
    b.addEventListener("click", () => {
      const on = b.dataset.sound === "on";
      persist.setSound(on);
      hooks.onSound(on);
      fill();
    }),
  );
  root.querySelectorAll<HTMLButtonElement>("[data-tone-pick]").forEach((b) =>
    b.addEventListener("click", () => {
      const tone = b.dataset.tonePick as persist.Tone;
      persist.setTone(tone);
      hooks.onTone(tone);
      fill();
    }),
  );

  q("[data-new-egg]").addEventListener("click", () => {
    if (confirm("Replace your pet with a new egg? The current pet is gone for good.")) {
      hooks.onNewEgg();
      close();
    }
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
      dot.className = `vp-statusdot vp-statusdot--${DOT_CLASS[level]} footer-status`;
      const label = dot.querySelector("#syncstatus");
      if (label) label.textContent = text;
    },
  };
}
