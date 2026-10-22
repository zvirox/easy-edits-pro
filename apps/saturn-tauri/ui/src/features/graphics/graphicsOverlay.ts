import type { GraphicItem } from "../../engine/playback/types";

/**
 * Slice 3 preview overlay for timeline graphics. Graphics are project-pixel
 * boxes; the program monitor shows them as a percentage of the video frame
 * (mirrors the title/subtitle DOM-overlay approach). The export path renders
 * them through `saturn-text` + the wgpu compositor in a later slice.
 */

export interface GraphicOverlayBox {
  /** Percentage of the program frame box, 0..100. */
  left: number;
  top: number;
  width: number;
  height: number;
  /** Stack order over video (higher renders on top). */
  zIndex: number;
}

export interface FrameSettings {
  width: number;
  height: number;
}

/** Graphics whose time range covers the playhead, in layer order. */
/** Graphics whose time range covers the playhead, in layer order. */
export function activeGraphics(
  graphics: readonly GraphicItem[] | undefined,
  playhead: number,
): GraphicItem[] {
  if (!graphics) return [];
  return graphics
    .filter(
      (graphic) =>
        graphic.visible !== false &&
        graphic.start_ticks <= playhead &&
        playhead < graphic.start_ticks + graphic.duration_ticks,
    )
    .sort((a, b) => a.layer - b.layer || a.id - b.id);
}

export function overlayBox(
  graphic: GraphicItem,
  settings: FrameSettings,
): GraphicOverlayBox {
  const width = Math.max(settings.width, 1);
  const height = Math.max(settings.height, 1);
  const box = graphic.transform;
  return {
    left: (clamp(box.x, -width, width) / width) * 100,
    top: (clamp(box.y, -height, height) / height) * 100,
    width: (Math.max(box.width, 0) / width) * 100,
    height: (Math.max(box.height, 0) / height) * 100,
    zIndex: 10 + graphic.layer,
  };
}

/** Text inside its box: CJK-safe, leading-aware, aligned per the payload. */
export function textOverlayStyle(
  graphic: GraphicItem,
  settings: FrameSettings,
): Record<string, string> {
  const text = graphic.text;
  if (!text) return {};
  const scale = 100 / Math.max(settings.height, 1);
  return {
    fontFamily: `"${text.font_family}", Inter, sans-serif`,
    // Container-query units: the overlay wrapper is a size container, so 1cqh
    // is one percent of the (letterboxed) video height — font sizes scale with
    // the preview exactly like project pixels.
    fontSize: `${clamp(text.font_size * scale, 0.5, 50)}cqh`,
    lineHeight: `${text.leading > 0 ? text.leading : 1.2}`,
    letterSpacing: `${clamp(text.tracking * scale, 0, 10)}cqh`,
    textAlign: text.align === "center" ? "center" : text.align === "right" ? "right" : "left",
    writingMode: text.vertical ? "vertical-rl" : "horizontal-tb",
    whiteSpace: "pre-wrap",
  };
}

/** Fill colour to CSS; falls back to white like the schema default. */
export function fillToCss(
  fill: { color: string; opacity: number } | null | undefined,
): string {
  const opacity = fill ? clamp(fill.opacity, 0, 1) : 1;
  return withAlpha(fill?.color || "#ffffff", opacity);
}

export function strokeToCss(
  stroke: { color: string; width: number } | null | undefined,
): Record<string, string> {
  if (!stroke || stroke.width <= 0) return {};
  return {
    WebkitTextStroke: `${stroke.width}px ${stroke.color || "#ffffff"}`,
  };
}

export function shadowToCss(
  shadow: { color: string; blur: number; offset_x: number; offset_y: number } | null | undefined,
  settings: FrameSettings,
): string | undefined {
  if (!shadow) return undefined;
  const scale = 100 / Math.max(settings.height, 1);
  const blur = clamp(shadow.blur, 0, 500) * scale;
  const dx = clamp(shadow.offset_x, -500, 500) * scale;
  const dy = clamp(shadow.offset_y, -500, 500) * scale;
  return `${dx}cqh ${dy}cqh ${blur}cqh ${withAlpha(shadow.color || "#000000", 0.75)}`;
}

/** Shape ink for non-text kinds rendered in CSS (approximate export path). */
export function shapeBackgroundCss(
  graphic: GraphicItem,
): { background: string; borderRadius: string } {
  const background = fillToCss(graphic.fill);
  if (graphic.kind === "ellipse") {
    return { background, borderRadius: "50%" };
  }
  return { background, borderRadius: "0px" };
}

/* ---------------- Slice 4: on-canvas interaction math ---------------- */

/** Default creation size per tool kind, in project pixels. */
export const CREATE_DEFAULT_SIZES: Record<"text" | "rectangle" | "ellipse", { width: number; height: number }> = {
  text: { width: 420, height: 120 },
  rectangle: { width: 400, height: 300 },
  ellipse: { width: 300, height: 300 },
};

/** A click at a normalized stage point creates a box centred there. */
export function creationBox(
  kind: "text" | "rectangle" | "ellipse",
  xNorm: number,
  yNorm: number,
  settings: FrameSettings,
): { x: number; y: number; width: number; height: number } {
  const size = CREATE_DEFAULT_SIZES[kind];
  return {
    x: xNorm * settings.width - size.width / 2,
    y: yNorm * settings.height - size.height / 2,
    width: size.width,
    height: size.height,
  };
}

/** Maps a pointer event position into project pixels within `stage`. */
export function projectPoint(
  clientX: number,
  clientY: number,
  stage: { left: number; top: number; width: number; height: number },
  settings: FrameSettings,
): { x: number; y: number } {
  return {
    x: ((clientX - stage.left) / Math.max(stage.width, 1)) * settings.width,
    y: ((clientY - stage.top) / Math.max(stage.height, 1)) * settings.height,
  };
}

/** The box after a move/resize gesture, clamped to sane minimums. */
export function draggedBox(
  start: { x: number; y: number; width: number; height: number },
  deltaX: number,
  deltaY: number,
  mode: "move" | "resize",
  minSize = 20,
): { x: number; y: number; width: number; height: number } {
  if (mode === "move") {
    return { ...start, x: start.x + deltaX, y: start.y + deltaY };
  }
  return {
    x: start.x,
    y: start.y,
    width: Math.max(minSize, start.width + deltaX),
    height: Math.max(minSize, start.height + deltaY),
  };
}

function clamp(value: number, min: number, max: number): number {
  if (!Number.isFinite(value)) return min;
  return Math.min(max, Math.max(min, value));
}

function withAlpha(color: string, opacity: number): string {
  const hex = color.trim();
  const match = /^#([0-9a-f]{6})$/i.exec(hex);
  if (!match || Number.isNaN(opacity)) return hex;
  const int = parseInt(match[1]!, 16);
  const r = (int >> 16) & 0xff;
  const g = (int >> 8) & 0xff;
  const b = int & 0xff;
  return `rgba(${r}, ${g}, ${b}, ${clamp(opacity, 0, 1)})`;
}
