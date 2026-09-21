// The ABI boundary: loads vpet.wasm and wraps its eight exports in a typed surface. See
// docs/HOST_ABI.md and crates/vpet-abi/src/lib.rs. No game logic lives here or anywhere else
// in this host (CLAUDE.md "Hosts are dumb blitters") — this module only marshals bytes.

import { decodeInspect, type Inspect } from "./inspect";

export const ABI_VERSION = 3; // 64x32 frames (ADR 0016) + vpet_peek_sim_now (docs/SYNC.md)

/** Mirrors `vpet_core::buttons` (crates/vpet-core/src/lib.rs). */
export const Buttons = {
  A: 1 << 0,
  B: 1 << 1,
  C: 1 << 2,
} as const;

/** Mirrors `vpet_core::flags` (crates/vpet-core/src/lib.rs). */
export const Flags = {
  FRAME_CHANGED: 1 << 0,
  SAVE_NEEDED: 1 << 1,
  BEEP: 1 << 2,
  ATTENTION: 1 << 3,
} as const;

/** Mirrors `vpet_core::save::LoadError` (crates/vpet-core/src/save/mod.rs). */
export const LoadError = {
  BAD_MAGIC: -1,
  BAD_CRC: -2,
  VERSION_TOO_NEW: -3,
  TOO_LONG: -4,
  DECODE: -5,
  BAD_CONTENT_REF: -6,
} as const;

export function loadErrorName(code: number): string {
  const entry = Object.entries(LoadError).find(([, v]) => v === code);
  return entry ? entry[0] : `UNKNOWN(${code})`;
}

const FRAME_LEN = 256;
const IO_CAP = 2048;

interface VpetExports {
  memory: WebAssembly.Memory;
  vpet_abi_version(): number;
  vpet_frame_ptr(): number;
  vpet_io_ptr(): number;
  vpet_reset(nowMs: bigint, seed: bigint): void;
  vpet_load(len: number): number;
  vpet_save(): number;
  vpet_update(nowMs: bigint, buttons: number): number;
  vpet_inspect(): number;
  vpet_peek_sim_now(len: number): number;
}

export class Core {
  private readonly exports: VpetExports;
  private frameView: Uint8Array;
  private ioView: Uint8Array;

  private constructor(exports: VpetExports) {
    this.exports = exports;
    this.frameView = new Uint8Array(exports.memory.buffer, exports.vpet_frame_ptr(), FRAME_LEN);
    this.ioView = new Uint8Array(exports.memory.buffer, exports.vpet_io_ptr(), IO_CAP);
  }

  static async load(wasmUrl: string): Promise<Core> {
    let instance: WebAssembly.Instance;
    try {
      ({ instance } = await WebAssembly.instantiateStreaming(fetch(wasmUrl), {}));
    } catch {
      // Fallback for servers that don't send `application/wasm` (docs/hosts/web.md doesn't
      // anticipate this, but Vite's own dev server has been known to get it wrong for some
      // static-file configurations).
      const resp = await fetch(wasmUrl);
      const bytes = await resp.arrayBuffer();
      ({ instance } = await WebAssembly.instantiate(bytes, {}));
    }
    const exports = instance.exports as unknown as VpetExports;
    const version = exports.vpet_abi_version();
    if (version !== ABI_VERSION) {
      throw new Error(`core/host ABI mismatch: core is v${version}, host expects v${ABI_VERSION}`);
    }
    return new Core(exports);
  }

  /** The core's linear memory never grows (no allocator), but re-view defensively in case that
   * ever changes; a detached buffer after a `memory.grow` would otherwise read as zeros. */
  private refreshViewsIfDetached(): void {
    if (this.frameView.buffer.byteLength === 0 || this.frameView.buffer !== this.exports.memory.buffer) {
      this.frameView = new Uint8Array(this.exports.memory.buffer, this.exports.vpet_frame_ptr(), FRAME_LEN);
      this.ioView = new Uint8Array(this.exports.memory.buffer, this.exports.vpet_io_ptr(), IO_CAP);
    }
  }

  reset(nowMs: number, seed: bigint): void {
    this.exports.vpet_reset(BigInt(Math.trunc(nowMs)), seed);
    this.refreshViewsIfDetached();
  }

  update(nowMs: number, buttons: number): number {
    const flags = this.exports.vpet_update(BigInt(Math.trunc(nowMs)), buttons);
    this.refreshViewsIfDetached();
    return flags;
  }

  /** A fresh copy of the current 64-byte frame (not a live view: callers may hold onto it). */
  frame(): Uint8Array {
    return this.frameView.slice();
  }

  /** `null` while Uninit (docs/HOST_ABI.md: `vpet_save` returns 0 for Uninit). */
  save(): Uint8Array | null {
    const len = this.exports.vpet_save();
    if (len === 0) return null;
    return this.ioView.slice(0, len);
  }

  /** Returns 0 on success, or a negative `LoadError` code. */
  load(blob: Uint8Array): number {
    if (blob.length > IO_CAP) return LoadError.TOO_LONG;
    this.ioView.set(blob, 0);
    return this.exports.vpet_load(blob.length);
  }

  inspect(): Inspect {
    const len = this.exports.vpet_inspect();
    return decodeInspect(this.ioView, len);
  }

  /** The `sim_now` inside `blob` without loading it (docs/SYNC.md: compare candidates, load
   * only the winner). `null` if the blob does not parse. */
  peekSimNow(blob: Uint8Array): number | null {
    if (blob.length > IO_CAP) return null;
    this.ioView.set(blob, 0);
    const v = this.exports.vpet_peek_sim_now(blob.length) >>> 0;
    return v === 0xffffffff ? null : v;
  }
}
