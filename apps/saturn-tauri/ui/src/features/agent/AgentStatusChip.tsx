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
  const isStopped = session.isStopped || session.status === "stopped";
  const isWorking = session.status === "thinking" || session.status === "editing";

  const handleOpenAgent = () => {
    window.dispatchEvent(new CustomEvent(PANEL_OPEN_EVENT, { detail: { id: "agent" } }));
  };

  const statusColor = isStopped
    ? "var(--warn-text, #f59e0b)"
    : isWorking
    ? "var(--accent-text, #f472b6)"
    : session.status === "waiting"
    ? "var(--accent-text-violet, #c084fc)"
    : "var(--text-muted, #9ca3af)";

  return (
    <div
      className="agent-status-chip glass"
      data-elev="2"
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
            boxShadow: isWorking ? "0 0 8px var(--accent-text)" : undefined,
            transition: "background-color var(--dur-fast, 120ms) var(--ease-out)",
          }}
        />
        <span style={{ fontWeight: 600 }}>AI Agent</span>
        <span style={{ color: "var(--text-muted, #9ca3af)", fontSize: "11px" }}>
          {isStopped
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
