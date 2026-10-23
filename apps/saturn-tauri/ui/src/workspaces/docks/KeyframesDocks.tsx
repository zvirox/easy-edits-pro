import { InteriorButton } from "../../components/interior/interior-button";
import { DockCard } from "../../features/dock/DockCard";
import {
  type MediaAsset,
  type SelectedClip,
  type Clip,
  type KeyframeProperty,
  type InterpolationKind,
} from "../../types";
import { formatTime } from "../../utils/formatters";
import { keyframeOptions, computeCurveGeometry } from "../keyframes/curveGeometry";
import { CurveCanvas } from "../keyframes/CurveCanvas";
import { CurveSidebar } from "../keyframes/CurveSidebar";

export { keyframeOptions };

export interface KeyframesDockProps {
  media: MediaAsset[];
  selectedClip: SelectedClip | null;
  selectedTimelineClip: Clip | null;
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
}

export function KeyframesDockRight({
  media,
  selectedClip,
  selectedTimelineClip,
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
}: KeyframesDockProps) {
  const geometry = computeCurveGeometry({
    clipKeyframes: selectedTimelineClip?.keyframes ?? [],
    keyframeProperty,
    selectedTimelineClip,
    playhead,
    selectedKeyframeTime,
  });
  const { relPlayhead, propKeyframes, activeKf } = geometry;

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

  return (
    <DockCard
        className="shell-dock shell-dock-right"
        ariaLabel="Keyframes dock"
        tabs={[
          {
            id: "keyframes-curves",
            title: "Curves",
            render: () => (
              <div className="keyframes-curve-dock-body">
                <div className="keyframes-target-info">
                  {selectedTimelineClip ? (
                    <>
                      Target: <strong>{media.find((m) => m.id === selectedTimelineClip.media_id)?.name ?? "Clip"}</strong>
                    </>
                  ) : (
                    <span className="keyframes-hint-text">Select a clip in the timeline to edit curves</span>
                  )}
                </div>
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
            <CurveCanvas
              geometry={geometry}
              selectedTimelineClip={selectedTimelineClip}
              seekProgramTo={seekProgramTo}
              setSelectedKeyframeTime={setSelectedKeyframeTime}
              setKeyframeValue={setKeyframeValue}
              setKeyframeInterpDraft={setKeyframeInterpDraft}
              setKeyframeHandleInX={setKeyframeHandleInX}
              setKeyframeHandleInY={setKeyframeHandleInY}
              setKeyframeHandleOutX={setKeyframeHandleOutX}
              setKeyframeHandleOutY={setKeyframeHandleOutY}
            />

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
              </div>
            ),
          },
          {
            id: "keyframes-tangents",
            title: "Tangents",
            render: () => (
              <div className="keyframes-tangent-dock-body">
                <CurveSidebar
          geometry={geometry}
          selectedClip={selectedClip}
          keyframeProperty={keyframeProperty}
          keyframeValue={keyframeValue}
          setKeyframeValue={setKeyframeValue}
          keyframeInterpDraft={keyframeInterpDraft}
          setKeyframeInterpDraft={setKeyframeInterpDraft}
          keyframeHandleInX={keyframeHandleInX}
          setKeyframeHandleInX={setKeyframeHandleInX}
          keyframeHandleInY={keyframeHandleInY}
          setKeyframeHandleInY={setKeyframeHandleInY}
          keyframeHandleOutX={keyframeHandleOutX}
          setKeyframeHandleOutX={setKeyframeHandleOutX}
          keyframeHandleOutY={keyframeHandleOutY}
          setKeyframeHandleOutY={setKeyframeHandleOutY}
          saveKeyframe={saveKeyframe}
                />
              </div>
            ),
          },
        ]}
      />
  );
}
