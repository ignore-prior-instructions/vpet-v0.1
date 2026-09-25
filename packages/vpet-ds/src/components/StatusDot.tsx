export interface StatusDotProps {
  /** `ok` green, `warn` orange, `error` red, `idle` lilac. */
  level: "ok" | "warn" | "error" | "idle";
  /** Text after the dot, e.g. "Synced 2 min ago". */
  children?: string;
}

/** The sync status indicator: a coloured dot with an optional caption. */
export function StatusDot({ level, children }: StatusDotProps) {
  return (
    <span className={`vp-statusdot vp-statusdot--${level}`}>
      <span className="vp-statusdot__dot" aria-hidden />
      {children ? <span>{children}</span> : null}
    </span>
  );
}
