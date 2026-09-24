import type { CSSProperties, ReactNode } from "react";
import { ShellButton } from "./ShellButton";

export type Tone = "pink" | "orange" | "lilac";

export interface ShellProps {
  /** The screen to frame, normally a `<Screen>`. */
  children: ReactNode;
  /** Body colour: `pink` (lalafu), `orange` (charamofu), `lilac` (ninjifu). Default `pink`. */
  tone?: Tone;
  /** Small caption under the screen, e.g. the pet's name. */
  name?: string;
  /** Show the red bow on top (lalafu's). Default true. */
  bow?: boolean;
  /** Button labels. Default A, B, C. */
  buttons?: [string, string, string];
  /** Called with the button's label on press and on release. */
  onPress?: (label: string) => void;
  onRelease?: (label: string) => void;
  className?: string;
  style?: CSSProperties;
}

/** The device body: an egg-shaped pastel shell with an ink outline, the screen in a white
 * window, and three round buttons. Everything on the screen itself is the core's job; this is
 * the frame around it. */
export function Shell({
  children,
  tone = "pink",
  name,
  bow = true,
  buttons = ["A", "B", "C"],
  onPress,
  onRelease,
  className,
  style,
}: ShellProps) {
  const cls = ["vp-shell", className ?? ""].join(" ").trim();
  return (
    <div className={cls} data-tone={tone} style={style}>
      {bow ? (
        <div className="vp-shell__bow" aria-hidden>
          <span className="vp-shell__bow-knot" />
        </div>
      ) : null}
      <span className="vp-shell__blush vp-shell__blush--left" aria-hidden />
      <span className="vp-shell__blush vp-shell__blush--right" aria-hidden />
      <div className="vp-shell__screen">{children}</div>
      {name ? <div className="vp-shell__name">{name}</div> : null}
      <div className="vp-shell__buttons">
        {buttons.map((label) => (
          <ShellButton
            key={label}
            label={label}
            onPointerDown={onPress ? () => onPress(label) : undefined}
            onPointerUp={onRelease ? () => onRelease(label) : undefined}
            onPointerCancel={onRelease ? () => onRelease(label) : undefined}
          />
        ))}
      </div>
    </div>
  );
}
