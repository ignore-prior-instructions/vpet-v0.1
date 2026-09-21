import { defineConfig, devices } from "@playwright/test";

// Runs against the production build (dist/), so the test exercises the same bundle and the
// same copied vpet.wasm that ships (docs/TESTING.md "Web smoke").
export default defineConfig({
  testDir: "./tests",
  // Spawns a real vpet-server for the sync test (built by `just web-e2e`); see global-setup.ts.
  globalSetup: "./tests/global-setup.ts",
  fullyParallel: false,
  retries: 0,
  reporter: [["list"]],
  use: {
    baseURL: "http://localhost:4173",
    screenshot: "only-on-failure",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: {
    command: "npm run preview -- --port 4173 --strictPort",
    url: "http://localhost:4173",
    reuseExistingServer: !process.env.CI,
    timeout: 30_000,
  },
});
