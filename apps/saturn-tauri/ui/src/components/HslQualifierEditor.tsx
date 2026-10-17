import React from "react";
import type { HslQualifier } from "../engine/playbackEngine";

interface HslQualifierEditorProps {
  qualifier?: HslQualifier | null;
  onChange: (updated: HslQualifier) => void;
  showMatte: boolean;
  onToggleShowMatte: (show: boolean) => void;
}

export const defaultQualifier: HslQualifier = {
  enabled: true,
  hue_center: 25.0, // Default to healthy skin tone
  hue_width: 35.0,
  hue_softness: 15.0,
  sat_center: 0.5,
  sat_width: 0.5,
  sat_softness: 0.15,
  luma_center: 0.55,
  luma_width: 0.5,
  luma_softness: 0.15,
  invert: false,
  blur_radius: 2.0,
  hue_shift: 0.0,
  sat_boost: 1.0,
  exposure_shift: 0.0,
};

export const HslQualifierEditor: React.FC<HslQualifierEditorProps> = ({
  qualifier,
  onChange,
  showMatte,
  onToggleShowMatte,
}) => {
  const current = qualifier || defaultQualifier;

  const updateField = <K extends keyof HslQualifier>(field: K, value: HslQualifier[K]) => {
    onChange({
      ...current,
      [field]: value,
    });
  };

  const applyPreset = (preset: "skin" | "sky" | "foliage" | "fire") => {
    switch (preset) {
      case "skin":
        onChange({
          ...current,
          enabled: true,
          hue_center: 25.0,
          hue_width: 35.0,
          hue_softness: 15.0,
          sat_center: 0.55,
          sat_width: 0.45,
          luma_center: 0.6,
          luma_width: 0.5,
          hue_shift: 0.0,
          sat_boost: 1.15,
          exposure_shift: 0.2,
        });
        break;
      case "sky":
        onChange({
          ...current,
          enabled: true,
          hue_center: 205.0,
          hue_width: 45.0,
          hue_softness: 20.0,
          sat_center: 0.6,
          sat_width: 0.5,
          luma_center: 0.65,
          luma_width: 0.5,
          hue_shift: -5.0,
          sat_boost: 1.3,
          exposure_shift: -0.2,
        });
        break;
      case "foliage":
        onChange({
          ...current,
          enabled: true,
          hue_center: 115.0,
          hue_width: 50.0,
          hue_softness: 20.0,
          sat_center: 0.5,
          sat_width: 0.5,
          luma_center: 0.5,
          luma_width: 0.5,
          hue_shift: 10.0,
          sat_boost: 1.25,
          exposure_shift: 0.1,
        });
        break;
      case "fire":
        onChange({
          ...current,
          enabled: true,
          hue_center: 10.0,
          hue_width: 30.0,
          hue_softness: 15.0,
          sat_center: 0.8,
          sat_width: 0.4,
          luma_center: 0.7,
          luma_width: 0.5,
          hue_shift: 0.0,
          sat_boost: 1.4,
          exposure_shift: 0.4,
        });
        break;
    }
  };

  return (
    <div
      className="hsl-qualifier-editor"
      style={{
        background: "#13171f",
        border: "1px solid #2d3748",
        borderRadius: 8,
        padding: 16,
        color: "#e2e8f0",
      }}
    >
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 14 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          <label style={{ display: "flex", alignItems: "center", gap: 6, fontWeight: 600, fontSize: 13, cursor: "pointer" }}>
            <input
              type="checkbox"
              checked={current.enabled}
              onChange={(e) => updateField("enabled", e.target.checked)}
            />
            HSL Secondary Qualifier
          </label>
          <span style={{ fontSize: 11, color: "#a0aec0", background: "#1a202c", padding: "2px 6px", borderRadius: 4 }}>
            Color Isolation &amp; Keying
          </span>
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
          <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 12, cursor: "pointer", color: showMatte ? "#63b3ed" : "#cbd5e0" }}>
            <input
              type="checkbox"
              checked={showMatte}
              onChange={(e) => onToggleShowMatte(e.target.checked)}
            />
            Show Matte (B&amp;W Key)
          </label>
          <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 12, cursor: "pointer", color: "#cbd5e0" }}>
            <input
              type="checkbox"
              checked={current.invert}
              onChange={(e) => updateField("invert", e.target.checked)}
            />
            Invert
          </label>
        </div>
      </div>

      {/* Target Presets */}
      <div style={{ display: "flex", gap: 8, marginBottom: 16, alignItems: "center" }}>
        <span style={{ fontSize: 11, color: "#718096" }}>Key Presets:</span>
        <button
          type="button"
          onClick={() => applyPreset("skin")}
          style={{ background: "#2d3748", border: "none", color: "#ecc94b", fontSize: 11, padding: "3px 8px", borderRadius: 4, cursor: "pointer" }}
        >
          👤 Skin Tones (25°)
        </button>
        <button
          type="button"
          onClick={() => applyPreset("sky")}
          style={{ background: "#2d3748", border: "none", color: "#63b3ed", fontSize: 11, padding: "3px 8px", borderRadius: 4, cursor: "pointer" }}
        >
          🌤️ Sky Blue (205°)
        </button>
        <button
          type="button"
          onClick={() => applyPreset("foliage")}
          style={{ background: "#2d3748", border: "none", color: "#68d391", fontSize: 11, padding: "3px 8px", borderRadius: 4, cursor: "pointer" }}
        >
          🌿 Foliage Green (115°)
        </button>
        <button
          type="button"
          onClick={() => applyPreset("fire")}
          style={{ background: "#2d3748", border: "none", color: "#fc8181", fontSize: 11, padding: "3px 8px", borderRadius: 4, cursor: "pointer" }}
        >
          🔥 Fire Red (10°)
        </button>
      </div>

      {/* Sliders Grid */}
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16, marginBottom: 16 }}>
        {/* Hue Isolation */}
        <div style={{ background: "#1a202c", padding: 12, borderRadius: 6, border: "1px solid #2d3748" }}>
          <div style={{ display: "flex", justifyContent: "space-between", fontSize: 12, fontWeight: 600, marginBottom: 8 }}>
            <span>🎨 Hue Isolation</span>
            <span style={{ color: "#cbd5e0" }}>{Math.round(current.hue_center)}° ± {Math.round(current.hue_width / 2)}°</span>
          </div>

          <div style={{ marginBottom: 10 }}>
            <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, color: "#a0aec0", marginBottom: 2 }}>
              <span>Center Hue</span>
              <span>{Math.round(current.hue_center)}°</span>
            </div>
            <input
              type="range"
              min="0"
              max="360"
              step="1"
              value={current.hue_center}
              onChange={(e) => updateField("hue_center", parseFloat(e.target.value))}
              style={{
                width: "100%",
                height: 8,
                borderRadius: 4,
                background: "linear-gradient(to right, #ff0000 0%, #ffff00 17%, #00ff00 33%, #00ffff 50%, #0000ff 67%, #ff00ff 83%, #ff0000 100%)",
                outline: "none",
                cursor: "pointer",
              }}
            />
          </div>

          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
            <div>
              <div style={{ display: "flex", justifyContent: "space-between", fontSize: 10, color: "#a0aec0" }}>
                <span>Width</span>
                <span>{Math.round(current.hue_width)}°</span>
              </div>
              <input
                type="range"
                min="5"
                max="180"
                step="1"
                value={current.hue_width}
                onChange={(e) => updateField("hue_width", parseFloat(e.target.value))}
                style={{ width: "100%" }}
              />
            </div>
            <div>
              <div style={{ display: "flex", justifyContent: "space-between", fontSize: 10, color: "#a0aec0" }}>
                <span>Softness</span>
                <span>{Math.round(current.hue_softness)}°</span>
              </div>
              <input
                type="range"
                min="1"
                max="90"
                step="1"
                value={current.hue_softness}
                onChange={(e) => updateField("hue_softness", parseFloat(e.target.value))}
                style={{ width: "100%" }}
              />
            </div>
          </div>
        </div>

        {/* Saturation & Luma Isolation */}
        <div style={{ background: "#1a202c", padding: 12, borderRadius: 6, border: "1px solid #2d3748" }}>
          <div style={{ display: "flex", justifyContent: "space-between", fontSize: 12, fontWeight: 600, marginBottom: 8 }}>
            <span>✨ Saturation &amp; Luma</span>
            <span style={{ color: "#a0aec0", fontSize: 11 }}>Range Gating</span>
          </div>

          <div style={{ marginBottom: 10 }}>
            <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, color: "#a0aec0", marginBottom: 2 }}>
              <span>Saturation Center</span>
              <span>{Math.round(current.sat_center * 100)}%</span>
            </div>
            <input
              type="range"
              min="0"
              max="1"
              step="0.01"
              value={current.sat_center}
              onChange={(e) => updateField("sat_center", parseFloat(e.target.value))}
              style={{ width: "100%" }}
            />
          </div>

          <div>
            <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, color: "#a0aec0", marginBottom: 2 }}>
              <span>Luminance Center</span>
              <span>{Math.round(current.luma_center * 100)}%</span>
            </div>
            <input
              type="range"
              min="0"
              max="1"
              step="0.01"
              value={current.luma_center}
              onChange={(e) => updateField("luma_center", parseFloat(e.target.value))}
              style={{ width: "100%" }}
            />
          </div>
        </div>
      </div>

      {/* Secondary Color Adjustment Controls */}
      <div style={{ background: "#18202f", padding: 12, borderRadius: 6, border: "1px solid #2b384e" }}>
        <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 10, color: "#63b3ed", display: "flex", alignItems: "center", gap: 6 }}>
          <span>🎯 Secondary Grade Adjustment (Applied inside Matte)</span>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 14 }}>
          <div>
            <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, color: "#cbd5e0", marginBottom: 4 }}>
              <span>Hue Shift</span>
              <span>{Math.round(current.hue_shift)}°</span>
            </div>
            <input
              type="range"
              min="-180"
              max="180"
              step="1"
              value={current.hue_shift}
              onChange={(e) => updateField("hue_shift", parseFloat(e.target.value))}
              style={{ width: "100%" }}
            />
          </div>

          <div>
            <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, color: "#cbd5e0", marginBottom: 4 }}>
              <span>Saturation Boost</span>
              <span>{current.sat_boost.toFixed(2)}x</span>
            </div>
            <input
              type="range"
              min="0.0"
              max="2.5"
              step="0.05"
              value={current.sat_boost}
              onChange={(e) => updateField("sat_boost", parseFloat(e.target.value))}
              style={{ width: "100%" }}
            />
          </div>

          <div>
            <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, color: "#cbd5e0", marginBottom: 4 }}>
              <span>Exposure Lift</span>
              <span>{current.exposure_shift > 0 ? `+${current.exposure_shift.toFixed(2)}` : current.exposure_shift.toFixed(2)} EV</span>
            </div>
            <input
              type="range"
              min="-2.0"
              max="2.0"
              step="0.05"
              value={current.exposure_shift}
              onChange={(e) => updateField("exposure_shift", parseFloat(e.target.value))}
              style={{ width: "100%" }}
            />
          </div>
        </div>
      </div>
    </div>
  );
};
