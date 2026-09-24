import type { CSSProperties } from "react";
import { pixelPath } from "../packed";
import { SPRITES, type SpriteName } from "../sprites.gen";

export interface PixelTextProps {
  /** Rendered in the game's 5x7 font: A-Z, 0-9, `! ? - . :` and space. Lowercase is uppercased,
   * anything else becomes a space. */
  children: string;
  /** Pixel size in CSS px. Default 3. */
  scale?: number;
  /** Ink colour. Defaults to the ink token. */
  color?: string;
  className?: string;
  style?: CSSProperties;
}

const ADVANCE = 6;
const GLYPH_H = 7;
const PUNCT: Record<string, string> = { "!": "excl", "?": "quest", "-": "dash", ".": "dot", ":": "colon" };

function glyphName(c: string): SpriteName | null {
  const up = c.toUpperCase();
  if (/[A-Z]/.test(up)) return `glyph/${up.toLowerCase()}` as SpriteName;
  if (/[0-9]/.test(up)) return `glyph/${up}` as SpriteName;
  const p = PUNCT[up];
  return p ? (`glyph/${p}` as SpriteName) : null;
}

/** Text set in the pet's own 5x7 pixel font, for labels that should feel like the screen. */
export function PixelText({ children, scale = 3, color, className, style }: PixelTextProps) {
  const parts: string[] = [];
  let x = 0;
  for (const c of children) {
    const name = glyphName(c);
    const g = name ? SPRITES[name as keyof typeof SPRITES] : undefined;
    if (g) {
      const d = pixelPath(g.img, g.w, g.h);
      if (d) parts.push(`<g transform="translate(${x} 0)"><path d="${d}"/></g>`);
    }
    x += ADVANCE;
  }
  const w = Math.max(x - 1, 1);
  const cls = ["vp-pixeltext", className ?? ""].join(" ").trim();
  const colorStyle = color ? ({ "--vp-sprite-color": color } as CSSProperties) : undefined;
  return (
    <span className={cls} style={{ ...colorStyle, ...style }} aria-label={children}>
      <svg
        width={w * scale}
        height={GLYPH_H * scale}
        viewBox={`0 0 ${w} ${GLYPH_H}`}
        dangerouslySetInnerHTML={{ __html: parts.join("") }}
      />
    </span>
  );
}
