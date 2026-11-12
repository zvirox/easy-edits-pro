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
    <div className={`video-scopes-panel ${className}`}>
      {/* Top Banner with AI Target Bounds controls */}
      <div className="scopes-target-toolbar">
        <div className="scopes-target-select-group">
          <span className="scopes-target-label">TARGET BOUNDS:</span>
          <select
            className="scopes-target-select"
            value={targetPreset}
            onChange={(e) => setTargetPreset(e.target.value)}
          >
            <option value="web-1080p">Web 1080p (0-100 IRE / -14 LUFS)</option>
            <option value="vertical-9x16">Vertical 9:16 (Safe Action 90%)</option>
            <option value="broadcast-r128">Broadcast R128 (Narrow Gamut / -23 LUFS)</option>
          </select>
        </div>

        <div className="scopes-target-actions">
          <InteriorButton
            variant="secondary"
            size="sm"
            onClick={() => setShowAiBounds((v) => !v)}
            title="Toggle AI Target IRE & Color Gamut Corridors"
            className={`scopes-ai-bounds-toggle ${showAiBounds ? "is-active" : ""}`}
          >
            {showAiBounds ? "● AI Bounds Active" : "○ AI Bounds Off"}
          </InteriorButton>
        </div>
      </div>

      {/* Dedicated non-overlapping AI Bounds status row when active */}
      {showAiBounds && (
        <div className="scopes-ai-bounds-bar">
          <span>Ceiling: 100 IRE (Clip guard)</span>
          <span>Floor: 0 IRE (Crush guard)</span>
          <span>Corridor: {targetPreset === "broadcast-r128" ? "Rec.709 Legal" : "sRGB Full"}</span>
        </div>
      )}

      {/* Main Scopes Canvas Area - raw surface without glass */}
      <div className="scopes-body-stage">
        <VideoScopes
          getActiveVideo={getActiveVideo}
          activeFilter={activeFilter}
          className="scope-canvas video-scopes"
        />
      </div>
    </div>
  );
};

export default VideoScopesPanel;
