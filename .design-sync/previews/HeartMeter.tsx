import { HeartMeter } from "@vpet/ds";

export const StatusPage = () => (
  <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
    <HeartMeter label="Hungry" value={3} />
    <HeartMeter label="Happy" value={4} />
    <HeartMeter label="Discipline" value={1} />
  </div>
);
export const Empty = () => <HeartMeter label="Hungry" value={0} />;
export const Bigger = () => <HeartMeter value={2} scale={3} />;
