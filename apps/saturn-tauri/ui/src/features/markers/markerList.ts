import type { Marker, MarkerKind } from "../../app/projectTypes";
import { TICKS_PER_SECOND } from "../../utils/formatters.ts";

/** Minimal marker shape accepted by the pure helpers. */
export type MarkerLike = Pick<
  Marker,
  "id" | "name" | "kind" | "start_ticks" | "duration_ticks" | "color"
>;

export const MARKER_KINDS: { value: MarkerKind; label: string; color: string }[] = [
  { value: "comment", label: "Comment", color: "var(--accent-violet-300)" },
  { value: "chapter", label: "Chapter", color: "var(--accent-orange)" },
  { value: "flash_cue", label: "Flash Cue", color: "var(--accent-primary)" },
  { value: "web_link", label: "Web Link", color: "var(--accent-bright)" },
  { value: "segmentation", label: "Segmentation", color: "var(--accent-green)" },
];

export function markerColor(marker: MarkerLike): string {
  if (marker.color) return marker.color;
  return MARKER_KINDS.find((kind) => kind.value === marker.kind)?.color ?? MARKER_KINDS[0].color;
}

/** Timeline order: start tick, then id so equal starts stay stable. */
export function sortedMarkers<T extends MarkerLike>(markers: readonly T[]): T[] {
  return [...markers].sort(
    (left, right) => left.start_ticks - right.start_ticks || left.id - right.id,
  );
}

export function nextMarker<T extends MarkerLike>(
  markers: readonly T[],
  from: number,
): T | null {
  let best: T | null = null;
  for (const marker of markers) {
    if (marker.start_ticks <= from) continue;
    if (!best || marker.start_ticks < best.start_ticks) best = marker;
  }
  return best;
}

export function previousMarker<T extends MarkerLike>(
  markers: readonly T[],
  from: number,
): T | null {
  let best: T | null = null;
  for (const marker of markers) {
    if (marker.start_ticks >= from) continue;
    if (!best || marker.start_ticks > best.start_ticks) best = marker;
  }
  return best;
}

export function isRangeMarker(marker: MarkerLike): boolean {
  return marker.duration_ticks > 0;
}

/** Left offset in pixels for the current timeline zoom (pixels per second). */
export function markerLeftPx(marker: MarkerLike, pixelsPerSecond: number): number {
  return (Math.max(0, marker.start_ticks) / TICKS_PER_SECOND) * pixelsPerSecond;
}

/** Rendered width in pixels; point markers get a small clickable target. */
export function markerWidthPx(marker: MarkerLike, pixelsPerSecond: number): number {
  if (!isRangeMarker(marker)) return 0;
  return Math.max(4, (marker.duration_ticks / TICKS_PER_SECOND) * pixelsPerSecond);
}

/**
 * Range for `Add Range Marker`: the sequence in/out when both are set and
 * ordered, otherwise two seconds from the playhead.
 */
export function defaultMarkerRange(
  playhead: number,
  markIn: number | null | undefined,
  markOut: number | null | undefined,
): { start: number; end: number } {
  if (
    typeof markIn === "number" &&
    typeof markOut === "number" &&
    Number.isFinite(markIn) &&
    Number.isFinite(markOut) &&
    markOut > markIn
  ) {
    return { start: markIn, end: markOut };
  }
  return { start: playhead, end: playhead + TICKS_PER_SECOND * 2 };
}

/** Convert a signed frame count into a signed tick delta for marker ripple. */
export function markerShiftTicks(
  frames: number,
  frameRate: { numerator: number; denominator: number },
): number {
  const frameTicks = Math.round((TICKS_PER_SECOND * frameRate.denominator) / frameRate.numerator);
  return Math.round(frames * frameTicks);
}
