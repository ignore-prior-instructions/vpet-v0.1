// The blitter: 64 bytes in, one putImageData out, nothing else (docs/hosts/web.md "Blitter").
// Frame format: docs/HOST_ABI.md — 32x16, one bit per pixel, row-major, 4 bytes per row, MSB is
// the leftmost pixel.

const WIDTH = 32;
const HEIGHT = 16;
const STRIDE = 4; // bytes per row, ceil(32/8)

export type Theme = "oled" | "lcd";

/** Same values as tools/spritekit/spritekit/render.py's STYLES, so a browser screenshot and a
 * `spritekit render --style` preview match. */
const PALETTES: Record<Theme, { on: [number, number, number]; off: [number, number, number] }> = {
  oled: { on: [159, 232, 255], off: [0, 0, 0] },
  lcd: { on: [26, 36, 24], off: [184, 200, 160] },
};

function getPixel(frame: Uint8Array, x: number, y: number): boolean {
  const byteIndex = y * STRIDE + (x >> 3);
  const bit = 7 - (x & 7);
  return ((frame[byteIndex] >> bit) & 1) !== 0;
}

export function blit(ctx: CanvasRenderingContext2D, frame: Uint8Array, theme: Theme): void {
  const { on, off } = PALETTES[theme];
  const image = ctx.createImageData(WIDTH, HEIGHT);
  for (let y = 0; y < HEIGHT; y++) {
    for (let x = 0; x < WIDTH; x++) {
      const [r, g, b] = getPixel(frame, x, y) ? on : off;
      const i = (y * WIDTH + x) * 4;
      image.data[i] = r;
      image.data[i + 1] = g;
      image.data[i + 2] = b;
      image.data[i + 3] = 255;
    }
  }
  ctx.putImageData(image, 0, 0);
}
