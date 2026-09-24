import { Button } from "@vpet/ds";

export const Variants = () => (
  <div style={{ display: "flex", gap: 12, flexWrap: "wrap" }}>
    <Button>Save</Button>
    <Button variant="secondary">Sync now</Button>
    <Button variant="danger">Sign out</Button>
    <Button variant="ghost">Cancel</Button>
  </div>
);
export const Sizes = () => (
  <div style={{ display: "flex", gap: 12, alignItems: "center" }}>
    <Button size="sm">Small</Button>
    <Button size="md">Medium</Button>
    <Button size="lg">Large</Button>
  </div>
);
export const Disabled = () => (
  <div style={{ display: "flex", gap: 12 }}>
    <Button disabled>Save</Button>
    <Button variant="secondary" disabled>Sync now</Button>
  </div>
);
