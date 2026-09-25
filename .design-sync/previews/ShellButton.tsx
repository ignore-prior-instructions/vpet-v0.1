import { ShellButton } from "@vpet/ds";

export const ABC = () => (
  <div style={{ display: "flex", gap: 16, alignItems: "flex-end" }}>
    <ShellButton label="A" />
    <ShellButton label="B" style={{ marginBottom: 10 }} />
    <ShellButton label="C" />
  </div>
);
export const Sizes = () => (
  <div style={{ display: "flex", gap: 16, alignItems: "center" }}>
    <ShellButton label="A" size="sm" />
    <ShellButton label="A" size="md" />
    <ShellButton label="A" size="lg" />
  </div>
);
export const Pressed = () => (
  <div style={{ display: "flex", gap: 16 }}>
    <ShellButton label="B" />
    <ShellButton label="B" pressed />
  </div>
);
export const Tones = () => (
  <div style={{ display: "flex", gap: 16 }}>
    <span data-tone="pink"><ShellButton label="A" /></span>
    <span data-tone="orange"><ShellButton label="B" /></span>
    <span data-tone="lilac"><ShellButton label="C" /></span>
  </div>
);
