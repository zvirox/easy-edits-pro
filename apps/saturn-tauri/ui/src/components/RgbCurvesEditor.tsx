import React, { useState, useRef, useEffect, useCallback } from "react";
import type { CurvePoint, RgbCurves } from "../engine/playbackEngine";

interface RgbCurvesEditorProps {
  curves: RgbCurves;
  onChange: (newCurves: RgbCurves) => void;
}

type Channel = "master" | "red" | "green" | "blue";

export const RgbCurvesEditor: React.FC<RgbCurvesEditorProps> = ({ curves, onChange }) => {
  const [activeChannel, setActiveChannel] = useState<Channel>("master");
  const [draggingIndex, setDraggingIndex] = useState<number | null>(null);
  const svgRef = useRef<SVGSVGElement | null>(null);

  const channelColor = {
    master: "#f8fafc",
    red: "#ef4444",
    green: "#22c55e",
    blue: "#3b82f6",
  }[activeChannel];

  const currentPoints = curves[activeChannel] || [
    { x: 0.0, y: 0.0 },
    { x: 1.0, y: 1.0 },
  ];

  // Helper to generate SVG cubic path
  const getSplinePath = useCallback((points: CurvePoint[]): string => {
    if (!points || points.length === 0) return "";
    const sorted = [...points].sort((a, b) => a.x - b.x);
    if (sorted.length === 1) {
      const y = (1 - sorted[0].y) * 200;
      return `M 0,${y} L 200,${y}`;
    }

    let path = `M ${(sorted[0].x * 200).toFixed(1)},${((1 - sorted[0].y) * 200).toFixed(1)}`;
    for (let i = 0; i < sorted.length - 1; i++) {
      const p0 = i > 0 ? sorted[i - 1] : sorted[i];
      const p1 = sorted[i];
      const p2 = sorted[i + 1];
      const p3 = i + 2 < sorted.length ? sorted[i + 2] : p2;

      // Control points for cubic bezier approximating Hermite
      const cp1x = (p1.x + (p2.x - p0.x) / 6) * 200;
      const cp1y = (1 - (p1.y + (p2.y - p0.y) / 6)) * 200;
      const cp2x = (p2.x - (p3.x - p1.x) / 6) * 200;
      const cp2y = (1 - (p2.y - (p3.y - p1.y) / 6)) * 200;
      const endX = p2.x * 200;
      const endY = (1 - p2.y) * 200;

      path += ` C ${cp1x.toFixed(1)},${cp1y.toFixed(1)} ${cp2x.toFixed(1)},${cp2y.toFixed(1)} ${endX.toFixed(1)},${endY.toFixed(1)}`;
    }
    return path;
  }, []);

  const handleMouseDownPoint = (index: number, e: React.MouseEvent) => {
    e.stopPropagation();
    setDraggingIndex(index);
  };

  const handleDoubleClickPoint = (index: number, e: React.MouseEvent) => {
    e.stopPropagation();
    // Do not delete endpoints
    if (index === 0 || index === currentPoints.length - 1) return;
    const nextPoints = currentPoints.filter((_, i) => i !== index);
    const updatedCurves = {
      ...curves,
      [activeChannel]: nextPoints,
    };
    onChange(updatedCurves);
  };

  const handleSvgClick = (e: React.MouseEvent) => {
    if (!svgRef.current) return;
    const rect = svgRef.current.getBoundingClientRect();
    const x = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    const y = Math.max(0, Math.min(1, 1 - (e.clientY - rect.top) / rect.height));

    // Avoid duplicate point very close to existing
    const threshold = 0.04;
    const exists = currentPoints.some((p) => Math.hypot(p.x - x, p.y - y) < threshold);
    if (exists) return;

    const nextPoints = [...currentPoints, { x, y }].sort((a, b) => a.x - b.x);
    onChange({
      ...curves,
      [activeChannel]: nextPoints,
    });
  };

  const handleMouseMove = useCallback(
    (e: MouseEvent) => {
      if (draggingIndex === null || !svgRef.current) return;
      const rect = svgRef.current.getBoundingClientRect();
      const x = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
      const y = Math.max(0, Math.min(1, 1 - (e.clientY - rect.top) / rect.height));

      const nextPoints = [...currentPoints];
      // Keep first point pinned at x=0 and last at x=1, but allow y movement
      if (draggingIndex === 0) {
        nextPoints[0] = { x: 0, y };
      } else if (draggingIndex === currentPoints.length - 1) {
        nextPoints[currentPoints.length - 1] = { x: 1, y };
      } else {
        nextPoints[draggingIndex] = { x, y };
      }

      nextPoints.sort((a, b) => a.x - b.x);
      onChange({
        ...curves,
        [activeChannel]: nextPoints,
      });
    },
    [draggingIndex, currentPoints, activeChannel, curves, onChange]
  );

  const handleMouseUp = useCallback(() => {
    setDraggingIndex(null);
  }, []);

  useEffect(() => {
    if (draggingIndex !== null) {
      window.addEventListener("mousemove", handleMouseMove);
      window.addEventListener("mouseup", handleMouseUp);
      return () => {
        window.removeEventListener("mousemove", handleMouseMove);
        window.removeEventListener("mouseup", handleMouseUp);
      };
    }
  }, [draggingIndex, handleMouseMove, handleMouseUp]);

  // Curve Presets
  const applyPreset = (preset: "linear" | "s_curve" | "high_contrast" | "lift_shadows" | "crush_blacks") => {
    let newPoints: CurvePoint[];
    switch (preset) {
      case "linear":
        newPoints = [
          { x: 0.0, y: 0.0 },
          { x: 1.0, y: 1.0 },
        ];
        break;
      case "s_curve":
        newPoints = [
          { x: 0.0, y: 0.0 },
          { x: 0.25, y: 0.18 },
          { x: 0.75, y: 0.82 },
          { x: 1.0, y: 1.0 },
        ];
        break;
      case "high_contrast":
        newPoints = [
          { x: 0.0, y: 0.0 },
          { x: 0.3, y: 0.15 },
          { x: 0.7, y: 0.88 },
          { x: 1.0, y: 1.0 },
        ];
        break;
      case "lift_shadows":
        newPoints = [
          { x: 0.0, y: 0.12 },
          { x: 0.35, y: 0.45 },
          { x: 1.0, y: 1.0 },
        ];
        break;
      case "crush_blacks":
        newPoints = [
          { x: 0.0, y: 0.0 },
          { x: 0.2, y: 0.05 },
          { x: 0.8, y: 0.85 },
          { x: 1.0, y: 1.0 },
        ];
        break;
    }

    onChange({
      ...curves,
      [activeChannel]: newPoints,
    });
  };

  return (
    <div
      className="rgb-curves-editor"
      style={{
        background: "#13171f",
        border: "1px solid #2d3748",
        borderRadius: 8,
        padding: 14,
        color: "#e2e8f0",
      }}
    >
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
        <div style={{ display: "flex", gap: 6 }}>
          {(["master", "red", "green", "blue"] as Channel[]).map((ch) => (
            <button
              key={ch}
              type="button"
              onClick={() => setActiveChannel(ch)}
              style={{
                padding: "4px 10px",
                fontSize: 11,
                fontWeight: 600,
                textTransform: "uppercase",
                borderRadius: 4,
                border: "none",
                cursor: "pointer",
                background: activeChannel === ch ? "#2d3748" : "#1a202c",
                color:
                  ch === "master"
                    ? "#ffffff"
                    : ch === "red"
                    ? "#ef4444"
                    : ch === "green"
                    ? "#22c55e"
                    : "#3b82f6",
                boxShadow: activeChannel === ch ? "0 0 0 1px #4a5568" : "none",
              }}
            >
              {ch}
            </button>
          ))}
        </div>
        <span style={{ fontSize: 11, color: "#718096" }}>
          Click grid to add • Drag to move • Double-click to remove
        </span>
      </div>

      <div style={{ position: "relative", width: 200, height: 200, margin: "0 auto", background: "#090d14", borderRadius: 4, border: "1px solid #1a202c" }}>
        <svg
          ref={svgRef}
          width="200"
          height="200"
          viewBox="0 0 200 200"
          onClick={handleSvgClick}
          style={{ width: "100%", height: "100%", cursor: "crosshair", overflow: "visible" }}
        >
          {/* Grid lines */}
          <line x1="50" y1="0" x2="50" y2="200" stroke="#1f2937" strokeDasharray="2,2" />
          <line x1="100" y1="0" x2="100" y2="200" stroke="#374151" strokeDasharray="2,2" />
          <line x1="150" y1="0" x2="150" y2="200" stroke="#1f2937" strokeDasharray="2,2" />
          <line x1="0" y1="50" x2="200" y2="50" stroke="#1f2937" strokeDasharray="2,2" />
          <line x1="0" y1="100" x2="200" y2="100" stroke="#374151" strokeDasharray="2,2" />
          <line x1="0" y1="150" x2="200" y2="150" stroke="#1f2937" strokeDasharray="2,2" />
          <line x1="0" y1="200" x2="200" y2="0" stroke="#232b38" strokeWidth="1" />

          {/* Other inactive channels faint outlines */}
          {(["master", "red", "green", "blue"] as Channel[])
            .filter((ch) => ch !== activeChannel && curves[ch])
            .map((ch) => (
              <path
                key={ch}
                d={getSplinePath(curves[ch])}
                fill="none"
                stroke={ch === "master" ? "#4b5563" : ch === "red" ? "#7f1d1d" : ch === "green" ? "#14532d" : "#1e3a8a"}
                strokeWidth="1.2"
                strokeOpacity="0.4"
              />
            ))}

          {/* Active channel spline */}
          <path
            d={getSplinePath(currentPoints)}
            fill="none"
            stroke={channelColor}
            strokeWidth="2.5"
            strokeLinecap="round"
          />

          {/* Interactive control points */}
          {currentPoints.map((p, idx) => (
            <circle
              key={idx}
              cx={(p.x * 200).toFixed(1)}
              cy={((1 - p.y) * 200).toFixed(1)}
              r={draggingIndex === idx ? 6 : 4.5}
              fill={channelColor}
              stroke="#090d14"
              strokeWidth="2"
              style={{ cursor: "grab" }}
              onMouseDown={(e) => handleMouseDownPoint(idx, e)}
              onDoubleClick={(e) => handleDoubleClickPoint(idx, e)}
            />
          ))}
        </svg>
      </div>

      {/* Preset Action Chips */}
      <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginTop: 12, justifyContent: "center" }}>
        <button
          type="button"
          onClick={() => applyPreset("linear")}
          style={{ padding: "4px 8px", fontSize: 11, background: "#1a202c", border: "1px solid #2d3748", borderRadius: 4, color: "#a0aec0", cursor: "pointer" }}
        >
          Linear Reset
        </button>
        <button
          type="button"
          onClick={() => applyPreset("s_curve")}
          style={{ padding: "4px 8px", fontSize: 11, background: "#1a202c", border: "1px solid #2d3748", borderRadius: 4, color: "#cbd5e0", cursor: "pointer" }}
        >
          S-Curve (Contrast)
        </button>
        <button
          type="button"
          onClick={() => applyPreset("high_contrast")}
          style={{ padding: "4px 8px", fontSize: 11, background: "#1a202c", border: "1px solid #2d3748", borderRadius: 4, color: "#cbd5e0", cursor: "pointer" }}
        >
          High Punch
        </button>
        <button
          type="button"
          onClick={() => applyPreset("lift_shadows")}
          style={{ padding: "4px 8px", fontSize: 11, background: "#1a202c", border: "1px solid #2d3748", borderRadius: 4, color: "#cbd5e0", cursor: "pointer" }}
        >
          Lift Shadows
        </button>
        <button
          type="button"
          onClick={() => applyPreset("crush_blacks")}
          style={{ padding: "4px 8px", fontSize: 11, background: "#1a202c", border: "1px solid #2d3748", borderRadius: 4, color: "#cbd5e0", cursor: "pointer" }}
        >
          Crush Blacks
        </button>
      </div>
    </div>
  );
};
