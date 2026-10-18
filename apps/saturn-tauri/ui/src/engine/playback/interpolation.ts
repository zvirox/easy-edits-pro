import type { KeyframeProperty, Clip } from "./types.ts";

export function solveCubicBezier(t: number, x1: number, y1: number, x2: number, y2: number): number {
  const clampedT = Math.max(0, Math.min(1, t));
  if (clampedT <= 0) return 0;
  if (clampedT >= 1) return 1;

  // B(s) = 3(1-s)^2*s*P1 + 3(1-s)*s^2*P2 + s^3
  const cx = 3 * x1;
  const bx = 3 * (x2 - x1) - cx;
  const ax = 1 - cx - bx;

  const cy = 3 * y1;
  const by = 3 * (y2 - y1) - cy;
  const ay = 1 - cy - by;

  const sampleCurveX = (s: number) => ((ax * s + bx) * s + cx) * s;
  const sampleCurveY = (s: number) => ((ay * s + by) * s + cy) * s;
  const sampleCurveDerivativeX = (s: number) => (3 * ax * s + 2 * bx) * s + cx;

  // Newton-Raphson iteration
  let s = clampedT;
  for (let i = 0; i < 8; i++) {
    const x = sampleCurveX(s) - clampedT;
    if (Math.abs(x) < 1e-5) return sampleCurveY(s);
    const d = sampleCurveDerivativeX(s);
    if (Math.abs(d) < 1e-5) break;
    s -= x / d;
  }

  // Binary subdivision fallback
  let lo = 0;
  let hi = 1;
  s = clampedT;
  for (let i = 0; i < 12; i++) {
    const x = sampleCurveX(s);
    if (Math.abs(x - clampedT) < 1e-4) return sampleCurveY(s);
    if (clampedT > x) lo = s;
    else hi = s;
    s = (hi + lo) / 2;
  }

  return sampleCurveY(s);
}

/**
 * Extracts active subtitle item at a specific timeline playhead in ticks.
 */
export function interpolateKeyframe(
  clip: Clip | undefined,
  property: KeyframeProperty,
  clipTimeTicks: number,
  fallback: number
): number {
  if (!clip || !clip.keyframes) return fallback;
  const values = clip.keyframes.filter((k) => k.property === property).sort((a, b) => a.time - b.time);
  if (!values.length) return fallback;
  if (clipTimeTicks <= values[0].time) return values[0].value;
  const last = values[values.length - 1];
  if (clipTimeTicks >= last.time) return last.value;
  const nextIndex = values.findIndex((k) => k.time >= clipTimeTicks);
  const prev = values[nextIndex - 1];
  const next = values[nextIndex];
  if (!prev || !next) return fallback;
  const span = next.time - prev.time;
  if (span <= 0) return prev.value;
  const ratio = Math.max(0, Math.min(1, (clipTimeTicks - prev.time) / span));

  const interp = prev.interpolation ?? "linear";
  let progress = ratio;

  switch (interp) {
    case "linear":
      progress = ratio;
      break;
    case "ease_in":
      progress = solveCubicBezier(ratio, 0.42, 0.0, 1.0, 1.0);
      break;
    case "ease_out":
      progress = solveCubicBezier(ratio, 0.0, 0.0, 0.58, 1.0);
      break;
    case "ease_in_out":
      progress = solveCubicBezier(ratio, 0.42, 0.0, 0.58, 1.0);
      break;
    case "bezier": {
      const x1 = prev.handle_out_x ?? 0.33;
      const y1 = prev.handle_out_y ?? 0.0;
      const x2 = next.handle_in_x ?? 0.67;
      const y2 = next.handle_in_y ?? 1.0;
      progress = solveCubicBezier(ratio, x1, y1, x2, y2);
      break;
    }
    default:
      progress = ratio;
      break;
  }

  return prev.value + (next.value - prev.value) * progress;
}


/**
 * Calculates real-time visual transition styling for Dual-Deck video playback.
 */
