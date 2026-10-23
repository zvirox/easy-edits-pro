import { InteriorButton } from "../components/interior/interior-button";
import type { ReactNode } from "react";
import { CustomShaderEditor } from "../components/CustomShaderEditor";
import { BackgroundMattingCard } from "../components/BackgroundMattingCard";
import {
  type ProjectDocument,
  type MediaAsset,
  type SelectedClip,
  type Clip,
  type EffectInstance,
  type EffectKind,
  type DavinciWorkspace,
} from "../types";
import {
  type EffectPayload,
  type ColorCorrectionParams,
  type GaussianBlurParams,
  type CropParams,
  type VignetteParams,
  type ChromaKeyParams,
  type LutParams,
  type CustomShaderParams,
} from "../engine/playbackEngine";
import { LUT_PRESETS } from "./ColorWorkspace";
import { formatTime } from "../utils/formatters";
import { DockCard } from "../features/dock/DockCard";

export interface EffectsWorkspaceProps {
  project: ProjectDocument["project"] | null;
  media: MediaAsset[];
  selectedClip: SelectedClip | null;
  selectedTimelineClip: Clip | null;
  setSelectedClip: (clip: SelectedClip | null) => void;
  setSelectedClips: (clips: SelectedClip[]) => void;
  addEffect: (kind: EffectKind, name?: string) => Promise<unknown> | unknown;
  removeEffect: (id: number) => Promise<void> | void;
  toggleEffect: (id: number, enabled?: boolean) => Promise<void> | void;
  reorderEffect: (fromIndex: number, toIndex: number) => Promise<void> | void;
  updateEffectPayload: (id: number, payload: EffectPayload) => Promise<void> | void;
  loadLutCubeDialog: (effectId?: number) => Promise<void> | void;
  selectDavinciWorkspace: (ws: DavinciWorkspace) => void;
  playhead: number;
  renderProgramMonitor: () => ReactNode;
  programPlaying: boolean;
  toggleProgramPlayback: () => void;
  stepProgramFrame: (direction: -1 | 1) => void;
  seekProgramTo: (ticks: number) => void;
}

export function EffectsWorkspace({
  project,
  media,
  selectedClip,
  selectedTimelineClip,
  setSelectedClip,
  setSelectedClips,
  addEffect,
  removeEffect,
  toggleEffect,
  reorderEffect,
  updateEffectPayload,
  loadLutCubeDialog,
  selectDavinciWorkspace,
  playhead,
  renderProgramMonitor,
  programPlaying,
  toggleProgramPlayback,
  stepProgramFrame,
  seekProgramTo,
}: EffectsWorkspaceProps) {
  const allClips = (project?.sequence.tracks ?? []).flatMap((track) =>
    track.clips.map((clip, index) => ({ track, clip, index, mediaItem: media.find((m) => m.id === clip.media_id) }))
  );
  const effects: EffectInstance[] = selectedTimelineClip?.effects ?? [];

  return (
    <main className="workspace-page effects-workspace-page">
      <header className="workspace-sub-header effects-workspace-header">
        <div className="workspace-header-title">
          <h1>Visual Effects Lab</h1>
          <p>Compound visual effects stack, blur, crop, vignette, chroma keying, and motion transforms.</p>
        </div>
        <div className="effects-header-actions">
          {selectedClip && (
            <div className="add-effect-control">
              <select
                className="add-effect-select"
                defaultValue=""
                onChange={(e) => {
                  if (e.target.value) {
                    void addEffect(e.target.value as EffectKind);
                    e.target.value = "";
                  }
                }}
              >
                <option value="" disabled>＋ Add Effect to Clip…</option>
                <option value="gaussian_blur">💧 Gaussian Blur</option>
                <option value="crop">✂️ Crop</option>
                <option value="vignette">🔘 Vignette</option>
                <option value="chroma_key">🟩 Chroma Key</option>
                <option value="color_correction">🎨 Color Correction</option>
                <option value="lut">🎬 3D LUT (.cube)</option>
                <option value="custom_shader">✨ Custom GLSL Shader</option>
              </select>
            </div>
          )}
          <InteriorButton
            className="btn-primary-subtle"
            onClick={() => selectDavinciWorkspace("edit")}
          >
            ← Back to Edit
          </InteriorButton>
        </div>
      </header>

      <div className="effects-studio-body">
        {/* Main Stage: Large Program Monitor Viewer + Transport Controls */}
        <section className="effects-monitor-stage">
          <div className="effects-viewer-container">
            <div className="effects-viewer-header">
              <span className="viewer-title">PROGRAM MONITOR &amp; FX COMPOSITE</span>
              <code>{formatTime(playhead)}</code>
            </div>
            <div className="effects-monitor-wrapper">
              {renderProgramMonitor()}
            </div>
            <div className="effects-transport-bar">
              <InteriorButton aria-label="Previous frame" onClick={() => stepProgramFrame(-1)}>◀</InteriorButton>
              <InteriorButton
                className={programPlaying ? "active-toggle" : ""}
                aria-label={programPlaying ? "Pause playback" : "Play clip"}
                onClick={() => toggleProgramPlayback()}
              >
                {programPlaying ? "⏸ Pause" : "▶ Play"}
              </InteriorButton>
              <InteriorButton aria-label="Next frame" onClick={() => stepProgramFrame(1)}>▶|</InteriorButton>
              <code>{formatTime(playhead)}</code>
            </div>
          </div>
        </section>

        <DockCard
          className="workspace-dock effects-dock-stack"
          tabs={[
            {
              id: "effects-stack",
              title: "Effect Stack",
              render: () => (
                <div className="effects-stack-sidebar">
          <div className="effects-stack-box">
            <div className="effects-section-header">
              <h3>Active Effect Stack</h3>
              <small>{effects.length} effect{effects.length !== 1 ? "s" : ""} on clip</small>
            </div>

            {!selectedTimelineClip ? (
              <div className="effect-empty-card">
                <p>Select a clip in the sequence reel below to inspect and configure its effects.</p>
              </div>
            ) : effects.length === 0 ? (
              <div className="effect-empty-card">
                <p>No effects applied to this clip yet.</p>
                <small>Choose an effect from the "＋ Add Effect" dropdown above to begin compounding visual effects.</small>
              </div>
            ) : (
              <div className="effects-stack-list">
                {effects.map((eff, idx) => (
                  <article key={eff.id} className={`effect-stack-card ${eff.enabled ? "is-active" : "is-bypassed"}`}>
                    <header className="effect-card-header">
                      <InteriorButton
                        type="button"
                        className={`effect-toggle-btn ${eff.enabled ? "enabled" : "bypassed"}`}
                        title={eff.enabled ? "Bypass Effect" : "Enable Effect"}
                        onClick={() => void toggleEffect(eff.id, !eff.enabled)}
                      >
                        {eff.enabled ? "👁" : "👁‍🗨"}
                      </InteriorButton>
                      <div className="effect-card-title">
                        <strong>{eff.name}</strong>
                        <span className="effect-kind-badge">{eff.kind.replace("_", " ")}</span>
                      </div>
                      <div className="effect-card-actions">
                        <InteriorButton
                          type="button"
                          className="effect-action-btn"
                          disabled={idx === 0}
                          title="Move Up"
                          onClick={() => void reorderEffect(idx, idx - 1)}
                        >
                          ▲
                        </InteriorButton>
                        <InteriorButton
                          type="button"
                          className="effect-action-btn"
                          disabled={idx === effects.length - 1}
                          title="Move Down"
                          onClick={() => void reorderEffect(idx, idx + 1)}
                        >
                          ▼
                        </InteriorButton>
                        <InteriorButton
                          type="button"
                          className="effect-action-btn delete"
                          title="Remove Effect"
                          onClick={() => void removeEffect(eff.id)}
                        >
                          ×
                        </InteriorButton>
                      </div>
                    </header>

                    {eff.enabled && (
                      <div className="effect-card-body">
                        {eff.kind === "color_correction" && (() => {
                          const p = eff.params as ColorCorrectionParams;
                          return (
                            <>
                              <label>
                                <span>Exposure ({p.exposure > 0 ? `+${p.exposure.toFixed(1)}` : p.exposure.toFixed(1)})</span>
                                <input
                                  type="range"
                                  min="-5"
                                  max="5"
                                  step="0.1"
                                  value={p.exposure ?? 0}
                                  onChange={(e) => void updateEffectPayload(eff.id, {
                                    kind: "color_correction",
                                    params: { ...p, exposure: Number(e.target.value) },
                                  })}
                                />
                              </label>
                              <label>
                                <span>Contrast ({p.contrast.toFixed(2)})</span>
                                <input
                                  type="range"
                                  min="0"
                                  max="2"
                                  step="0.01"
                                  value={p.contrast ?? 1}
                                  onChange={(e) => void updateEffectPayload(eff.id, {
                                    kind: "color_correction",
                                    params: { ...p, contrast: Number(e.target.value) },
                                  })}
                                />
                              </label>
                              <label>
                                <span>Saturation ({p.saturation.toFixed(2)})</span>
                                <input
                                  type="range"
                                  min="0"
                                  max="2"
                                  step="0.01"
                                  value={p.saturation ?? 1}
                                  onChange={(e) => void updateEffectPayload(eff.id, {
                                    kind: "color_correction",
                                    params: { ...p, saturation: Number(e.target.value) },
                                  })}
                                />
                              </label>
                              <label>
                                <span>Hue Shift ({Math.round((p.hue ?? 0) * 180)}°)</span>
                                <input
                                  type="range"
                                  min="-1"
                                  max="1"
                                  step="0.01"
                                  value={p.hue ?? 0}
                                  onChange={(e) => void updateEffectPayload(eff.id, {
                                    kind: "color_correction",
                                    params: { ...p, hue: Number(e.target.value) },
                                  })}
                                />
                              </label>
                            </>
                          );
                        })()}

                        {eff.kind === "gaussian_blur" && (() => {
                          const p = eff.params as GaussianBlurParams;
                          return (
                            <label>
                              <span>Blur Radius ({p.sigma.toFixed(1)} px)</span>
                              <input
                                type="range"
                                min="0"
                                max="20"
                                step="0.1"
                                value={p.sigma ?? 2}
                                onChange={(e) => void updateEffectPayload(eff.id, {
                                  kind: "gaussian_blur",
                                  params: { sigma: Number(e.target.value) },
                                })}
                              />
                            </label>
                          );
                        })()}

                        {eff.kind === "crop" && (() => {
                          const p = eff.params as CropParams;
                          return (
                            <div className="crop-controls-grid">
                              <label>
                                <span>Top ({p.top ?? 0}%)</span>
                                <input
                                  type="range"
                                  min="0"
                                  max="100"
                                  step="0.5"
                                  value={p.top ?? 0}
                                  onChange={(e) => void updateEffectPayload(eff.id, {
                                    kind: "crop",
                                    params: { ...p, top: Number(e.target.value) },
                                  })}
                                />
                              </label>
                              <label>
                                <span>Bottom ({p.bottom ?? 0}%)</span>
                                <input
                                  type="range"
                                  min="0"
                                  max="100"
                                  step="0.5"
                                  value={p.bottom ?? 0}
                                  onChange={(e) => void updateEffectPayload(eff.id, {
                                    kind: "crop",
                                    params: { ...p, bottom: Number(e.target.value) },
                                  })}
                                />
                              </label>
                              <label>
                                <span>Left ({p.left ?? 0}%)</span>
                                <input
                                  type="range"
                                  min="0"
                                  max="100"
                                  step="0.5"
                                  value={p.left ?? 0}
                                  onChange={(e) => void updateEffectPayload(eff.id, {
                                    kind: "crop",
                                    params: { ...p, left: Number(e.target.value) },
                                  })}
                                />
                              </label>
                              <label>
                                <span>Right ({p.right ?? 0}%)</span>
                                <input
                                  type="range"
                                  min="0"
                                  max="100"
                                  step="0.5"
                                  value={p.right ?? 0}
                                  onChange={(e) => void updateEffectPayload(eff.id, {
                                    kind: "crop",
                                    params: { ...p, right: Number(e.target.value) },
                                  })}
                                />
                              </label>
                            </div>
                          );
                        })()}

                        {eff.kind === "vignette" && (() => {
                          const p = eff.params as VignetteParams;
                          return (
                            <>
                              <label>
                                <span>Intensity ({Math.round((p.intensity ?? 0.5) * 100)}%)</span>
                                <input
                                  type="range"
                                  min="0"
                                  max="1"
                                  step="0.01"
                                  value={p.intensity ?? 0.5}
                                  onChange={(e) => void updateEffectPayload(eff.id, {
                                    kind: "vignette",
                                    params: { ...p, intensity: Number(e.target.value) },
                                  })}
                                />
                              </label>
                              <label>
                                <span>Radius ({p.radius?.toFixed(2) ?? "0.80"})</span>
                                <input
                                  type="range"
                                  min="0.1"
                                  max="2"
                                  step="0.01"
                                  value={p.radius ?? 0.8}
                                  onChange={(e) => void updateEffectPayload(eff.id, {
                                    kind: "vignette",
                                    params: { ...p, radius: Number(e.target.value) },
                                  })}
                                />
                              </label>
                              <label>
                                <span>Softness ({Math.round((p.softness ?? 0.5) * 100)}%)</span>
                                <input
                                  type="range"
                                  min="0"
                                  max="1"
                                  step="0.01"
                                  value={p.softness ?? 0.5}
                                  onChange={(e) => void updateEffectPayload(eff.id, {
                                    kind: "vignette",
                                    params: { ...p, softness: Number(e.target.value) },
                                  })}
                                />
                              </label>
                            </>
                          );
                        })()}

                        {eff.kind === "chroma_key" && (() => {
                          const p = eff.params as ChromaKeyParams;
                          return (
                            <>
                              <div className="chroma-color-picker-row">
                                <span>Key Color</span>
                                <div className="chroma-inputs">
                                  <input
                                    type="color"
                                    value={p.color ?? "#00ff00"}
                                    onChange={(e) => void updateEffectPayload(eff.id, {
                                      kind: "chroma_key",
                                      params: { ...p, color: e.target.value },
                                    })}
                                  />
                                  <code>{p.color ?? "#00ff00"}</code>
                                </div>
                              </div>
                              <label>
                                <span>Tolerance ({Math.round(p.tolerance ?? 25)}%)</span>
                                <input
                                  type="range"
                                  min="0"
                                  max="100"
                                  step="1"
                                  value={p.tolerance ?? 25}
                                  onChange={(e) => void updateEffectPayload(eff.id, {
                                    kind: "chroma_key",
                                    params: { ...p, tolerance: Number(e.target.value) },
                                  })}
                                />
                              </label>
                              <label>
                                <span>Edge Softness ({Math.round(p.softness ?? 10)}%)</span>
                                <input
                                  type="range"
                                  min="0"
                                  max="50"
                                  step="1"
                                  value={p.softness ?? 10}
                                  onChange={(e) => void updateEffectPayload(eff.id, {
                                    kind: "chroma_key",
                                    params: { ...p, softness: Number(e.target.value) },
                                  })}
                                />
                              </label>
                              <label>
                                <span>Spill Suppression ({Math.round((p.spill_suppression ?? 0.5) * 100)}%)</span>
                                <input
                                  type="range"
                                  min="0"
                                  max="1"
                                  step="0.01"
                                  value={p.spill_suppression ?? 0.5}
                                  onChange={(e) => void updateEffectPayload(eff.id, {
                                    kind: "chroma_key",
                                    params: { ...p, spill_suppression: Number(e.target.value) },
                                  })}
                                />
                              </label>
                            </>
                          );
                        })()}

                        {eff.kind === "lut" && (() => {
                          const p = eff.params as LutParams;
                          return (
                            <div className="lut-effect-body">
                              <div className="lut-preset-selector">
                                <span>Cinematic Look</span>
                                <select
                                  value={p.preset_id || (p.path ? "custom" : "rec709")}
                                  onChange={(e) => {
                                    const val = e.target.value;
                                    if (val === "custom") {
                                      void loadLutCubeDialog(eff.id);
                                    } else {
                                      const preset = LUT_PRESETS.find((pr) => pr.id === val);
                                      if (preset) {
                                        void updateEffectPayload(eff.id, {
                                          kind: "lut",
                                          params: {
                                            name: preset.name,
                                            preset_id: preset.id,
                                            intensity: p.intensity ?? 1.0,
                                            path: null,
                                          },
                                        });
                                      }
                                    }
                                  }}
                                >
                                  {LUT_PRESETS.map((pr) => (
                                    <option key={pr.id} value={pr.id}>{pr.icon} {pr.name}</option>
                                  ))}
                                  <option value="custom">📁 Custom .cube File…</option>
                                </select>
                              </div>

                              {p.path && (
                                <div className="lut-file-info">
                                  <small title={p.path}>File: {p.path.split("/").pop()}</small>
                                </div>
                              )}

                              <InteriorButton
                                type="button"
                                className="btn-secondary lut-browse-btn"
                                onClick={() => void loadLutCubeDialog(eff.id)}
                              >
                                📂 Browse .cube File…
                              </InteriorButton>

                              <label>
                                <span>LUT Intensity ({Math.round((p.intensity ?? 1.0) * 100)}%)</span>
                                <input
                                  type="range"
                                  min="0"
                                  max="1"
                                  step="0.01"
                                  value={p.intensity ?? 1.0}
                                  onChange={(e) => void updateEffectPayload(eff.id, {
                                    kind: "lut",
                                    params: { ...p, intensity: Number(e.target.value) },
                                  })}
                                />
                              </label>
                            </div>
                          );
                        })()}

                        {eff.kind === "custom_shader" && (() => {
                          const p = (eff.params as CustomShaderParams) || {
                            name: "Custom GLSL Shader",
                            shader_code: "",
                            uniforms: {},
                          };
                          return (
                            <CustomShaderEditor
                              shaderParams={p}
                              onApply={(updatedShader) => {
                                void updateEffectPayload(eff.id, {
                                  kind: "custom_shader",
                                  params: updatedShader,
                                });
                              }}
                            />
                          );
                        })()}
                      </div>
                    )}
                  </article>
                ))}
              </div>
            )}
          </div>

          {/* AI Background Matting Card */}
          <div style={{ marginTop: "16px" }}>
            <BackgroundMattingCard clipId={selectedTimelineClip?.media_id} />
          </div>
                  </div>
                ),
              },
        ]}
      />
      </div>

      {/* Bottom Clips Reel Tray */}
      <section className="clips-reel-tray">
        <div className="reel-tray-header">
          <strong>SEQUENCE CLIPS REEL</strong>
          <small>{allClips.length} clips on timeline · Click any clip to apply or edit effects</small>
        </div>
        <div className="reel-clips-strip">
          {allClips.map(({ track, clip, index, mediaItem }) => {
            const isSelected = selectedClip?.track_id === track.id && selectedClip.clip_index === index;
            const clipFxCount = clip.effects?.length ?? 0;
            return (
              <InteriorButton
                key={`reel-fx-${track.id}-${clip.media_id}-${index}`}
                className={`reel-clip-card ${isSelected ? "selected" : ""}`}
                onClick={() => {
                  setSelectedClip({ track_id: track.id, clip_index: index });
                  setSelectedClips([{ track_id: track.id, clip_index: index }]);
                  seekProgramTo(clip.timeline_start);
                }}
              >
                <div className="reel-clip-badge">
                  <span className={`reel-track-kind ${track.kind}`}>{track.name}</span>
                  {clipFxCount > 0 && <span className="reel-fx-count-pill">{clipFxCount} FX</span>}
                </div>
                <strong className="reel-clip-name">{mediaItem?.name ?? "Clip"}</strong>
                <span className="reel-clip-dur">{formatTime(clip.duration)}</span>
              </InteriorButton>
            );
          })}
        </div>
      </section>
    </main>
  );
}
