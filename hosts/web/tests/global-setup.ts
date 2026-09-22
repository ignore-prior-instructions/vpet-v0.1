// Starts a real vpet-server (built by `just web-e2e` / `cargo build -p vpet-server`) for the
// sync end-to-end test in smoke.spec.ts, and stops it afterwards. docs/TESTING.md "Web smoke".
import { spawn, type ChildProcess } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

export const SYNC_SERVER_URL = "http://127.0.0.1:18081";
export const SYNC_TOKEN = "e2e-token";

let child: ChildProcess | null = null;
let dataDir: string | null = null;

async function waitForHealth(url: string, timeoutMs: number): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const resp = await fetch(`${url}/v1/health`);
      if (resp.ok) return;
    } catch {
      // not up yet
    }
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error(`vpet-server did not answer /v1/health at ${url} within ${timeoutMs} ms`);
}

export default async function globalSetup(): Promise<() => Promise<void>> {
  const bin = resolve(import.meta.dirname, "../../../target/debug/vpet-server");
  dataDir = mkdtempSync(join(tmpdir(), "vpet-e2e-"));
  child = spawn(bin, [], {
    env: {
      ...process.env,
      VPET_TOKEN: SYNC_TOKEN,
      VPET_DATA: dataDir,
      VPET_BIND: "127.0.0.1:18081",
      VPET_CORS_ORIGINS: "http://localhost:4173,http://127.0.0.1:4173",
    },
    stdio: ["ignore", "inherit", "inherit"],
  });
  child.on("exit", (code) => {
    if (code !== null && code !== 0) console.error(`vpet-server exited with ${code}`);
  });
  await waitForHealth(SYNC_SERVER_URL, 10_000);

  return async () => {
    child?.kill();
    child = null;
    if (dataDir) rmSync(dataDir, { recursive: true, force: true });
    dataDir = null;
  };
}
