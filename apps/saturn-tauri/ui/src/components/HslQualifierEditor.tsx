// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import { InteriorButton } from "./interior/interior-button";
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
      className="hsl-qualifier-editor" style={{
        background: "var(--bg-surface)",
        border: "1px solid var(--border)",
        borderRadius: 8,
        padding: 16,
        color: "var(--text)",
      }}
    >
      <div style={{ display: "flex", flexWrap: "wrap", justifyContent: "space-between", alignItems: "center", gap: 8, marginBottom: 14 }}>
        <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: 8 }}>
          <label style={{ display: "flex", alignItems: "center", gap: 6, fontWeight: 600, fontSize: 13, cursor: "pointer" }}>
            <input
              type="checkbox" checked={current.enabled}
              onChange={(e) => updateField("enabled", e.target.checked)}
            />
            HSL Secondary Qualifier
          </label>
          <span style={{ fontSize: 11, color: "var(--text-muted)", background: "var(--bg-surface-2)", padding: "2px 6px", borderRadius: 4 }}>
            Color Isolation &amp; Keying
          </span>
        </div>
        <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: 10 }}>
          <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 12, cursor: "pointer", color: showMatte ? "var(--primary-text)" : "var(--text-secondary)" }}>
            <input
              type="checkbox" checked={showMatte}
              onChange={(e) => onToggleShowMatte(e.target.checked)}
            />
            Show Matte (B&amp;W Key)
          </label>
          <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 12, cursor: "pointer", color: "var(--text-secondary)" }}>
            <input
              type="checkbox" checked={current.invert}
              onChange={(e) => updateField("invert", e.target.checked)}
            />
            Invert
          </label>
        </div>
      </div>

      {/* Target Presets */}
      <div style={{ display: "flex", flexWrap: "wrap", gap: 6, marginBottom: 14, alignItems: "center" }}>
        <span style={{ fontSize: 11, color: "var(--text-muted)" }}>Key Presets:</span>
        <InteriorButton
          type="button" onClick={() => applyPreset("skin")}
          style={{ background: "var(--bg-surface-2)", border: "1px solid var(--border)", color: "var(--amber)", fontSize: 11, padding: "3px 8px", borderRadius: 4, cursor: "pointer" }}
        >
           Skin Tones (25°)
        </InteriorButton>
        <InteriorButton
          type="button" onClick={() => applyPreset("sky")}
          style={{ background: "var(--bg-surface-2)", border: "1px solid var(--border)", color: "var(--sky-text)", fontSize: 11, padding: "3px 8px", borderRadius: 4, cursor: "pointer" }}
        >
           Sky Blue (205°)
        </InteriorButton>
        <InteriorButton
          type="button" onClick={() => applyPreset("foliage")}
          style={{ background: "var(--bg-surface-2)", border: "1px solid var(--border)", color: "var(--primary-text)", fontSize: 11, padding: "3px 8px", borderRadius: 4, cursor: "pointer" }}
        >
           Foliage Accent (115°)
        </InteriorButton>
        <InteriorButton
          type="button" onClick={() => applyPreset("fire")}
          style={{ background: "var(--bg-surface-2)", border: "1px solid var(--border)", color: "var(--red-text)", fontSize: 11, padding: "3px 8px", borderRadius: 4, cursor: "pointer" }}
        >
           Fire Red (10°)
        </InteriorButton>
      </div>

      {/* Sliders Grid */}
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))", gap: 12, marginBottom: 14 }}>
        {/* Hue Isolation */}
        <div style={{ background: "var(--bg-surface-2)", padding: 12, borderRadius: 6, border: "1px solid var(--border)" }}>
          <div style={{ display: "flex", justifyContent: "space-between", fontSize: 12, fontWeight: 600, marginBottom: 8 }}>
            <span> Hue Isolation</span>
            <span style={{ color: "var(--text-secondary)" }}>{Math.round(current.hue_center)}° ± {Math.round(current.hue_width / 2)}°</span>
          </div>

          <div style={{ marginBottom: 10 }}>
            <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, color: "var(--text-muted)", marginBottom: 2 }}>
              <span>Center Hue</span>
              <span>{Math.round(current.hue_center)}°</span>
            </div>
            <input
              type="range" min="0" max="360" step="1" value={current.hue_center}
              onChange={(e) => updateField("hue_center", parseFloat(e.target.value))}
              style={{
                width: "100%",
                height: 8,
                borderRadius: 4,
                background: "linear-gradient(to right, var(--red) 0%, var(--amber) 20%, var(--sky) 40%, var(--blue) 60%, var(--purple) 80%, var(--pink) 100%)",
                outline: "none",
                cursor: "pointer",
              }}
            />
          </div>

          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
            <div>
              <div style={{ display: "flex", justifyContent: "space-between", fontSize: 10, color: "var(--text-muted)" }}>
                <span>Width</span>
                <span>{Math.round(current.hue_width)}°</span>
              </div>
              <input
                type="range" min="5" max="180" step="1" value={current.hue_width}
                onChange={(e) => updateField("hue_width", parseFloat(e.target.value))}
                style={{ width: "100%" }}
              />
            </div>
            <div>
              <div style={{ display: "flex", justifyContent: "space-between", fontSize: 10, color: "var(--text-muted)" }}>
                <span>Softness</span>
                <span>{Math.round(current.hue_softness)}°</span>
              </div>
              <input
                type="range" min="1" max="90" step="1" value={current.hue_softness}
                onChange={(e) => updateField("hue_softness", parseFloat(e.target.value))}
                style={{ width: "100%" }}
              />
            </div>
          </div>
        </div>

        {/* Saturation & Luma Isolation */}
        <div style={{ background: "var(--bg-surface-2)", padding: 12, borderRadius: 6, border: "1px solid var(--border)" }}>
          <div style={{ display: "flex", justifyContent: "space-between", fontSize: 12, fontWeight: 600, marginBottom: 8 }}>
            <span>✨ Saturation &amp; Luma</span>
            <span style={{ color: "var(--text-muted)", fontSize: 11 }}>Range Gating</span>
          </div>

          <div style={{ marginBottom: 10 }}>
            <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, color: "var(--text-muted)", marginBottom: 2 }}>
              <span>Saturation Center</span>
              <span>{Math.round(current.sat_center * 100)}%</span>
            </div>
            <input
              type="range" min="0" max="1" step="0.01" value={current.sat_center}
              onChange={(e) => updateField("sat_center", parseFloat(e.target.value))}
              style={{ width: "100%" }}
            />
          </div>

          <div>
            <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, color: "var(--text-muted)", marginBottom: 2 }}>
              <span>Luminance Center</span>
              <span>{Math.round(current.luma_center * 100)}%</span>
            </div>
            <input
              type="range" min="0" max="1" step="0.01" value={current.luma_center}
              onChange={(e) => updateField("luma_center", parseFloat(e.target.value))}
              style={{ width: "100%" }}
            />
          </div>
        </div>
      </div>

      {/* Secondary Color Adjustment Controls */}
      <div style={{ background: "var(--bg-surface-2)", padding: 12, borderRadius: 6, border: "1px solid var(--border)" }}>
        <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 10, color: "var(--primary-text)", display: "flex", alignItems: "center", gap: 6 }}>
          <span> Secondary Grade Adjustment (Applied inside Matte)</span>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(110px, 1fr))", gap: 10 }}>
          <div>
            <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 }}>
              <span>Hue Shift</span>
              <span>{Math.round(current.hue_shift)}°</span>
            </div>
            <input
              type="range" min="-180" max="180" step="1" value={current.hue_shift}
              onChange={(e) => updateField("hue_shift", parseFloat(e.target.value))}
              style={{ width: "100%" }}
            />
          </div>

          <div>
            <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 }}>
              <span>Saturation Boost</span>
              <span>{current.sat_boost.toFixed(2)}x</span>
            </div>
            <input
              type="range" min="0.0" max="2.5" step="0.05" value={current.sat_boost}
              onChange={(e) => updateField("sat_boost", parseFloat(e.target.value))}
              style={{ width: "100%" }}
            />
          </div>

          <div>
            <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 }}>
              <span>Exposure Lift</span>
              <span>{current.exposure_shift > 0 ? `+${current.exposure_shift.toFixed(2)}` : current.exposure_shift.toFixed(2)} EV</span>
            </div>
            <input
              type="range" min="-2.0" max="2.0" step="0.05" value={current.exposure_shift}
              onChange={(e) => updateField("exposure_shift", parseFloat(e.target.value))}
              style={{ width: "100%" }}
            />
          </div>
        </div>
      </div>
    </div>
  );
};
