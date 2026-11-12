// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import { useSyncExternalStore } from "react";
import { getAgentStoreSnapshot, subscribeAgentStore } from "../features/agent/agentSessionStore";
import { PANEL_OPEN_EVENT } from "../features/dock/panelEvents";
import { InteriorButton } from "./interior/interior-button";

export interface StatusBarSheetSummary {
  id: string;
  title: string;
  status: string;
  spentMin?: number;
  budgetMin?: number;
  spentUsd?: number;
  budgetUsd?: number;
}

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
  sheetSummary?: StatusBarSheetSummary;
  onFetchGpuTelemetry: () => Promise<void>;
  onToggleGpuTelemetry: () => void;
  onOpenSheet?: () => void;
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
  sheetSummary,
  onFetchGpuTelemetry,
  onToggleGpuTelemetry,
  onOpenSheet,
}: StatusBarProps) {
  const { session } = useSyncExternalStore(subscribeAgentStore, getAgentStoreSnapshot);
  const sheet = session.taskSheet;
  const summary: StatusBarSheetSummary | null =
    sheetSummary ??
    (sheet
      ? {
          id: sheet.id,
          title: sheet.title,
          status: sheet.status,
          spentMin: Math.round((session.budgetSpent.wall_clock_seconds ?? 0) / 60),
          budgetMin: sheet.budget?.wall_clock_min ?? 60,
          spentUsd: session.budgetSpent.model_cost_usd ?? 0,
          budgetUsd: sheet.budget?.model_cost_usd ?? 5,
        }
      : null);
  const resolvedBranch =
    activeBranch ??
    (session.activeBranch && session.activeBranch !== "main" ? session.activeBranch : undefined);
  const resolvedMode = activeMode ?? (session.activeSessionId ? session.activeMode : undefined);
  const resolvedActor = lastActor ?? session.lastActor ?? undefined;

  const handleOpenSheet = () => {
    if (onOpenSheet) {
      onOpenSheet();
    } else {
      window.dispatchEvent(new CustomEvent(PANEL_OPEN_EVENT, { detail: { id: "sheet" } }));
    }
  };

  return (
    <footer className="status-bar glass" data-elev="1">
      <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
        <span>{busy ? "Working…" : notice || "Autosave recovery enabled · saves every 30 seconds"}</span>
        {resolvedBranch && (
          <span style={{ color: "var(--accent-text, #f472b6)", fontWeight: 600 }}>
            🌿 {resolvedBranch}
          </span>
        )}
        {resolvedMode && (
          <span style={{ color: "var(--text-muted)" }}>
            Profile: {resolvedMode}
          </span>
        )}
        {resolvedActor && (
          <span style={{ color: "var(--text-muted)" }}>
            Actor: {resolvedActor.type === "agent" ? `🤖 ${resolvedActor.id}` : `👤 ${resolvedActor.id}`}
          </span>
        )}
        {summary && (
          <span
            className="status-bar-sheet"
            onClick={handleOpenSheet}
            style={{
              cursor: "pointer",
              display: "inline-flex",
              alignItems: "center",
              gap: "6px",
              padding: "1px 6px",
              borderRadius: "var(--radius-xs, 4px)",
              background: "var(--card-inset, rgba(255,255,255,0.05))",
            }}
            title={`Active Task Sheet: ${summary.title} (Click to open)`}
          >
            <span>📋 {summary.id}</span>
            <span
              style={{
                fontSize: "10px",
                fontWeight: 600,
                padding: "1px 4px",
                borderRadius: "3px",
                textTransform: "uppercase",
                background: "var(--surface-sunken)",
                color:
                  summary.status === "ready" || summary.status === "running"
                    ? "var(--success-text, #10b981)"
                    : summary.status === "needs_input"
                    ? "var(--warn-text, #f59e0b)"
                    : "var(--text-muted)",
              }}
            >
              {summary.status}
            </span>
            <span style={{ fontSize: "11px", color: "var(--text-muted)" }}>
              ⏱ {summary.spentMin ?? 0}/{summary.budgetMin ?? 60}m · 💲 ${(summary.spentUsd ?? 0).toFixed(2)}/${(summary.budgetUsd ?? 5).toFixed(2)}
            </span>
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
