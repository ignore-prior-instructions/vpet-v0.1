import { MenuIcon } from "@vpet/ds";

export const AllEight = () => (
  <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
    <MenuIcon action="feed" />
    <MenuIcon action="lights" />
    <MenuIcon action="play" />
    <MenuIcon action="medicine" />
    <MenuIcon action="clean" />
    <MenuIcon action="status" />
    <MenuIcon action="discipline" />
    <MenuIcon action="battle" />
  </div>
);
export const Selected = () => (
  <div style={{ display: "flex", gap: 8 }}>
    <MenuIcon action="feed" />
    <MenuIcon action="feed" selected />
  </div>
);
