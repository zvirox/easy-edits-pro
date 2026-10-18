import type { Dispatch, SetStateAction, ReactNode, RefObject } from "react";
import { ColorWheels, type ColorWheelsData, DEFAULT_COLOR_WHEELS } from "../components/ColorWheels";
import { RgbCurvesEditor } from "../components/RgbCurvesEditor";
import { HslQualifierEditor } from "../components/HslQualifierEditor";
import { VideoScopes } from "../components/VideoScopes";
import {
  type ProjectDocument,
  type MediaAsset,
  type SelectedClip,
  type Clip,
  type ColorAdjustments,
  type DavinciWorkspace,
  COLOR_PRESETS,
} from "../types";
import {
  type RgbCurves,
  type HslQualifier,
  type TimelinePlaybackSnapshot,
} from "../engine/playbackEngine";
import { formatTime } from "../utils/formatters";

export interface LutPresetItem {
  id: string;
  name: string;
  description: string;
  icon: string;
}

export const LUT_PRESETS: LutPresetItem[] = [
  {
    id: "teal_orange",
    name: "Teal & Orange",
    description: "Blockbuster cinematic contrast with cool cyan shadows & warm skin tones",
    icon: "🌅",
  },
  {
    id: "vintage",
    name: "Warm Vintage",
    description: "Golden 70s celluloid film with rich amber warmth & soft roll-off",
    icon: "🎞️",
  },
  {
    id: "film_noir",
    name: "Film Noir",
    description: "Deep dramatic black & white with rich silver tones and crisp highlights",
    icon: "🎥",
  },
  {
    id: "rec709",
    name: "Clean Rec.709",
    description: "Neutral studio broadcast calibration curve with accurate skin tones",
    icon: "📺",
  },
];

export interface ColorWorkspaceProps {
  project: ProjectDocument["project"] | null;
  media: MediaAsset[];
  selectedClip: SelectedClip | null;
  selectedTimelineClip: Clip | null;
  setSelectedClip: (clip: SelectedClip | null) => void;
  setSelectedClips: (clips: SelectedClip[]) => void;
  colorDraft: ColorAdjustments;
  setColorDraft: Dispatch<SetStateAction<ColorAdjustments>>;
  colorWheelsDraft: ColorWheelsData;
  setColorWheelsDraft: Dispatch<SetStateAction<ColorWheelsData>>;
  rgbCurvesDraft: RgbCurves;
  hslQualifierDraft: HslQualifier;
  showMatteDraft: boolean;
  setShowMatteDraft: (v: boolean) => void;
  colorStudioTab: "wheels" | "curves" | "qualifier";
  setColorStudioTab: (tab: "wheels" | "curves" | "qualifier") => void;
  updateColorWheels: (wheels: ColorWheelsData) => void;
  updateRgbCurves: (curves: RgbCurves) => void;
  updateHslQualifier: (qualifier: HslQualifier) => void;
  applyColorPreset: (preset: { id: string; label: string; icon: string; exposure: number; contrast: number; saturation: number }) => Promise<void> | void;
  applyLutPreset: (lut: LutPresetItem) => Promise<void> | void;
  loadLutCubeDialog: () => Promise<void> | void;
  saveColor: (adjustments: ColorAdjustments) => Promise<void>;
  runCommand: (id: string, params: Record<string, unknown>) => Promise<unknown>;
  selectDavinciWorkspace: (ws: DavinciWorkspace) => void;
  playhead: number;
  renderProgramMonitor: () => ReactNode;
  programPlaying: boolean;
  toggleProgramPlayback: () => void;
  stepProgramFrame: (direction: -1 | 1) => void;
  seekProgramTo: (ticks: number) => void;
  deckSnapshot: TimelinePlaybackSnapshot | null;
  deckARef: RefObject<HTMLVideoElement | null>;
  deckBRef: RefObject<HTMLVideoElement | null>;
}

export function ColorWorkspace({
  project,
  media,
  selectedClip,
  selectedTimelineClip,
  setSelectedClip,
  setSelectedClips,
  colorDraft,
  setColorDraft,
  colorWheelsDraft,
  setColorWheelsDraft,
  rgbCurvesDraft,
  hslQualifierDraft,
  showMatteDraft,
  setShowMatteDraft,
  colorStudioTab,
  setColorStudioTab,
  updateColorWheels,
  updateRgbCurves,
  updateHslQualifier,
  applyColorPreset,
  applyLutPreset,
  loadLutCubeDialog,
  saveColor,
  runCommand,
  selectDavinciWorkspace,
  playhead,
  renderProgramMonitor,
  programPlaying,
  toggleProgramPlayback,
  stepProgramFrame,
  seekProgramTo,
  deckSnapshot,
  deckARef,
  deckBRef,
}: ColorWorkspaceProps) {
  const allClips = (project?.sequence.tracks ?? []).flatMap((track) =>
    track.clips.map((clip, index) => ({ track, clip, index, mediaItem: media.find((m) => m.id === clip.media_id) }))
  );

  const updateColorField = (field: keyof ColorAdjustments, value: number) => {
    const next = { ...colorDraft, [field]: value };
    setColorDraft(next);
    void saveColor(next);
  };

  const resetColorGrading = () => {
    const resetWheels = {
      lift: { red: 0, green: 0, blue: 0, luma: 0 },
      gamma: { red: 0, green: 0, blue: 0, luma: 0 },
      gain: { red: 0, green: 0, blue: 0, luma: 0 },
      offset: { red: 0, green: 0, blue: 0, luma: 0 },
    };
    const reset = { exposure: 0, contrast: 1.0, saturation: 1.0, hue: 0.0, wheels: resetWheels };
    setColorDraft(reset);
    setColorWheelsDraft(DEFAULT_COLOR_WHEELS);
    void saveColor(reset);
    if (selectedClip) {
      void runCommand("clip.set_color_wheels", {
        track_id: selectedClip.track_id,
        clip_index: selectedClip.clip_index,
        wheels: resetWheels,
      }).catch(() => {});
    }
  };

  return (
    <main className="workspace-page color-workspace-page">
      <header className="workspace-sub-header color-workspace-header">
        <div className="workspace-header-title">
          <span className="workspace-eyebrow">COLOR · PRIMARY GRADING &amp; SCOPES</span>
          <h1>Color Grading Studio</h1>
          <p>
            Real-time GPU video scopes (Waveform, Vectorscope, Parade, Histogram), 3-way color balance wheels &amp; cinematic presets.
          </p>
        </div>
        <div className="color-header-actions">
          <span className="color-target-info">
            {selectedTimelineClip ? (
              <>
                Target: <strong>{media.find((m) => m.id === selectedTimelineClip.media_id)?.name ?? "Selected Clip"}</strong>
              </>
            ) : (
              <span className="color-hint-text">Select a clip in the reel below to grade</span>
            )}
          </span>
          <button
            className="btn-secondary"
            onClick={resetColorGrading}
            disabled={!selectedClip}
            title="Reset exposure, contrast, saturation, and color wheels to neutral"
          >
            Reset All
          </button>
          <button
            className="btn-primary-subtle"
            onClick={() => selectDavinciWorkspace("edit")}
          >
            ← Back to Edit
          </button>
        </div>
      </header>

      <div className="color-studio-body">
        {/* Top Half: Side-by-Side Program Monitor + Video Scopes */}
        <div className="color-top-row">
          <section className="color-monitor-stage">
            <div className="color-viewer-container">
              <div className="color-viewer-header">
                <span className="viewer-title">PROGRAM MONITOR</span>
                <code>{formatTime(playhead)}</code>
              </div>
              <div className="color-monitor-wrapper">
                {renderProgramMonitor()}
              </div>
              <div className="color-transport-bar">
                <button aria-label="Previous frame" onClick={() => stepProgramFrame(-1)}>◀</button>
                <button
                  className={programPlaying ? "active-toggle" : ""}
                  aria-label={programPlaying ? "Pause playback" : "Play clip"}
                  onClick={() => toggleProgramPlayback()}
                >
                  {programPlaying ? "⏸ Pause" : "▶ Play"}
                </button>
                <button aria-label="Next frame" onClick={() => stepProgramFrame(1)}>▶|</button>
                <code>{formatTime(playhead)}</code>
              </div>
            </div>
          </section>

          {/* Video Scopes Stage */}
          <section className="color-scopes-stage">
            <VideoScopes
              getActiveVideo={() => {
                if (deckSnapshot?.deckB.visible && deckBRef.current) return deckBRef.current;
                return deckARef.current;
              }}
              activeFilter={
                deckSnapshot?.deckB.visible
                  ? (deckSnapshot.deckB.style.filter as string | undefined)
                  : (deckSnapshot?.deckA.style.filter as string | undefined)
              }
            />
          </section>
        </div>

        {/* Bottom Half: 4 Primary Color Wheels Dock + Sliders & Presets */}
        <div className="color-bottom-row">
          <div className="color-wheels-dock-wrapper">
            <div
              className="color-studio-tabs"
              style={{
                display: "flex",
                gap: "8px",
                marginBottom: "12px",
                borderBottom: "1px solid #2d3748",
                paddingBottom: "8px",
              }}
            >
              <button
                type="button"
                className={`btn-secondary ${colorStudioTab === "wheels" ? "active" : ""}`}
                style={{
                  background: colorStudioTab === "wheels" ? "#3182ce" : "#1a202c",
                  color: colorStudioTab === "wheels" ? "#ffffff" : "#a0aec0",
                  fontWeight: 600,
                  fontSize: "12px",
                  padding: "6px 12px",
                  borderRadius: "4px",
                  border: "1px solid #2d3748",
                  cursor: "pointer",
                }}
                onClick={() => setColorStudioTab("wheels")}
              >
                🎡 3-Way Color Wheels
              </button>
              <button
                type="button"
                className={`btn-secondary ${colorStudioTab === "curves" ? "active" : ""}`}
                style={{
                  background: colorStudioTab === "curves" ? "#3182ce" : "#1a202c",
                  color: colorStudioTab === "curves" ? "#ffffff" : "#a0aec0",
                  fontWeight: 600,
                  fontSize: "12px",
                  padding: "6px 12px",
                  borderRadius: "4px",
                  border: "1px solid #2d3748",
                  cursor: "pointer",
                }}
                onClick={() => setColorStudioTab("curves")}
              >
                📈 RGB Curves
              </button>
              <button
                type="button"
                className={`btn-secondary ${colorStudioTab === "qualifier" ? "active" : ""}`}
                style={{
                  background: colorStudioTab === "qualifier" ? "#3182ce" : "#1a202c",
                  color: colorStudioTab === "qualifier" ? "#ffffff" : "#a0aec0",
                  fontWeight: 600,
                  fontSize: "12px",
                  padding: "6px 12px",
                  borderRadius: "4px",
                  border: "1px solid #2d3748",
                  cursor: "pointer",
                }}
                onClick={() => setColorStudioTab("qualifier")}
              >
                🎯 HSL Secondary Qualifier
              </button>
            </div>

            {colorStudioTab === "wheels" && (
              <ColorWheels
                value={colorWheelsDraft}
                onChange={updateColorWheels}
                disabled={!selectedClip}
              />
            )}
            {colorStudioTab === "curves" && (
              <RgbCurvesEditor
                curves={rgbCurvesDraft}
                onChange={updateRgbCurves}
              />
            )}
            {colorStudioTab === "qualifier" && (
              <HslQualifierEditor
                qualifier={hslQualifierDraft}
                onChange={updateHslQualifier}
                showMatte={showMatteDraft}
                onToggleShowMatte={setShowMatteDraft}
              />
            )}
          </div>

          <aside className="color-sidebar-dock">
            <div className="color-grading-box">
              <div className="color-section-header">
                <h3>Primary Sliders</h3>
                <small>Exposure / Contrast / Sat</small>
              </div>

              <div className="color-sliders-group">
                <label className="color-slider-item">
                  <div className="slider-label-row">
                    <span>Exposure</span>
                    <b>{colorDraft.exposure > 0 ? `+${colorDraft.exposure.toFixed(1)}` : colorDraft.exposure.toFixed(1)}</b>
                  </div>
                  <input
                    type="range"
                    min="-5"
                    max="5"
                    step="0.1"
                    disabled={!selectedClip}
                    value={colorDraft.exposure}
                    onChange={(e) => setColorDraft((v) => ({ ...v, exposure: Number(e.target.value) }))}
                    onPointerUp={(e) => updateColorField("exposure", Number(e.currentTarget.value))}
                    onKeyUp={(e) => updateColorField("exposure", Number(e.currentTarget.value))}
                  />
                </label>

                <label className="color-slider-item">
                  <div className="slider-label-row">
                    <span>Contrast</span>
                    <b>{colorDraft.contrast.toFixed(2)}</b>
                  </div>
                  <input
                    type="range"
                    min="0"
                    max="2"
                    step="0.01"
                    disabled={!selectedClip}
                    value={colorDraft.contrast}
                    onChange={(e) => setColorDraft((v) => ({ ...v, contrast: Number(e.target.value) }))}
                    onPointerUp={(e) => updateColorField("contrast", Number(e.currentTarget.value))}
                    onKeyUp={(e) => updateColorField("contrast", Number(e.currentTarget.value))}
                  />
                </label>

                <label className="color-slider-item">
                  <div className="slider-label-row">
                    <span>Saturation</span>
                    <b>{colorDraft.saturation.toFixed(2)}</b>
                  </div>
                  <input
                    type="range"
                    min="0"
                    max="2"
                    step="0.01"
                    disabled={!selectedClip}
                    value={colorDraft.saturation}
                    onChange={(e) => setColorDraft((v) => ({ ...v, saturation: Number(e.target.value) }))}
                    onPointerUp={(e) => updateColorField("saturation", Number(e.currentTarget.value))}
                    onKeyUp={(e) => updateColorField("saturation", Number(e.currentTarget.value))}
                  />
                </label>

                <label className="color-slider-item">
                  <div className="slider-label-row">
                    <span>Hue Tint</span>
                    <b>{((colorDraft.hue ?? 0) * 180).toFixed(0)}°</b>
                  </div>
                  <input
                    type="range"
                    min="-1"
                    max="1"
                    step="0.01"
                    disabled={!selectedClip}
                    value={colorDraft.hue ?? 0}
                    onChange={(e) => setColorDraft((v) => ({ ...v, hue: Number(e.target.value) }))}
                    onPointerUp={(e) => updateColorField("hue", Number(e.currentTarget.value))}
                    onKeyUp={(e) => updateColorField("hue", Number(e.currentTarget.value))}
                  />
                </label>
              </div>
            </div>

            <div className="color-presets-box">
              <div className="color-section-header">
                <h3>Look Presets</h3>
                <small>1-Click</small>
              </div>
              <div className="color-presets-grid compact">
                {COLOR_PRESETS.map((preset) => (
                  <button
                    key={preset.id}
                    className="color-preset-card compact"
                    disabled={!selectedClip}
                    onClick={() => void applyColorPreset(preset)}
                    title={`Apply ${preset.label} grade`}
                  >
                    <span className="preset-card-icon">{preset.icon}</span>
                    <strong className="preset-card-title">{preset.label}</strong>
                  </button>
                ))}
              </div>
            </div>

            <div className="color-presets-box lut-presets-box">
              <div className="color-section-header">
                <h3>3D Film Looks (.cube)</h3>
                <small>Trilinear 3D Tables</small>
              </div>
              <div className="color-presets-grid compact">
                {LUT_PRESETS.map((lut) => (
                  <button
                    key={lut.id}
                    className="color-preset-card compact"
                    disabled={!selectedClip}
                    onClick={() => void applyLutPreset(lut)}
                    title={`Apply ${lut.name}: ${lut.description}`}
                  >
                    <span className="preset-card-icon">{lut.icon}</span>
                    <strong className="preset-card-title">{lut.name}</strong>
                  </button>
                ))}
              </div>
              <button
                type="button"
                className="btn-secondary lut-open-btn"
                disabled={!selectedClip}
                onClick={() => void loadLutCubeDialog()}
                style={{ marginTop: "8px", width: "100%", fontSize: "11px", padding: "6px" }}
              >
                📂 Load Custom .cube File…
              </button>
            </div>
          </aside>
        </div>
      </div>

      {/* Bottom Clips Reel Tray */}
      <section className="clips-reel-tray">
        <div className="reel-tray-header">
          <strong>SEQUENCE CLIPS REEL</strong>
          <small>{allClips.length} clips on timeline · Click any clip to grade</small>
        </div>
        <div className="reel-clips-strip">
          {allClips.map(({ track, clip, index, mediaItem }) => {
            const isSelected = selectedClip?.track_id === track.id && selectedClip.clip_index === index;
            return (
              <button
                key={`reel-${track.id}-${clip.media_id}-${index}`}
                className={`reel-clip-card ${isSelected ? "selected" : ""}`}
                onClick={() => {
                  setSelectedClip({ track_id: track.id, clip_index: index });
                  setSelectedClips([{ track_id: track.id, clip_index: index }]);
                  seekProgramTo(clip.timeline_start);
                }}
              >
                <div className="reel-clip-badge">
                  <span className={`reel-track-kind ${track.kind}`}>{track.name}</span>
                  <code>{formatTime(clip.timeline_start)}</code>
                </div>
                <strong className="reel-clip-name">{mediaItem?.name ?? "Clip"}</strong>
                <span className="reel-clip-dur">{formatTime(clip.duration)}</span>
              </button>
            );
          })}
        </div>
      </section>
    </main>
  );
}
