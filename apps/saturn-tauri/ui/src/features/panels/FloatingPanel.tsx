import { AnimatePresence, motion } from "motion/react";
import type { ReactNode } from "react";
import {
  backdropVariants,
  dialogVariants,
  useMotionPreset,
} from "../../motion/variants";
import { usePanelDrag } from "./usePanelDrag.ts";
import type { DockZone } from "./docking.ts";
import {
  usePanelResize,
  RESIZE_DIRECTIONS,
  type ResizeDirection,
} from "./usePanelResize.ts";
import type { PanelConstraints, PanelRect, PanelRegion } from "./types.ts";

export interface FloatingPanelProps {
  panelId: string;
  title: ReactNode;
  children: ReactNode;
  rect: PanelRect;
  region: PanelRegion;
  constraints: PanelConstraints;
  zIndex: number;
  collapsed: boolean;
  pinned: boolean;
  otherRects?: {
    id: string;
    rect: { x: number; y: number; width: number; height: number };
  }[];
  onMove: (next: PanelRect, reason: "drag" | "resize") => void;
  onRaise: () => void;
  onToggleCollapse: () => void;
  onTogglePin: () => void;
  onClose: () => void;
  onOpen: () => void;
  /** Dock-zone tracking during drags (ghost preview); null clears it. */
  onDockTarget?: (zone: DockZone | null) => void;
  /** Called on drag end with the drop dock zone, if any. */
  onDropDock?: (zone: DockZone | null) => void;
}

/**
 * Floating glass panel shell. Renders its own drag header with collapse/pin/
 * close actions plus 8 resize handles, and animates close with the shared
 * presence preset (opacity/scale only; layout properties never animate).
 */
export function FloatingPanel({
  panelId,
  title,
  children,
  rect,
  region,
  constraints,
  zIndex,
  collapsed,
  pinned,
  otherRects,
  onMove,
  onRaise,
  onToggleCollapse,
  onTogglePin,
  onClose,
  onOpen,
  onDockTarget,
  onDropDock,
}: FloatingPanelProps) {
  const preset = useMotionPreset();
  const { startDrag } = usePanelDrag({
    viewport: region,
    onMove: (next) => onMove(next, "drag"),
    others: otherRects,
    // Pin semantics (G4a deferred): a pinned panel never docks.
    dockable: !pinned,
    onDockTarget,
    onDrop: onDropDock,
  });
  const startResize = usePanelResize({
    viewport: region,
    onResize: (next) => onMove(next, "resize"),
    others: otherRects,
  });

  if (region.width <= 0 || region.height <= 0) return null;

  // Fixed positioning resolves against the window, so the region origin has to
  // be added explicitly; sizes come straight from the region dimensions.
  const leftPx = region.left + rect.x * region.width;
  const topPx = region.top + rect.y * region.height;
  const widthPx = rect.width * region.width;
  const heightPx = rect.height * region.height;

  const style: React.CSSProperties = {
    left: `${leftPx.toFixed(2)}px`,
    top: `${topPx.toFixed(2)}px`,
    width: `${widthPx.toFixed(2)}px`,
    height: `${heightPx.toFixed(2)}px`,
    zIndex,
  };

  const handleResizeStart = (
    event: React.PointerEvent,
    direction: ResizeDirection,
  ) => {
    startResize({ event, direction, current: rect, constraints });
  };

  return (
    <AnimatePresence>
      {collapsed ? (
        <motion.button
          key={`${panelId}-chip`}
          type="button"
          className="floating-panel-chip glass"
          data-elev="2"
          style={{ left: style.left, top: style.top, zIndex: style.zIndex }}
          onClick={onOpen}
          initial={preset.initial}
          animate="animate"
          exit="exit"
          variants={backdropVariants}
          transition={preset.transition}
          title={typeof title === "string" ? title : panelId}
          onPointerDown={onRaise}
        >
          {title}
          <span className="floating-panel-chip-hint">Restore</span>
        </motion.button>
      ) : (
        <motion.section
          key={panelId}
          className="floating-panel glass"
          data-elev="2"
          data-floating-panel={panelId}
          aria-label={typeof title === "string" ? title : panelId}
          style={style}
          initial={preset.initial}
          animate="animate"
          exit="exit"
          variants={dialogVariants}
          transition={preset.transition}
          onPointerDown={onRaise}
        >
          <header
            className="floating-panel-header"
            data-drag-handle="true"
            onPointerDown={(event) => startDrag({ event, current: rect })}
            onDoubleClick={onToggleCollapse}
          >
            <span className="floating-panel-title">{title}</span>
            <div className="floating-panel-actions" data-no-drag="true">
              <button
                type="button"
                className="floating-panel-action"
                onClick={onTogglePin}
                aria-pressed={pinned}
                title="Pin panel (stays above workspace changes)"
              >
                {pinned ? "📌" : "📍"}
              </button>
              <button
                type="button"
                className="floating-panel-action"
                onClick={onToggleCollapse}
                title="Collapse panel to chip"
              >
                –
              </button>
              <button
                type="button"
                className="floating-panel-action"
                onClick={onClose}
                title="Close panel (reopen from the panel rail)"
              >
                ✕
              </button>
            </div>
          </header>
          <div className="floating-panel-body">{children}</div>
          {RESIZE_DIRECTIONS.map((direction) => (
            <div
              key={direction}
              className={`panel-resize-handle panel-resize-${direction}`}
              data-resize={direction}
              onPointerDown={(event) => handleResizeStart(event, direction)}
            />
          ))}
        </motion.section>
      )}
    </AnimatePresence>
  );
}
