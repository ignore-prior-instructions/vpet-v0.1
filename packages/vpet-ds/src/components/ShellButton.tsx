import type { ButtonHTMLAttributes } from "react";

export interface ShellButtonProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children"> {
  /** The physical button: A selects left / cycles, B confirms, C cancels (docs/GAME_DESIGN.md). */
  label: "A" | "B" | "C" | string;
  /** Default `md`. */
  size?: "sm" | "md" | "lg";
  /** Render in the pressed-down position (for mockups of a held button). */
  pressed?: boolean;
}

/** One of the three round buttons on the device body. Held-state input: the host reports press
 * and release, so wire both `onPointerDown` and `onPointerUp` in a live app. */
export function ShellButton({ label, size = "md", pressed = false, className, ...rest }: ShellButtonProps) {
  const cls = ["vp-shellbutton", `vp-shellbutton--${size}`, pressed ? "vp-shellbutton--pressed" : "", className ?? ""]
    .join(" ")
    .trim();
  return (
    <button type="button" className={cls} aria-label={`button ${label}`} {...rest}>
      {label}
    </button>
  );
}
