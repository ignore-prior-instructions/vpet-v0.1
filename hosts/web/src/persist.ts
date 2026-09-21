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

export function getTheme(): "oled" | "lcd" {
  return localStorage.getItem(THEME_KEY) === "lcd" ? "lcd" : "oled";
}

export function setTheme(theme: "oled" | "lcd"): void {
  localStorage.setItem(THEME_KEY, theme);
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
