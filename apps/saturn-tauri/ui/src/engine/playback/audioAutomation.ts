import type { AutomationPoint } from "./types.ts";

/** Evaluate an automation lane in timeline ticks; static parameter value is the
 * fallback when the lane is empty. Values hold at the first and last points. */
export function evaluateAutomation(points: AutomationPoint[] | undefined, timeTicks: number, fallback: number): number {
  if (!points?.length) return fallback;
  const sorted = points
    .filter((point) => Number.isFinite(point.time) && Number.isFinite(point.value))
    .slice()
    .sort((a, b) => a.time - b.time);
  if (!sorted.length) return fallback;
  if (timeTicks <= sorted[0].time) return sorted[0].value;
  const last = sorted[sorted.length - 1];
  if (timeTicks >= last.time) return last.value;
  for (let index = 1; index < sorted.length; index += 1) {
    const right = sorted[index];
    if (timeTicks > right.time) continue;
    const left = sorted[index - 1];
    if (right.time === left.time) return right.value;
    const progress = (timeTicks - left.time) / (right.time - left.time);
    return left.value + (right.value - left.value) * progress;
  }
  return last.value;
}
