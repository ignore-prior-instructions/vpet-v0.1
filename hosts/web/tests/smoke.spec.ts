// Web smoke test (docs/TESTING.md "Web smoke"): load the page, press A/B/C, screenshot the
// canvas. Runs against the production build via `playwright.config.ts`'s webServer.

import { test, expect, type Page } from "@playwright/test";

async function waitReady(page: Page): Promise<void> {
  await page.waitForSelector('canvas#screen[data-ready="true"]', { timeout: 10_000 });
}

async function canvasIsBlank(page: Page): Promise<boolean> {
  return page.evaluate(() => {
    const canvas = document.getElementById("screen") as HTMLCanvasElement;
    const data = canvas.getContext("2d")!.getImageData(0, 0, 32, 16).data;
    for (let i = 0; i < data.length; i += 4) {
      if (data[i] !== 0 || data[i + 1] !== 0 || data[i + 2] !== 0) return false;
    }
    return true;
  });
}

async function canvasSnapshot(page: Page): Promise<string> {
  return page.evaluate(() => (document.getElementById("screen") as HTMLCanvasElement).toDataURL());
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
  await page.getByRole("button", { name: label, exact: true }).click();
  await page.waitForTimeout(150);
}

test("egg hatches, the menu responds to A/B/C, and state survives a reload", async ({ page }) => {
  await page.goto("/?dev");
  await waitReady(page);

  expect(await canvasIsBlank(page)).toBe(false);
  await expect.soft(inspectField(page, "stage")).resolves.toBe("Egg");

  // "an egg hatches in the browser" (docs/ROADMAP.md Phase 2 done criterion): jump the dev
  // clock forward past hatch_secs (300s for the Phase 1 placeholder species).
  await setDevOffset(page, "+1 day");
  await expect(inspectField(page, "stage")).resolves.toBe("Baby");

  // A/B/C navigate the menu.
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
