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

A 64x32 canvas with `putImageData` from the 256-byte frame, scaled by CSS (`aspect-ratio:
2 / 1; image-rendering: pixelated`). The default palette is `ink`: ink pixels on a warm pale
panel, the design system's `--vp-screen-on`/`--vp-screen-off`. OLED cyan-on-black and LCD green
remain as dev-panel options that match `spritekit render --style`. Sixty-four bytes in, one
`putImageData` out; nothing else.

## The page around the screen

The device body, header, and sheets are plain HTML styled with `@vpet/ds`
(`packages/vpet-ds/src/styles.css`, imported by `style.css`): the same tokens and classes Claude
Design builds with, so a mockup and the shipped page share one stylesheet and there is no
framework at runtime (ADR 0010 still holds). The shell is the DS `vp-shell` markup with the
canvas in its screen window and three `vp-shellbutton`s; its `data-tone` (pink / lilac / orange)
comes from Settings. Phones: the shell shrinks to the viewport, keyboard hints are hidden on
touch devices, and the dev panel starts folded.

## Ticking

`setInterval(tick, 100)`, not `requestAnimationFrame`. rAF runs at 60 Hz, which is wasteful for
a display that changes at 4 Hz, and it pauses in background tabs. Ten Hz gives button latency
under 100 ms; the core is designed for any call rate.

- `visibilitychange` to visible: call `update` immediately so the fast-forward happens before
  the first paint, then pull from the server and reconcile (below).
- `pagehide`: save and push with `fetch(url, { keepalive: true })`.

## Buttons

Keyboard `Z X C` (or `1 2 3`) and three on-screen buttons. `pointerdown`/`pointerup` maintain
the held mask; `pointercancel` and `blur` clear it. Touch works without special handling.

## Persistence

- IndexedDB, raw API, one object store, key `pet:default` -> `Uint8Array`. About 40 lines.
  Chosen over localStorage because it stores bytes natively.
- localStorage only for host settings: server URL, token, pet id, theme, sound, shell colour,
  dev clock offset.
- Save on `SAVE_NEEDED` debounced 500 ms, a checkpoint every 30 s, and on `pagehide`.
- Server sync as in [SYNC.md](../SYNC.md), debounced 2 s for pushes.

## Sync

`src/sync.ts` is the [SYNC.md](../SYNC.md) client protocol with no game logic: settings in
localStorage (`vpet.sync.url`, `.token`, `.petId`; an empty URL means sync off), `pull()`,
`push(blob, simNow, keepalive)` and `remove()`. `src/main.ts` owns the policy:

- **Boot**: the stored blob (or a fresh egg) is loaded and painted first; only then does the
  page pull, so a slow server never delays the pet appearing. The pull has a 5 s timeout.
- **Reconcile** (boot, tab becomes visible, "Save" or "Sync now" in the sheet): compare the
  local blob's `sim_now` with the server's using `vpet_peek_sim_now` -- the candidate is never
  loaded into the live core just to inspect it, so a stale remote can't clobber the pet. If the
  server's is higher, load it, store it, and say so; if ours is higher or the server has
  nothing, push; if equal, nothing.
- **Push** on `SAVE_NEEDED`, debounced 2 s, with the blob as it is when the timer fires; on
  `pagehide` with `keepalive`. A 409 carries the server's blob: adopt it if its `sim_now` is
  higher, otherwise push again with the now-current blob.
- Any network failure leaves the pet playing locally; a 422 `VERSION_TOO_NEW` means the server
  binary needs rebuilding and is reported as such.

The core never moves time backwards, so a browser that adopts a blob from further ahead holds
at that `sim_now` until its own clock catches up (see `advance_to` in `vpet-core`).

## Settings sheet

**Settings** in the header (no `?dev` needed, phone-friendly) opens a sheet with sound on/off,
the shell colour, **New egg** (replaces a living pet through the host's own `reset` path, the
one boot uses; a dead pet restarts with A+C on the tombstone), and the sync section: server URL, token (password field), pet id, **Save** (which reconciles at once), **Sync now**,
**Start over on server** (`DELETE` behind a confirm dialog; the local pet is kept and uploaded
on its next change) and a status line: `sync off`, `synced <n>s ago`, `offline, playing
locally`, `adopted the newer save from the server`, `server needs updating; playing locally`.
The footer shows the status dot with its text next to it (`data-level` off / ok / warn /
error); clicking it opens Settings. **How to play** opens a static help sheet, which links to
the **cheat sheet** (also at `?cheatsheet`): the evolution rules in plain words.

## Dev panel

Shown on `localhost` or with `?dev`. It folds to a "Dev" button: click it to open, click
anywhere outside to fold (except the device's A/B/C, so you can play while watching it). It renders `inspect` as a table, offers a clock-offset
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
