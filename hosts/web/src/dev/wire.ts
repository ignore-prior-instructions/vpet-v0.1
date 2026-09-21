// Binary encoders for the `dev_set_pose` / `dev_set_global` / `dev_render_clip` payloads.
// Layouts are documented (and must stay in lockstep with) the `dev_exports` module doc comment
// in crates/vpet-abi/src/lib.rs.

class Writer {
  private bytes: number[] = [];

  u8(v: number): this {
    this.bytes.push(v & 0xff);
    return this;
  }

  i8(v: number): this {
    return this.u8(v < 0 ? v + 256 : v);
  }

  u32le(v: number): this {
    this.bytes.push(v & 0xff, (v >>> 8) & 0xff, (v >>> 16) & 0xff, (v >>> 24) & 0xff);
    return this;
  }

  strU8Len(s: string): this {
    const encoded = new TextEncoder().encode(s);
    if (encoded.length > 255) throw new Error(`string too long for a u8 length prefix: ${s}`);
    this.u8(encoded.length);
    this.bytes.push(...encoded);
    return this;
  }

  bytesRaw(b: Uint8Array): this {
    this.bytes.push(...b);
    return this;
  }

  finish(): Uint8Array {
    return new Uint8Array(this.bytes);
  }
}

export interface PoseOverride {
  slug: string;
  stage: number; // 0=baby, 1=child, 2=adult, 3=adult_alt
  name: string;
  w: number;
  h: number;
  img: Uint8Array;
  mask: Uint8Array | null;
  mouth: [number, number];
  bbox: [number, number, number, number];
}

export function encodeSetPose(p: PoseOverride): Uint8Array {
  const w = new Writer()
    .strU8Len(p.slug)
    .u8(p.stage)
    .strU8Len(p.name)
    .u8(p.w)
    .u8(p.h)
    .i8(p.mouth[0])
    .i8(p.mouth[1])
    .u8(p.bbox[0])
    .u8(p.bbox[1])
    .u8(p.bbox[2])
    .u8(p.bbox[3])
    .u8(p.mask ? 1 : 0)
    .bytesRaw(p.img);
  if (p.mask) w.bytesRaw(p.mask);
  return w.finish();
}

export interface GlobalOverride {
  category: string;
  name: string;
  w: number;
  h: number;
  img: Uint8Array;
}

export function encodeSetGlobal(g: GlobalOverride): Uint8Array {
  return new Writer().strU8Len(g.category).strU8Len(g.name).u8(g.w).u8(g.h).bytesRaw(g.img).finish();
}

export function encodeRenderClip(slug: string, stage: number, activity: string, tick: number): Uint8Array {
  return new Writer().strU8Len(slug).u8(stage).strU8Len(activity).u32le(tick).finish();
}
