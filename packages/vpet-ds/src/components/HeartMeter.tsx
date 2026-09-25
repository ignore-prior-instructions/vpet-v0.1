import { Sprite } from "./Sprite";

export interface HeartMeterProps {
  /** Hearts filled, 0 to `max`. The status page shows hunger and happiness this way. */
  value: number;
  /** Default 4, like the status page. */
  max?: number;
  /** Label to the left, e.g. "Hungry" or "Happy". */
  label?: string;
  /** Heart pixel size. Default 2. */
  scale?: number;
}

/** A row of the game's own heart sprites, full and empty. */
export function HeartMeter({ value, max = 4, label, scale = 2 }: HeartMeterProps) {
  const filled = Math.max(0, Math.min(max, Math.round(value)));
  return (
    <div className="vp-heartmeter" role="meter" aria-valuenow={filled} aria-valuemin={0} aria-valuemax={max} aria-label={label}>
      {label ? <span className="vp-heartmeter__label">{label}</span> : null}
      <span className="vp-heartmeter__hearts">
        {Array.from({ length: max }, (_, i) => (
          <Sprite key={i} sprite={i < filled ? "heart/heart_full" : "heart/heart_empty"} scale={scale} />
        ))}
      </span>
    </div>
  );
}
