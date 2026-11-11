// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import { InteriorButton } from "./interior/interior-button";
export interface StatusBarProps {
  busy: boolean;
  notice: string | null;
  connection: string;
  /** Active timeline tool, shown so the icon-only rail stays discoverable. */
  toolLabel?: string;
  /** Live trim/gesture detail shown beside the tool while dragging. */
  editDetail?: string;
  activeBranch?: string;
  activeMode?: string;
  lastActor?: { type: "human" | "agent"; id: string };
  onFetchGpuTelemetry: () => Promise<void>;
  onToggleGpuTelemetry: () => void;
}

export function StatusBar({
  busy,
  notice,
  connection,
  toolLabel,
  editDetail,
  activeBranch,
  activeMode,
  lastActor,
  onFetchGpuTelemetry,
  onToggleGpuTelemetry,
}: StatusBarProps) {
  return (
    <footer className="status-bar glass" data-elev="1">
      <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
        <span>{busy ? "Working…" : notice || "Autosave recovery enabled · saves every 30 seconds"}</span>
        {activeBranch && (
          <span style={{ color: "var(--accent-text, #f472b6)", fontWeight: 600 }}>
            🌿 {activeBranch}
          </span>
        )}
        {activeMode && (
          <span style={{ color: "var(--text-muted)" }}>
            Profile: {activeMode}
          </span>
        )}
        {lastActor && (
          <span style={{ color: "var(--text-muted)" }}>
            Actor: {lastActor.type === "agent" ? `🤖 ${lastActor.id}` : `👤 ${lastActor.id}`}
          </span>
        )}
      </div>
      <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
        {toolLabel ? <span className="status-bar-tool">Tool · {toolLabel}</span> : null}
        {editDetail ? <span className="status-bar-tool">{editDetail}</span> : null}
        <InteriorButton
          type="button"
          className="gpu-status-pill"
          onClick={() => {
            void onFetchGpuTelemetry();
            onToggleGpuTelemetry();
          }}
          title="Native wgpu GPU Preview Surface Telemetry"
        >
          <span className="gpu-status-dot" />
          <span>⚡ Native GPU Preview (60 FPS)</span>
        </InteriorButton>
        <span>{connection}</span>
      </div>
    </footer>
  );
}
