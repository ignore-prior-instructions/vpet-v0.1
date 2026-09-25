// Persistence. docs/hosts/web.md: "IndexedDB, raw API, one object store, key `pet:default` ->
// `Uint8Array`. Chosen over localStorage because it stores bytes natively." localStorage is
// only for host settings, never the blob.

const DB_NAME = "vpet";
const STORE = "blobs";
const KEY = "pet:default";

function openDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, 1);
    req.onupgradeneeded = () => {
      req.result.createObjectStore(STORE);
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

export async function getBlob(): Promise<Uint8Array | null> {
  const db = await openDb();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(STORE, "readonly");
    const req = tx.objectStore(STORE).get(KEY);
    req.onsuccess = () => resolve((req.result as Uint8Array | undefined) ?? null);
    req.onerror = () => reject(req.error);
  });
}

export async function putBlob(blob: Uint8Array): Promise<void> {
  const db = await openDb();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(STORE, "readwrite");
    tx.objectStore(STORE).put(blob, KEY);
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
  });
}

// --- host settings (localStorage; never the blob itself) --------------------------------------

const THEME_KEY = "vpet.theme";
const DEV_OFFSET_KEY = "vpet.devOffsetMs";

export type Theme = "ink" | "oled" | "lcd";
export type Tone = "pink" | "lilac" | "orange";

const SOUND_KEY = "vpet.sound";
const TONE_KEY = "vpet.tone";

export function getTheme(): Theme {
  const t = localStorage.getItem(THEME_KEY);
  return t === "oled" || t === "lcd" ? t : "ink";
}

export function setTheme(theme: Theme): void {
  localStorage.setItem(THEME_KEY, theme);
}

/** Beep on the core's BEEP flag. Default on. */
export function getSound(): boolean {
  return localStorage.getItem(SOUND_KEY) !== "off";
}

export function setSound(on: boolean): void {
  localStorage.setItem(SOUND_KEY, on ? "on" : "off");
}

/** The shell's body colour (`data-tone` on the device, packages/vpet-ds). Default pink. */
export function getTone(): Tone {
  const t = localStorage.getItem(TONE_KEY);
  return t === "lilac" || t === "orange" ? t : "pink";
}

export function setTone(tone: Tone): void {
  localStorage.setItem(TONE_KEY, tone);
}

/** Dev-only clock offset in ms, added to `Date.now()`. Persisted so a reload doesn't lose it
 * mid-test (docs/hosts/web.md "Dev panel"). */
export function getDevOffsetMs(): number {
  const raw = localStorage.getItem(DEV_OFFSET_KEY);
  const n = raw ? Number(raw) : 0;
  return Number.isFinite(n) ? n : 0;
}

export function setDevOffsetMs(offsetMs: number): void {
  localStorage.setItem(DEV_OFFSET_KEY, String(offsetMs));
}
