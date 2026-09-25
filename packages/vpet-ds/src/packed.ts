/** A sprite exactly as the core stores it (docs/art/SPRITE_FORMAT.md "Bit packing"): `w` x `h`
 * pixels, `img` is hex, one bit per pixel, row-major, rows padded to whole bytes, MSB is the
 * leftmost pixel. `mask` is the optional blink mask in the same packing. */
export interface PackedSprite {
  w: number;
  h: number;
  img: string;
  mask: string | null;
}

/** The 64x32 screen (docs/HOST_ABI.md "Frame format"): 256 bytes, 8 per row. */
export const SCREEN_W = 64;
export const SCREEN_H = 32;

function hexByte(hex: string, i: number): number {
  return parseInt(hex.substr(i * 2, 2), 16);
}

/** Is pixel (x, y) lit in a packed bitmap of width `w`? */
export function bit(hex: string, w: number, x: number, y: number): boolean {
  const stride = Math.ceil(w / 8);
  const byte = hexByte(hex, y * stride + (x >> 3));
  return ((byte >> (7 - (x & 7))) & 1) === 1;
}

/** One SVG path covering every lit pixel, one unit per pixel: `M x y h1 v1 h-1 z` runs merged
 * horizontally so the path stays short. Pure, so it renders identically in a static card and a
 * live app. */
export function pixelPath(hex: string, w: number, h: number): string {
  const parts: string[] = [];
  for (let y = 0; y < h; y++) {
    let x = 0;
    while (x < w) {
      if (!bit(hex, w, x, y)) {
        x++;
        continue;
      }
      let run = 1;
      while (x + run < w && bit(hex, w, x + run, y)) run++;
      parts.push(`M${x} ${y}h${run}v1h-${run}z`);
      x += run;
    }
  }
  return parts.join("");
}

/** Pack text rows (`#`/`o` lit, anything else off, the spritekit grid alphabet) into hex. Handy
 * for pasting a golden frame from `tests/golden/*.expected` straight into a preview. */
export function packRows(rows: string[], w: number): string {
  const stride = Math.ceil(w / 8);
  let out = "";
  for (const row of rows) {
    for (let b = 0; b < stride; b++) {
      let byte = 0;
      for (let i = 0; i < 8; i++) {
        const c = row[b * 8 + i];
        if (c === "#" || c === "o") byte |= 1 << (7 - i);
      }
      out += byte.toString(16).padStart(2, "0");
    }
  }
  return out;
}
