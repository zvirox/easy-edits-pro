import { useRef, useState } from "react";
import type { PointerEvent as ReactPointerEvent } from "react";
import { InteriorButton } from "../../components/interior/interior-button";
import { TICKS_PER_SECOND } from "../../utils/formatters";
import type { AutomationInterpolation, Track } from "../../types";
import {
  AUTOMATION_VOLUME_MAX_DB,
  AUTOMATION_VOLUME_MIN_DB,
  clampAutomationValue,
  findPointIndexNear,
  type AutomationLaneName,
} from "../../engine/playback/audioAutomation";
import type { AudioClockMaster } from "../../engine/playbackEngine";

interface AutomationLaneProps {
  track: Track;
  playhead: number;
  timelineZoom: number;
  timelineWidth: number;
  lane: AutomationLaneName;
  onChangeLane: (lane: AutomationLaneName) => void;
  addPoint: (trackId: number, lane: AutomationLaneName, time: number, value: number) => Promise<void>;
  removePoint: (trackId: number, lane: AutomationLaneName, time: number) => Promise<void>;
  clearLane: (trackId: number, lane: AutomationLaneName) => Promise<void>;
  setInterpolation: (trackId: number, interpolation: AutomationInterpolation) => Promise<void>;
  audioClock: AudioClockMaster;
}

const LANE_HEIGHT = 120;

/** Maps a lane value to the lane's vertical pixel position (0 = top). */
function valueToY(lane: AutomationLaneName, value: number): number {
  const ratio =
    lane === "volume"
      ? (AUTOMATION_VOLUME_MAX_DB - value) / (AUTOMATION_VOLUME_MAX_DB - AUTOMATION_VOLUME_MIN_DB)
      : (1 - value) / 2;
  return Math.max(0, Math.min(1, ratio)) * LANE_HEIGHT;
}

function yToValue(lane: AutomationLaneName, y: number): number {
  const ratio = Math.max(0, Math.min(1, y / LANE_HEIGHT));
  return lane === "volume"
    ? AUTOMATION_VOLUME_MAX_DB - ratio * (AUTOMATION_VOLUME_MAX_DB - AUTOMATION_VOLUME_MIN_DB)
    : 1 - ratio * 2;
}

interface DragState {
  pointTime: number;
  originalTime: number;
  value: number;
}

/** Track automation lane editor: draws the volume/pan breakpoint curve, and
 * supports click-to-add, drag-to-move, alt-click-to-remove point editing.
 * Drag previews over the Web Audio clock and commits on pointer-up. */
export function AutomationLane({
  track,
  playhead,
  timelineZoom,
  timelineWidth,
  lane,
  onChangeLane,
  addPoint,
  removePoint,
  clearLane,
  setInterpolation,
  audioClock,
}: AutomationLaneProps) {
  const [activeLane, setActiveLane] = useState<AutomationLaneName>(lane);
  const [drag, setDrag] = useState<DragState | null>(null);
  const svgRef = useRef<SVGSVGElement | null>(null);

  const points =
    (activeLane === "volume" ? track.automation?.volume_points : track.automation?.pan_points) ?? [];
  const interpolation = track.automation?.interpolation ?? "linear";
  const mode = track.automation?.mode ?? "off";

  const timeFromX = (clientX: number): number => {
    const svg = svgRef.current;
    if (!svg) return 0;
    const rect = svg.getBoundingClientRect();
    const ratio = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
    return Math.max(0, Math.round(ratio * timelineWidth));
  };

  const yFromEvent = (event: ReactPointerEvent<SVGSVGElement>): number => {
    const rect = event.currentTarget.getBoundingClientRect();
    return Math.max(0, Math.min(LANE_HEIGHT, event.clientY - rect.top));
  };

  const handleLanePointerDown = (event: ReactPointerEvent<SVGSVGElement>) => {
    const time = timeFromX(event.clientX);
    const value = clampAutomationValue(activeLane, yToValue(activeLane, yFromEvent(event)));
    const hitPoint = findPointIndexNear(points, time);
    if (event.altKey) {
      if (hitPoint >= 0) void removePoint(track.id, activeLane, points[hitPoint].time);
      return;
    }
    if (hitPoint >= 0) {
      const point = points[hitPoint];
      setDrag({ pointTime: point.time, originalTime: point.time, value: point.value });
      event.currentTarget.setPointerCapture(event.pointerId);
      return;
    }
    void addPoint(track.id, activeLane, time, value);
  };

  const handleLanePointerMove = (event: ReactPointerEvent<SVGSVGElement>) => {
    if (!drag) return;
    const time = timeFromX(event.clientX);
    const value = clampAutomationValue(activeLane, yToValue(activeLane, yFromEvent(event)));
    // Live preview through the Web Audio clock; committed on pointer-up.
    if (activeLane === "volume") {
      audioClock.updateTrackGain(track.id, value);
    } else {
      audioClock.updateTrackPan(track.id, value);
    }
    setDrag({ ...drag, pointTime: time, value });
  };

  const handleLanePointerUp = (event: ReactPointerEvent<SVGSVGElement>) => {
    if (!drag) return;
    const { originalTime, pointTime, value } = drag;
    setDrag(null);
    event.currentTarget.releasePointerCapture(event.pointerId);
    void (async () => {
      await removePoint(track.id, activeLane, originalTime);
      await addPoint(track.id, activeLane, pointTime, value);
    })();
  };

  const curvePath = (() => {
    const livePoints = drag
      ? points.map((point) =>
          point.time === drag.originalTime ? { time: drag.pointTime, value: drag.value } : point,
        )
      : points;
    if (livePoints.length === 0) return null;
    const xOf = (time: number) => ((time / TICKS_PER_SECOND) * timelineZoom).toFixed(1);
    if (livePoints.length === 1) {
      const y = valueToY(activeLane, livePoints[0].value).toFixed(1);
      return `M 0 ${y} H ${timelineWidth}`;
    }
    let path = `M ${xOf(livePoints[0].time)} ${valueToY(activeLane, livePoints[0].value).toFixed(1)}`;
    for (let index = 1; index < livePoints.length; index += 1) {
      const previous = livePoints[index - 1];
      const current = livePoints[index];
      if (interpolation === "constant") {
        path += ` L ${xOf(current.time)} ${valueToY(activeLane, previous.value).toFixed(1)}`;
      }
      path += ` L ${xOf(current.time)} ${valueToY(activeLane, current.value).toFixed(1)}`;
    }
    return path;
  })();

  const playheadX = (playhead / TICKS_PER_SECOND) * timelineZoom;
  const livePoints = drag
    ? points.map((point) =>
        point.time === drag.originalTime ? { time: drag.pointTime, value: drag.value } : point,
      )
    : points;

  return (
    <section className="automation-lane-panel" aria-label={`Automation lane for ${track.name}`}>
      <div className="automation-lane-header">
        <strong className="automation-lane-title">{track.name} · AUTOMATION</strong>
        <div className="automation-lane-tools">
          {(["volume", "pan"] as AutomationLaneName[]).map((laneName) => (
            <InteriorButton
              key={laneName}
              type="button"
              className={`auto-pill ${activeLane === laneName ? "active" : ""}`}
              onClick={() => {
                setActiveLane(laneName);
                onChangeLane(laneName);
              }}
              title={`${laneName} automation lane`}
            >
              {laneName === "volume" ? "VOL" : "PAN"}
            </InteriorButton>
          ))}
          <select
            className="strip-bus-select"
            value={interpolation}
            title="Curve interpolation between points"
            onChange={(e) => void setInterpolation(track.id, e.target.value as AutomationInterpolation)}
          >
            <option value="linear">LINEAR</option>
            <option value="constant">CONSTANT</option>
            <option value="smooth">SMOOTH</option>
          </select>
          <span className={`auto-mode-badge ${mode}`}>
            {mode.toUpperCase()}
            {mode === "off" ? " — points will not play" : ""}
          </span>
          <InteriorButton
            type="button"
            className="strip-unity-btn"
            title="Clear all points on this lane"
            onClick={() => void clearLane(track.id, activeLane)}
          >
            CLEAR
          </InteriorButton>
        </div>
      </div>
      <div className="automation-lane-scroller">
        <svg
          ref={svgRef}
          className="automation-lane-svg"
          width={timelineWidth}
          height={LANE_HEIGHT}
          viewBox={`0 0 ${timelineWidth} ${LANE_HEIGHT}`}
          preserveAspectRatio="none"
          onPointerDown={handleLanePointerDown}
          onPointerMove={handleLanePointerMove}
          onPointerUp={handleLanePointerUp}
          onPointerCancel={() => {
            setDrag(null);
          }}
        >
          <line x1="0" y1={LANE_HEIGHT / 2} x2={timelineWidth} y2={LANE_HEIGHT / 2} className="automation-unity-line" />
          {curvePath && <path d={curvePath} className="automation-curve-path" />}
          {livePoints.map((point, index) => (
            <circle
              key={`${point.time}-${index}`}
              cx={((point.time / TICKS_PER_SECOND) * timelineZoom).toFixed(1)}
              cy={valueToY(activeLane, point.value).toFixed(1)}
              r="4"
              className={`automation-point-dot ${drag && point.time === drag.pointTime ? "dragging" : ""}`}
            />
          ))}
          <line x1={playheadX} y1="0" x2={playheadX} y2={LANE_HEIGHT} className="automation-playhead-line" />
        </svg>
      </div>
    </section>
  );
}
