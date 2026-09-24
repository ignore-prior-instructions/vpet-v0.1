import type { InputHTMLAttributes } from "react";

export interface FieldProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "id"> {
  label: string;
  /** Small text under the input. */
  help?: string;
  /** Input id; derived from the label when omitted. */
  id?: string;
}

/** A labelled text input on the cream background. */
export function Field({ label, help, id, className, ...rest }: FieldProps) {
  const inputId = id ?? `vp-field-${label.toLowerCase().replace(/[^a-z0-9]+/g, "-")}`;
  const cls = ["vp-field", className ?? ""].join(" ").trim();
  return (
    <label className={cls} htmlFor={inputId}>
      <span className="vp-field__label">{label}</span>
      <input id={inputId} className="vp-field__input" {...rest} />
      {help ? <span className="vp-field__help">{help}</span> : null}
    </label>
  );
}
