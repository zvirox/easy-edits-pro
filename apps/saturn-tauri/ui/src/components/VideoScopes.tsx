import React, { useEffect, useRef, useState, useCallback } from "react";

export type ScopeMode = "waveform" | "vectorscope" | "parade" | "histogram" | "quad";

export interface VideoScopesProps {
  getActiveVideo: () => HTMLVideoElement | null;
  activeFilter?: string;
  className?: string;
}

export const VideoScopes: React.FC<VideoScopesProps> = ({
  getActiveVideo,
  activeFilter = "none",
  className = "",
}) => {
  const [scopeMode, setScopeMode] = useState<ScopeMode>("waveform");
  const [gain, setGain] = useState<number>(1.2);
  const [showGraticule, setShowGraticule] = useState<boolean>(true);

  const canvasRef = useRef<HTMLCanvasElement>(null);
  const offscreenCanvasRef = useRef<HTMLCanvasElement | null>(null);
  const animFrameRef = useRef<number | null>(null);

  // Initialize offscreen sampling canvas (downsampled to 160x90 for fast 60fps sampling)
  useEffect(() => {
    if (!offscreenCanvasRef.current) {
      const c = document.createElement("canvas");
      c.width = 160;
      c.height = 90;
      offscreenCanvasRef.current = c;
    }
  }, []);

  // Main scope drawing function
  const drawScope = useCallback(() => {
    const canvas = canvasRef.current;
    const offscreen = offscreenCanvasRef.current;
    if (!canvas || !offscreen) return;

    const ctx = canvas.getContext("2d");
    const offCtx = offscreen.getContext("2d", { willReadFrequently: true });
    if (!ctx || !offCtx) return;

    const video = getActiveVideo();
    const width = canvas.width;
    const height = canvas.height;

    // Clear background to obsidian
    ctx.fillStyle = "#050505";
    ctx.fillRect(0, 0, width, height);

    // If no video or video not ready, draw clean standby graticule
    let sampleData: Uint8ClampedArray | null = null;
    if (video && video.readyState >= 1 && video.videoWidth > 0 && video.videoHeight > 0) {
      try {
        offCtx.filter = activeFilter && activeFilter !== "none" ? activeFilter : "none";
        offCtx.drawImage(video, 0, 0, offscreen.width, offscreen.height);
        const imgData = offCtx.getImageData(0, 0, offscreen.width, offscreen.height);
        sampleData = imgData.data;
      } catch {
        // Fallback if crossOrigin restriction or temporary read failure
        sampleData = null;
      }
    }

    if (scopeMode === "waveform") {
      drawWaveform(ctx, width, height, sampleData, offscreen.width, offscreen.height, gain, showGraticule);
    } else if (scopeMode === "vectorscope") {
      drawVectorscope(ctx, width, height, sampleData, offscreen.width, offscreen.height, gain, showGraticule);
    } else if (scopeMode === "parade") {
      drawRgbParade(ctx, width, height, sampleData, offscreen.width, offscreen.height, gain, showGraticule);
    } else if (scopeMode === "histogram") {
      drawHistogram(ctx, width, height, sampleData, offscreen.width, offscreen.height, showGraticule);
    } else if (scopeMode === "quad") {
      drawQuadScopes(ctx, width, height, sampleData, offscreen.width, offscreen.height, gain, showGraticule);
    }
  }, [getActiveVideo, activeFilter, scopeMode, gain, showGraticule]);

  // Continuous animation render loop
  useEffect(() => {
    let active = true;
    const loop = () => {
      if (!active) return;
      drawScope();
      animFrameRef.current = requestAnimationFrame(loop);
    };
    animFrameRef.current = requestAnimationFrame(loop);
    return () => {
      active = false;
      if (animFrameRef.current) cancelAnimationFrame(animFrameRef.current);
    };
  }, [drawScope]);

  return (
    <div className={`video-scopes-container ${className}`}>
      <header className="scopes-header">
        <div className="scopes-title-group">
          <strong>VIDEO SCOPES</strong>
          <span className="scopes-fps-badge">REAL-TIME · 60 FPS</span>
        </div>

        {/* Scope Mode Selector Tabs */}
        <div className="scope-tabs-bar" role="tablist">
          <button
            className={`scope-tab-btn ${scopeMode === "waveform" ? "active" : ""}`}
            onClick={() => setScopeMode("waveform")}
            title="Waveform (Luma Luminance 0-100 IRE)"
          >
            Waveform
          </button>
          <button
            className={`scope-tab-btn ${scopeMode === "vectorscope" ? "active" : ""}`}
            onClick={() => setScopeMode("vectorscope")}
            title="Vectorscope (Chrominance Hue & Saturation + Skin-Tone Line)"
          >
            Vectorscope
          </button>
          <button
            className={`scope-tab-btn ${scopeMode === "parade" ? "active" : ""}`}
            onClick={() => setScopeMode("parade")}
            title="RGB Parade (Red, Green, Blue Channels Side-by-Side)"
          >
            RGB Parade
          </button>
          <button
            className={`scope-tab-btn ${scopeMode === "histogram" ? "active" : ""}`}
            onClick={() => setScopeMode("histogram")}
            title="Histogram (Color Tonal Frequency Distribution)"
          >
            Histogram
          </button>
          <button
            className={`scope-tab-btn quad-btn ${scopeMode === "quad" ? "active" : ""}`}
            onClick={() => setScopeMode("quad")}
            title="Quad View (All 4 scopes simultaneously)"
          >
            Quad
          </button>
        </div>

        {/* Scope Controls */}
        <div className="scopes-controls-bar">
          <label className="scope-gain-control" title="Trace brightness / gain">
            <span>Gain</span>
            <input
              type="range"
              min="0.5"
              max="2.5"
              step="0.1"
              value={gain}
              onChange={(e) => setGain(Number(e.target.value))}
            />
          </label>
          <button
            className={`scope-toggle-btn ${showGraticule ? "active" : ""}`}
            onClick={() => setShowGraticule((prev) => !prev)}
            title="Toggle IRE and chromaticity reference graticule"
          >
            Graticule
          </button>
        </div>
      </header>

      <div className="scopes-canvas-stage">
        <canvas
          ref={canvasRef}
          width={scopeMode === "quad" ? 640 : 420}
          height={scopeMode === "quad" ? 360 : 240}
          className="scopes-canvas"
        />
      </div>
    </div>
  );
};

/* ==========================================================================
   Scope Renderers: Waveform, Vectorscope, RGB Parade, Histogram
   ========================================================================== */

function drawWaveform(
  ctx: CanvasRenderingContext2D,
  w: number,
  h: number,
  data: Uint8ClampedArray | null,
  sw: number,
  sh: number,
  gain: number,
  graticule: boolean,
  xOffset = 0,
  yOffset = 0,
  targetWidth = w,
  targetHeight = h
) {
  const topPad = 14;
  const btmPad = 16;
  const graphH = targetHeight - topPad - btmPad;

  // Graticule IRE reference lines
  if (graticule) {
    ctx.save();
    ctx.font = "8.5px monospace";
    ctx.textAlign = "right";
    const ires = [
      { ire: 100, label: "100" },
      { ire: 80, label: "80" },
      { ire: 60, label: "60" },
      { ire: 40, label: "40" },
      { ire: 20, label: "20" },
      { ire: 0, label: "0" },
    ];
    for (const item of ires) {
      const y = yOffset + topPad + graphH * (1 - item.ire / 100);
      ctx.strokeStyle = item.ire === 100 || item.ire === 0 ? "rgba(100, 116, 139, 0.45)" : "rgba(71, 85, 105, 0.22)";
      ctx.lineWidth = 1;
      ctx.setLineDash(item.ire === 100 || item.ire === 0 ? [] : [3, 4]);
      ctx.beginPath();
      ctx.moveTo(xOffset + 26, y);
      ctx.lineTo(xOffset + targetWidth - 6, y);
      ctx.stroke();

      ctx.fillStyle = item.ire === 100 ? "#38bdf8" : item.ire === 0 ? "#94a3b8" : "rgba(148, 163, 184, 0.6)";
      ctx.fillText(item.label, xOffset + 22, y + 3);
    }
    ctx.setLineDash([]);
    ctx.restore();
  }

  // Draw Waveform Traces
  if (data) {
    ctx.save();
    const pointW = (targetWidth - 36) / sw;
    const baseAlpha = Math.min(0.85, 0.14 * gain);
    ctx.fillStyle = `rgba(56, 189, 248, ${baseAlpha.toFixed(3)})`;

    for (let sy = 0; sy < sh; sy += 2) {
      for (let sx = 0; sx < sw; sx += 1) {
        const idx = (sy * sw + sx) * 4;
        const r = data[idx];
        const g = data[idx + 1];
        const b = data[idx + 2];
        // Rec.709 Luma calculation
        const luma = (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255;
        const px = xOffset + 28 + sx * pointW;
        const py = yOffset + topPad + graphH * (1 - luma);
        ctx.fillRect(px, py, 1.2, 1.2);
      }
    }
    ctx.restore();
  } else {
    // Standby indicator
    ctx.fillStyle = "rgba(148, 163, 184, 0.4)";
    ctx.font = "10px sans-serif";
    ctx.textAlign = "center";
    ctx.fillText("STANDBY · NO VIDEO FRAME", xOffset + targetWidth / 2, yOffset + targetHeight / 2);
  }
}

function drawVectorscope(
  ctx: CanvasRenderingContext2D,
  w: number,
  h: number,
  data: Uint8ClampedArray | null,
  _sw: number,
  _sh: number,
  gain: number,
  graticule: boolean,
  xOffset = 0,
  yOffset = 0,
  targetWidth = w,
  targetHeight = h
) {
  const cx = xOffset + targetWidth / 2;
  const cy = yOffset + targetHeight / 2;
  const radius = Math.min(targetWidth, targetHeight) / 2 - 18;

  // Graticule: 75% and 100% Saturation Rings & Crosshairs
  if (graticule) {
    ctx.save();
    // Center axes
    ctx.strokeStyle = "rgba(71, 85, 105, 0.35)";
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(cx - radius - 6, cy);
    ctx.lineTo(cx + radius + 6, cy);
    ctx.moveTo(cx, cy - radius - 6);
    ctx.lineTo(cx, cy + radius + 6);
    ctx.stroke();

    // 100% and 75% saturation circles
    ctx.strokeStyle = "rgba(100, 116, 139, 0.4)";
    ctx.beginPath();
    ctx.arc(cx, cy, radius, 0, Math.PI * 2);
    ctx.stroke();

    ctx.strokeStyle = "rgba(71, 85, 105, 0.3)";
    ctx.setLineDash([3, 4]);
    ctx.beginPath();
    ctx.arc(cx, cy, radius * 0.75, 0, Math.PI * 2);
    ctx.stroke();
    ctx.setLineDash([]);

    // Skin Tone Line (I-axis at ~57 degrees, 10:30 o'clock)
    const skinAngle = (57 * Math.PI) / 180;
    const skinX = cx - Math.sin(skinAngle) * radius * 1.1;
    const skinY = cy - Math.cos(skinAngle) * radius * 1.1;
    ctx.strokeStyle = "rgba(251, 146, 60, 0.45)";
    ctx.lineWidth = 1.2;
    ctx.beginPath();
    ctx.moveTo(cx, cy);
    ctx.lineTo(skinX, skinY);
    ctx.stroke();

    ctx.fillStyle = "#fb923c";
    ctx.font = "8px sans-serif";
    ctx.fillText("SKIN", skinX - 10, skinY - 4);

    // Standard Color Target Boxes: R, Mg, B, Cy, G, Yl at 75% saturation
    const targets = [
      { name: "R", angle: 103, color: "#ef4444" },
      { name: "Mg", angle: 61, color: "#ec4899" },
      { name: "B", angle: 347, color: "#3b82f6" },
      { name: "Cy", angle: 283, color: "#06b6d4" },
      { name: "G", angle: 241, color: "#22c55e" },
      { name: "Yl", angle: 167, color: "#eab308" },
    ];

    for (const t of targets) {
      const rad = ((t.angle - 90) * Math.PI) / 180;
      const tx = cx + Math.cos(rad) * radius * 0.75;
      const ty = cy + Math.sin(rad) * radius * 0.75;

      ctx.strokeStyle = t.color;
      ctx.strokeRect(tx - 3.5, ty - 3.5, 7, 7);
      ctx.fillStyle = t.color;
      ctx.font = "7.5px monospace";
      ctx.fillText(t.name, tx + 6, ty + 3);
    }
    ctx.restore();
  }

  // Draw Chrominance Cloud
  if (data) {
    ctx.save();
    const baseAlpha = Math.min(0.85, 0.16 * gain);
    ctx.fillStyle = `rgba(245, 158, 11, ${baseAlpha.toFixed(3)})`;

    for (let i = 0; i < data.length; i += 8) {
      const r = data[i] / 255;
      const g = data[i + 1] / 255;
      const b = data[i + 2] / 255;

      // Rec.709 Cb & Cr difference coordinates
      const cb = -0.1146 * r - 0.3854 * g + 0.5000 * b;
      const cr = 0.5000 * r - 0.4542 * g - 0.0458 * b;

      const px = cx + cb * (radius / 0.55);
      const py = cy - cr * (radius / 0.55);

      ctx.fillRect(px, py, 1.2, 1.2);
    }
    ctx.restore();
  } else {
    ctx.fillStyle = "rgba(148, 163, 184, 0.4)";
    ctx.font = "10px sans-serif";
    ctx.textAlign = "center";
    ctx.fillText("STANDBY", cx, cy);
  }
}

function drawRgbParade(
  ctx: CanvasRenderingContext2D,
  w: number,
  h: number,
  data: Uint8ClampedArray | null,
  sw: number,
  sh: number,
  gain: number,
  graticule: boolean,
  xOffset = 0,
  yOffset = 0,
  targetWidth = w,
  targetHeight = h
) {
  const topPad = 16;
  const btmPad = 16;
  const graphH = targetHeight - topPad - btmPad;
  const colW = (targetWidth - 20) / 3;

  const channels = [
    { label: "RED", color: "rgba(239, 68, 68, ", hex: "#ef4444", colIdx: 0 },
    { label: "GREEN", color: "rgba(34, 197, 94, ", hex: "#22c55e", colIdx: 1 },
    { label: "BLUE", color: "rgba(59, 130, 246, ", hex: "#3b82f6", colIdx: 2 },
  ];

  ctx.save();
  for (const ch of channels) {
    const startX = xOffset + 10 + ch.colIdx * colW;

    // Graticule
    if (graticule) {
      ctx.strokeStyle = "rgba(71, 85, 105, 0.25)";
      ctx.setLineDash([2, 3]);
      ctx.beginPath();
      // 100% and 0% lines
      ctx.moveTo(startX, yOffset + topPad);
      ctx.lineTo(startX + colW - 6, yOffset + topPad);
      ctx.moveTo(startX, yOffset + topPad + graphH);
      ctx.lineTo(startX + colW - 6, yOffset + topPad + graphH);
      ctx.stroke();
      ctx.setLineDash([]);

      ctx.fillStyle = ch.hex;
      ctx.font = "8.5px monospace";
      ctx.fillText(ch.label, startX + 4, yOffset + 12);
    }

    // Trace
    if (data) {
      const alpha = Math.min(0.85, 0.15 * gain);
      ctx.fillStyle = `${ch.color}${alpha.toFixed(3)})`;
      const stepX = (colW - 8) / sw;

      for (let sy = 0; sy < sh; sy += 2) {
        for (let sx = 0; sx < sw; sx += 1) {
          const idx = (sy * sw + sx) * 4;
          const val = data[idx + ch.colIdx] / 255;
          const px = startX + sx * stepX;
          const py = yOffset + topPad + graphH * (1 - val);
          ctx.fillRect(px, py, 1.2, 1.2);
        }
      }
    }
  }
  ctx.restore();
}

function drawHistogram(
  ctx: CanvasRenderingContext2D,
  w: number,
  h: number,
  data: Uint8ClampedArray | null,
  _sw: number,
  _sh: number,
  graticule: boolean,
  xOffset = 0,
  yOffset = 0,
  targetWidth = w,
  targetHeight = h
) {
  const topPad = 16;
  const btmPad = 16;
  const graphH = targetHeight - topPad - btmPad;
  const graphW = targetWidth - 28;

  const binsR = new Uint32Array(256);
  const binsG = new Uint32Array(256);
  const binsB = new Uint32Array(256);
  const binsY = new Uint32Array(256);

  if (data) {
    for (let i = 0; i < data.length; i += 4) {
      const r = data[i];
      const g = data[i + 1];
      const b = data[i + 2];
      const luma = Math.round(0.2126 * r + 0.7152 * g + 0.0722 * b);

      binsR[r]++;
      binsG[g]++;
      binsB[b]++;
      binsY[luma]++;
    }
  }

  let maxCount = 1;
  for (let i = 0; i < 256; i++) {
    if (binsY[i] > maxCount) maxCount = binsY[i];
    if (binsR[i] > maxCount) maxCount = binsR[i];
    if (binsG[i] > maxCount) maxCount = binsG[i];
    if (binsB[i] > maxCount) maxCount = binsB[i];
  }

  // Graticule
  if (graticule) {
    ctx.save();
    ctx.strokeStyle = "rgba(71, 85, 105, 0.25)";
    ctx.setLineDash([2, 3]);
    const ticks = [0, 64, 128, 192, 255];
    ctx.font = "8px monospace";
    ctx.fillStyle = "rgba(148, 163, 184, 0.5)";

    for (const t of ticks) {
      const x = xOffset + 14 + (t / 255) * graphW;
      ctx.beginPath();
      ctx.moveTo(x, yOffset + topPad);
      ctx.lineTo(x, yOffset + topPad + graphH);
      ctx.stroke();
      ctx.fillText(String(t), x - 6, yOffset + targetHeight - 4);
    }
    ctx.setLineDash([]);
    ctx.restore();
  }

  // Draw Channels
  const drawCurve = (bins: Uint32Array, color: string, fill: boolean) => {
    ctx.save();
    ctx.strokeStyle = color;
    ctx.lineWidth = 1.2;
    ctx.beginPath();
    for (let i = 0; i < 256; i++) {
      const x = xOffset + 14 + (i / 255) * graphW;
      const val = bins[i] / maxCount;
      const y = yOffset + topPad + graphH * (1 - Math.min(1, val * 1.3));
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    }
    if (fill) {
      ctx.lineTo(xOffset + 14 + graphW, yOffset + topPad + graphH);
      ctx.lineTo(xOffset + 14, yOffset + topPad + graphH);
      ctx.closePath();
      ctx.fillStyle = "rgba(148, 163, 184, 0.12)";
      ctx.fill();
    }
    ctx.stroke();
    ctx.restore();
  };

  if (data) {
    drawCurve(binsY, "rgba(226, 232, 240, 0.8)", true);
    drawCurve(binsR, "#ef4444", false);
    drawCurve(binsG, "#22c55e", false);
    drawCurve(binsB, "#3b82f6", false);
  }
}

function drawQuadScopes(
  ctx: CanvasRenderingContext2D,
  w: number,
  h: number,
  data: Uint8ClampedArray | null,
  sw: number,
  sh: number,
  gain: number,
  graticule: boolean
) {
  const halfW = w / 2;
  const halfH = h / 2;

  // Grid dividers
  ctx.strokeStyle = "rgba(100, 116, 139, 0.3)";
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(halfW, 0);
  ctx.lineTo(halfW, h);
  ctx.moveTo(0, halfH);
  ctx.lineTo(w, halfH);
  ctx.stroke();

  // Top-Left: Waveform
  drawWaveform(ctx, w, h, data, sw, sh, gain, graticule, 0, 0, halfW, halfH);
  // Top-Right: Vectorscope
  drawVectorscope(ctx, w, h, data, sw, sh, gain, graticule, halfW, 0, halfW, halfH);
  // Bottom-Left: RGB Parade
  drawRgbParade(ctx, w, h, data, sw, sh, gain, graticule, 0, halfH, halfW, halfH);
  // Bottom-Right: Histogram
  drawHistogram(ctx, w, h, data, sw, sh, graticule, halfW, halfH, halfW, halfH);
}

export default VideoScopes;
