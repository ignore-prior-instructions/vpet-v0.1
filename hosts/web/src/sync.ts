// Sync client: the client half of docs/SYNC.md. Talks to vpet-server, moves bytes, and
// applies exactly one rule (highest `sim_now` wins); it never touches the simulation. `main.ts`
// owns when to pull/push and what to load; this module owns how.

export interface SyncSettings {
  url: string; // server origin, e.g. https://vpet.example.com ; empty = sync off
  token: string; // bearer token (VPET_TOKEN on the server)
  petId: string; // client-chosen id; "default" for one pet
}

const URL_KEY = "vpet.sync.url";
const TOKEN_KEY = "vpet.sync.token";
const PET_ID_KEY = "vpet.sync.petId";

/** A minimal `Storage` so tests can pass a `Map`-backed fake; browsers pass `localStorage`. */
export interface KeyValue {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

export function getSettings(storage: KeyValue = localStorage): SyncSettings {
  return {
    url: (storage.getItem(URL_KEY) ?? "").trim().replace(/\/+$/, ""),
    token: storage.getItem(TOKEN_KEY) ?? "",
    petId: (storage.getItem(PET_ID_KEY) ?? "").trim() || "default",
  };
}

export function setSettings(s: SyncSettings, storage: KeyValue = localStorage): void {
  storage.setItem(URL_KEY, s.url.trim().replace(/\/+$/, ""));
  storage.setItem(TOKEN_KEY, s.token);
  storage.setItem(PET_ID_KEY, s.petId.trim() || "default");
}

export function isConfigured(s: SyncSettings): boolean {
  return s.url.length > 0;
}

export interface Remote {
  blob: Uint8Array;
  simNow: number;
  seq: number;
  savedAt: string;
}

export type PushResult =
  | { kind: "ok"; seq: number }
  | { kind: "conflict"; remote: Remote }
  | { kind: "rejected"; status: number; message: string; versionTooNew: boolean };

/** Thrown for anything that is not a well-formed server answer: offline, DNS, CORS, 5xx. */
export class SyncNetworkError extends Error {}

export function fromB64(b64: string): Uint8Array {
  const bin = atob(b64);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}

export function toB64(bytes: Uint8Array): string {
  let bin = "";
  for (const b of bytes) bin += String.fromCharCode(b);
  return btoa(bin);
}

interface PetBody {
  sim_now: number;
  seq: number;
  saved_at: string;
  blob_b64: string;
}

function parseRemote(body: PetBody): Remote {
  return { blob: fromB64(body.blob_b64), simNow: body.sim_now >>> 0, seq: body.seq, savedAt: body.saved_at };
}

/** The docs/SYNC.md rule: among candidates, the one that simulated furthest. `null` if none. */
export function pickNewest<T extends { simNow: number }>(candidates: (T | null | undefined)[]): T | null {
  let best: T | null = null;
  for (const c of candidates) {
    if (c && (best === null || c.simNow > best.simNow)) best = c;
  }
  return best;
}

export class SyncClient {
  constructor(
    private readonly settings: SyncSettings,
    private readonly fetchFn: typeof fetch = (...args) => fetch(...args),
  ) {}

  private petUrl(): string {
    return `${this.settings.url}/v1/pets/${encodeURIComponent(this.settings.petId)}`;
  }

  private headers(extra: Record<string, string> = {}): Record<string, string> {
    return { Authorization: `Bearer ${this.settings.token}`, ...extra };
  }

  private async request(init: RequestInit): Promise<Response> {
    try {
      return await this.fetchFn(this.petUrl(), init);
    } catch (e) {
      throw new SyncNetworkError((e as Error).message || "network error");
    }
  }

  /** The server's save, or `null` if it has none yet (404). Throws `SyncNetworkError` offline. */
  async pull(signal?: AbortSignal): Promise<Remote | null> {
    const resp = await this.request({ method: "GET", headers: this.headers(), signal });
    if (resp.status === 404) return null;
    if (resp.status === 401) throw new SyncNetworkError("server rejected the token (401)");
    if (!resp.ok) throw new SyncNetworkError(`GET failed: ${resp.status}`);
    return parseRemote((await resp.json()) as PetBody);
  }

  /** PUT with `X-Sim-Now`. A 409 comes back as `conflict` with the server's (newer) save;
   * 422 as `rejected` (the server could not load our blob; `versionTooNew` when it needs a
   * rebuild). `keepalive` lets the request outlive a closing page (docs/SYNC.md). */
  async push(blob: Uint8Array, simNow: number, keepalive = false): Promise<PushResult> {
    const resp = await this.request({
      method: "PUT",
      headers: this.headers({ "Content-Type": "application/octet-stream", "X-Sim-Now": String(simNow >>> 0) }),
      body: blob.slice().buffer as ArrayBuffer,
      keepalive,
    });
    if (resp.status === 409) return { kind: "conflict", remote: parseRemote((await resp.json()) as PetBody) };
    if (resp.status === 422 || resp.status === 400) {
      let message = `PUT rejected: ${resp.status}`;
      try {
        message = ((await resp.json()) as { error?: string }).error ?? message;
      } catch {
        // keep the generic message
      }
      return { kind: "rejected", status: resp.status, message, versionTooNew: /newer core|rebuild/i.test(message) };
    }
    if (resp.status === 401) throw new SyncNetworkError("server rejected the token (401)");
    if (!resp.ok) throw new SyncNetworkError(`PUT failed: ${resp.status}`);
    return { kind: "ok", seq: ((await resp.json()) as { seq: number }).seq };
  }

  /** "Start over on the server": the only path that deletes the server blob (docs/SYNC.md). */
  async remove(): Promise<void> {
    const resp = await this.request({ method: "DELETE", headers: this.headers() });
    if (!resp.ok && resp.status !== 404) throw new SyncNetworkError(`DELETE failed: ${resp.status}`);
  }
}
