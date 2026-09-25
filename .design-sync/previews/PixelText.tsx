import { PixelText } from "@vpet/ds";

export const Label = () => <PixelText>Lalafu</PixelText>;
export const Larger = () => <PixelText scale={5}>Feed me!</PixelText>;
export const Toned = () => (
  <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
    <PixelText color="var(--vp-pink)">Hungry: 3 - 4</PixelText>
    <PixelText color="var(--vp-purple)">Age 2 days?</PixelText>
  </div>
);
