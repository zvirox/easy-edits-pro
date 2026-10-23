import type { AutomationInterpolation, AutomationPoint } from "./types.ts";

export type { AutomationInterpolation };

/** Minimum playhead distance (ticks) between two recorded automation points;
 * writes closer than this collapse into the previous point (~4 ms apart).
 * Mirrors AUTOMATION_RECORD_GAP_TICKS in saturn-core. */
export const AUTOMATION_RECORD_GAP_TICKS = 1_000_000_000;

/** Click tolerance (ticks) when selecting an automation lane point (~4 ms).
 * Mirrors AUTOMATION_EDIT_SNAP_TICKS in saturn-core. */
export const AUTOMATION_EDIT_SNAP_TICKS = 2_000_000_000;

export type AutomationLaneName = "volume" | "pan";

/** Volume automation lives in dB, pan in -1..+1. */
export const AUTOMATION_VOLUME_MIN_DB = -60;
export const AUTOMATION_VOLUME_MAX_DB = 12;
export const AUTOMATION_PAN_RANGE = 1;

function normalize(points: AutomationPoint[] | undefined): AutomationPoint[] {
  return (points ?? [])
    .filter((point) => Number.isFinite(point.time) && Number.isFinite(point.value))
    .slice()
    .sort((a, b) => a.time - b.time);
}

/** Half-cosine ease between two points; at 0.5 progress the value sits in the
 * middle (mirrors AutomationInterpolation::Smooth in saturn-core). */
function smooth(progress: number): number {
  return 0.5 * (1 - Math.cos(progress * Math.PI));
}

/** Evaluate an automation lane in timeline ticks with the given interpolation
 * mode. Static parameter value is the fallback when the lane is empty. Values
 * hold at the first and last points. */
export function evaluateAutomationLane(
  points: AutomationPoint[] | undefined,
  timeTicks: number,
  fallback: number,
  interpolation: AutomationInterpolation = "linear",
): number {
  const sorted = normalize(points);
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
    const eased =
      interpolation === "constant"
        ? 0
        : interpolation === "smooth"
        ? smooth(progress)
        : progress;
    return left.value + (right.value - left.value) * eased;
  }
  return last.value;
}

/** Legacy-linear evaluation kept for callers that pre-date interpolation
 * modes (behaves identically to `evaluateAutomationLane(..., "linear")`). */
export function evaluateAutomation(
  points: AutomationPoint[] | undefined,
  timeTicks: number,
  fallback: number,
): number {
  return evaluateAutomationLane(points, timeTicks, fallback, "linear");
}

/** Clamp an automation value to its lane's legal domain. */
export function clampAutomationValue(lane: AutomationLaneName, value: number): number {
  if (!Number.isFinite(value)) return lane === "volume" ? 0 : 0;
  return lane === "volume"
    ? Math.min(AUTOMATION_VOLUME_MAX_DB, Math.max(AUTOMATION_VOLUME_MIN_DB, value))
    : Math.min(AUTOMATION_PAN_RANGE, Math.max(-AUTOMATION_PAN_RANGE, value));
}

/** Records a Latch/Write-style point into a lane copy: the written value
 * replaces any point written within the record gap and keeps the lane
 * time-sorted. Returns the new point list (does not mutate the input). */
export function recordLanePoint(
  points: AutomationPoint[] | undefined,
  timeTicks: number,
  value: number,
): AutomationPoint[] {
  const sorted = normalize(points);
  const last = sorted[sorted.length - 1];
  if (last && timeTicks <= last.time + AUTOMATION_RECORD_GAP_TICKS) {
    return [...sorted.slice(0, -1), { time: timeTicks, value }];
  }
  return [...sorted, { time: timeTicks, value }];
}

/** Finds the index of the lane point nearest to `timeTicks` (within the edit
 * snap window), or -1 when none is close enough. */
export function findPointIndexNear(
  points: AutomationPoint[] | undefined,
  timeTicks: number,
): number {
  const sorted = normalize(points);
  const half = AUTOMATION_EDIT_SNAP_TICKS / 2;
  let best = -1;
  let bestDistance = Infinity;
  sorted.forEach((point, index) => {
    const distance = Math.abs(point.time - timeTicks);
    if (distance <= half && distance < bestDistance) {
      best = index;
      bestDistance = distance;
    }
  });
  return best;
}

/** True when playback should follow a track's recorded curve. */
export function automationPlaysCurve(
  automation: { mode: string; volume_points?: AutomationPoint[]; pan_points?: AutomationPoint[] } | undefined,
): boolean {
  if (!automation || automation.mode === "off") return false;
  return Boolean(automation.volume_points?.length || automation.pan_points?.length);
}
