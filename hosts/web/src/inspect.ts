// Decodes the `Inspect` struct the core writes into the IO buffer on `vpet_inspect()`. The
// layout is `#[repr(C)]` and pinned by a compile-time assertion in
// crates/vpet-abi/src/lib.rs (`const _: () = { ... }`); this decoder's offsets must match that
// assertion exactly. See docs/HOST_ABI.md "Inspect layout".

export interface Inspect {
  abiVersion: number;
  saveVersion: number;
  contentHash: number;
  state: number; // 0 Uninit, 1 Alive, 2 Dead
  species: number;
  stage: number; // 0 Egg, 1 Baby, 2 Child, 3 Adult, 4 AdultAlt
  hunger: number;
  happiness: number;
  discipline: number;
  health: number;
  weight: number;
  ageSecs: number;
  flags: number; // bit0 sick, bit1 sleeping, bit2 lights_off, bit3 attention_any
  attention: number;
  poops: number;
  careMistakes: number;
  simNow: number;
  nextEventAt: number; // Sec; NEVER (u32::MAX) if none
}

export const INSPECT_SIZE = 40;

export function decodeInspect(io: Uint8Array, len: number): Inspect {
  if (len !== INSPECT_SIZE) {
    throw new Error(`vpet_inspect returned ${len} bytes, expected ${INSPECT_SIZE}`);
  }
  const view = new DataView(io.buffer, io.byteOffset, INSPECT_SIZE);
  return {
    abiVersion: view.getUint32(0, true),
    saveVersion: view.getUint16(4, true),
    contentHash: view.getUint32(8, true),
    state: view.getUint8(12),
    species: view.getUint8(13),
    stage: view.getUint8(14),
    hunger: view.getUint8(15),
    happiness: view.getUint8(16),
    discipline: view.getUint8(17),
    health: view.getUint8(18),
    weight: view.getUint16(20, true),
    ageSecs: view.getUint32(24, true),
    flags: view.getUint8(28),
    attention: view.getUint8(29),
    poops: view.getUint8(30),
    careMistakes: view.getUint8(31),
    simNow: view.getUint32(32, true),
    nextEventAt: view.getUint32(36, true),
  };
}

export const NEVER = 0xffffffff;

export const Stage = {
  0: "Egg",
  1: "Baby",
  2: "Child",
  3: "Adult",
  4: "AdultAlt",
} as const;

export const CartState = {
  0: "Uninit",
  1: "Alive",
  2: "Dead",
} as const;
