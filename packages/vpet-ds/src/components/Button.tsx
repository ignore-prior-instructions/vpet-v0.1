import type { ButtonHTMLAttributes, ReactNode } from "react";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  children: ReactNode;
  /** `primary` is pink, `secondary` lilac, `danger` bow-red, `ghost` borderless. Default `primary`. */
  variant?: "primary" | "secondary" | "danger" | "ghost";
  /** Default `md`. */
  size?: "sm" | "md" | "lg";
}

/** A chunky rounded button with the ink outline and a drop edge. For app UI (settings, dev
 * panel), not for the device's physical buttons: that's `ShellButton`. */
export function Button({ children, variant = "primary", size = "md", className, ...rest }: ButtonProps) {
  const cls = ["vp-button", `vp-button--${variant}`, `vp-button--${size}`, className ?? ""].join(" ").trim();
  return (
    <button type="button" className={cls} {...rest}>
      {children}
    </button>
  );
}
