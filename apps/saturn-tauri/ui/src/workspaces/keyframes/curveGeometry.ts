import type { Clip, Keyframe, KeyframeProperty } from "../../types";
import { TICKS_PER_SECOND } from "../../types";

export const keyframeOptions: {
  value: KeyframeProperty;
  label: string;
  defaultValue: number;
  min: number;
  max: number;
  step: number;
}[] = [
  { value: "position_x", label: "Position X", defaultValue: 0, min: -2000, max: 2000, step: 1 },
  { value: "position_y", label: "Position Y", defaultValue: 0, min: -2000, max: 2000, step: 1 },
  { value: "scale", label: "Scale", defaultValue: 1, min: 0.01, max: 10, step: 0.01 },
  { value: "rotation", label: "Rotation", defaultValue: 0, min: -360, max: 360, step: 1 },
  { value: "opacity", label: "Opacity", defaultValue: 1, min: 0, max: 1, step: 0.01 },
];

export interface CurveGeometryInput {
  clipKeyframes: Keyframe[];
  keyframeProperty: KeyframeProperty;
  selectedTimelineClip: Clip | null;
  playhead: number;
  selectedKeyframeTime: number | null;
}

export interface CurveGeometry {
  option: (typeof keyframeOptions)[number];
  svgW: number;
  svgH: number;
  padL: number;
  padR: number;
  padT: number;
  padB: number;
  plotW: number;
  plotH: number;
  clipDuration: number;
  minVal: number;
  maxVal: number;
  timeToX: (t: number) => number;
  valToY: (v: number) => number;
  relPlayhead: number;
  propKeyframes: Keyframe[];
  activeKf: Keyframe | null;
  curvePath: string;
  areaPath: string;
  activeTangents: {
    outLine?: { x1: number; y1: number; x2: number; y2: number };
    inLine?: { x1: number; y1: number; x2: number; y2: number };
  };
  vTicks: { val: number; y: number }[];
  tTicks: { t: number; x: number }[];
}

export function computeCurveGeometry({
  clipKeyframes,
  keyframeProperty,
  selectedTimelineClip,
  playhead,
  selectedKeyframeTime,
}: CurveGeometryInput): CurveGeometry {
  const option = keyframeOptions.find((item) => item.value === keyframeProperty)!;
  const propKeyframes = clipKeyframes.filter((k) => k.property === keyframeProperty).sort((a, b) => a.time - b.time);
  const activeKf = propKeyframes.find((k) => k.time === selectedKeyframeTime) ?? propKeyframes[0] ?? null;

  // Viewport coordinates for the graph
  const svgW = 860;
  const svgH = 340;
  const padL = 70;
  const padR = 40;
  const padT = 30;
  const padB = 40;
  const plotW = svgW - padL - padR;
  const plotH = svgH - padT - padB;

  const clipDuration = Math.max(TICKS_PER_SECOND, selectedTimelineClip?.duration ?? (5 * TICKS_PER_SECOND));
  const minVal = option.min;
  const maxVal = option.max;
  const valRange = maxVal - minVal || 1;

  const timeToX = (t: number) => padL + (Math.max(0, Math.min(clipDuration, t)) / clipDuration) * plotW;
  const valToY = (v: number) => padT + plotH - ((Math.max(minVal, Math.min(maxVal, v)) - minVal) / valRange) * plotH;

  const relPlayhead = selectedTimelineClip
    ? Math.max(0, Math.min(selectedTimelineClip.duration, playhead - selectedTimelineClip.timeline_start))
    : 0;

  // Build the SVG bezier curve path
  let curvePath = "";
  let areaPath = "";

  if (propKeyframes.length === 0) {
    const yDef = valToY(option.defaultValue);
    curvePath = `M ${timeToX(0)} ${yDef} L ${timeToX(clipDuration)} ${yDef}`;
    areaPath = `${curvePath} L ${timeToX(clipDuration)} ${padT + plotH} L ${timeToX(0)} ${padT + plotH} Z`;
  } else if (propKeyframes.length === 1) {
    const yVal = valToY(propKeyframes[0].value);
    curvePath = `M ${timeToX(0)} ${yVal} L ${timeToX(clipDuration)} ${yVal}`;
    areaPath = `${curvePath} L ${timeToX(clipDuration)} ${padT + plotH} L ${timeToX(0)} ${padT + plotH} Z`;
  } else {
    const k0 = propKeyframes[0];
    const y0 = valToY(k0.value);
    curvePath = `M ${timeToX(0)} ${y0} L ${timeToX(k0.time)} ${y0}`;

    for (let i = 0; i < propKeyframes.length - 1; i++) {
      const k1 = propKeyframes[i];
      const k2 = propKeyframes[i + 1];
      const x1 = timeToX(k1.time);
      const y1 = valToY(k1.value);
      const x2 = timeToX(k2.time);
      const y2 = valToY(k2.value);

      const interp = k1.interpolation ?? "linear";
      if (interp === "linear") {
        curvePath += ` L ${x2.toFixed(1)} ${y2.toFixed(1)}`;
      } else {
        const hOutX = interp === "ease_in" ? 0.42 : interp === "ease_out" ? 0.0 : interp === "ease_in_out" ? 0.42 : (k1.handle_out_x ?? 0.33);
        const hOutY = interp === "ease_in" ? 0.0 : interp === "ease_out" ? 0.58 : interp === "ease_in_out" ? 0.0 : (k1.handle_out_y ?? 0.33);
        const hInX = interp === "ease_in" ? 1.0 : interp === "ease_out" ? 0.58 : interp === "ease_in_out" ? 0.58 : (k2.handle_in_x ?? 0.67);
        const hInY = interp === "ease_in" ? 0.42 : interp === "ease_out" ? 1.0 : interp === "ease_in_out" ? 1.0 : (k2.handle_in_y ?? 0.67);

        const cx1 = x1 + hOutX * (x2 - x1);
        const cy1 = y1 + hOutY * (y2 - y1);
        const cx2 = x1 + hInX * (x2 - x1);
        const cy2 = y1 + hInY * (y2 - y1);
        curvePath += ` C ${cx1.toFixed(1)} ${cy1.toFixed(1)}, ${cx2.toFixed(1)} ${cy2.toFixed(1)}, ${x2.toFixed(1)} ${y2.toFixed(1)}`;
      }
    }

    const kLast = propKeyframes[propKeyframes.length - 1];
    const yLast = valToY(kLast.value);
    curvePath += ` L ${timeToX(clipDuration)} ${yLast.toFixed(1)}`;
    areaPath = `${curvePath} L ${timeToX(clipDuration)} ${padT + plotH} L ${timeToX(0)} ${padT + plotH} Z`;
  }

  // Tangent handles for active keyframe
  const activeTangents: {
    outLine?: { x1: number; y1: number; x2: number; y2: number };
    inLine?: { x1: number; y1: number; x2: number; y2: number };
  } = {};

  if (activeKf) {
    const idx = propKeyframes.findIndex((k) => k.time === activeKf.time);
    const kfX = timeToX(activeKf.time);
    const kfY = valToY(activeKf.value);

    if (idx < propKeyframes.length - 1) {
      const nextKf = propKeyframes[idx + 1];
      const nextX = timeToX(nextKf.time);
      const nextY = valToY(nextKf.value);
      const interp = activeKf.interpolation ?? "linear";
      if (interp !== "linear") {
        const hOutX = interp === "ease_in" ? 0.42 : interp === "ease_out" ? 0.0 : interp === "ease_in_out" ? 0.42 : (activeKf.handle_out_x ?? 0.33);
        const hOutY = interp === "ease_in" ? 0.0 : interp === "ease_out" ? 0.58 : interp === "ease_in_out" ? 0.0 : (activeKf.handle_out_y ?? 0.33);
        const cx1 = kfX + hOutX * (nextX - kfX);
        const cy1 = kfY + hOutY * (nextY - kfY);
        activeTangents.outLine = { x1: kfX, y1: kfY, x2: cx1, y2: cy1 };
      }
    }

    if (idx > 0) {
      const prevKf = propKeyframes[idx - 1];
      const prevX = timeToX(prevKf.time);
      const prevY = valToY(prevKf.value);
      const interp = prevKf.interpolation ?? "linear";
      if (interp !== "linear") {
        const hInX = interp === "ease_in" ? 1.0 : interp === "ease_out" ? 0.58 : interp === "ease_in_out" ? 0.58 : (activeKf.handle_in_x ?? 0.67);
        const hInY = interp === "ease_in" ? 0.42 : interp === "ease_out" ? 1.0 : interp === "ease_in_out" ? 1.0 : (activeKf.handle_in_y ?? 0.67);
        const cx2 = prevX + hInX * (kfX - prevX);
        const cy2 = prevY + hInY * (kfY - prevY);
        activeTangents.inLine = { x1: kfX, y1: kfY, x2: cx2, y2: cy2 };
      }
    }
  }

  // Grid ticks
  const vSteps = 5;
  const vTicks = Array.from({ length: vSteps + 1 }, (_, i) => {
    const val = minVal + (valRange / vSteps) * (vSteps - i);
    const y = padT + (plotH / vSteps) * i;
    return { val, y };
  });

  const durSec = clipDuration / TICKS_PER_SECOND;
  const tSteps = Math.min(8, Math.max(3, Math.round(durSec)));
  const tTicks = Array.from({ length: tSteps + 1 }, (_, i) => {
    const frac = i / tSteps;
    const t = frac * clipDuration;
    const x = padL + frac * plotW;
    return { t, x };
  });

  return {
    option,
    svgW,
    svgH,
    padL,
    padR,
    padT,
    padB,
    plotW,
    plotH,
    clipDuration,
    minVal,
    maxVal,
    timeToX,
    valToY,
    relPlayhead,
    propKeyframes,
    activeKf,
    curvePath,
    areaPath,
    activeTangents,
    vTicks,
    tTicks,
  };
}
