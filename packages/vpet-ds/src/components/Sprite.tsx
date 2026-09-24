import type { CSSProperties } from "react";
import { pixelPath, type PackedSprite } from "../packed";
import { SPRITES, type SpriteName } from "../sprites.gen";

export interface SpriteProps {
  /** A sprite by name (`"lalafu/adult/idle_a"`, `"icon/feed"`, `"effect/heart"`, `"egg/egg_a"`)
   * or a packed sprite object. Names are the real game art from `assets/`. */
  sprite: SpriteName | PackedSprite;
  /** Pixel size in CSS px. Default 4. */
  scale?: number;
  /** Draw the sprite with its blink mask applied (eyes closed), if it has one. */
  blink?: boolean;
  /** Mirror horizontally (the core flips the pet when it walks right). */
  flip?: boolean;
  /** Ink colour. Defaults to the ink token; species tones look good here too. */
  color?: string;
  className?: string;
  style?: CSSProperties;
  title?: string;
}

export function resolveSprite(sprite: SpriteName | PackedSprite): PackedSprite {
  return typeof sprite === "string" ? SPRITES[sprite] : sprite;
}

function xorHex(a: string, b: string): string {
  let out = "";
  for (let i = 0; i < a.length; i += 2) {
    const v = parseInt(a.substr(i, 2), 16) ^ parseInt(b.substr(i, 2), 16);
    out += v.toString(16).padStart(2, "0");
  }
  return out;
}

/** One sprite from the game's art, scaled up with crisp pixels. */
export function Sprite({ sprite, scale = 4, blink = false, flip = false, color, className, style, title }: SpriteProps) {
  const s = resolveSprite(sprite);
  const img = blink && s.mask ? xorHex(s.img, s.mask) : s.img;
  const cls = ["vp-sprite", className ?? ""].join(" ").trim();
  const colorStyle = color ? ({ "--vp-sprite-color": color } as CSSProperties) : undefined;
  return (
    <span className={cls} style={{ ...colorStyle, ...style }} title={title}>
      <svg
        width={s.w * scale}
        height={s.h * scale}
        viewBox={`0 0 ${s.w} ${s.h}`}
        style={flip ? { transform: "scaleX(-1)" } : undefined}
        aria-hidden={title ? undefined : true}
      >
        {title ? <title>{title}</title> : null}
        <path d={pixelPath(img, s.w, s.h)} />
      </svg>
    </span>
  );
}
