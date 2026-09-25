import { PetSprite } from "@vpet/ds";

export const ThreeAdults = () => (
  <div style={{ display: "flex", gap: 16, alignItems: "flex-end" }}>
    <PetSprite species="lalafu" color="var(--vp-pink)" />
    <PetSprite species="ninjifu" color="var(--vp-purple)" />
    <PetSprite species="charamofu" color="var(--vp-orange)" />
  </div>
);
export const Lifecycle = () => (
  <div style={{ display: "flex", gap: 16, alignItems: "flex-end" }}>
    <PetSprite species="lalafu" stage="baby" />
    <PetSprite species="lalafu" stage="child" />
    <PetSprite species="lalafu" stage="adult" />
  </div>
);
export const Poses = () => (
  <div style={{ display: "flex", gap: 12, alignItems: "flex-end" }}>
    <PetSprite species="charamofu" pose="idle_a" scale={3} />
    <PetSprite species="charamofu" pose="happy" scale={3} />
    <PetSprite species="charamofu" pose="eat" scale={3} />
    <PetSprite species="charamofu" pose="sleep" scale={3} />
    <PetSprite species="charamofu" pose="sad" scale={3} />
  </div>
);
