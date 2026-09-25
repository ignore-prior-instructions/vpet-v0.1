// The blitter: 256 bytes in, one putImageData out, nothing else (docs/hosts/web.md "Blitter").
// Frame format: docs/HOST_ABI.md — 64x32, one bit per pixel, row-major, 8 bytes per row, MSB is
// the leftmost pixel (docs/adr/0016-screen-64x32.md).

const WIDTH = 64;
const HEIGHT = 32;
const STRIDE = 8; // bytes per row, ceil(64/8)

export type Theme = "ink" | "oled" | "lcd";

/** `ink` is the shipped look: the design system's `--vp-screen-on`/`--vp-screen-off`
 * (packages/vpet-ds/src/styles.css), ink pixels on a warm pale panel. `oled`/`lcd` match
 * tools/spritekit/spritekit/render.py's STYLES so a screenshot and a `spritekit render --style`
 * preview can be compared; they are dev-panel options. */
const PALETTES: Record<Theme, { on: [number, number, number]; off: [number, number, number] }> = {
  ink: { on: [39, 30, 56], off: [253, 241, 234] },
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
