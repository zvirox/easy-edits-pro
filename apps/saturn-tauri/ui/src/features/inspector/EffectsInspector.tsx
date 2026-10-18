import type {
  Clip,
  ColorCorrectionParams,
  CropParams,
  EffectInstance,
  EffectKind,
  EffectPayload,
  GaussianBlurParams,
  LutParams,
  VignetteParams,
  ChromaKeyParams,
} from "../../engine/playbackEngine";
import type { SelectedClip } from "../menubar";

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

export interface EffectsInspectorProps {
  selectedClip: SelectedClip | null;
  selectedTimelineClip: Clip | null;
  addEffect: (kind: EffectKind) => Promise<void>;
  toggleEffect: (id: number, enabled: boolean) => Promise<void>;
  reorderEffect: (fromIndex: number, toIndex: number) => Promise<void>;
  removeEffect: (id: number) => Promise<void>;
  updateEffectPayload: (id: number, payload: EffectPayload) => Promise<void>;
  loadLutCubeDialog: (effectId: number) => Promise<void>;
}

export function EffectsInspector({
  selectedClip,
  selectedTimelineClip,
  addEffect,
  toggleEffect,
  reorderEffect,
  removeEffect,
  updateEffectPayload,
  loadLutCubeDialog,
}: EffectsInspectorProps) {
  const effects: EffectInstance[] = selectedTimelineClip?.effects ?? [];

  return (
    <div className="effects-inspector">
      <div className="effects-inspector-header">
        <h2>Effect Stack</h2>
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
              <option value="" disabled>
                + Add Effect…
              </option>
              <option value="color_correction">🎨 Color Correction</option>
              <option value="gaussian_blur">💧 Gaussian Blur</option>
              <option value="crop">✂️ Crop</option>
              <option value="vignette">🔘 Vignette</option>
              <option value="chroma_key">🟩 Chroma Key</option>
              <option value="lut">🎬 3D LUT (.cube)</option>
            </select>
          </div>
        )}
      </div>

      {!selectedTimelineClip ? (
        <div className="effect-empty-card">
          <p>Select a clip on the timeline to inspect and edit its effect stack.</p>
        </div>
      ) : effects.length === 0 ? (
        <div className="effect-empty-card">
          <p>No effects applied to this clip.</p>
          <small>Select an effect from the dropdown above to begin compounding visual effects.</small>
        </div>
      ) : (
        <div className="effects-stack-list">
          {effects.map((eff, idx) => {
            return (
              <article key={eff.id} className={`effect-stack-card ${eff.enabled ? "is-active" : "is-bypassed"}`}>
                <header className="effect-card-header">
                  <button
                    type="button"
                    className={`effect-toggle-btn ${eff.enabled ? "enabled" : "bypassed"}`}
                    title={eff.enabled ? "Bypass (Disable) Effect" : "Enable Effect"}
                    onClick={() => void toggleEffect(eff.id, !eff.enabled)}
                  >
                    {eff.enabled ? "👁" : "👁‍🗨"}
                  </button>
                  <div className="effect-card-title">
                    <strong>{eff.name}</strong>
                    <span className="effect-kind-badge">{eff.kind.replace("_", " ")}</span>
                  </div>
                  <div className="effect-card-actions">
                    <button
                      type="button"
                      className="effect-action-btn"
                      disabled={idx === 0}
                      title="Move Up"
                      onClick={() => void reorderEffect(idx, idx - 1)}
                    >
                      ▲
                    </button>
                    <button
                      type="button"
                      className="effect-action-btn"
                      disabled={idx === effects.length - 1}
                      title="Move Down"
                      onClick={() => void reorderEffect(idx, idx + 1)}
                    >
                      ▼
                    </button>
                    <button
                      type="button"
                      className="effect-action-btn delete"
                      title="Remove Effect"
                      onClick={() => void removeEffect(eff.id)}
                    >
                      ×
                    </button>
                  </div>
                </header>

                {eff.enabled && (
                  <div className="effect-card-body">
                    {eff.kind === "color_correction" &&
                      (() => {
                        const p = eff.params as ColorCorrectionParams;
                        return (
                          <>
                            <label>
                              <span>
                                Exposure ({p.exposure > 0 ? `+${p.exposure.toFixed(1)}` : p.exposure.toFixed(1)})
                              </span>
                              <input
                                type="range"
                                min="-5"
                                max="5"
                                step="0.1"
                                value={p.exposure ?? 0}
                                onChange={(e) =>
                                  void updateEffectPayload(eff.id, {
                                    kind: "color_correction",
                                    params: { ...p, exposure: Number(e.target.value) },
                                  })
                                }
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
                                onChange={(e) =>
                                  void updateEffectPayload(eff.id, {
                                    kind: "color_correction",
                                    params: { ...p, contrast: Number(e.target.value) },
                                  })
                                }
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
                                onChange={(e) =>
                                  void updateEffectPayload(eff.id, {
                                    kind: "color_correction",
                                    params: { ...p, saturation: Number(e.target.value) },
                                  })
                                }
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
                                onChange={(e) =>
                                  void updateEffectPayload(eff.id, {
                                    kind: "color_correction",
                                    params: { ...p, hue: Number(e.target.value) },
                                  })
                                }
                              />
                            </label>
                          </>
                        );
                      })()}

                    {eff.kind === "gaussian_blur" &&
                      (() => {
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
                              onChange={(e) =>
                                void updateEffectPayload(eff.id, {
                                  kind: "gaussian_blur",
                                  params: { sigma: Number(e.target.value) },
                                })
                              }
                            />
                          </label>
                        );
                      })()}

                    {eff.kind === "crop" &&
                      (() => {
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
                                onChange={(e) =>
                                  void updateEffectPayload(eff.id, {
                                    kind: "crop",
                                    params: { ...p, top: Number(e.target.value) },
                                  })
                                }
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
                                onChange={(e) =>
                                  void updateEffectPayload(eff.id, {
                                    kind: "crop",
                                    params: { ...p, bottom: Number(e.target.value) },
                                  })
                                }
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
                                onChange={(e) =>
                                  void updateEffectPayload(eff.id, {
                                    kind: "crop",
                                    params: { ...p, left: Number(e.target.value) },
                                  })
                                }
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
                                onChange={(e) =>
                                  void updateEffectPayload(eff.id, {
                                    kind: "crop",
                                    params: { ...p, right: Number(e.target.value) },
                                  })
                                }
                              />
                            </label>
                          </div>
                        );
                      })()}

                    {eff.kind === "vignette" &&
                      (() => {
                        const p = eff.params as VignetteParams;
                        return (
                          <>
                            <label>
                              <span>Radius ({Math.round((p.radius ?? 0.8) * 100)}%)</span>
                              <input
                                type="range"
                                min="0.1"
                                max="1.5"
                                step="0.05"
                                value={p.radius ?? 0.8}
                                onChange={(e) =>
                                  void updateEffectPayload(eff.id, {
                                    kind: "vignette",
                                    params: { ...p, radius: Number(e.target.value) },
                                  })
                                }
                              />
                            </label>
                            <label>
                              <span>Intensity ({Math.round((p.intensity ?? 0.5) * 100)}%)</span>
                              <input
                                type="range"
                                min="0"
                                max="1"
                                step="0.05"
                                value={p.intensity ?? 0.5}
                                onChange={(e) =>
                                  void updateEffectPayload(eff.id, {
                                    kind: "vignette",
                                    params: { ...p, intensity: Number(e.target.value) },
                                  })
                                }
                              />
                            </label>
                            <label>
                              <span>Softness ({Math.round((p.softness ?? 0.5) * 100)}%)</span>
                              <input
                                type="range"
                                min="0.1"
                                max="1"
                                step="0.05"
                                value={p.softness ?? 0.5}
                                onChange={(e) =>
                                  void updateEffectPayload(eff.id, {
                                    kind: "vignette",
                                    params: { ...p, softness: Number(e.target.value) },
                                  })
                                }
                              />
                            </label>
                          </>
                        );
                      })()}

                    {eff.kind === "chroma_key" &&
                      (() => {
                        const p = eff.params as ChromaKeyParams;
                        return (
                          <>
                            <div className="chroma-key-color-row">
                              <span>Target Screen Color</span>
                              <input
                                type="color"
                                value={p.color || "#00ff00"}
                                onChange={(e) =>
                                  void updateEffectPayload(eff.id, {
                                    kind: "chroma_key",
                                    params: { ...p, color: e.target.value },
                                  })
                                }
                              />
                            </div>
                            <label>
                              <span>Tolerance ({p.tolerance ?? 25}%)</span>
                              <input
                                type="range"
                                min="1"
                                max="100"
                                step="1"
                                value={p.tolerance ?? 25}
                                onChange={(e) =>
                                  void updateEffectPayload(eff.id, {
                                    kind: "chroma_key",
                                    params: { ...p, tolerance: Number(e.target.value) },
                                  })
                                }
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
                                onChange={(e) =>
                                  void updateEffectPayload(eff.id, {
                                    kind: "chroma_key",
                                    params: { ...p, softness: Number(e.target.value) },
                                  })
                                }
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
                                onChange={(e) =>
                                  void updateEffectPayload(eff.id, {
                                    kind: "chroma_key",
                                    params: { ...p, spill_suppression: Number(e.target.value) },
                                  })
                                }
                              />
                            </label>
                          </>
                        );
                      })()}

                    {eff.kind === "lut" &&
                      (() => {
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
                                  <option key={pr.id} value={pr.id}>
                                    {pr.icon} {pr.name}
                                  </option>
                                ))}
                                <option value="custom">📁 Custom .cube File…</option>
                              </select>
                            </div>

                            {p.path && (
                              <div className="lut-file-info">
                                <small title={p.path}>File: {p.path.split("/").pop()}</small>
                              </div>
                            )}

                            <button
                              type="button"
                              className="btn-secondary lut-browse-btn"
                              onClick={() => void loadLutCubeDialog(eff.id)}
                            >
                              📂 Browse .cube File…
                            </button>

                            <label>
                              <span>LUT Intensity ({Math.round((p.intensity ?? 1.0) * 100)}%)</span>
                              <input
                                type="range"
                                min="0"
                                max="1"
                                step="0.01"
                                value={p.intensity ?? 1.0}
                                onChange={(e) =>
                                  void updateEffectPayload(eff.id, {
                                    kind: "lut",
                                    params: { ...p, intensity: Number(e.target.value) },
                                  })
                                }
                              />
                            </label>
                          </div>
                        );
                      })()}
                  </div>
                )}
              </article>
            );
          })}
        </div>
      )}
      <small>Effects are rendered on GPU using hardware-accelerated shaders.</small>
    </div>
  );
}
