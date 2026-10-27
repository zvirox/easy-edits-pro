import { InteriorButton } from "../../components/interior/interior-button";
import { keyframeOptions } from "../../workspaces/keyframes/curveGeometry";
import { formatTime } from "../../utils/formatters";
import type { KeyframeProperty, InterpolationKind } from "../../engine/playback/types";
import type { Clip, SelectedClip } from "../../app/projectTypes";

export interface KeyframesInspectorProps {
  selectedTimelineClip: Clip | null;
  selectedClip: SelectedClip | null;
  playhead: number;
  keyframeProperty: KeyframeProperty;
  setKeyframeProperty: (prop: KeyframeProperty) => void;
  keyframeValue: string;
  setKeyframeValue: (val: string) => void;
  keyframeInterpDraft: InterpolationKind;
  setKeyframeInterpDraft: (kind: InterpolationKind) => void;
  keyframeHandleInX: number;
  keyframeHandleInY: number;
  keyframeHandleOutX: number;
  keyframeHandleOutY: number;
  selectedKeyframeTime: number | null;
  setSelectedKeyframeTime: (time: number | null) => void;
  selectEditorWorkspace: (ws: any) => void;
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

export function KeyframesInspector({
  selectedTimelineClip,
  selectedClip,
  playhead,
  keyframeProperty,
  setKeyframeProperty,
  keyframeValue,
  setKeyframeValue,
  keyframeInterpDraft,
  setKeyframeInterpDraft,
  keyframeHandleInX,
  keyframeHandleInY,
  keyframeHandleOutX,
  keyframeHandleOutY,
  selectedKeyframeTime,
  setSelectedKeyframeTime,
  selectEditorWorkspace,
  saveKeyframe,
  deleteKeyframe,
}: KeyframesInspectorProps) {
  const option = keyframeOptions.find((item) => item.value === keyframeProperty)!;
  const keyframes = selectedTimelineClip?.keyframes ?? [];
  const propKeyframes = keyframes.filter((k) => k.property === keyframeProperty);
  const selectedKf = propKeyframes.find((k) => k.time === selectedKeyframeTime) ?? propKeyframes[0] ?? null;

  return (
    <div className="keyframes-inspector">
      <div className="keyframes-inspector-header">
        <h2>Keyframes &amp; Curves</h2>
        <InteriorButton
          type="button"
          className="btn-primary-subtle"
          onClick={() => selectEditorWorkspace("keyframes")}
          title="Open Dedicated Full-Screen Curve Editor (Shift+6)"
        >
          📈 Full Curve Editor
        </InteriorButton>
      </div>
      <p>{selectedTimelineClip ? "Edit animated properties and smooth bezier transitions." : "Select a timeline clip to edit its keyframes."}</p>

      <label>
        Property
        <select
          disabled={!selectedClip}
          value={keyframeProperty}
          onChange={(event) => {
            const value = event.target.value as KeyframeProperty;
            setKeyframeProperty(value);
            setKeyframeValue(String(keyframeOptions.find((item) => item.value === value)?.defaultValue ?? 0));
          }}
        >
          {keyframeOptions.map((item) => (
            <option value={item.value} key={item.value}>{item.label}</option>
          ))}
        </select>
      </label>

      <div className="control">
        Clip time <span>{selectedTimelineClip ? formatTime(Math.max(0, Math.min(selectedTimelineClip.duration, playhead - selectedTimelineClip.timeline_start))) : "—"}</span>
      </div>

      <label>
        Value ({option.label})
        <input
          type="number"
          min={option.min}
          max={option.max}
          step={option.step}
          disabled={!selectedClip}
          value={keyframeValue}
          onChange={(event) => setKeyframeValue(event.target.value)}
        />
      </label>

      <div className="keyframe-ease-bar">
        <span className="ease-label">Interpolation</span>
        <div className="ease-preset-pills">
          {(["linear", "ease_in", "ease_out", "ease_in_out", "bezier"] as InterpolationKind[]).map((kind) => {
            const isCurrent = (selectedKf?.interpolation ?? keyframeInterpDraft) === kind;
            return (
              <InteriorButton
                key={kind}
                type="button"
                className={`ease-pill-btn ${isCurrent ? "active" : ""}`}
                disabled={!selectedClip}
                onClick={() => {
                  setKeyframeInterpDraft(kind);
                  if (selectedKf) {
                    const hInX = kind === "ease_in" ? 1.0 : kind === "ease_out" ? 0.58 : kind === "ease_in_out" ? 0.58 : keyframeHandleInX;
                    const hInY = kind === "ease_in" ? 0.42 : kind === "ease_out" ? 1.0 : kind === "ease_in_out" ? 1.0 : keyframeHandleInY;
                    const hOutX = kind === "ease_in" ? 0.42 : kind === "ease_out" ? 0.0 : kind === "ease_in_out" ? 0.42 : keyframeHandleOutX;
                    const hOutY = kind === "ease_in" ? 0.0 : kind === "ease_out" ? 0.58 : kind === "ease_in_out" ? 0.0 : keyframeHandleOutY;
                    void saveKeyframe(keyframeProperty, selectedKf.value, selectedKf.time, kind, hInX, hInY, hOutX, hOutY);
                  }
                }}
              >
                {kind === "linear" ? "Linear" : kind === "ease_in" ? "Ease In" : kind === "ease_out" ? "Ease Out" : kind === "ease_in_out" ? "Ease In-Out" : "Bezier"}
              </InteriorButton>
            );
          })}
        </div>
      </div>

      <InteriorButton
        className="primary"
        disabled={!selectedClip || !Number.isFinite(Number(keyframeValue))}
        onClick={() => {
          const hInX = keyframeInterpDraft === "ease_in" ? 1.0 : keyframeInterpDraft === "ease_out" ? 0.58 : keyframeInterpDraft === "ease_in_out" ? 0.58 : keyframeHandleInX;
          const hInY = keyframeInterpDraft === "ease_in" ? 0.42 : keyframeInterpDraft === "ease_out" ? 1.0 : keyframeInterpDraft === "ease_in_out" ? 1.0 : keyframeHandleInY;
          const hOutX = keyframeInterpDraft === "ease_in" ? 0.42 : keyframeInterpDraft === "ease_out" ? 0.0 : keyframeInterpDraft === "ease_in_out" ? 0.42 : keyframeHandleOutX;
          const hOutY = keyframeInterpDraft === "ease_in" ? 0.0 : keyframeInterpDraft === "ease_out" ? 0.58 : keyframeInterpDraft === "ease_in_out" ? 0.0 : keyframeHandleOutY;
          void saveKeyframe(keyframeProperty, Number(keyframeValue), undefined, keyframeInterpDraft, hInX, hInY, hOutX, hOutY);
        }}
      >
        Set keyframe at Playhead
      </InteriorButton>

      <div className="keyframe-list">
        {propKeyframes.map((kf) => (
          <div
            className={`control keyframe-row ${selectedKeyframeTime === kf.time ? "is-selected" : ""}`}
            key={`${kf.property}-${kf.time}`}
            onClick={() => {
              setSelectedKeyframeTime(kf.time);
              setKeyframeValue(String(kf.value));
              setKeyframeInterpDraft(kf.interpolation ?? "linear");
            }}
          >
            <span>
              <b>{formatTime(kf.time)}</b> · {kf.value} · <i>{(kf.interpolation ?? "linear").replace("_", " ")}</i>
            </span>
            <InteriorButton
              aria-label="Remove keyframe"
              onClick={(e) => {
                e.stopPropagation();
                void deleteKeyframe(kf.property, kf.time);
              }}
            >
              ×
            </InteriorButton>
          </div>
        ))}
      </div>
      <small>Keyframe curves are evaluated with hardware cubic bezier math in real-time preview and export.</small>
    </div>
  );
}
