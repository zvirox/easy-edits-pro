import type { PanelRect } from "./types.ts";
import type { ViewportSize } from "./snapping.ts";

/**
 * Phase 4 deferred: dock zones with ghost preview (G4a).
 *
 * A dock zone is an edge of the floating region. Dropping a panel onto a zone
 * reflows it to cover that half of the region, like a split view. Zone maths is
 * pure and pixel-based so it can be unit tested without a DOM.
 */

export type DockZone = "left" | "right" | "top" | "bottom";

export const DOCK_ZONES: readonly DockZone[] = ["left", "right", "top", "bottom"];

/**
 * Distance from a region edge (in pixels) within which a drag counts as a
 * dock gesture. Deliberately larger than the 12px snap threshold so docking
 * is an explicit, easy-to-hit action rather than an accident.
 */
export const DOCK_ZONE_THRESHOLD_PX = 64;

/**
 * Which region edge is the pointer nearest, or null when it is beyond the
 * dock threshold from every edge. Corners resolve to the closer axis: the
 * smaller of the two edge distances wins, and a horizontal/vertical tie
 * prefers the horizontal zones because side docks are more common in editors.
 */
export function dockZoneAt(
  pointerX: number,
  pointerY: number,
  region: ViewportSize,
  threshold: number = DOCK_ZONE_THRESHOLD_PX,
): DockZone | null {
  if (region.width <= 0 || region.height <= 0) return null;
  const left = pointerX;
  const right = region.width - pointerX;
  const top = pointerY;
  const bottom = region.height - pointerY;

  const horizontal = left <= right ? "left" : "right";
  const horizontalDistance = Math.min(left, right);
  const vertical = top <= bottom ? "top" : "bottom";
  const verticalDistance = Math.min(top, bottom);

  if (horizontalDistance <= threshold || verticalDistance <= threshold) {
    return horizontalDistance <= verticalDistance ? horizontal : vertical;
  }
  return null;
}

/**
 * The rect a panel occupies when docked to a zone, in viewport fractions so it
 * survives window resizes like every other layout value. Left/right zones take
 * the full height; top/bottom zones take the full width.
 */
export function zoneRect(zone: DockZone, _viewport: ViewportSize): PanelRect {
  const half = 0.5;
  switch (zone) {
    case "left":
      return { x: 0, y: 0, width: half, height: 1 };
    case "right":
      return { x: half, y: 0, width: half, height: 1 };
    case "top":
      return { x: 0, y: 0, width: 1, height: half };
    case "bottom":
      return { x: 0, y: half, width: 1, height: half };
  }
}

/**
 * The ghost overlay rect for a zone, in region-local pixels, for the drop
 * target highlight the plan calls for.
 */
export function zonePixels(
  zone: DockZone,
  viewport: ViewportSize,
): { x: number; y: number; width: number; height: number } {
  const rect = zoneRect(zone, viewport);
  return {
    x: rect.x * viewport.width,
    y: rect.y * viewport.height,
    width: rect.width * viewport.width,
    height: rect.height * viewport.height,
  };
}
