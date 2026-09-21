// Loads vpet-dev.wasm (built by `just wasm-dev`, feature `dev-overrides`) and wraps its four
// `dev_*` exports. Separate from `../core.ts`'s `Core`: this never touches a `Cart` at all
// (docs/CONTENT.md "Hot preview without rebuilding the core" — `dev_render_clip` renders any
// tick of any clip without simulation state), so there is no reset/update/save/load surface
// here, only the override setters and the clip renderer.

import { encodeRenderClip, encodeSetGlobal, encodeSetPose, type GlobalOverride, type PoseOverride } from "./wire";

const IO_CAP = 2048;
const FRAME_LEN = 256;

interface DevExports {
  memory: WebAssembly.Memory;
  vpet_io_ptr(): number;
  dev_set_pose(len: number): number;
  dev_set_global(len: number): number;
  dev_clear_overrides(): void;
  dev_render_clip(len: number): number;
}

export class DevCore {
  private readonly exports: DevExports;
  private ioView: Uint8Array;

  private constructor(exports: DevExports) {
    this.exports = exports;
    this.ioView = new Uint8Array(exports.memory.buffer, exports.vpet_io_ptr(), IO_CAP);
  }

  static async load(wasmUrl: string): Promise<DevCore> {
    let instance: WebAssembly.Instance;
    try {
      ({ instance } = await WebAssembly.instantiateStreaming(fetch(wasmUrl), {}));
    } catch {
      const resp = await fetch(wasmUrl);
      const bytes = await resp.arrayBuffer();
      ({ instance } = await WebAssembly.instantiate(bytes, {}));
    }
    return new DevCore(instance.exports as unknown as DevExports);
  }

  private refreshViewIfDetached(): void {
    if (this.ioView.buffer.byteLength === 0 || this.ioView.buffer !== this.exports.memory.buffer) {
      this.ioView = new Uint8Array(this.exports.memory.buffer, this.exports.vpet_io_ptr(), IO_CAP);
    }
  }

  private writeIo(payload: Uint8Array): void {
    if (payload.length > IO_CAP) throw new Error(`dev payload (${payload.length} bytes) exceeds IO_CAP (${IO_CAP})`);
    this.ioView.set(payload, 0);
  }

  /** Throws on a malformed payload (a bug in the encoder, not user error) rather than
   * returning a code the caller would just throw on anyway. */
  setPose(p: PoseOverride): void {
    const payload = encodeSetPose(p);
    this.writeIo(payload);
    const rc = this.exports.dev_set_pose(payload.length);
    this.refreshViewIfDetached();
    if (rc !== 0) throw new Error(`dev_set_pose failed with code ${rc}`);
  }

  setGlobal(g: GlobalOverride): void {
    const payload = encodeSetGlobal(g);
    this.writeIo(payload);
    const rc = this.exports.dev_set_global(payload.length);
    this.refreshViewIfDetached();
    if (rc !== 0) throw new Error(`dev_set_global failed with code ${rc}`);
  }

  clearOverrides(): void {
    this.exports.dev_clear_overrides();
  }

  /** A fresh 64-byte frame (docs/HOST_ABI.md "Frame format"), or all-zero if `activity` is
   * unrecognized or the species/stage has no pose data yet (see `crates/vpet-core/src/dev.rs`
   * — this is a preview tool, not simulation, so there is no error path to surface here). */
  renderClip(slug: string, stage: number, activity: string, tick: number): Uint8Array {
    const payload = encodeRenderClip(slug, stage, activity, tick);
    this.writeIo(payload);
    const len = this.exports.dev_render_clip(payload.length);
    this.refreshViewIfDetached();
    if (len !== FRAME_LEN) return new Uint8Array(FRAME_LEN);
    return this.ioView.slice(0, FRAME_LEN);
  }
}
