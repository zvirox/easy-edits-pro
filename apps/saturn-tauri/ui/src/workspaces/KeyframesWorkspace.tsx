import { InteriorButton } from "../components/interior/interior-button";
import { type MouseEvent } from "react";
import {
  type ProjectDocument,
  type MediaAsset,
  type SelectedClip,
  type Clip,
  type KeyframeProperty,
  type InterpolationKind,
  type DavinciWorkspace,
  TICKS_PER_SECOND,
} from "../types";
import { formatTime } from "../utils/formatters";

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

export interface KeyframesWorkspaceProps {
  project: ProjectDocument["project"] | null;
  media: MediaAsset[];
  selectedClip: SelectedClip | null;
  selectedTimelineClip: Clip | null;
  setSelectedClip: (clip: SelectedClip | null) => void;
  setSelectedClips: (clips: SelectedClip[]) => void;
  keyframeProperty: KeyframeProperty;
  setKeyframeProperty: (prop: KeyframeProperty) => void;
  keyframeValue: string;
  setKeyframeValue: (val: string) => void;
  keyframeInterpDraft: InterpolationKind;
  setKeyframeInterpDraft: (kind: InterpolationKind) => void;
  keyframeHandleInX: number;
  setKeyframeHandleInX: (v: number) => void;
  keyframeHandleInY: number;
  setKeyframeHandleInY: (v: number) => void;
  keyframeHandleOutX: number;
  setKeyframeHandleOutX: (v: number) => void;
  keyframeHandleOutY: number;
  setKeyframeHandleOutY: (v: number) => void;
  selectedKeyframeTime: number | null;
  setSelectedKeyframeTime: (t: number | null) => void;
  playhead: number;
  seekProgramTo: (ticks: number) => void;
  programPlaying: boolean;
  toggleProgramPlayback: () => void;
  saveKeyframe: (
    property: KeyframeProperty,
    value: number,
    time?: number,
    interpolation?: InterpolationKind,
    hInX?: number,
    hInY?: number,
    hOutX?: number,
    hOutY?: number
  ) => Promise<void>;
  deleteKeyframe: (property: KeyframeProperty, time: number) => Promise<void>;
  selectDavinciWorkspace: (ws: DavinciWorkspace) => void;
}

export function KeyframesWorkspace({
  project,
  media,
  selectedClip,
  selectedTimelineClip,
  setSelectedClip,
  setSelectedClips,
  keyframeProperty,
  setKeyframeProperty,
  keyframeValue,
  setKeyframeValue,
  keyframeInterpDraft,
  setKeyframeInterpDraft,
  keyframeHandleInX,
  setKeyframeHandleInX,
  keyframeHandleInY,
  setKeyframeHandleInY,
  keyframeHandleOutX,
  setKeyframeHandleOutX,
  keyframeHandleOutY,
  setKeyframeHandleOutY,
  selectedKeyframeTime,
  setSelectedKeyframeTime,
  playhead,
  seekProgramTo,
  programPlaying,
  toggleProgramPlayback,
  saveKeyframe,
  deleteKeyframe,
  selectDavinciWorkspace,
}: KeyframesWorkspaceProps) {
  const allClips = (project?.sequence.tracks ?? []).flatMap((track) =>
    track.clips.map((clip, index) => ({ track, clip, index, mediaItem: media.find((m) => m.id === clip.media_id) }))
  );

  const option = keyframeOptions.find((item) => item.value === keyframeProperty)!;
  const clipKeyframes = selectedTimelineClip?.keyframes ?? [];
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

  const jumpPrevKeyframe = () => {
    if (!selectedTimelineClip) return;
    const current = relPlayhead;
    const prev = [...propKeyframes].reverse().find((k) => k.time < current - 1000);
    if (prev) {
      seekProgramTo(selectedTimelineClip.timeline_start + prev.time);
      setSelectedKeyframeTime(prev.time);
      setKeyframeValue(String(prev.value));
      setKeyframeInterpDraft(prev.interpolation ?? "linear");
    }
  };

  const jumpNextKeyframe = () => {
    if (!selectedTimelineClip) return;
    const current = relPlayhead;
    const next = propKeyframes.find((k) => k.time > current + 1000);
    if (next) {
      seekProgramTo(selectedTimelineClip.timeline_start + next.time);
      setSelectedKeyframeTime(next.time);
      setKeyframeValue(String(next.value));
      setKeyframeInterpDraft(next.interpolation ?? "linear");
    }
  };

  const handleCanvasClick = (event: MouseEvent<SVGSVGElement>) => {
    if (!selectedTimelineClip) return;
    const rect = event.currentTarget.getBoundingClientRect();
    const clickX = ((event.clientX - rect.left) / rect.width) * svgW;
    if (clickX >= padL && clickX <= padL + plotW) {
      const clickedFrac = (clickX - padL) / plotW;
      const newTime = Math.round(clickedFrac * clipDuration);
      seekProgramTo(selectedTimelineClip.timeline_start + newTime);
    }
  };

  return (
    <main className="workspace-page keyframes-workspace-page">
      <header className="workspace-sub-header keyframes-workspace-header">
        <div className="workspace-header-title">
          <h1>Keyframe Bezier Curve Editor</h1>
          <p>
            Sculpt property curves with interactive cubic bezier tangents, smooth ease-in/out transitions, and live preview sync.
          </p>
        </div>
        <div className="keyframes-header-actions">
          <span className="keyframes-target-info">
            {selectedTimelineClip ? (
              <>
                Target: <strong>{media.find((m) => m.id === selectedTimelineClip.media_id)?.name ?? "Clip"}</strong>
              </>
            ) : (
              <span className="keyframes-hint-text">Select a clip in the reel below to edit curves</span>
            )}
          </span>
          <InteriorButton
            className="btn-primary-subtle"
            onClick={() => selectDavinciWorkspace("edit")}
          >
            ← Back to Edit
          </InteriorButton>
        </div>
      </header>

      <div className="keyframes-studio-body">
        {/* Main Visual Curve Editor Area */}
        <section className="curve-editor-stage">
          <div className="curve-editor-container">
            <div className="curve-editor-toolbar">
              <div className="property-pills-selector">
                {keyframeOptions.map((opt) => (
                  <InteriorButton
                    key={opt.value}
                    type="button"
                    className={`property-pill-btn ${keyframeProperty === opt.value ? "active" : ""}`}
                    onClick={() => {
                      setKeyframeProperty(opt.value);
                      setKeyframeValue(String(opt.defaultValue));
                      setSelectedKeyframeTime(null);
                    }}
                  >
                    {opt.label}
                  </InteriorButton>
                ))}
              </div>

              <div className="curve-header-readouts">
                <span className="curve-tc-tag">
                  PLAYHEAD: <b>{formatTime(playhead)}</b>
                </span>
                <span className="curve-val-tag">
                  CLIP OFFSET: <b>{formatTime(relPlayhead)}</b>
                </span>
              </div>
            </div>

            {/* The Interactive SVG Graph Canvas */}
            <div className="curve-canvas-wrapper">
              <svg
                className="curve-canvas-svg"
                viewBox={`0 0 ${svgW} ${svgH}`}
                preserveAspectRatio="none"
                onClick={handleCanvasClick}
              >
                <defs>
                  <linearGradient id="curveGradient" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0%" stopColor="#6b21a8" stopOpacity="0.35" />
                    <stop offset="100%" stopColor="#6b21a8" stopOpacity="0.02" />
                  </linearGradient>
                </defs>

                {/* Horizontal Grid lines */}
                {vTicks.map((tick, i) => (
                  <g key={`vtick-${i}`}>
                    <line
                      x1={padL}
                      y1={tick.y}
                      x2={padL + plotW}
                      y2={tick.y}
                      stroke="rgba(255, 255, 255, 0.07)"
                      strokeDasharray="4 4"
                    />
                    <text
                      x={padL - 10}
                      y={tick.y + 4}
                      textAnchor="end"
                      className="curve-grid-text"
                    >
                      {tick.val.toFixed(option.step < 0.1 ? 2 : 1)}
                    </text>
                  </g>
                ))}

                {/* Vertical Time Grid lines */}
                {tTicks.map((tick, i) => (
                  <g key={`ttick-${i}`}>
                    <line
                      x1={tick.x}
                      y1={padT}
                      x2={tick.x}
                      y2={padT + plotH}
                      stroke="rgba(255, 255, 255, 0.05)"
                    />
                    <text
                      x={tick.x}
                      y={padT + plotH + 20}
                      textAnchor="middle"
                      className="curve-grid-text"
                    >
                      {(tick.t / TICKS_PER_SECOND).toFixed(1)}s
                    </text>
                  </g>
                ))}

                {/* Bezier Curve Filled Area */}
                {areaPath && (
                  <path d={areaPath} fill="url(#curveGradient)" pointerEvents="none" />
                )}

                {/* Bezier Spline Stroke */}
                {curvePath && (
                  <path
                    d={curvePath}
                    fill="none"
                    stroke="#6b21a8"
                    strokeWidth="2.5"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    pointerEvents="none"
                  />
                )}

                {/* Active Keyframe Tangent Handles */}
                {activeTangents.outLine && (
                  <g className="tangent-handle-group">
                    <line
                      x1={activeTangents.outLine.x1}
                      y1={activeTangents.outLine.y1}
                      x2={activeTangents.outLine.x2}
                      y2={activeTangents.outLine.y2}
                      stroke="#f59e0b"
                      strokeWidth="1.5"
                      strokeDasharray="3 3"
                    />
                    <circle
                      cx={activeTangents.outLine.x2}
                      cy={activeTangents.outLine.y2}
                      r="5"
                      fill="#f59e0b"
                      stroke="#ffffff"
                      strokeWidth="1.5"
                      className="tangent-circle-handle"
                    />
                  </g>
                )}

                {activeTangents.inLine && (
                  <g className="tangent-handle-group">
                    <line
                      x1={activeTangents.inLine.x1}
                      y1={activeTangents.inLine.y1}
                      x2={activeTangents.inLine.x2}
                      y2={activeTangents.inLine.y2}
                      stroke="#f59e0b"
                      strokeWidth="1.5"
                      strokeDasharray="3 3"
                    />
                    <circle
                      cx={activeTangents.inLine.x2}
                      cy={activeTangents.inLine.y2}
                      r="5"
                      fill="#f59e0b"
                      stroke="#ffffff"
                      strokeWidth="1.5"
                      className="tangent-circle-handle"
                    />
                  </g>
                )}

                {/* Playhead vertical line */}
                <line
                  x1={timeToX(relPlayhead)}
                  y1={padT - 6}
                  x2={timeToX(relPlayhead)}
                  y2={padT + plotH + 8}
                  stroke="#ef4444"
                  strokeWidth="2"
                  pointerEvents="none"
                />
                <polygon
                  points={`${timeToX(relPlayhead) - 6},${padT - 6} ${timeToX(relPlayhead) + 6},${padT - 6} ${timeToX(relPlayhead)},${padT + 4}`}
                  fill="#ef4444"
                  pointerEvents="none"
                />

                {/* Keyframe Diamond Markers */}
                {propKeyframes.map((kf) => {
                  const kfX = timeToX(kf.time);
                  const kfY = valToY(kf.value);
                  const isSelected = activeKf?.time === kf.time;
                  return (
                    <g
                      key={`kf-${kf.property}-${kf.time}`}
                      transform={`translate(${kfX}, ${kfY})`}
                      className={`keyframe-marker-node ${isSelected ? "is-selected" : ""}`}
                      onClick={(e) => {
                        e.stopPropagation();
                        setSelectedKeyframeTime(kf.time);
                        setKeyframeValue(String(kf.value));
                        setKeyframeInterpDraft(kf.interpolation ?? "linear");
                        if (kf.handle_in_x != null) setKeyframeHandleInX(kf.handle_in_x);
                        if (kf.handle_in_y != null) setKeyframeHandleInY(kf.handle_in_y);
                        if (kf.handle_out_x != null) setKeyframeHandleOutX(kf.handle_out_x);
                        if (kf.handle_out_y != null) setKeyframeHandleOutY(kf.handle_out_y);
                        if (selectedTimelineClip) {
                          seekProgramTo(selectedTimelineClip.timeline_start + kf.time);
                        }
                      }}
                    >
                      <rect
                        x="-7"
                        y="-7"
                        width="14"
                        height="14"
                        transform="rotate(45)"
                        className={`kf-diamond ${isSelected ? "selected" : ""}`}
                      />
                    </g>
                  );
                })}
              </svg>
            </div>

            {/* Curve Transport Bar */}
            <div className="curve-transport-bar">
              <InteriorButton
                type="button"
                className="transport-btn"
                onClick={jumpPrevKeyframe}
                title="Jump to Previous Keyframe"
              >
                ⏮
              </InteriorButton>
              <InteriorButton
                type="button"
                className={`transport-btn ${programPlaying ? "active-toggle" : ""}`}
                onClick={() => toggleProgramPlayback()}
                title={programPlaying ? "Pause Playback" : "Play Clip"}
              >
                {programPlaying ? "⏸ Pause" : "▶ Play"}
              </InteriorButton>
              <InteriorButton
                type="button"
                className="transport-btn"
                onClick={jumpNextKeyframe}
                title="Jump to Next Keyframe"
              >
                ⏭
              </InteriorButton>

              <div className="curve-transport-divider" />

              <InteriorButton
                type="button"
                className="btn-primary-subtle"
                disabled={!selectedClip}
                onClick={() => {
                  const hInX =
                    keyframeInterpDraft === "ease_in"
                      ? 1.0
                      : keyframeInterpDraft === "ease_out"
                      ? 0.58
                      : keyframeInterpDraft === "ease_in_out"
                      ? 0.58
                      : keyframeHandleInX;
                  const hInY =
                    keyframeInterpDraft === "ease_in"
                      ? 0.42
                      : keyframeInterpDraft === "ease_out"
                      ? 1.0
                      : keyframeInterpDraft === "ease_in_out"
                      ? 1.0
                      : keyframeHandleInY;
                  const hOutX =
                    keyframeInterpDraft === "ease_in"
                      ? 0.42
                      : keyframeInterpDraft === "ease_out"
                      ? 0.0
                      : keyframeInterpDraft === "ease_in_out"
                      ? 0.42
                      : keyframeHandleOutX;
                  const hOutY =
                    keyframeInterpDraft === "ease_in"
                      ? 0.0
                      : keyframeInterpDraft === "ease_out"
                      ? 0.58
                      : keyframeInterpDraft === "ease_in_out"
                      ? 0.0
                      : keyframeHandleOutY;
                  void saveKeyframe(keyframeProperty, Number(keyframeValue), undefined, keyframeInterpDraft, hInX, hInY, hOutX, hOutY);
                }}
                title="Add Keyframe at Current Playhead"
              >
                ＋ Add Keyframe at Playhead
              </InteriorButton>

              {activeKf && (
                <InteriorButton
                  type="button"
                  className="danger-button"
                  onClick={() => void deleteKeyframe(activeKf.property, activeKf.time)}
                  title="Delete Selected Keyframe"
                >
                  🗑 Delete Keyframe
                </InteriorButton>
              )}
            </div>
          </div>
        </section>

        {/* Right Sidebar: Tangents & Interpolation Controls */}
        <aside className="curve-sidebar-dock">
          <div className="curve-controls-box">
            <div className="curve-section-header">
              <h3>Interpolation Style</h3>
              <small>{(activeKf?.interpolation ?? keyframeInterpDraft).toUpperCase().replace("_", " ")}</small>
            </div>

            <div className="ease-presets-grid">
              {(["linear", "ease_in", "ease_out", "ease_in_out", "bezier"] as InterpolationKind[]).map((kind) => {
                const isCurrent = (activeKf?.interpolation ?? keyframeInterpDraft) === kind;
                return (
                  <InteriorButton
                    key={kind}
                    type="button"
                    className={`ease-preset-card ${isCurrent ? "active" : ""}`}
                    disabled={!selectedClip}
                    onClick={() => {
                      setKeyframeInterpDraft(kind);
                      if (activeKf) {
                        const hInX = kind === "ease_in" ? 1.0 : kind === "ease_out" ? 0.58 : kind === "ease_in_out" ? 0.58 : keyframeHandleInX;
                        const hInY = kind === "ease_in" ? 0.42 : kind === "ease_out" ? 1.0 : kind === "ease_in_out" ? 1.0 : keyframeHandleInY;
                        const hOutX = kind === "ease_in" ? 0.42 : kind === "ease_out" ? 0.0 : kind === "ease_in_out" ? 0.42 : keyframeHandleOutX;
                        const hOutY = kind === "ease_in" ? 0.0 : kind === "ease_out" ? 0.58 : kind === "ease_in_out" ? 0.0 : keyframeHandleOutY;
                        void saveKeyframe(keyframeProperty, activeKf.value, activeKf.time, kind, hInX, hInY, hOutX, hOutY);
                      }
                    }}
                  >
                    <strong className="ease-card-title">
                      {kind === "linear"
                        ? "Linear (Straight)"
                        : kind === "ease_in"
                        ? "Ease In (Smooth Entry)"
                        : kind === "ease_out"
                        ? "Ease Out (Smooth Exit)"
                        : kind === "ease_in_out"
                        ? "Ease In-Out (S-Curve)"
                        : "Custom Bezier"}
                    </strong>
                    <span className="ease-card-desc">
                      {kind === "linear"
                        ? "Constant rate of change without smoothing"
                        : kind === "ease_in"
                        ? "Starts slow and accelerates toward end"
                        : kind === "ease_out"
                        ? "Starts fast and decelerates into destination"
                        : kind === "ease_in_out"
                        ? "Natural organic ease on both endpoints"
                        : "Sculpt custom tangents with precision handles"}
                    </span>
                  </InteriorButton>
                );
              })}
            </div>

            {/* Tangent Handle Precision Sliders */}
            <div className="tangent-sliders-section">
              <div className="curve-section-header">
                <h3>Tangent Handles</h3>
                <small>Cubic Coordinates</small>
              </div>

              <div className="tangent-sliders-grid">
                <label className="tangent-slider-item">
                  <span>In Handle X: {keyframeHandleInX.toFixed(2)}</span>
                  <input
                    type="range"
                    min="0"
                    max="1"
                    step="0.01"
                    value={keyframeHandleInX}
                    onChange={(e) => {
                      const val = Number(e.target.value);
                      setKeyframeHandleInX(val);
                      if (activeKf) {
                        void saveKeyframe(
                          keyframeProperty,
                          activeKf.value,
                          activeKf.time,
                          "bezier",
                          val,
                          keyframeHandleInY,
                          keyframeHandleOutX,
                          keyframeHandleOutY
                        );
                      }
                    }}
                  />
                </label>

                <label className="tangent-slider-item">
                  <span>In Handle Y: {keyframeHandleInY.toFixed(2)}</span>
                  <input
                    type="range"
                    min="-1"
                    max="2"
                    step="0.01"
                    value={keyframeHandleInY}
                    onChange={(e) => {
                      const val = Number(e.target.value);
                      setKeyframeHandleInY(val);
                      if (activeKf) {
                        void saveKeyframe(
                          keyframeProperty,
                          activeKf.value,
                          activeKf.time,
                          "bezier",
                          keyframeHandleInX,
                          val,
                          keyframeHandleOutX,
                          keyframeHandleOutY
                        );
                      }
                    }}
                  />
                </label>

                <label className="tangent-slider-item">
                  <span>Out Handle X: {keyframeHandleOutX.toFixed(2)}</span>
                  <input
                    type="range"
                    min="0"
                    max="1"
                    step="0.01"
                    value={keyframeHandleOutX}
                    onChange={(e) => {
                      const val = Number(e.target.value);
                      setKeyframeHandleOutX(val);
                      if (activeKf) {
                        void saveKeyframe(
                          keyframeProperty,
                          activeKf.value,
                          activeKf.time,
                          "bezier",
                          keyframeHandleInX,
                          keyframeHandleInY,
                          val,
                          keyframeHandleOutY
                        );
                      }
                    }}
                  />
                </label>

                <label className="tangent-slider-item">
                  <span>Out Handle Y: {keyframeHandleOutY.toFixed(2)}</span>
                  <input
                    type="range"
                    min="-1"
                    max="2"
                    step="0.01"
                    value={keyframeHandleOutY}
                    onChange={(e) => {
                      const val = Number(e.target.value);
                      setKeyframeHandleOutY(val);
                      if (activeKf) {
                        void saveKeyframe(
                          keyframeProperty,
                          activeKf.value,
                          activeKf.time,
                          "bezier",
                          keyframeHandleInX,
                          keyframeHandleInY,
                          keyframeHandleOutX,
                          val
                        );
                      }
                    }}
                  />
                </label>
              </div>
            </div>

            {/* Value and Time Direct Tuning */}
            <div className="keyframe-direct-edit">
              <div className="curve-section-header">
                <h3>Keyframe Numerical Value</h3>
                <small>{option.label}</small>
              </div>

              <div className="direct-edit-row">
                <label>
                  <span>Value</span>
                  <input
                    type="number"
                    min={option.min}
                    max={option.max}
                    step={option.step}
                    disabled={!selectedClip}
                    value={keyframeValue}
                    onChange={(e) => setKeyframeValue(e.target.value)}
                    onBlur={() => {
                      if (activeKf) {
                        void saveKeyframe(keyframeProperty, Number(keyframeValue), activeKf.time);
                      }
                    }}
                  />
                </label>
              </div>
            </div>
          </div>
        </aside>
      </div>

      {/* Bottom Clips Reel Tray */}
      <section className="clips-reel-tray">
        <div className="reel-tray-header">
          <strong>SEQUENCE CLIPS REEL</strong>
          <small>{allClips.length} clips on timeline · Click any clip to view its curves</small>
        </div>
        <div className="reel-clips-strip">
          {allClips.map(({ track, clip, index, mediaItem }) => {
            const isSelected = selectedClip?.track_id === track.id && selectedClip.clip_index === index;
            const clipKfCount = clip.keyframes?.length ?? 0;
            return (
              <InteriorButton
                key={`reel-curves-${track.id}-${clip.media_id}-${index}`}
                className={`reel-clip-card ${isSelected ? "selected" : ""}`}
                onClick={() => {
                  setSelectedClip({ track_id: track.id, clip_index: index });
                  setSelectedClips([{ track_id: track.id, clip_index: index }]);
                  seekProgramTo(clip.timeline_start);
                  setSelectedKeyframeTime(null);
                }}
              >
                <div className="reel-clip-badge">
                  <span className={`reel-track-kind ${track.kind}`}>{track.name}</span>
                  <span className="reel-clip-name">{mediaItem?.name ?? "Clip"}</span>
                </div>
                <span className="reel-fx-count">
                  {clipKfCount} keyframe{clipKfCount !== 1 ? "s" : ""}
                </span>
              </InteriorButton>
            );
          })}
        </div>
      </section>
    </main>
  );
}
