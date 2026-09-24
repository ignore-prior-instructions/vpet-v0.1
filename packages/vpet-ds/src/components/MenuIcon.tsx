import { Sprite } from "./Sprite";
import type { SpriteName } from "../sprites.gen";

export type MenuAction = "feed" | "lights" | "play" | "medicine" | "clean" | "status" | "discipline" | "battle";

export const MENU_ACTIONS: MenuAction[] = ["feed", "lights", "play", "medicine", "clean", "status", "discipline", "battle"];

export interface MenuIconProps {
  /** One of the eight menu actions, in cursor order. */
  action: MenuAction;
  /** Drawn inverted, the way the cursor highlights an icon on the screen. */
  selected?: boolean;
  /** Icon pixel size. Default 3. */
  scale?: number;
  onClick?: () => void;
}

/** One 12x12 menu icon from the game, in a rounded tile. */
export function MenuIcon({ action, selected = false, scale = 3, onClick }: MenuIconProps) {
  const cls = ["vp-menuicon", selected ? "vp-menuicon--selected" : ""].join(" ").trim();
  const body = <Sprite sprite={`icon/${action}` as SpriteName} scale={scale} title={action} />;
  return onClick ? (
    <button type="button" className={cls} onClick={onClick} aria-pressed={selected} aria-label={action}>
      {body}
    </button>
  ) : (
    <span className={cls} aria-label={action}>
      {body}
    </span>
  );
}

export interface IconBarProps {
  /** Index of the highlighted action, 0-7, or undefined for none. */
  selected?: number;
  /** Which actions to show; default all eight. */
  actions?: MenuAction[];
  onSelect?: (index: number) => void;
}

/** The eight menu icons in a row, one highlighted, like the screen's top and bottom edges. */
export function IconBar({ selected, actions = MENU_ACTIONS, onSelect }: IconBarProps) {
  return (
    <div className="vp-iconbar" role="toolbar" aria-label="menu">
      {actions.map((a, i) => (
        <MenuIcon key={a} action={a} selected={i === selected} onClick={onSelect ? () => onSelect(i) : undefined} />
      ))}
    </div>
  );
}
