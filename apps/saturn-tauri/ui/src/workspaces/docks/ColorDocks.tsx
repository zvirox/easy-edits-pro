import { useState } from "react";
import { InteriorButton } from "../../components/interior/interior-button";
import type { Dispatch, SetStateAction, RefObject } from "react";
import { ColorWheels, type ColorWheelsData, DEFAULT_COLOR_WHEELS } from "../../components/ColorWheels";
import { RgbCurvesEditor } from "../../components/RgbCurvesEditor";
import { HslQualifierEditor } from "../../components/HslQualifierEditor";
import { VideoScopes } from "../../components/VideoScopes";
import { DockCard, type DockTab } from "../../features/dock/DockCard";
import { useClipSelection } from "../../app/selectionStore";
import { MediaPanel, type MediaPanelProps } from "../../features/media-pool";
import {
  type ProjectDocument,
  type MediaAsset,
  type SelectedClip,
  type Clip,
  type ColorAdjustments,
  COLOR_PRESETS,
} from "../../types";
import {
  type RgbCurves,
  type HslQualifier,
  type TimelinePlaybackSnapshot,
} from "../../engine/playbackEngine";

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

export interface ColorDockProps {
  project: ProjectDocument["project"] | null;
  media: MediaAsset[];
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
  updateColorWheels: (wheels: ColorWheelsData) => void;
  updateRgbCurves: (curves: RgbCurves) => void;
  updateHslQualifier: (qualifier: HslQualifier) => void;
  applyColorPreset: (preset: { id: string; label: string; icon: string; exposure: number; contrast: number; saturation: number }) => Promise<void> | void;
  applyLutPreset: (lut: LutPresetItem) => Promise<void> | void;
  loadLutCubeDialog: () => Promise<void> | void;
  saveColor: (adjustments: ColorAdjustments) => Promise<void>;
  runCommand: (id: string, params: Record<string, unknown>) => Promise<unknown>;
  deckSnapshot: TimelinePlaybackSnapshot | null;
  deckARef: RefObject<HTMLVideoElement | null>;
  deckBRef: RefObject<HTMLVideoElement | null>;
  mediaPanelProps: MediaPanelProps;
}

/**
 * Color-mode dock tabs for the WorkspaceShell (plan v4 G3): scopes and the
 * media pool sit left of the persistent program monitor; the grade studio
 * (wheels / curves / qualifier) and the looks side (primary sliders, presets,
 * LUTs) sit right. Clip targeting comes from the persistent timeline instead
 * of the retired SEQUENCE CLIPS REEL.
 */
export function ColorDockLeft({
  deckSnapshot,
  deckARef,
  deckBRef,
  mediaPanelProps,
}: Pick<ColorDockProps, "deckSnapshot" | "deckARef" | "deckBRef" | "mediaPanelProps">) {
  const tabs: DockTab[] = [
    {
      id: "scopes",
      title: "Video Scopes",
      render: () => (
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
      ),
    },
    { id: "media", title: "Media pool", render: () => <MediaPanel {...mediaPanelProps} /> },
  ];
  return <DockCard className="shell-dock shell-dock-left" tabs={tabs} ariaLabel="Color dock" />;
}

export function ColorDockRight({
  colorDraft,
  setColorDraft,
  colorWheelsDraft,
  setColorWheelsDraft,
  rgbCurvesDraft,
  hslQualifierDraft,
  showMatteDraft,
  setShowMatteDraft,
  updateColorWheels,
  updateRgbCurves,
  updateHslQualifier,
  applyColorPreset,
  applyLutPreset,
  loadLutCubeDialog,
  saveColor,
  runCommand,
}: Omit<
  ColorDockProps,
  | "project"
  | "media"
  | "selectedTimelineClip"
  | "setSelectedClip"
  | "setSelectedClips"
  | "deckSnapshot"
  | "deckARef"
  | "deckBRef"
  | "mediaPanelProps"
>) {
  // F1: selection comes from the shared store; the prop is gone.
  const { clip: selectedClip } = useClipSelection();
  const [studioTab, setStudioTab] = useState<"wheels" | "curves" | "qualifier">("wheels");

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

  const gradeTabs: DockTab[] = [
    {
      id: "grade",
      title: "Grade",
      render: () => (
        <div className="color-wheels-dock-wrapper">
          <div
            className="color-studio-tabs"
            style={{
              display: "flex",
              gap: "8px",
              marginBottom: "12px",
              borderBottom: "1px solid #462d40",
              paddingBottom: "8px",
            }}
          >
            <InteriorButton
              type="button"
              className={`btn-secondary ${studioTab === "wheels" ? "active" : ""}`}
              style={{
                background: studioTab === "wheels" ? "#9f1249" : "#100810",
                color: studioTab === "wheels" ? "#ffffff" : "#a0aec0",
                fontWeight: 600,
                fontSize: "12px",
                padding: "6px 12px",
                borderRadius: "4px",
                border: "1px solid #462d40",
                cursor: "pointer",
              }}
              onClick={() => setStudioTab("wheels")}
            >
              🎡 3-Way Color Wheels
            </InteriorButton>
            <InteriorButton
              type="button"
              className={`btn-secondary ${studioTab === "curves" ? "active" : ""}`}
              style={{
                background: studioTab === "curves" ? "#9f1249" : "#100810",
                color: studioTab === "curves" ? "#ffffff" : "#a0aec0",
                fontWeight: 600,
                fontSize: "12px",
                padding: "6px 12px",
                borderRadius: "4px",
                border: "1px solid #462d40",
                cursor: "pointer",
              }}
              onClick={() => setStudioTab("curves")}
            >
              📈 RGB Curves
            </InteriorButton>
            <InteriorButton
              type="button"
              className={`btn-secondary ${studioTab === "qualifier" ? "active" : ""}`}
              style={{
                background: studioTab === "qualifier" ? "#9f1249" : "#100810",
                color: studioTab === "qualifier" ? "#ffffff" : "#a0aec0",
                fontWeight: 600,
                fontSize: "12px",
                padding: "6px 12px",
                borderRadius: "4px",
                border: "1px solid #462d40",
                cursor: "pointer",
              }}
              onClick={() => setStudioTab("qualifier")}
            >
              🎯 HSL Secondary Qualifier
            </InteriorButton>
          </div>

          {studioTab === "wheels" && (
            <ColorWheels
              value={colorWheelsDraft}
              onChange={updateColorWheels}
              disabled={!selectedClip}
            />
          )}
          {studioTab === "curves" && (
            <RgbCurvesEditor
              curves={rgbCurvesDraft}
              onChange={updateRgbCurves}
            />
          )}
          {studioTab === "qualifier" && (
            <HslQualifierEditor
              qualifier={hslQualifierDraft}
              onChange={updateHslQualifier}
              showMatte={showMatteDraft}
              onToggleShowMatte={setShowMatteDraft}
            />
          )}
        </div>
      ),
    },
    {
      id: "looks",
      title: "Looks",
      render: () => (
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
            <InteriorButton
              className="btn-secondary"
              style={{ padding: "3px 10px", fontSize: "11px" }}
              onClick={resetColorGrading}
              disabled={!selectedClip}
              title="Reset exposure, contrast, saturation, and color wheels to neutral"
            >
              Reset All
            </InteriorButton>
          </div>

          <div className="color-presets-box">
            <div className="color-section-header">
              <h3>Look Presets</h3>
              <small>1-Click</small>
            </div>
            <div className="color-presets-grid compact">
              {COLOR_PRESETS.map((preset) => (
                <InteriorButton
                  key={preset.id}
                  className="color-preset-card compact"
                  disabled={!selectedClip}
                  onClick={() => void applyColorPreset(preset)}
                  title={`Apply ${preset.label} grade`}
                >
                  <span className="preset-card-icon">{preset.icon}</span>
                  <strong className="preset-card-title">{preset.label}</strong>
                </InteriorButton>
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
                <InteriorButton
                  key={lut.id}
                  className="color-preset-card compact"
                  disabled={!selectedClip}
                  onClick={() => void applyLutPreset(lut)}
                  title={`Apply ${lut.name}: ${lut.description}`}
                >
                  <span className="preset-card-icon">{lut.icon}</span>
                  <strong className="preset-card-title">{lut.name}</strong>
                </InteriorButton>
              ))}
            </div>
            <InteriorButton
              type="button"
              className="btn-secondary lut-open-btn"
              disabled={!selectedClip}
              onClick={() => void loadLutCubeDialog()}
              style={{ marginTop: "8px", width: "100%", fontSize: "11px", padding: "6px" }}
            >
              📂 Load Custom .cube File…
            </InteriorButton>
          </div>
        </aside>
      ),
    },
  ];
  return <DockCard className="shell-dock shell-dock-right" tabs={gradeTabs} ariaLabel="Color grading dock" />;
}
