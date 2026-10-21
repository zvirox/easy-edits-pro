import { useRef } from "react";
import { snapRect, ALL_EDGES, type ViewportSize } from "./snapping.ts";
import { SNAP_THRESHOLD_PX, type PanelRect } from "./types.ts";

interface DragOptions {
  viewport: ViewportSize;
  onMove: (next: PanelRect) => void;
  /** Called on drag end so persistence can settle. */
  onEnd?: () => void;
  /** Rects of other floating panels in pixels, for snap targets. */
  others?: {
    id: string;
    rect: { x: number; y: number; width: number; height: number };
  }[];
}

interface DragStart {
  event: React.PointerEvent;
  current: PanelRect;
}

/**
 * Drag a floating panel by its header. Returns a start handler for the header
 * element and an is-dragging ref.
 *
 * Buttons and interactive elements inside headers opt out via `closest()`, so
 * panel actions stay clickable while the rest of the header drags.
 */
export function usePanelDrag({ viewport, onMove, onEnd, others }: DragOptions) {
  const dragging = useRef(false);

  function startDrag({ event, current }: DragStart) {
    if (event.button !== 0) return;
    if (viewport.width <= 0 || viewport.height <= 0) return;
    const target = event.target as HTMLElement;
    if (target.closest("button, input, textarea, select, a, [data-no-drag]"))
      return;

    event.preventDefault();
    const startPointer = { x: event.clientX, y: event.clientY };
    const startRect = {
      x: current.x * viewport.width,
      y: current.y * viewport.height,
      width: current.width * viewport.width,
      height: current.height * viewport.height,
    };
    dragging.current = true;
    const handle = event.currentTarget as HTMLElement;
    handle.setPointerCapture(event.pointerId);

    const handleMove = (moveEvent: PointerEvent) => {
      if (!dragging.current) return;
      const position = snapRect(
        {
          x: startRect.x + (moveEvent.clientX - startPointer.x),
          y: startRect.y + (moveEvent.clientY - startPointer.y),
          width: startRect.width,
          height: startRect.height,
        },
        { viewport, others, threshold: SNAP_THRESHOLD_PX, edges: ALL_EDGES },
      );
      onMove({
        x: position.x / viewport.width,
        y: position.y / viewport.height,
        width: current.width,
        height: current.height,
      });
    };

    const handleUp = () => {
      dragging.current = false;
      window.removeEventListener("pointermove", handleMove);
      window.removeEventListener("pointerup", handleUp);
      try {
        handle.releasePointerCapture(event.pointerId);
      } catch {
        // The handle already unmounted; nothing to release.
      }
      // The clamped set position comes from the layout layer during onMove.
      onEnd?.();
    };

    window.addEventListener("pointermove", handleMove);
    window.addEventListener("pointerup", handleUp);
  }

  return { startDrag, isDragging: dragging };
}
