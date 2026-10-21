import {
  DEFAULT_CONSTRAINTS,
  type PanelConstraints,
  type PanelRect,
  type PanelRectMap,
} from "./types.ts";

export interface ViewportSize {
  width: number;
  height: number;
}

/**
 * Clamp a panel rect into the viewport, honouring the panel's min/max sizes.
 * Fractions in, fractions out; pixel conversions happen only at the edges.
 */
export function clampPanelRect(
  rect: PanelRect,
  viewport: ViewportSize,
  constraints: PanelConstraints = DEFAULT_CONSTRAINTS,
): PanelRect {
  if (viewport.width <= 0 || viewport.height <= 0) return { ...rect };

  const minWidthPx = constraints.minWidth * viewport.width;
  const minHeightPx = constraints.minHeight * viewport.height;
  const maxWidthPx = Math.min(
    constraints.maxWidth * viewport.width,
    viewport.width,
  );
  const maxHeightPx = Math.min(
    constraints.maxHeight * viewport.height,
    viewport.height,
  );

  const height = clamp(rect.height * viewport.height, minHeightPx, maxHeightPx);
  const width = clamp(rect.width * viewport.width, minWidthPx, maxWidthPx);
  const x = clamp(
    rect.x * viewport.width,
    0,
    Math.max(0, viewport.width - width),
  );
  const y = clamp(
    rect.y * viewport.height,
    0,
    Math.max(0, viewport.height - height),
  );

  return {
    x: x / viewport.width,
    y: y / viewport.height,
    width: width / viewport.width,
    height: height / viewport.height,
  };
}

function clamp(value: number, min: number, max: number): number {
  if (!Number.isFinite(value)) return min;
  if (min > max) return min;
  return Math.min(Math.max(value, min), max);
}

export interface SnapOptions {
  viewport: ViewportSize;
  /** Other floating panels' current pixel rects (id -> rect). */
  others?: {
    id: string;
    rect: { x: number; y: number; width: number; height: number };
  }[];
  threshold?: number;
  /** Interior margin from the viewport edges. */
  margin?: number;
  /** Directions that may snap; drag uses all, resize passes the moving edge. */
  edges?: readonly Edge[];
}

export type Edge = "left" | "right" | "top" | "bottom";

export const ALL_EDGES: readonly Edge[] = ["left", "right", "top", "bottom"];

/**
 * Snap a pixel-space rect during drag: find the closest matching target for the
 * left/right edges on the X axis and the top/bottom edges on the Y axis within
 * the threshold, then translate the whole rect by the winning deltas. Width and
 * height are untouched — snapping is a positional correction.
 */
export function snapRect(
  rect: { x: number; y: number; width: number; height: number },
  options: SnapOptions,
): { x: number; y: number; width: number; height: number } {
  const threshold = options.threshold ?? 12;
  const margin = options.margin ?? 0;
  const allowed = new Set(options.edges ?? ALL_EDGES);

  const horizontalTargets: number[] = [];
  const verticalTargets: number[] = [];
  if (allowed.has("left") || allowed.has("right")) {
    horizontalTargets.push(margin, options.viewport.width - margin);
    for (const panel of options.others ?? []) {
      horizontalTargets.push(panel.rect.x, panel.rect.x + panel.rect.width);
    }
  }
  if (allowed.has("top") || allowed.has("bottom")) {
    verticalTargets.push(margin, options.viewport.height - margin);
    for (const panel of options.others ?? []) {
      verticalTargets.push(panel.rect.y, panel.rect.y + panel.rect.height);
    }
  }

  const bestDelta = (
    targets: number[],
    current: number,
    edge: Edge,
  ): number | null => {
    if (!allowed.has(edge)) return null;
    let best: number | null = null;
    for (const target of targets) {
      const delta = target - current;
      if (
        Math.abs(delta) <= threshold &&
        (best === null || Math.abs(delta) < Math.abs(best))
      ) {
        best = delta;
      }
    }
    return best;
  };

  const snapped = { ...rect };
  const horizontalDelta =
    allowed.has("left") || allowed.has("right")
      ? (bestDelta(horizontalTargets, rect.x, "left") ??
        bestDelta(horizontalTargets, rect.x + rect.width, "right"))
      : null;
  const verticalDelta =
    allowed.has("top") || allowed.has("bottom")
      ? (bestDelta(verticalTargets, rect.y, "top") ??
        bestDelta(verticalTargets, rect.y + rect.height, "bottom"))
      : null;

  if (horizontalDelta !== null) snapped.x += horizontalDelta;
  if (verticalDelta !== null) snapped.y += verticalDelta;
  return snapped;
}

export interface EdgeTargets {
  viewport: ViewportSize;
  others?: { rect: { x: number; y: number; width: number; height: number } }[];
  threshold?: number;
  margin?: number;
}

/**
 * Snap one moving edge coordinate (in pixels) to the viewport margin or to any
 * other panel's edge within the snap threshold. Resize uses this so the
 * anchored edges never shift, unlike drag which shifts the whole rect.
 */
export function snapEdge(
  value: number,
  edge: Edge,
  options: EdgeTargets,
): number {
  const threshold = options.threshold ?? 12;
  const margin = options.margin ?? 0;
  const targets: number[] = [];
  if (edge === "left" || edge === "right") {
    targets.push(margin, options.viewport.width - margin);
    for (const panel of options.others ?? []) {
      targets.push(panel.rect.x, panel.rect.x + panel.rect.width);
    }
  } else {
    targets.push(margin, options.viewport.height - margin);
    for (const panel of options.others ?? []) {
      targets.push(panel.rect.y, panel.rect.y + panel.rect.height);
    }
  }
  let best = value;
  let bestDistance = Math.abs(threshold) + 1;
  for (const target of targets) {
    const distance = Math.abs(target - value);
    if (distance <= threshold && distance < bestDistance) {
      best = target;
      bestDistance = distance;
    }
  }
  return best;
}

export function rectToPixels(rect: PanelRect, viewport: ViewportSize) {
  return {
    x: rect.x * viewport.width,
    y: rect.y * viewport.height,
    width: rect.width * viewport.width,
    height: rect.height * viewport.height,
  };
}

export function pixelsToRect(
  pixels: { x: number; y: number; width: number; height: number },
  viewport: ViewportSize,
  constraints?: PanelConstraints,
): PanelRect {
  const rect: PanelRect = {
    x: viewport.width > 0 ? pixels.x / viewport.width : 0,
    y: viewport.height > 0 ? pixels.y / viewport.height : 0,
    width:
      viewport.width > 0
        ? pixels.width / viewport.width
        : DEFAULT_CONSTRAINTS.minWidth,
    height:
      viewport.height > 0
        ? pixels.height / viewport.height
        : DEFAULT_CONSTRAINTS.minHeight,
  };
  return clampPanelRect(rect, viewport, constraints);
}

/**
 * Validate layout data coming from disk. Returns a clamped map that only
 * contains entries for known panel ids. Unknown or malformed data is dropped,
 * never merged.
 */
export function sanitizePanelLayout(
  raw: unknown,
  knownPanelIds: readonly string[],
  viewport: ViewportSize,
): PanelRectMap {
  const rects: PanelRectMap = {};
  const allowed = new Set(knownPanelIds);
  if (typeof raw !== "object" || raw === null) return rects;
  for (const [id, value] of Object.entries(raw as Record<string, unknown>)) {
    if (!allowed.has(id) || typeof value !== "object" || value === null)
      continue;
    const candidate = value as Record<string, unknown>;
    const rect: PanelRect = {
      x: Number(candidate.x),
      y: Number(candidate.y),
      width: Number(candidate.width),
      height: Number(candidate.height),
    };
    if (![rect.x, rect.y, rect.width, rect.height].every(Number.isFinite))
      continue;
    if (rect.width <= 0 || rect.height <= 0) continue;
    rects[id] = clampPanelRect(rect, viewport);
  }
  return rects;
}
