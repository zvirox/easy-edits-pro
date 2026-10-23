import { type MouseEvent } from "react";
import type { Clip, InterpolationKind } from "../../types";
import { TICKS_PER_SECOND } from "../../types";
import type { CurveGeometry } from "./curveGeometry";

interface CurveCanvasProps {
  geometry: CurveGeometry;
  selectedTimelineClip: Clip | null;
  seekProgramTo: (ticks: number) => void;
  setSelectedKeyframeTime: (t: number | null) => void;
  setKeyframeValue: (val: string) => void;
  setKeyframeInterpDraft: (kind: InterpolationKind) => void;
  setKeyframeHandleInX: (v: number) => void;
  setKeyframeHandleInY: (v: number) => void;
  setKeyframeHandleOutX: (v: number) => void;
  setKeyframeHandleOutY: (v: number) => void;
}

export function CurveCanvas({
  geometry,
  selectedTimelineClip,
  seekProgramTo,
  setSelectedKeyframeTime,
  setKeyframeValue,
  setKeyframeInterpDraft,
  setKeyframeHandleInX,
  setKeyframeHandleInY,
  setKeyframeHandleOutX,
  setKeyframeHandleOutY,
}: CurveCanvasProps) {
  const {
    svgW,
    svgH,
    padL,
    padT,
    plotW,
    plotH,
    clipDuration,
    option,
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
  } = geometry;

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
    <div className="curve-canvas-wrapper">
      <svg
        className="curve-canvas-svg"
        viewBox={`0 0 ${svgW} ${svgH}`}
        preserveAspectRatio="none"
        onClick={handleCanvasClick}
      >
        <defs>
          <linearGradient id="curveGradient" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" style={{ stopColor: "var(--accent-secondary)", stopOpacity: 0.35 }} />
            <stop offset="100%" style={{ stopColor: "var(--accent-secondary)", stopOpacity: 0.02 }} />
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
              style={{ stroke: "var(--curve-grid-line)" }}
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
              style={{ stroke: "var(--curve-grid-line-subtle)" }}
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
            style={{ stroke: "var(--accent-secondary)" }}
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
              style={{ stroke: "var(--status-warning)" }}
              strokeWidth="1.5"
              strokeDasharray="3 3"
            />
            <circle
              cx={activeTangents.outLine.x2}
              cy={activeTangents.outLine.y2}
              r="5"
              style={{ fill: "var(--status-warning)", stroke: "var(--color-white)" }}
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
              style={{ stroke: "var(--status-warning)" }}
              strokeWidth="1.5"
              strokeDasharray="3 3"
            />
            <circle
              cx={activeTangents.inLine.x2}
              cy={activeTangents.inLine.y2}
              r="5"
              style={{ fill: "var(--status-warning)", stroke: "var(--color-white)" }}
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
          style={{ stroke: "var(--status-danger-bright)" }}
          strokeWidth="2"
          pointerEvents="none"
        />
        <polygon
          points={`${timeToX(relPlayhead) - 6},${padT - 6} ${timeToX(relPlayhead) + 6},${padT - 6} ${timeToX(relPlayhead)},${padT + 4}`}
          style={{ fill: "var(--status-danger-bright)" }}
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
  );
}
