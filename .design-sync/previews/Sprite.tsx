import { Sprite } from "@vpet/ds";

export const Pets = () => (
  <div style={{ display: "flex", gap: 16, alignItems: "flex-end" }}>
    <Sprite sprite="lalafu/adult/idle_a" title="lalafu" />
    <Sprite sprite="ninjifu/adult/idle_a" title="ninjifu" />
    <Sprite sprite="charamofu/adult/idle_a" title="charamofu" />
  </div>
);
export const Icons = () => (
  <div style={{ display: "flex", gap: 12 }}>
    <Sprite sprite="icon/feed" scale={3} />
    <Sprite sprite="icon/play" scale={3} />
    <Sprite sprite="icon/status" scale={3} />
    <Sprite sprite="icon/battle" scale={3} />
  </div>
);
export const EffectsAndItems = () => (
  <div style={{ display: "flex", gap: 12 }}>
    <Sprite sprite="effect/heart" scale={3} color="var(--vp-red)" />
    <Sprite sprite="effect/zz_a" scale={3} color="var(--vp-sky)" />
    <Sprite sprite="item/food_a" scale={3} color="var(--vp-orange)" />
    <Sprite sprite="egg/egg_a" scale={2} />
  </div>
);
export const BlinkAndFlip = () => (
  <div style={{ display: "flex", gap: 16 }}>
    <Sprite sprite="lalafu/adult/idle_a" />
    <Sprite sprite="lalafu/adult/idle_a" blink />
    <Sprite sprite="lalafu/adult/idle_a" flip />
  </div>
);
