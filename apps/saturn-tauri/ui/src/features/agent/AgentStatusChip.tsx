// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import React from "react";
import { InteriorButton } from "../../components/interior/interior-button";
import { PANEL_OPEN_EVENT } from "../dock/panelEvents";
import type { AgentSessionState } from "./types";

export interface AgentStatusChipProps {
  session: AgentSessionState;
  onStop: () => void;
  onResume?: () => void;
}

export const AgentStatusChip: React.FC<AgentStatusChipProps> = ({ session, onStop, onResume }) => {
  const hasActiveSession = Boolean(session.activeSessionId);
  const isStopped = hasActiveSession && (session.isStopped || session.status === "stopped");
  const isWorking = hasActiveSession && (session.status === "thinking" || session.status === "editing");

  const handleOpenAgent = () => {
    window.dispatchEvent(new CustomEvent(PANEL_OPEN_EVENT, { detail: { id: "agent" } }));
  };

  const statusColor = isStopped
    ? "var(--amber)"
    : isWorking
    ? "var(--primary)"
    : session.status === "waiting"
    ? "var(--purple)"
    : "var(--text-faint)";

  return (
    <div
      className="agent-status-chip surface-1"
      role="region"
      aria-label="Agent Session Status"
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: "6px",
        padding: "3px 8px",
        borderRadius: "var(--radius-sm, 10px)",
        fontSize: "12px",
        color: "var(--text-primary)",
        whiteSpace: "nowrap",
      }}
    >
      <button
        type="button"
        onClick={handleOpenAgent}
        title="Click to view Agent details and plan"
        style={{
          display: "inline-flex",
          alignItems: "center",
          gap: "6px",
          background: "transparent",
          border: "none",
          color: "inherit",
          cursor: "pointer",
          padding: 0,
        }}
      >
        <span
          style={{
            width: "8px",
            height: "8px",
            borderRadius: "50%",
            backgroundColor: statusColor,
            transition: "background-color var(--dur-fast) var(--ease-out)",
          }}
        />
        <span className="agent-chip-title" style={{ fontWeight: 600 }}>AI Agent</span>
        <span className="agent-chip-status" style={{ color: "var(--text-muted)", fontSize: "11px" }}>
          {!hasActiveSession
            ? "No session"
            : isStopped
            ? "Stopped"
            : isWorking
            ? `Step ${session.stepCurrent}/${session.stepTotal}`
            : session.status === "waiting"
            ? "Waiting"
            : "Idle"}
        </span>
      </button>

      {isStopped ? (
        <InteriorButton
          variant="secondary"
          size="sm"
          onClick={onResume}
          disabled={!hasActiveSession}
          title="Resume agent task execution"
          style={{ padding: "2px 8px", height: "22px", fontSize: "11px" }}
        >
          Resume
        </InteriorButton>
      ) : (
        <InteriorButton
          variant="secondary"
          size="sm"
          onClick={onStop}
          disabled={!hasActiveSession}
          title="Immediate Stop: halt agent session (≤ 1s, no late apply)"
          style={{
            padding: "2px 8px",
            height: "22px",
            fontSize: "11px",
            color: isWorking ? "var(--accent-text)" : undefined,
          }}
        >
          Stop
        </InteriorButton>
      )}
    </div>
  );
};
