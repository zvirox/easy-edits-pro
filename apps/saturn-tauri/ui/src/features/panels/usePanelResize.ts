import { snapEdge, type ViewportSize } from "./snapping.ts";
import {
  SNAP_THRESHOLD_PX,
  type PanelConstraints,
  type PanelRect,
} from "./types.ts";

export const RESIZE_DIRECTIONS = [
  "n",
  "ne",
  "e",
  "se",
  "s",
  "sw",
  "w",
  "nw",
] as const;
export type ResizeDirection = (typeof RESIZE_DIRECTIONS)[number];

interface ResizeOptions {
  viewport: ViewportSize;
  onResize: (next: PanelRect, direction: ResizeDirection) => void;
  onEnd?: () => void;
  others?: {
    id: string;
    rect: { x: number; y: number; width: number; height: number };
  }[];
}

interface ResizeStart {
  event: React.PointerEvent;
  direction: ResizeDirection;
  current: PanelRect;
  constraints: PanelConstraints;
}

/** Which rect edges a given handle moves; true = left/top edge, false = right/bottom edge, null = fixed. */
const DIRECTION_EDGES: Record<
  ResizeDirection,
  { left: boolean | null; top: boolean | null }
> = {
  n: { left: null, top: true },
  s: { left: null, top: false },
  e: { left: false, top: null },
  w: { left: true, top: null },
  ne: { left: false, top: true },
  nw: { left: true, top: true },
  se: { left: false, top: false },
  sw: { left: true, top: false },
};

/**
 * 8-handle resize (n, ne, e, se, s, sw, w, nw). The active rect is snapped
 * along the edges being moved only; the opposite edges stay anchored.
 */
export function usePanelResize({
  viewport,
  onResize,
  onEnd,
  others,
}: ResizeOptions) {
  function startResize({
    event,
    direction,
    current,
    constraints,
  }: ResizeStart) {
    if (event.button !== 0) return;
    if (viewport.width <= 0 || viewport.height <= 0) return;
    event.preventDefault();
    event.stopPropagation();

    const startPointer = { x: event.clientX, y: event.clientY };
    const oldRect = {
      x: current.x * viewport.width,
      y: current.y * viewport.height,
      width: current.width * viewport.width,
      height: current.height * viewport.height,
    };
    const oldRight = oldRect.x + oldRect.width;
    const oldBottom = oldRect.y + oldRect.height;
    const moveLeft = DIRECTION_EDGES[direction].left;
    const moveTop = DIRECTION_EDGES[direction].top;
    const handle = event.currentTarget as HTMLElement;
    handle.setPointerCapture(event.pointerId);

    const handleMove = (moveEvent: PointerEvent) => {
      const deltaX = moveEvent.clientX - startPointer.x;
      const deltaY = moveEvent.clientY - startPointer.y;
      const snapOptions = { viewport, others, threshold: SNAP_THRESHOLD_PX };

      const next = { ...oldRect };
      if (moveLeft !== null) {
        if (moveLeft) {
          const left = snapEdge(oldRect.x + deltaX, "left", snapOptions);
          next.x = left;
          next.width = oldRight - left;
        } else {
          next.x = oldRect.x;
          next.width =
            snapEdge(oldRight + deltaX, "right", snapOptions) - oldRect.x;
        }
      }
      if (moveTop !== null) {
        if (moveTop) {
          const top = snapEdge(oldRect.y + deltaY, "top", snapOptions);
          next.y = top;
          next.height = oldBottom - top;
        } else {
          next.y = oldRect.y;
          next.height =
            snapEdge(oldBottom + deltaY, "bottom", snapOptions) - oldRect.y;
        }
      }

      if (moveLeft) {
        const minWidth = constraints.minWidth * viewport.width;
        next.x = Math.min(next.x, oldRight - minWidth);
        next.width = oldRight - next.x;
      }
      if (moveTop) {
        const minHeight = constraints.minHeight * viewport.height;
        next.y = Math.min(next.y, oldBottom - minHeight);
        next.height = oldBottom - next.y;
      }

      onResize(
        {
          x: next.x / viewport.width,
          y: next.y / viewport.height,
          width: Math.max(next.width, 1) / viewport.width,
          height: Math.max(next.height, 1) / viewport.height,
        },
        direction,
      );
    };

    const handleUp = () => {
      window.removeEventListener("pointermove", handleMove);
      window.removeEventListener("pointerup", handleUp);
      try {
        handle.releasePointerCapture(event.pointerId);
      } catch {
        // The handle already unmounted; nothing to release.
      }
      onEnd?.();
    };

    window.addEventListener("pointermove", handleMove);
    window.addEventListener("pointerup", handleUp);
  }

  return startResize;
}
