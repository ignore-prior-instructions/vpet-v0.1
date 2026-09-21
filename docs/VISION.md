# Vision

## What this is

A virtual pet in the Gen-1 lineage: a 64x32 one-bit screen, three buttons, a creature that
hatches, eats, poops, sleeps, gets sick, evolves based on how well you looked after it, and
eventually dies. It should feel like the 1997 object, not like a phone game.

It should also be *one* thing that lives in several places. The same pet in a browser tab, on a
server so it follows you between devices, and eventually on a small board with an OLED and three
tactile buttons that you can carry around.

## What happened in v0

`vpet-v0-ts` was a Vite + TypeScript browser app. It got two things right that carry forward:

- The game design docs: lifecycle, needs, actions, the idea of care mistakes driving evolution.
- Timestamp-based state: stats stored as "value at time T plus a rate", so the current state is
  derived on demand and a closed tab costs nothing. This idea is the spine of v0.1.

It got stuck on two things:

- **The stack had no seam between simulation and presentation.** Rules, DOM, storage, and dev tools
  were interleaved in one process. There was no way to run the simulation anywhere else.
- **The art was unusable.** Sprites were 32x20 full-screen frames drawn by a language model as raw
  character grids, one frame per position of a walk cycle, with no validator. One species has 17
  near-duplicate frames; one frame has 21 rows and nothing noticed. The model was asked to do a
  job with no ruler, no mirror, and no way to see its output.

The game modes were never implemented, so there is no game logic to port. Only the design and the
timestamp idea come across.

## Four principles for v0.1

1. **One core, thin hosts.** The simulation, the rules, and the renderer are a single `no_std`
   Rust crate with a tiny host interface. It compiles to WebAssembly for the browser and natively
   for the ESP32. Hosts pass in time and buttons and copy out 64 bytes of pixels. Nothing else.
   If a host is making a decision about the game, the design is wrong.

2. **Deterministic by construction.** Integer math only, time and entropy supplied by the host,
   event-stepped fast-forward. Given a save blob and "now", every host computes the same pet and
   the same frame. This is what makes "runs on the server" and "runs offline" the same code, and it
   is what makes the whole thing testable with golden replays.

3. **The server is dumb.** It stores a blob. It does not tick the pet, because the pet does not
   need ticking; the core derives elapsed time on the next `update`. Multi-device sync is
   last-writer-wins by simulated time. A hobby project does not need CRDTs.

4. **Model-made art, with tooling.** Language and image models draw the sprites, but each pose
   is drawn once at cell size, motion is procedural, and every frame passes a validator and a
   render-and-critique loop before a human looks at a contact sheet and approves it. The model gets
   a ruler, a mirror, and a picture of what it drew.

## Non-goals

- Multiplayer or pet-to-pet interaction over the network.
- Accounts, logins, or anyone but the owner playing.
- Color, sound beyond a beep, or a screen larger than 64x32.
- Running the core as an interpreter on the device. Native compilation is the same source.
