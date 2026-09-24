import type { ReactNode } from "react";
import type { Tone } from "./Shell";

export interface PillProps {
  children: ReactNode;
  /** Colour by species: `pink`, `orange`, `lilac`. Default lilac. */
  tone?: Tone;
}

/** A small rounded tag, for species names and stages. */
export function Pill({ children, tone }: PillProps) {
  return (
    <span className="vp-pill" data-tone={tone}>
      {children}
    </span>
  );
}
