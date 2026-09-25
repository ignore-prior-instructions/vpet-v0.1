import type { CSSProperties } from "react";
import { packRows, pixelPath, SCREEN_H, SCREEN_W } from "../packed";

export interface ScreenProps {
  /** The 64x32 frame: 512 hex chars as the core emits it, or 32 text rows of `.`/`#` pasted
   * from a golden (`tests/golden/*.expected`). */
  frame: string | string[];
  /** Pixel size in CSS px. 6 gives a 384x192 screen. Default 6. */
  scale?: number;
  /** Swap ink and background, the way the core does when the lights are off. */
  inverted?: boolean;
  className?: string;
  style?: CSSProperties;
}

/** The pet's 64x32 one-bit screen, drawn with ink pixels on a warm pale panel. Pure: the same
 * frame always renders the same SVG, so it works in static cards and live apps alike. */
export function Screen({ frame, scale = 6, inverted = false, className, style }: ScreenProps) {
  const hex = Array.isArray(frame) ? packRows(frame, SCREEN_W) : frame;
  const cls = ["vp-screen", inverted ? "vp-screen--inverted" : "", className ?? ""].join(" ").trim();
  return (
    <div
      className={cls}
      style={{ width: SCREEN_W * scale, height: SCREEN_H * scale, ...style }}
      role="img"
      aria-label="pet screen"
    >
      <svg viewBox={`0 0 ${SCREEN_W} ${SCREEN_H}`} preserveAspectRatio="none">
        <path d={pixelPath(hex, SCREEN_W, SCREEN_H)} />
      </svg>
    </div>
  );
}
