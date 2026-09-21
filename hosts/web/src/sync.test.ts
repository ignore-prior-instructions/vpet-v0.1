// The sync client against a scripted `fetch` (docs/SYNC.md "Client protocol").
import { describe, expect, it } from "vitest";
import { fromB64, getSettings, pickNewest, setSettings, SyncClient, SyncNetworkError, toB64, type KeyValue } from "./sync";

class MapStorage implements KeyValue {
  private m = new Map<string, string>();
  getItem(k: string): string | null {
    return this.m.get(k) ?? null;
  }
  setItem(k: string, v: string): void {
    this.m.set(k, v);
  }
}

const settings = { url: "https://vpet.example", token: "t0k", petId: "default" };
const blob = new Uint8Array([86, 80, 69, 84, 1, 0, 9, 8, 7, 6, 5, 0, 1, 2, 3]);

function scripted(handler: (url: string, init: RequestInit) => Response): typeof fetch {
  return (async (input: RequestInfo | URL, init?: RequestInit) => handler(String(input), init ?? {})) as typeof fetch;
}

describe("settings", () => {
  it("round-trip, trims trailing slashes, defaults the pet id", () => {
    const s = new MapStorage();
    setSettings({ url: "https://x.example/// ", token: "abc", petId: "  " }, s);
    expect(getSettings(s)).toEqual({ url: "https://x.example", token: "abc", petId: "default" });
    expect(getSettings(new MapStorage())).toEqual({ url: "", token: "", petId: "default" });
  });
});

describe("base64", () => {
  it("round-trips bytes", () => {
    expect(fromB64(toB64(blob))).toEqual(blob);
  });
});

describe("pickNewest", () => {
  it("chooses the highest simNow and ignores nulls", () => {
    const a = { simNow: 10, tag: "a" };
    const b = { simNow: 20, tag: "b" };
    expect(pickNewest([a, null, b])?.tag).toBe("b");
    expect(pickNewest([null, undefined])).toBeNull();
    // ties keep the first candidate (the local one, by main.ts's ordering)
    expect(pickNewest([{ simNow: 5, tag: "first" }, { simNow: 5, tag: "second" }])?.tag).toBe("first");
  });
});

describe("SyncClient", () => {
  it("pull decodes the server body and sends the bearer token", async () => {
    let seen: RequestInit | null = null;
    const client = new SyncClient(
      settings,
      scripted((url, init) => {
        seen = init;
        expect(url).toBe("https://vpet.example/v1/pets/default");
        return new Response(JSON.stringify({ sim_now: 123, seq: 4, saved_at: "2026-09-21T00:00:00Z", blob_b64: toB64(blob) }), { status: 200 });
      }),
    );
    const remote = await client.pull();
    expect(remote?.simNow).toBe(123);
    expect(remote?.seq).toBe(4);
    expect(remote?.blob).toEqual(blob);
    expect((seen!.headers as Record<string, string>).Authorization).toBe("Bearer t0k");
  });

  it("pull returns null on 404 and throws a network error on 401/5xx/offline", async () => {
    expect(await new SyncClient(settings, scripted(() => new Response("", { status: 404 }))).pull()).toBeNull();
    await expect(new SyncClient(settings, scripted(() => new Response("", { status: 401 }))).pull()).rejects.toBeInstanceOf(SyncNetworkError);
    await expect(new SyncClient(settings, scripted(() => new Response("", { status: 503 }))).pull()).rejects.toBeInstanceOf(SyncNetworkError);
    const offline = (async () => {
      throw new TypeError("Failed to fetch");
    }) as unknown as typeof fetch;
    await expect(new SyncClient(settings, offline).pull()).rejects.toBeInstanceOf(SyncNetworkError);
  });

  it("push sends X-Sim-Now and the raw bytes, and maps 200 / 409 / 422", async () => {
    let seen: RequestInit | null = null;
    const ok = new SyncClient(
      settings,
      scripted((_url, init) => {
        seen = init;
        return new Response(JSON.stringify({ seq: 7 }), { status: 200 });
      }),
    );
    expect(await ok.push(blob, 99)).toEqual({ kind: "ok", seq: 7 });
    const headers = seen!.headers as Record<string, string>;
    expect(headers["X-Sim-Now"]).toBe("99");
    expect(headers["Content-Type"]).toBe("application/octet-stream");
    expect(new Uint8Array(seen!.body as ArrayBuffer)).toEqual(blob);

    const conflict = new SyncClient(
      settings,
      scripted(() => new Response(JSON.stringify({ sim_now: 500, seq: 9, saved_at: "x", blob_b64: toB64(blob) }), { status: 409 })),
    );
    const r = await conflict.push(blob, 99);
    expect(r.kind).toBe("conflict");
    if (r.kind === "conflict") expect(r.remote.simNow).toBe(500);

    const tooNew = new SyncClient(
      settings,
      scripted(() => new Response(JSON.stringify({ error: "blob is from a newer core; the server needs rebuilding" }), { status: 422 })),
    );
    const rej = await tooNew.push(blob, 99);
    expect(rej.kind).toBe("rejected");
    if (rej.kind === "rejected") expect(rej.versionTooNew).toBe(true);
  });
});
