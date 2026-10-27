import { InteriorButton } from "../../components/interior/interior-button";
import type { Dispatch, SetStateAction } from "react";
import { formatTime } from "../../utils/formatters";
import type { Clip, MediaAsset, ProjectDocument, SelectedClip } from "../../app/projectTypes";
import type { MotionBlurParams, SpeedRamp, TrackingMode } from "../../engine/playbackEngine";

export interface ClipInspectorProps {
  selectedTimelineClip: Clip | null;
  selectedClip: SelectedClip | null;
  selectedMedia: MediaAsset | null;
  media: MediaAsset[];
  project: ProjectDocument["project"] | null;
  busy: boolean;
  unlinkSelectedClips: () => Promise<void>;
  removeSelectedClip: () => void;
  trackingMode: TrackingMode;
  setTrackingMode: (mode: TrackingMode) => void;
  showTrackingBox: boolean;
  setShowTrackingBox: Dispatch<SetStateAction<boolean>>;
  trackingSmoothness: number;
  updateStabilizationSmoothness: (val: number) => Promise<void>;
  trackingTargetClipId: number | null;
  pinClipToMotionTrack: (targetId: number | null) => Promise<void>;
  isAnalyzingTracking: boolean;
  runMotionTracker: (forward: boolean) => Promise<void>;
  saveSpeedRamp: (speedRamp: SpeedRamp) => Promise<void>;
  saveMotionBlur: (motionBlur: MotionBlurParams) => Promise<void>;
  saveClipVolumeDb: (volumeDb: number) => Promise<void>;
  frameFit: (mode: "fill" | "fit" | "scale100") => Promise<void>;
  makeSubclip: () => Promise<void>;
  flattenMulticam: () => Promise<void>;
  revealClipMedia: () => void;
}

export function ClipInspector({
  selectedTimelineClip,
  selectedClip,
  selectedMedia,
  media,
  project,
  busy,
  unlinkSelectedClips,
  removeSelectedClip,
  trackingMode,
  setTrackingMode,
  showTrackingBox,
  setShowTrackingBox,
  trackingSmoothness,
  updateStabilizationSmoothness,
  trackingTargetClipId,
  pinClipToMotionTrack,
  isAnalyzingTracking,
  runMotionTracker,
  saveSpeedRamp,
  saveMotionBlur,
  saveClipVolumeDb,
  frameFit,
  makeSubclip,
  flattenMulticam,
  revealClipMedia,
}: ClipInspectorProps) {
  const clampVolume = (value: number) => Math.min(12, Math.max(-60, value));
  if (selectedTimelineClip && selectedClip) {
    const mediaForClip = media.find((item) => item.id === selectedTimelineClip.media_id);
    return (
      <>
        <h2>Clip inspector</h2>
        <p>{mediaForClip?.name ?? "Timeline clip"}</p>
        <div className="control">
          Start <span>{formatTime(selectedTimelineClip.timeline_start)}</span>
        </div>
        <div className="control">
          Duration <span>{formatTime(selectedTimelineClip.duration)}</span>
        </div>
        <div className="control">
          Source in <span>{formatTime(selectedTimelineClip.source_in)}</span>
        </div>
        <div className="control">
          Speed{" "}
          <span>
            {Math.round((selectedTimelineClip.speed ?? 1.0) * 100)}% ({selectedTimelineClip.speed ?? 1.0}x)
          </span>
        </div>
        <div className="control">
          Group Link{" "}
          <span>
            {selectedTimelineClip.group_id != null ? `Group #${selectedTimelineClip.group_id}` : "Unlinked"}
          </span>
        </div>
        {selectedTimelineClip.group_id != null ? (
          <InteriorButton className="secondary" onClick={() => void unlinkSelectedClips()} disabled={busy}>
            Unlink from group
          </InteriorButton>
        ) : null}

        {/* Per-clip audio trim (FC audio gain / volume nudges) */}
        <div className="control">
          Volume{" "}
          <span>{(selectedTimelineClip.volume_db ?? 0).toFixed(1)} dB</span>
        </div>
        <div className="speed-ramp-presets">
          {[-3, -1, 1, 3].map((delta) => (
            <InteriorButton
              key={delta}
              type="button"
              className="speed-preset-btn"
              disabled={busy}
              onClick={() =>
                void saveClipVolumeDb(
                  clampVolume((selectedTimelineClip.volume_db ?? 0) + delta),
                )
              }
            >
              {delta > 0 ? `+${delta}` : delta} dB
            </InteriorButton>
          ))}
          <InteriorButton
            type="button"
            className="speed-preset-btn"
            disabled={busy}
            onClick={() => void saveClipVolumeDb(0)}
          >
            0 dB
          </InteriorButton>
        </div>

        {/* Frame fit + clip-to-media actions */}
        <div className="speed-ramp-inspector-card">
          <div className="speed-ramp-header">
            <strong>Frame Fit &amp; Clip Actions</strong>
          </div>
          <div className="speed-ramp-presets">
            <InteriorButton type="button" className="speed-preset-btn" disabled={busy}
              onClick={() => void frameFit("fill")}>
              Fill
            </InteriorButton>
            <InteriorButton type="button" className="speed-preset-btn" disabled={busy}
              onClick={() => void frameFit("fit")}>
              Fit
            </InteriorButton>
            <InteriorButton type="button" className="speed-preset-btn" disabled={busy}
              onClick={() => void frameFit("scale100")}>
              100%
            </InteriorButton>
          </div>
          <div className="speed-ramp-presets">
            <InteriorButton type="button" className="speed-preset-btn" disabled={busy}
              onClick={() => void makeSubclip()}>
              Make Subclip
            </InteriorButton>
            <InteriorButton type="button" className="speed-preset-btn" disabled={busy}
              onClick={() => revealClipMedia()}>
              Reveal in Project
            </InteriorButton>
          </div>
        </div>
        {selectedTimelineClip.multicam_id != null ? (
          <InteriorButton className="secondary" onClick={() => void flattenMulticam()} disabled={busy}>
            Flatten multicam to active angle
          </InteriorButton>
        ) : null}

        {/* Motion Tracking & Camera Stabilization (Phase 7.3) */}
        <div className="tracking-inspector-card">
          <div className="tracking-header-row">
            <strong>🎯 Motion Tracking &amp; Stabilization</strong>
            <InteriorButton
              type="button"
              className="track-mini-btn"
              style={{ fontSize: "10px", padding: "2px 6px" }}
              onClick={() => setShowTrackingBox((v) => !v)}
            >
              {showTrackingBox ? "Hide Box" : "Show Box"}
            </InteriorButton>
          </div>
          <div className="tracking-mode-pills">
            {(["stabilize", "transform", "pin"] as TrackingMode[]).map((mode) => (
              <InteriorButton
                key={mode}
                type="button"
                className={`tracking-mode-btn ${trackingMode === mode ? "active" : ""}`}
                onClick={() => setTrackingMode(mode)}
              >
                {mode === "stabilize" ? "Stabilize" : mode === "transform" ? "Point Track" : "Pin Overlay"}
              </InteriorButton>
            ))}
          </div>
          {trackingMode === "stabilize" && (
            <label className="tracking-smoothness-row">
              <div
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  fontSize: "11px",
                  color: "var(--text-sub)",
                  marginBottom: "4px",
                }}
              >
                <span>Camera Shake Reduction</span>
                <b>{Math.round(trackingSmoothness * 100)}%</b>
              </div>
              <input
                type="range"
                min="0"
                max="1"
                step="0.05"
                value={trackingSmoothness}
                onChange={(e) => void updateStabilizationSmoothness(Number(e.target.value))}
              />
            </label>
          )}
          {trackingMode === "pin" && (
            <label style={{ display: "flex", flexDirection: "column", gap: "3px", fontSize: "11px" }}>
              <span style={{ color: "var(--text-sub)" }}>Pin Element / Sticker:</span>
              <select
                value={trackingTargetClipId ?? ""}
                onChange={(e) => void pinClipToMotionTrack(e.target.value ? Number(e.target.value) : null)}
                style={{
                  fontSize: "11px",
                  padding: "4px",
                  background: "var(--bg-input)",
                  color: "var(--text-main)",
                  border: "1px solid var(--border-color)",
                  borderRadius: "4px",
                }}
              >
                <option value="">No pinned clip</option>
                {project?.sequence.tracks
                  .flatMap((t) =>
                    t.clips.map((c) => ({
                      id: c.media_id,
                      label: media.find((m) => m.id === c.media_id)?.name ?? `Media #${c.media_id}`,
                    }))
                  )
                  .filter(
                    (item, idx, arr) =>
                      item.id !== selectedTimelineClip.media_id && arr.findIndex((x) => x.id === item.id) === idx
                  )
                  .map((item) => (
                    <option key={item.id} value={item.id}>
                      {item.label}
                    </option>
                  ))}
              </select>
            </label>
          )}
          <div className="tracking-actions-row">
            <InteriorButton
              type="button"
              className="tracking-action-btn"
              disabled={isAnalyzingTracking}
              onClick={() => void runMotionTracker(false)}
              title="Track feature backwards"
            >
              ◀ Track Backward
            </InteriorButton>
            <InteriorButton
              type="button"
              className="tracking-action-btn"
              disabled={isAnalyzingTracking}
              onClick={() => void runMotionTracker(true)}
              title="Track feature forwards"
            >
              Track Forward ▶
            </InteriorButton>
          </div>
          {isAnalyzingTracking && (
            <div className="tracking-progress-status">
              <span className="job-pill-spinner" />
              <span>Analyzing optical feature points…</span>
            </div>
          )}
        </div>

        {/* Optical Flow Speed Ramping (Phase 11.1) */}
        <div className="speed-ramp-inspector-card">
          <div className="speed-ramp-header">
            <strong>⚡ Optical Flow Speed Ramp</strong>
            <span className="speed-factor-pill">
              {((selectedTimelineClip.speed_ramp?.points[0]?.speed_factor ?? selectedTimelineClip.speed ?? 1.0) * 100).toFixed(0)}%
            </span>
          </div>
          <div className="speed-ramp-presets">
            {[0.25, 0.5, 1.0, 2.0, 4.0].map((factor) => (
              <InteriorButton
                key={factor}
                type="button"
                className={`speed-preset-btn ${
                  (selectedTimelineClip.speed_ramp?.points[0]?.speed_factor ?? selectedTimelineClip.speed ?? 1.0) === factor
                    ? "active"
                    : ""
                }`}
                onClick={() => {
                  void saveSpeedRamp({
                    points: [{ time: 0, speed_factor: factor, bezier_tangent: [0, 0] }],
                    optical_flow_interpolation: true,
                    frame_blend: false,
                  });
                }}
              >
                {factor === 0.25 ? "0.25x (Smooth)" : factor === 0.5 ? "0.5x" : factor === 1.0 ? "1x (Normal)" : `${factor}x`}
              </InteriorButton>
            ))}
          </div>
          <label className="checkbox-field" style={{ marginTop: "6px" }}>
            <input
              type="checkbox"
              checked={selectedTimelineClip.speed_ramp?.frame_blend ?? false}
              onChange={(e) => {
                const cur = selectedTimelineClip.speed_ramp ?? {
                  points: [
                    { time: 0, speed_factor: selectedTimelineClip.speed ?? 1.0, bezier_tangent: [0, 0] },
                  ],
                  optical_flow_interpolation: true,
                  frame_blend: false,
                };
                void saveSpeedRamp({ ...cur, frame_blend: e.target.checked });
              }}
            />
            <span>Temporal Frame Blending</span>
          </label>
        </div>

        {/* Dynamic Shutter Angle Motion Blur (Phase 11.2) */}
        <div className="motion-blur-inspector-card">
          <div className="motion-blur-header">
            <strong>🌀 Dynamic Shutter Motion Blur</strong>
            <label className="switch-label">
              <input
                type="checkbox"
                checked={selectedTimelineClip.motion_blur?.enabled ?? false}
                onChange={(e) => {
                  const cur = selectedTimelineClip.motion_blur ?? {
                    enabled: false,
                    shutter_angle_deg: 180.0,
                    samples: 8,
                  };
                  void saveMotionBlur({ ...cur, enabled: e.target.checked });
                }}
              />
              <span className="switch-toggle" />
            </label>
          </div>
          {selectedTimelineClip.motion_blur?.enabled && (
            <div className="motion-blur-controls">
              <label className="param-field">
                <div style={{ display: "flex", justifyContent: "space-between", fontSize: "11px" }}>
                  <span>Shutter Angle:</span>
                  <b>{(selectedTimelineClip.motion_blur.shutter_angle_deg ?? 180).toFixed(0)}°</b>
                </div>
                <input
                  type="range"
                  min="0"
                  max="360"
                  step="15"
                  value={selectedTimelineClip.motion_blur.shutter_angle_deg ?? 180}
                  onChange={(e) => {
                    const cur = selectedTimelineClip.motion_blur!;
                    void saveMotionBlur({ ...cur, shutter_angle_deg: Number(e.target.value) });
                  }}
                />
              </label>
              <div className="shutter-presets">
                {[90, 180, 270, 360].map((deg) => (
                  <InteriorButton
                    key={deg}
                    type="button"
                    className={`shutter-btn ${
                      Math.round(selectedTimelineClip.motion_blur?.shutter_angle_deg ?? 180) === deg ? "active" : ""
                    }`}
                    onClick={() => {
                      const cur = selectedTimelineClip.motion_blur!;
                      void saveMotionBlur({ ...cur, shutter_angle_deg: deg });
                    }}
                  >
                    {deg === 180 ? "180° (Cinema)" : `${deg}°`}
                  </InteriorButton>
                ))}
              </div>
              <label className="param-field" style={{ marginTop: "6px" }}>
                <div style={{ display: "flex", justifyContent: "space-between", fontSize: "11px" }}>
                  <span>Temporal Multi-Samples:</span>
                  <b>{selectedTimelineClip.motion_blur.samples ?? 8} samples</b>
                </div>
                <input
                  type="range"
                  min="4"
                  max="32"
                  step="2"
                  value={selectedTimelineClip.motion_blur.samples ?? 8}
                  onChange={(e) => {
                    const cur = selectedTimelineClip.motion_blur!;
                    void saveMotionBlur({ ...cur, samples: Number(e.target.value) });
                  }}
                />
              </label>
            </div>
          )}
        </div>

        <InteriorButton className="danger-button" onClick={removeSelectedClip}>
          Remove clip
        </InteriorButton>
      </>
    );
  }

  return (
    <>
      <h2>Inspector</h2>
      <p>Select media or a clip to inspect its properties.</p>
      {selectedMedia ? (
        <>
          <div className="control">
            Type <span>{selectedMedia.kind}</span>
          </div>
          <div className="control">
            Duration <span>{selectedMedia.duration ? formatTime(selectedMedia.duration) : "—"}</span>
          </div>
          <div className="control">
            Dimensions{" "}
            <span>
              {selectedMedia.width && selectedMedia.height
                ? `${selectedMedia.width} × ${selectedMedia.height}`
                : "—"}
            </span>
          </div>
        </>
      ) : (
        <div className="control">No selection</div>
      )}
    </>
  );
}
