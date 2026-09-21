// Phase 0 skeleton: proves the canvas blitter pipeline works, with a hardcoded test pattern
// standing in for a real frame. Phase 2 replaces `testPatternFrame()` with bytes read from
// `vpet.wasm`'s `vpet_frame_ptr()` (docs/hosts/web.md "Blitter"; docs/HOST_ABI.md "Frame
// format").

const WIDTH = 32;
const HEIGHT = 16;
const STRIDE = 4; // bytes per row, ceil(32/8)

// Theme colours: OLED cyan-on-black default (docs/hosts/web.md).
const ON_COLOR: [number, number, number] = [159, 232, 255];
const OFF_COLOR: [number, number, number] = [0, 0, 0];

function testPatternFrame(): Uint8Array {
  // A simple diagonal stripe pattern, decodable by eye: pixel (x, y) on iff (x + y) % 4 === 0.
  const frame = new Uint8Array(64);
  for (let y = 0; y < HEIGHT; y++) {
    for (let x = 0; x < WIDTH; x++) {
      if ((x + y) % 4 === 0) {
        const byteIndex = y * STRIDE + Math.floor(x / 8);
        const bit = 7 - (x % 8);
        frame[byteIndex] |= 1 << bit;
      }
    }
  }
  return frame;
}

/** Reads pixel (x, y) from a 64-byte frame per docs/HOST_ABI.md's "Frame format": row-major, 4
 * bytes per row, MSB is the leftmost pixel. */
function getPixel(frame: Uint8Array, x: number, y: number): boolean {
  const byteIndex = y * STRIDE + Math.floor(x / 8);
  const bit = 7 - (x % 8);
  return ((frame[byteIndex] >> bit) & 1) !== 0;
}

function blit(ctx: CanvasRenderingContext2D, frame: Uint8Array): void {
  const image = ctx.createImageData(WIDTH, HEIGHT);
  for (let y = 0; y < HEIGHT; y++) {
    for (let x = 0; x < WIDTH; x++) {
      const on = getPixel(frame, x, y);
      const [r, g, b] = on ? ON_COLOR : OFF_COLOR;
      const i = (y * WIDTH + x) * 4;
      image.data[i] = r;
      image.data[i + 1] = g;
      image.data[i + 2] = b;
      image.data[i + 3] = 255;
    }
  }
  ctx.putImageData(image, 0, 0);
}

function main(): void {
  const canvas = document.getElementById("screen") as HTMLCanvasElement;
  const ctx = canvas.getContext("2d");
  if (!ctx) {
    throw new Error("2D canvas context unavailable");
  }
  blit(ctx, testPatternFrame());
}

main();
