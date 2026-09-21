// Multi-page build (the main app + docs/CONTENT.md's `/dev/sprites` hot-preview page) and the
// Vitest config for the grid-parser parity test, in one file since Vitest reads the `test`
// key straight out of a Vite config.
import { resolve } from "node:path";
import { defineConfig } from "vitest/config";

const root = import.meta.dirname;

export default defineConfig({
  server: {
    fs: {
      // `/dev/sprites` glob-imports `assets/**/*.txt` as raw text (docs/CONTENT.md), which
      // lives at the repo root, two levels above this project's root (hosts/web) -- outside
      // Vite's default served-files allowlist.
      allow: [resolve(root, "..", "..")],
    },
  },
  build: {
    rollupOptions: {
      input: {
        main: resolve(root, "index.html"),
        devSprites: resolve(root, "dev/sprites.html"),
      },
    },
  },
  test: {
    include: ["src/**/*.test.ts"],
  },
});
