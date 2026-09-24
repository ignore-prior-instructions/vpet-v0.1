import { Sprite, type SpriteProps } from "./Sprite";
import type { SpriteName } from "../sprites.gen";

export type Species = "lalafu" | "ninjifu" | "charamofu";
export type Stage = "baby" | "child" | "adult";
export type Pose = "idle_a" | "idle_b" | "happy" | "eat" | "sleep" | "sad" | "attack";

export interface PetSpriteProps extends Omit<SpriteProps, "sprite"> {
  /** `lalafu` is the ghost, `ninjifu` the ninja, `charamofu` the dino (docs/art/LINEAGE.md). */
  species: Species;
  /** Default `adult`. Every line shares lalafu's baby and ninjifu's child. */
  stage?: Stage;
  /** Default `idle_a`. */
  pose?: Pose;
}

/** Which species actually owns each stage's art (`[assets.share]` in species.toml). */
const SHARED: Record<Stage, Partial<Record<Species, Species>>> = {
  baby: { ninjifu: "lalafu", charamofu: "lalafu" },
  child: { lalafu: "ninjifu", charamofu: "ninjifu" },
  adult: {},
};

/** Resolve a species/stage/pose to the sprite name that holds its art. */
export function petSpriteName(species: Species, stage: Stage = "adult", pose: Pose = "idle_a"): SpriteName {
  const owner = SHARED[stage][species] ?? species;
  return `${owner}/${stage}/${pose}` as SpriteName;
}

/** A pet by species, stage, and pose: the same 32x32 art the core draws. */
export function PetSprite({ species, stage = "adult", pose = "idle_a", scale = 4, ...rest }: PetSpriteProps) {
  return <Sprite sprite={petSpriteName(species, stage, pose)} scale={scale} {...rest} />;
}
