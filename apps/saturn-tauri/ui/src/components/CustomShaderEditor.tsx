import { InteriorButton } from "./interior/interior-button";
import React, { useState, useEffect } from "react";
import type { CustomShaderParams } from "../engine/playbackEngine";

interface CustomShaderEditorProps {
  shaderParams?: CustomShaderParams | null;
  onApply: (params: CustomShaderParams) => void;
}

interface ShaderPreset {
  id: string;
  name: string;
  uniforms: Record<string, number>;
  code: string;
}

const SHADER_PRESETS: ShaderPreset[] = [
  {
    id: "crt_scanlines",
    name: "Retro CRT Scanlines",
    uniforms: { u_scanline_density: 0.5, u_scanline_intensity: 0.35 },
    code: `// Retro CRT Scanlines Fragment Shader
precision highp float;
uniform sampler2D u_texture;
uniform vec2 u_resolution;
uniform float u_time;
uniform float u_scanline_density;
uniform float u_scanline_intensity;
varying vec2 v_texCoord;

void main() {
    vec4 color = texture2D(u_texture, v_texCoord);
    float scanline = sin(v_texCoord.y * u_resolution.y * u_scanline_density * 3.14159) * 0.5 + 0.5;
    color.rgb *= mix(1.0, scanline, u_scanline_intensity);
    gl_FragColor = color;
}`,
  },
  {
    id: "chromatic_aberration",
    name: "Radial Chromatic Aberration",
    uniforms: { u_amount: 0.5 },
    code: `// Radial Chromatic Aberration
precision highp float;
uniform sampler2D u_texture;
uniform vec2 u_resolution;
uniform float u_amount;
varying vec2 v_texCoord;

void main() {
    vec2 dir = v_texCoord - vec2(0.5);
    vec2 offset = dir * (u_amount * 0.05);
    float r = texture2D(u_texture, v_texCoord + offset).r;
    float g = texture2D(u_texture, v_texCoord).g;
    float b = texture2D(u_texture, v_texCoord - offset).b;
    float a = texture2D(u_texture, v_texCoord).a;
    gl_FragColor = vec4(r, g, b, a);
}`,
  },
  {
    id: "vignette",
    name: "Cinematic Vignette Falloff",
    uniforms: { u_radius: 0.75, u_softness: 0.45 },
    code: `// Cinematic Radial Vignette
precision highp float;
uniform sampler2D u_texture;
uniform vec2 u_resolution;
uniform float u_radius;
uniform float u_softness;
varying vec2 v_texCoord;

void main() {
    vec4 color = texture2D(u_texture, v_texCoord);
    vec2 pos = v_texCoord - vec2(0.5);
    float dist = length(pos);
    float vignette = smoothstep(u_radius, u_radius - u_softness, dist);
    color.rgb *= vignette;
    gl_FragColor = color;
}`,
  },
  {
    id: "film_bloom",
    name: "Dreamy Film Bloom & Glow",
    uniforms: { u_threshold: 0.65, u_intensity: 0.8 },
    code: `// Dreamy Highlight Bloom & Glow
precision highp float;
uniform sampler2D u_texture;
uniform vec2 u_resolution;
uniform float u_threshold;
uniform float u_intensity;
varying vec2 v_texCoord;

void main() {
    vec4 base = texture2D(u_texture, v_texCoord);
    vec3 highlights = max(base.rgb - vec3(u_threshold), vec3(0.0));
    base.rgb += highlights * u_intensity;
    gl_FragColor = base;
}`,
  },
  {
    id: "invert",
    name: "Negative Color Inversion",
    uniforms: {},
    code: `// Negative Color Inversion
precision highp float;
uniform sampler2D u_texture;
varying vec2 v_texCoord;

void main() {
    vec4 color = texture2D(u_texture, v_texCoord);
    gl_FragColor = vec4(1.0 - color.rgb, color.a);
}`,
  },
];

export const CustomShaderEditor: React.FC<CustomShaderEditorProps> = ({
  shaderParams,
  onApply,
}) => {
  const [name, setName] = useState(shaderParams?.name || "Custom GLSL Shader");
  const [selectedPreset, setSelectedPreset] = useState(shaderParams?.preset_id || "crt_scanlines");
  const [code, setCode] = useState(shaderParams?.shader_code || SHADER_PRESETS[0].code);
  const [uniforms, setUniforms] = useState<Record<string, number>>(
    shaderParams?.uniforms || SHADER_PRESETS[0].uniforms
  );

  useEffect(() => {
    if (shaderParams) {
      setName(shaderParams.name);
      if (shaderParams.preset_id) setSelectedPreset(shaderParams.preset_id);
      setCode(shaderParams.shader_code);
      setUniforms(shaderParams.uniforms || {});
    }
  }, [shaderParams]);

  const handlePresetSelect = (presetId: string) => {
    setSelectedPreset(presetId);
    const p = SHADER_PRESETS.find((item) => item.id === presetId);
    if (p) {
      setName(p.name);
      setCode(p.code);
      setUniforms({ ...p.uniforms });
    }
  };

  const handleUniformChange = (key: string, val: number) => {
    setUniforms((prev) => ({
      ...prev,
      [key]: val,
    }));
  };

  const handleSave = () => {
    onApply({
      name,
      shader_code: code,
      preset_id: selectedPreset,
      uniforms,
    });
  };

  return (
    <div
      className="custom-shader-editor"
      style={{
        background: "#180d17",
        border: "1px solid #462d40",
        borderRadius: 8,
        padding: 16,
        color: "#e2e8f0",
      }}
    >
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <span style={{ fontSize: 18 }}>✨</span>
          <div>
            <h3 style={{ margin: 0, fontSize: 14, fontWeight: 600 }}>Custom GLSL Shader Plugin</h3>
            <span style={{ fontSize: 11, color: "#a0aec0" }}>
              GPU fragment shader with customizable uniforms and presets
            </span>
          </div>
        </div>
        <InteriorButton
          type="button"
          onClick={handleSave}
          style={{
            padding: "6px 14px",
            background: "#9f1249",
            color: "#ffffff",
            fontWeight: 600,
            fontSize: 12,
            border: "none",
            borderRadius: 4,
            cursor: "pointer",
          }}
        >
          ✓ Apply Shader
        </InteriorButton>
      </div>

      {/* Preset Selector */}
      <div style={{ display: "flex", gap: 10, alignItems: "center", marginBottom: 12 }}>
        <label style={{ fontSize: 11, color: "#a0aec0", fontWeight: 500 }}>Shader Preset:</label>
        <select
          value={selectedPreset}
          onChange={(e) => handlePresetSelect(e.target.value)}
          style={{
            background: "#100810",
            border: "1px solid #5a4053",
            borderRadius: 4,
            color: "#f7fafc",
            fontSize: 12,
            padding: "4px 8px",
            outline: "none",
          }}
        >
          {SHADER_PRESETS.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
          <option value="custom">✏️ Custom GLSL Code</option>
        </select>

        <input
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Plugin Display Name"
          style={{
            flex: 1,
            background: "#100810",
            border: "1px solid #5a4053",
            borderRadius: 4,
            color: "#e2e8f0",
            fontSize: 12,
            padding: "4px 8px",
          }}
        />
      </div>

      {/* Uniform parameter sliders */}
      {Object.keys(uniforms).length > 0 && (
        <div
          style={{
            background: "#342033",
            border: "1px solid #462d40",
            borderRadius: 6,
            padding: "10px 14px",
            marginBottom: 12,
          }}
        >
          <span style={{ fontSize: 11, fontWeight: 600, color: "#d9468f", display: "block", marginBottom: 8 }}>
            🎛️ Active Uniform Parameters:
          </span>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
            {Object.entries(uniforms).map(([key, val]) => (
              <div key={key}>
                <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, color: "#cbd5e0", marginBottom: 2 }}>
                  <code>{key}</code>
                  <span>{val.toFixed(2)}</span>
                </div>
                <input
                  type="range"
                  min="0.0"
                  max="1.5"
                  step="0.01"
                  value={val}
                  onChange={(e) => handleUniformChange(key, parseFloat(e.target.value))}
                  style={{ width: "100%" }}
                />
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Code Editor */}
      <div>
        <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, color: "#8b7284", marginBottom: 4 }}>
          <span>GLSL Fragment Shader Code (OpenGL ES / WebGL compatible):</span>
          <span>sampler2D u_texture, vec2 u_resolution, float u_time</span>
        </div>
        <textarea
          value={code}
          onChange={(e) => {
            setCode(e.target.value);
            setSelectedPreset("custom");
          }}
          rows={10}
          spellCheck={false}
          style={{
            width: "100%",
            background: "#110910",
            border: "1px solid #462d40",
            borderRadius: 4,
            color: "#68d391",
            fontFamily: "monospace",
            fontSize: 12,
            lineHeight: 1.5,
            padding: 10,
            boxSizing: "border-box",
            resize: "vertical",
          }}
        />
      </div>
    </div>
  );
};
