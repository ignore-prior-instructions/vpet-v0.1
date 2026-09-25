import { StatusDot } from "@vpet/ds";

export const Levels = () => (
  <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
    <StatusDot level="ok">Synced 2 minutes ago</StatusDot>
    <StatusDot level="warn">Offline, will retry</StatusDot>
    <StatusDot level="error">Server rejected the pet code</StatusDot>
    <StatusDot level="idle">Not set up</StatusDot>
  </div>
);
export const DotOnly = () => <StatusDot level="ok" />;
