# Web host

Plain TypeScript, Vite, a canvas. No framework, no wasm-bindgen
([ADR 0010](../adr/0010-web-host-plain-ts-setinterval.md)).

## Loading

`just wasm` builds `vpet-abi` for `wasm32-unknown-unknown` with `opt-level = "z"`, `lto = "fat"`,
`panic = "abort"`, runs `wasm-opt -Oz`, and copies `vpet.wasm` into `hosts/web/public/`.

```ts
const { instance } = await WebAssembly.instantiateStreaming(fetch(wasmUrl), {});
const e = instance.exports as VpetExports;
if (e.vpet_abi_version() !== ABI_VERSION) throw new Error("core/host ABI mismatch");
const mem = e.memory as WebAssembly.Memory;
const frame = new Uint8Array(mem.buffer, e.vpet_frame_ptr(), 64);
const io    = new Uint8Array(mem.buffer, e.vpet_io_ptr(), 2048);
```

`now_ms` is passed as `BigInt(Date.now() + devOffset)`.

## Blitter

A 32x16 canvas with `putImageData` from the 64-byte frame, scaled by CSS
(`width: 512px; image-rendering: pixelated`). Theme colours (OLED cyan-on-black default, LCD
green as an option) are applied when filling the `ImageData`. Sixty-four bytes in, one
`putImageData` out; nothing else.

## Ticking

`setInterval(tick, 100)`, not `requestAnimationFrame`. rAF runs at 60 Hz, which is wasteful for
a display that changes at 4 Hz, and it pauses in background tabs. Ten Hz gives button latency
under 100 ms; the core is designed for any call rate.

- `visibilitychange` to visible: pull from the server, then call `update` immediately so the
  fast-forward happens before the first paint.
- `pagehide`: save and push with `fetch(url, { keepalive: true })`.

## Buttons

Keyboard `Z X C` (or `1 2 3`) and three on-screen buttons. `pointerdown`/`pointerup` maintain
the held mask; `pointercancel` and `blur` clear it. Touch works without special handling.

## Persistence

- IndexedDB, raw API, one object store, key `pet:default` -> `Uint8Array`. About 40 lines.
  Chosen over localStorage because it stores bytes natively.
- localStorage only for host settings: server URL, token, pet id, theme, dev clock offset.
- Save on `SAVE_NEEDED` debounced 500 ms, a checkpoint every 30 s, and on `pagehide`.
- Server sync as in [SYNC.md](../SYNC.md), debounced 2 s for pushes.

## Dev panel

Shown on `localhost` or with `?dev`. It renders `inspect` as a table, offers a clock-offset
control (`+1 h`, `+1 day`, custom) added to `Date.now()` for fast-forward testing, a `.vlog`
recorder and downloader, and blob import/export as hex (the manual transfer path to an ESP32
over serial before WiFi sync exists).

`/dev/sprites` is the hot-preview page described in [CONTENT.md](../CONTENT.md): it loads the
`dev-overrides` build of the core, watches `assets/**/*.txt` through Vite HMR, and renders any
clip at any tick through the real compositor.

## Deployment

Static. GitHub Pages from `hosts/web/dist` with `base` set for the repo path; the server URL is
a runtime setting, not a build-time one, so one build serves local-only and synced use.

Live at https://ignore-prior-instructions.github.io/vpet-v0.1/ . `.github/workflows/pages.yml`
publishes on every push to `main` (and on demand): it builds `vpet.wasm` exactly as `just wasm`
does, then `npm run build` with `VITE_BASE=/vpet-v0.1/`, which `vite.config.ts` passes to Vite
as `base`. Anything fetched from `public/` must go through `import.meta.env.BASE_URL` (the wasm
URLs do), since Vite only rewrites references it can see in HTML and imports. The Pages site is
configured for workflow deploys (`build_type: workflow`), so no `gh-pages` branch exists.
