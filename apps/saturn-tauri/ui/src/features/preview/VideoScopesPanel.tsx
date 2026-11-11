// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
/**
 * VideoScopesPanel (AF6 / Packet 5.2):
 * Studio panel with raw <canvas> scopes (waveform, vectorscope, parade, histogram).
 * Strictly NO glass or backdrop blur over media/scope canvas surfaces.
 */
import React, { useState } from "react";
import { VideoScopes } from "../../components/VideoScopes";
import { InteriorButton } from "../../components/interior/interior-button";

export interface VideoScopesPanelProps {
  getActiveVideo?: () => HTMLVideoElement | null;
  activeFilter?: string;
  className?: string;
}

export const VideoScopesPanel: React.FC<VideoScopesPanelProps> = ({
  getActiveVideo = () => null,
  activeFilter = "none",
  className = "",
}) => {
  const [showAiBounds, setShowAiBounds] = useState<boolean>(true);
  const [targetPreset, setTargetPreset] = useState<string>("web-1080p");

  return (
    <div
      className={`video-scopes-panel ${className}`}
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100%",
        width: "100%",
        minHeight: 0,
        backgroundColor: "#050505", // Media/scope surface stays raw obsidian (no glass, no backdrop blur)
        color: "var(--text-primary)",
      }}
    >
      {/* Top Banner with AI Target Bounds controls */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "4px 8px",
          background: "var(--bg-subpanel, #120914)",
          borderBottom: "1px solid var(--border-control, rgba(255,255,255,0.08))",
          fontSize: "11px",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
          <span style={{ fontWeight: 600, color: "var(--text-muted)" }}>TARGET BOUNDS:</span>
          <select
            value={targetPreset}
            onChange={(e) => setTargetPreset(e.target.value)}
            style={{
              padding: "2px 6px",
              background: "var(--bg-app, #0a040b)",
              border: "1px solid var(--border-color, rgba(255,255,255,0.1))",
              color: "var(--text-primary)",
              borderRadius: "4px",
              fontSize: "10.5px",
            }}
          >
            <option value="web-1080p">Web 1080p (0-100 IRE / -14 LUFS)</option>
            <option value="vertical-9x16">Vertical 9:16 (Safe Action 90%)</option>
            <option value="broadcast-r128">Broadcast R128 (Narrow Gamut / -23 LUFS)</option>
          </select>
        </div>

        <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
          <InteriorButton
            variant="secondary"
            size="sm"
            onClick={() => setShowAiBounds((v) => !v)}
            title="Toggle AI Target IRE & Color Gamut Corridors"
            style={{
              height: "20px",
              padding: "2px 6px",
              fontSize: "10.5px",
              color: showAiBounds ? "var(--accent-text, #f472b6)" : "var(--text-muted)",
              borderColor: showAiBounds ? "var(--accent-text, #f472b6)" : undefined,
            }}
          >
            {showAiBounds ? "● AI Bounds Active" : "○ AI Bounds Off"}
          </InteriorButton>
        </div>
      </div>

      {/* Main Scopes Canvas Area - raw surface without glass */}
      <div style={{ flex: 1, minHeight: 0, position: "relative" }}>
        <VideoScopes
          getActiveVideo={getActiveVideo}
          activeFilter={activeFilter}
          className="scope-canvas video-scopes"
        />

        {/* AI Bounds overlay notes when active */}
        {showAiBounds && (
          <div
            style={{
              position: "absolute",
              top: "40px",
              right: "12px",
              pointerEvents: "none",
              fontSize: "10px",
              fontFamily: "monospace",
              color: "var(--accent-text-cyan, #38bdf8)",
              background: "rgba(0,0,0,0.75)",
              padding: "4px 8px",
              borderRadius: "4px",
              border: "1px solid rgba(56, 189, 248, 0.3)",
              display: "flex",
              flexDirection: "column",
              gap: "2px",
              zIndex: 10,
            }}
          >
            <span>Ceiling: 100 IRE (Clip guard)</span>
            <span>Floor: 0 IRE (Crush guard)</span>
            <span>Corridor: {targetPreset === "broadcast-r128" ? "Rec.709 Legal" : "sRGB Full"}</span>
          </div>
        )}
      </div>
    </div>
  );
};

export default VideoScopesPanel;
