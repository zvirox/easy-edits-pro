import { useCallback, useRef, useState } from "react";
import type { CSSProperties } from "react";
import type { GraphicItem } from "../../engine/playback/types";
import {
  activeGraphics,
  draggedBox,
  overlayBox,
  projectPoint,
  shadowToCss,
  shapeBackgroundCss,
  strokeToCss,
  textOverlayStyle,
} from "./graphicsOverlay";

export type GraphicsCanvasTool =
  | "select"
  | "type"
  | "verticalType"
  | "rectangle"
  | "ellipse"
  | (string & {});

export interface GraphicsEditing {
  activeTool: GraphicsCanvasTool;
  selectedId: number | null;
  onSelect: (id: number | null) => void;
  onCreate: (
    kind: "text" | "rectangle" | "ellipse",
    xNorm: number,
    yNorm: number,
    vertical?: boolean,
  ) => void | Promise<void>;
  onTransform: (
    id: number,
    patch: { x?: number; y?: number; width?: number; height?: number },
  ) => void | Promise<void>;
}

const CREATE_TOOLS: Record<string, { kind: "text" | "rectangle" | "ellipse"; vertical?: boolean }> = {
  type: { kind: "text" },
  verticalType: { kind: "text", vertical: true },
  rectangle: { kind: "rectangle" },
  ellipse: { kind: "ellipse" },
};

interface DragState {
  id: number;
  mode: "move" | "resize";
  origin: { pointer: { x: number; y: number }; box: { x: number; y: number; width: number; height: number } };
}

/**
 * Slice 4: interactive graphics overlay. Renders the active graphics and,
 * when the program is paused, supports the canvas tools (type/verticalType/
 * rectangle/ellipse click-to-create) and drag/resize of the selected graphic.
 * Gestures commit one engine command on pointer-up, so undo is granular and
 * no command storm happens per-frame.
 */
export function CanvasGraphicsLayer({
  graphics,
  playhead,
  frame,
  editing,
  programPlaying,
}: {
  graphics: GraphicItem[];
  playhead: number;
  frame: { width: number; height: number };
  editing?: GraphicsEditing;
  programPlaying: boolean;
}) {
  const layerRef = useRef<HTMLDivElement>(null);
  const [drag, setDrag] = useState<DragState | null>(null);
  const [draft, setDraft] = useState<
    { id: number; box: { x: number; y: number; width: number; height: number } } | null
  >(null);

  const interactive = Boolean(editing) && !programPlaying;
  const active = activeGraphics(graphics, playhead);

  const stagePoint = useCallback(
    (event: { clientX: number; clientY: number }) => {
      const rect = layerRef.current?.getBoundingClientRect();
      if (!rect) return null;
      return projectPoint(event.clientX, event.clientY, rect, frame);
    },
    [frame],
  );

  const onStagePointerDown = useCallback(
    (event: React.PointerEvent<HTMLDivElement>) => {
      if (!interactive || !editing) return;
      const tool = CREATE_TOOLS[editing.activeTool];
      if (tool) {
        event.preventDefault();
        const rect = layerRef.current?.getBoundingClientRect();
        if (!rect) return;
        const point = projectPoint(event.clientX, event.clientY, rect, frame);
        void editing.onCreate(
          tool.kind,
          point.x / frame.width,
          point.y / frame.height,
          tool.vertical,
        );
        return;
      }
      if (editing.activeTool === "select") {
        // Click on empty stage clears the graphics selection.
        editing.onSelect(null);
      }
    },
    [interactive, editing, frame],
  );

  const beginDrag = useCallback(
    (event: React.PointerEvent<HTMLDivElement>, graphic: GraphicItem, mode: "move" | "resize") => {
      if (!interactive || !editing) return;
      event.preventDefault();
      event.stopPropagation();
      editing.onSelect(graphic.id);
      if (graphic.locked) {
        // Locked graphics select (so the panel can unlock them) but never move.
        return;
      }
      const point = stagePoint(event);
      if (!point) return;
      setDrag({
        id: graphic.id,
        mode,
        origin: {
          pointer: point,
          box: {
            x: graphic.transform.x,
            y: graphic.transform.y,
            width: graphic.transform.width,
            height: graphic.transform.height,
          },
        },
      });
      // Capture on the layer so moves outside the stage still stream to the
      // layer's move/up handlers instead of losing the gesture.
      layerRef.current?.setPointerCapture?.(event.pointerId);
    },
    [interactive, editing, stagePoint],
  );

  const onPointerMove = useCallback(
    (event: React.PointerEvent<HTMLDivElement>) => {
      if (!drag) return;
      const point = stagePoint(event);
      if (!point) return;
      const box = draggedBox(
        drag.origin.box,
        point.x - drag.origin.pointer.x,
        point.y - drag.origin.pointer.y,
        drag.mode,
      );
      setDraft({ id: drag.id, box });
    },
    [drag, stagePoint],
  );

  const endDrag = useCallback(() => {
    if (!drag || !editing) {
      setDrag(null);
      setDraft(null);
      return;
    }
    if (draft) {
      void editing.onTransform(drag.id, draft.box);
    }
    setDrag(null);
    setDraft(null);
  }, [drag, draft, editing]);

  return (
    <div
      ref={layerRef}
      className={`graphics-overlay-layer${interactive ? " is-interactive" : ""}`}
      data-tool={editing?.activeTool}
      onPointerDown={
        interactive && (editing?.activeTool === "select" || CREATE_TOOLS[editing?.activeTool ?? ""] !== undefined)
          ? onStagePointerDown
          : undefined
      }
      onPointerMove={drag ? onPointerMove : undefined}
      onPointerUp={drag ? endDrag : undefined}
      onPointerCancel={drag ? endDrag : undefined}
      aria-hidden={!interactive}
    >
      {active.map((graphic) => {
        const boxed =
          draft && draft.id === graphic.id
            ? { ...graphic, transform: { ...graphic.transform, ...draft.box } }
            : graphic;
        return (
          <CanvasGraphicBox
            key={graphic.id}
            graphic={boxed}
            frame={frame}
            selected={editing?.selectedId === graphic.id}
            interactive={interactive}
            dragging={drag?.id === graphic.id}
            onBoxPointerDown={(event, mode) => beginDrag(event, graphic, mode)}
          />
        );
      })}
    </div>
  );
}

/** One graphic box; interactive only when the canvas is editable. */
function CanvasGraphicBox({
  graphic,
  frame,
  selected,
  interactive,
  dragging,
  onBoxPointerDown,
}: {
  graphic: GraphicItem;
  frame: { width: number; height: number };
  selected: boolean;
  interactive: boolean;
  dragging: boolean;
  onBoxPointerDown: (
    event: React.PointerEvent<HTMLDivElement>,
    mode: "move" | "resize",
  ) => void;
}) {
  const box = overlayBox(graphic, frame);
  const style: CSSProperties = {
    left: `${box.left}%`,
    top: `${box.top}%`,
    width: `${box.width}%`,
    height: `${box.height}%`,
    zIndex: box.zIndex,
    opacity: graphic.transform.opacity,
    transform: graphic.transform.rotation
      ? `rotate(${graphic.transform.rotation}deg)`
      : undefined,
  };
  if (interactive) {
    // Hit-testing needs real pointer events on the box itself.
    style.pointerEvents = "auto";
  }

  const shadowCss = shadowToCss(graphic.shadow, frame);
  const stroke = graphic.stroke;
  const className = [
    graphic.text ? "graphics-overlay-text" : "graphics-overlay-shape",
    `kind-${graphic.kind}`,
    selected ? "is-selected" : "",
    dragging ? "is-dragging" : "",
    graphic.locked ? "is-locked" : "",
  ]
    .filter(Boolean)
    .join(" ");
  const handlers = interactive
    ? {
        onPointerDown: (event: React.PointerEvent<HTMLDivElement>) => onBoxPointerDown(event, "move"),
      }
    : {};

  return (
    <div className={className} style={{ ...style, ...(graphic.text ? textOverlayStyle(graphic, frame) : {}) }} {...handlers}>
      {graphic.text ? (
        <span
          style={{
            textShadow: shadowCss,
            ...strokeToCss(graphic.stroke),
          }}
        >
          {graphic.text.content}
        </span>
      ) : (
        <div
          style={{
            width: "100%",
            height: "100%",
            ...shapeBackgroundCss(graphic),
            boxShadow: shadowCss,
            border:
              stroke && stroke.width > 0
                ? `${stroke.width}px solid ${stroke.color || "#ffffff"}`
                : undefined,
          }}
        />
      )}
      {interactive && selected && !graphic.locked && (
        <div
          className="graphics-resize-handle"
          title="Resize"
          onPointerDown={(event) => onBoxPointerDown(event, "resize")}
        />
      )}
    </div>
  );
}
