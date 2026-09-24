import type { CSSProperties, ReactNode } from "react";
import { Button } from "./Button";

export interface SheetProps {
  title: string;
  /** One line under the title, in the soft ink. */
  hint?: string;
  children: ReactNode;
  /** Buttons for the bottom row, e.g. Save / Sign out. */
  actions?: ReactNode;
  /** Shows a close button in the header when set. */
  onClose?: () => void;
  className?: string;
  style?: CSSProperties;
}

/** A rounded card for settings and dialogs: title, hint, fields, then an action row. Sits at the
 * bottom of the page as a sheet on phones, centred on desktop. */
export function Sheet({ title, hint, children, actions, onClose, className, style }: SheetProps) {
  const cls = ["vp-sheet", className ?? ""].join(" ").trim();
  return (
    <section className={cls} style={style} aria-label={title}>
      <header className="vp-sheet__head">
        <h2 className="vp-sheet__title">{title}</h2>
        {onClose ? (
          <Button variant="ghost" size="sm" onClick={onClose} aria-label="close">
            ✕
          </Button>
        ) : null}
      </header>
      {hint ? <p className="vp-sheet__hint">{hint}</p> : null}
      <div className="vp-sheet__body">{children}</div>
      {actions ? <div className="vp-sheet__row">{actions}</div> : null}
    </section>
  );
}
