// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import React, { useState } from "react";
import { InteriorButton } from "../../components/interior/interior-button";
import type { ReviewChange } from "../agent/types";
import { TICKS_PER_SECOND } from "./types";

export interface AiBranchDiffRibbonProps {
  activeBranch?: string;
  isAgentBranch?: boolean;
  ghostPlayheadTick?: number | null;
  timelineZoom: number;
  changes?: ReviewChange[];
  onAcceptAll?: () => void;
  onRejectAll?: () => void;
  onSeek?: (seconds: number) => void;
  onLockForMe?: (changeId: number) => void;
  onAskAgentRedo?: (changeId: number) => void;
  onExplainEdit?: (changeId: number) => void;
}

export const AiBranchDiffRibbon: React.FC<AiBranchDiffRibbonProps> = ({
  activeBranch = "main",
  isAgentBranch = false,
  ghostPlayheadTick,
  timelineZoom,
  changes = [],
  onAcceptAll,
  onRejectAll,
  onSeek,
  onLockForMe,
  onAskAgentRedo,
  onExplainEdit,
}) => {
  const [selectedChangeId, setSelectedChangeId] = useState<number | null>(null);

  // If on main branch and no changes, show minimal or nothing
  const hasAgentActivity = isAgentBranch || activeBranch !== "main" || changes.length > 0;
  if (!hasAgentActivity) {
    return null;
  }

  const ghostX = ghostPlayheadTick != null ? (ghostPlayheadTick / TICKS_PER_SECOND) * timelineZoom : null;

  return (
    <div
      className="ai-branch-diff-ribbon glass"
      data-elev="1"
      role="region"
      aria-label="AI Branch Diff Ribbon"
      style={{
        position: "relative",
        height: "28px",
        minHeight: "28px",
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        padding: "0 10px",
        fontSize: "11px",
        borderBottom: "1px solid var(--border-subtle, rgba(255,255,255,0.08))",
        backgroundColor: "var(--bg-elev-1, rgba(25, 12, 28, 0.6))",
        color: "var(--text-primary)",
        zIndex: 5,
      }}
    >
      {/* Branch Badge & Change summary */}
      <div style={{ display: "flex", alignItems: "center", gap: "8px", zIndex: 2 }}>
        <span
          style={{
            padding: "2px 6px",
            borderRadius: "var(--radius-sm, 4px)",
            backgroundColor: "var(--accent-bg, rgba(244,114,182,0.15))",
            color: "var(--accent-text, #f472b6)",
            fontWeight: 600,
            display: "inline-flex",
            alignItems: "center",
            gap: "4px",
          }}
        >
          🌿 {activeBranch}
        </span>
        <span style={{ color: "var(--text-muted)" }}>
          {changes.length} proposed {changes.length === 1 ? "edit" : "edits"}
        </span>
      </div>

      {/* Changes markers preview along ribbon */}
      <div
        style={{
          position: "absolute",
          left: "220px",
          right: "240px",
          top: 0,
          bottom: 0,
          pointerEvents: "auto",
          overflow: "hidden",
        }}
      >
        {changes.map((c) => {
          const [start, end] = c.tick_range;
          const leftPx = (start / TICKS_PER_SECOND) * timelineZoom;
          const widthPx = Math.max(12, ((end - start) / TICKS_PER_SECOND) * timelineZoom);
          const isSelected = selectedChangeId === c.change_id;

          const badgeColor =
            c.confidence === "low"
              ? "var(--warn-text, #f59e0b)"
              : c.confidence === "medium"
              ? "var(--accent-text-cyan, #38bdf8)"
              : "var(--success-text, #10b981)";

          return (
            <div
              key={c.change_id}
              onClick={() => {
                setSelectedChangeId(isSelected ? null : c.change_id);
                onSeek?.(start / TICKS_PER_SECOND);
              }}
              title={`Change #${c.change_id}: ${c.rationale} (${c.confidence} confidence)`}
              style={{
                position: "absolute",
                left: `${leftPx}px`,
                width: `${widthPx}px`,
                top: "4px",
                bottom: "4px",
                borderRadius: "2px",
                backgroundColor: badgeColor,
                opacity: isSelected ? 0.95 : 0.65,
                border: isSelected ? "1px solid #ffffff" : undefined,
                cursor: "pointer",
                transition: "opacity var(--dur-fast, 120ms) var(--ease-out)",
              }}
            />
          );
        })}

        {/* Ghost Playhead in Ribbon */}
        {ghostX != null && (
          <div
            title={`Agent ghost playhead: ${(ghostPlayheadTick! / TICKS_PER_SECOND).toFixed(2)}s`}
            style={{
              position: "absolute",
              left: `${ghostX}px`,
              top: 0,
              bottom: 0,
              width: "2px",
              backgroundColor: "var(--accent-text-violet, #c084fc)",
              boxShadow: "0 0 6px var(--accent-text-violet, #c084fc)",
              zIndex: 3,
            }}
          />
        )}
      </div>

      {/* Actions and selected change popover */}
      <div style={{ display: "flex", alignItems: "center", gap: "6px", zIndex: 2 }}>
        {selectedChangeId != null && (
          <div style={{ display: "flex", alignItems: "center", gap: "4px", marginRight: "6px" }}>
            <InteriorButton
              variant="secondary"
              size="sm"
              style={{ height: "20px", padding: "1px 6px", fontSize: "10px" }}
              onClick={() => onLockForMe?.(selectedChangeId)}
              title="Lock this clip against agent modifications"
            >
              🔒 Lock for me
            </InteriorButton>
            <InteriorButton
              variant="secondary"
              size="sm"
              style={{ height: "20px", padding: "1px 6px", fontSize: "10px" }}
              onClick={() => onAskAgentRedo?.(selectedChangeId)}
              title="Request agent to redo this specific edit"
            >
              🔄 Redo
            </InteriorButton>
            <InteriorButton
              variant="secondary"
              size="sm"
              style={{ height: "20px", padding: "1px 6px", fontSize: "10px" }}
              onClick={() => onExplainEdit?.(selectedChangeId)}
              title="Explain the agent rationale for this cut"
            >
              💬 Explain
            </InteriorButton>
          </div>
        )}

        <InteriorButton
          variant="secondary"
          size="sm"
          style={{ height: "20px", padding: "2px 8px", fontSize: "11px" }}
          onClick={onRejectAll}
          title="Discard all proposed changes and keep current sequence (project.branch_merge reject)"
        >
          Reject All
        </InteriorButton>
        <InteriorButton
          variant="primary"
          size="sm"
          style={{ height: "20px", padding: "2px 8px", fontSize: "11px" }}
          onClick={onAcceptAll}
          title="Merge agent branch changes into main (project.branch_merge accept)"
        >
          Accept All
        </InteriorButton>
      </div>
    </div>
  );
};
