import { InteriorButton } from "../components/interior/interior-button";
import {
  type ProjectDocument,
  type MediaAsset,
  type SelectedClip,
  type Clip,
  type KeyframeProperty,
  type InterpolationKind,
} from "../types";
import { formatTime } from "../utils/formatters";
import { keyframeOptions, computeCurveGeometry } from "./keyframes/curveGeometry";
import { CurveCanvas } from "./keyframes/CurveCanvas";
import { CurveSidebar } from "./keyframes/CurveSidebar";

export { keyframeOptions };

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
}: KeyframesWorkspaceProps) {
  const allClips = (project?.sequence.tracks ?? []).flatMap((track) =>
    track.clips.map((clip, index) => ({ track, clip, index, mediaItem: media.find((m) => m.id === clip.media_id) }))
  );

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

        {/* Right Sidebar: Tangents & Interpolation Controls */}
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
