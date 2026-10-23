import { InteriorButton } from "../../components/interior/interior-button";
import type { KeyframeProperty, InterpolationKind, SelectedClip } from "../../types";
import type { CurveGeometry } from "./curveGeometry";

interface CurveSidebarProps {
  geometry: CurveGeometry;
  selectedClip: SelectedClip | null;
  keyframeProperty: KeyframeProperty;
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
}

export function CurveSidebar({
  geometry,
  selectedClip,
  keyframeProperty,
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
  saveKeyframe,
}: CurveSidebarProps) {
  const { activeKf, option } = geometry;
  return (
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
  );
}
