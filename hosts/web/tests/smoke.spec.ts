// Web smoke test (docs/TESTING.md "Web smoke"): load the page, press A/B/C, screenshot the
// canvas. Runs against the production build via `playwright.config.ts`'s webServer.

import { test, expect, type Page } from "@playwright/test";
import { SYNC_SERVER_URL, SYNC_TOKEN } from "./global-setup";

async function waitReady(page: Page): Promise<void> {
  await page.waitForSelector('canvas#screen[data-ready="true"]', { timeout: 10_000 });
}

async function canvasIsBlank(page: Page): Promise<boolean> {
  return page.evaluate(() => {
    const canvas = document.getElementById("screen") as HTMLCanvasElement;
    const data = canvas.getContext("2d")!.getImageData(0, 0, 64, 32).data;
    for (let i = 0; i < data.length; i += 4) {
      if (data[i] !== 0 || data[i + 1] !== 0 || data[i + 2] !== 0) return false;
    }
    return true;
  });
}

async function canvasSnapshot(page: Page): Promise<string> {
  return page.evaluate(() => (document.getElementById("screen") as HTMLCanvasElement).toDataURL());
}

/** The dev panel folds on any click outside it (e.g. the Settings button), so reopen it
 * before pressing anything inside, the way a person would. Reading `inspect` needs no reopen:
 * the table keeps updating while folded. */
async function openDevPanel(page: Page): Promise<void> {
  if (await page.locator("#devpanel").isHidden()) await page.locator("#dev-toggle").click();
}

async function inspectField(page: Page, label: string): Promise<string> {
  return page.locator(`table.inspect tr:has(td:text-is("${label}")) td:nth-child(2)`).innerText();
}

/** A key held down and released with an explicit gap: an instant down+up can complete between
 * two 100ms polls of the held-state mask and never be observed (confirmed manually — see
 * hosts/web/src/main.ts's host-flow comment and docs/HOST_ABI.md "Buttons"). */
async function press(page: Page, key: string): Promise<void> {
  await page.keyboard.down(key);
  await page.waitForTimeout(150);
  await page.keyboard.up(key);
  await page.waitForTimeout(150);
}

async function setDevOffset(page: Page, label: string): Promise<void> {
  await openDevPanel(page);
  await page.getByRole("button", { name: label, exact: true }).click();
  await page.waitForTimeout(150);
}

test("egg hatches, the menu responds to A/B/C, and state survives a reload", async ({ page }) => {
  await page.goto("/?dev");
  await waitReady(page);

  expect(await canvasIsBlank(page)).toBe(false);
  await expect.soft(inspectField(page, "stage")).resolves.toBe("Egg");

  // "an egg hatches in the browser" (docs/ROADMAP.md Phase 2 done criterion): jump the dev
  // clock forward past hatch_secs (300s for lalafu, the Phase 3 species). `+1 h` clears that
  // comfortably while staying well under a freshly-hatched baby's ~17h starve deadline (see
  // tests/golden/neglect_24h.vlog) -- `+1 day` did reach it, which silently made every button
  // press below a no-op (Cart::apply_input returns immediately once `state != Alive`) without
  // this test ever checking the `state` field to notice.
  await setDevOffset(page, "+1 h");
  await expect(inspectField(page, "stage")).resolves.toBe("Baby");
  await expect(inspectField(page, "state")).resolves.toBe("Alive"); // a dead pet ignores input below
  const beforeMenu = await canvasSnapshot(page);
  await press(page, "z"); // A: open the menu
  const menuOpen = await canvasSnapshot(page);
  expect(menuOpen).not.toBe(beforeMenu);

  await press(page, "z"); // A again: cursor moves
  const cursorMoved = await canvasSnapshot(page);
  expect(cursorMoved).not.toBe(menuOpen);

  await press(page, "c"); // C: close the menu
  const menuClosed = await canvasSnapshot(page);
  expect(menuClosed).not.toBe(cursorMoved);

  await page.screenshot({ path: "test-results/hatched-pet.png", fullPage: true });

  // "survives a reload" (docs/ROADMAP.md Phase 2 done criterion).
  const simNowBefore = Number(await inspectField(page, "sim_now"));
  await page.reload();
  await waitReady(page);
  await expect(inspectField(page, "stage")).resolves.toBe("Baby"); // no flash of a fresh egg
  const simNowAfter = Number(await inspectField(page, "sim_now"));
  expect(simNowAfter).toBeGreaterThanOrEqual(simNowBefore);
});

/** Open the gear sheet, enter the e2e server, save (which pulls and reconciles), close. */
async function configureSync(page: Page): Promise<void> {
  await page.locator("#gear").click();
  await page.locator("#settings [data-url]").fill(SYNC_SERVER_URL);
  await page.locator("#settings [data-token]").fill(SYNC_TOKEN);
  await page.locator("#settings [data-save]").click();
  await page.locator("#settings [data-close]").click();
}

function syncStatus(page: Page) {
  return page.locator("#syncdot");
}

/** The `sim_now` the server currently holds for the default pet, or -1 if none. */
async function serverSimNow(): Promise<number> {
  const resp = await fetch(`${SYNC_SERVER_URL}/v1/pets/default`, { headers: { Authorization: `Bearer ${SYNC_TOKEN}` } });
  if (resp.status === 404) return -1;
  return ((await resp.json()) as { sim_now: number }).sim_now;
}

// docs/ROADMAP.md Phase 5 done-criterion: two browsers hand the same pet back and forth, and a
// deliberate conflict resolves to the blob with the higher sim_now (docs/SYNC.md).
//
// Clock model: each context's sim clock is Date.now() plus its own dev offset. The core never
// moves time backwards (Cart::advance_to), so a context that adopts a blob from further ahead
// simply holds at that sim_now until its own clock catches up. Every "did the push land"
// check below therefore reads the server, not a page: a debounced push can fire between a
// dev-clock jump and the next inspect read, so a page's sim_now is not a lower bound for what
// the server holds.
test("two browsers share one pet through the server; a conflict resolves to the higher sim_now", async ({ browser }) => {
  // Browser A: a fresh pet, sync configured, hatched by jumping its (context-local) dev clock.
  const ctxA = await browser.newContext();
  const pageA = await ctxA.newPage();
  await pageA.goto("/?dev");
  await waitReady(pageA);
  await configureSync(pageA);
  await expect(syncStatus(pageA)).toHaveAttribute("title", /synced/, { timeout: 10_000 }); // uploaded: server had nothing
  const simA0 = Number(await inspectField(pageA, "sim_now"));
  await setDevOffset(pageA, "+1 h");
  await expect(inspectField(pageA, "stage")).resolves.toBe("Baby");
  // The hatch sets SAVE_NEEDED; its debounced push (2 s) has landed once the server holds a
  // save from an hour past where A started.
  await expect.poll(serverSimNow, { timeout: 10_000 }).toBeGreaterThanOrEqual(simA0 + 3600);
  const simA = await serverSimNow();

  // Browser B (separate context = separate IndexedDB/localStorage, no dev offset): starts a
  // fresh egg, then adopts A's pet on the first pull because it has simulated further.
  const ctxB = await browser.newContext();
  const pageB = await ctxB.newPage();
  await pageB.goto("/?dev");
  await waitReady(pageB);
  await expect(inspectField(pageB, "stage")).resolves.toBe("Egg");
  await configureSync(pageB);
  await expect.poll(() => inspectField(pageB, "stage"), { timeout: 10_000 }).toBe("Baby");
  await expect(syncStatus(pageB)).toHaveAttribute("title", /adopted/);
  const simB = Number(await inspectField(pageB, "sim_now"));
  expect(simB).toBeGreaterThanOrEqual(simA); // B holds at A's sim_now: its own clock is an hour behind

  // The deliberate conflict: B jumps two hours ahead (A's live clock is only one hour ahead of
  // real time, so one hour would not beat it) and pushes; A's next sync must adopt B's save
  // rather than overwrite it.
  await setDevOffset(pageB, "+1 h");
  await setDevOffset(pageB, "+1 h");
  const simB2 = Number(await inspectField(pageB, "sim_now"));
  expect(simB2).toBeGreaterThan(simB);
  await expect.poll(serverSimNow, { timeout: 10_000 }).toBeGreaterThanOrEqual(simB2);
  const serverB = await serverSimNow();

  await pageA.locator("#gear").click();
  await pageA.locator("#settings [data-sync]").click();
  await expect(syncStatus(pageA)).toHaveAttribute("title", /adopted/, { timeout: 10_000 });
  await expect.poll(() => inspectField(pageA, "sim_now").then(Number), { timeout: 10_000 }).toBeGreaterThanOrEqual(serverB);

  await ctxA.close();
  await ctxB.close();
});
