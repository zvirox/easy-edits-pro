// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import React, { useEffect, useMemo, useRef, useState } from "react";
import { InteriorButton } from "../../components/interior/interior-button";
import { ReviewNotesSection } from "./ReviewNotesSection";
import type { AgentSessionState, ReviewChange } from "./types";

export interface ReviewCardProps {
  session: AgentSessionState;
  playhead?: number;
  selectedRoundNumber?: number | "all";
  onSelectRound?: (round: number | "all") => void;
  selectedNoteId?: string | null;
  onSelectNote?: (noteId: string | null) => void;
  onAddReviewNote?: (params: { text: string; targetTick?: number }) => Promise<void>;
  onResolveReviewNote?: (
    noteId: string,
    resolution: string,
    status: "resolved" | "dismissed",
  ) => Promise<void>;
  onOpenReviewRound?: (label?: string) => Promise<void>;
  onCloseReviewRound?: (outcome: "revision_requested" | "approved", reason?: string) => Promise<void>;
  onAcceptChange: (changeId: number) => void;
  onRejectChange: (changeId: number) => void;
  onFixChange: (changeId: number, feedback: string) => void;
  onAcceptAllBranch: () => void;
  onRejectAllBranch: () => void;
  onJumpToTick: (tick: number) => void;
}

export const ReviewCard: React.FC<ReviewCardProps> = ({
  session,
  playhead = 0,
  selectedRoundNumber = "all",
  onSelectRound,
  selectedNoteId = null,
  onSelectNote,
  onAddReviewNote,
  onResolveReviewNote,
  onOpenReviewRound,
  onCloseReviewRound,
  onAcceptChange,
  onRejectChange,
  onFixChange,
  onAcceptAllBranch,
  onRejectAllBranch,
  onJumpToTick,
}) => {
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [fixInputOpen, setFixInputOpen] = useState<number | null>(null);
  const [fixFeedback, setFixFeedback] = useState("");
  const noteInputRef = useRef<HTMLInputElement | null>(null);

  // Sort: low confidence first, then by change_id
  const sortedChanges = useMemo(() => {
    const rank = { low: 0, medium: 1, high: 2 };
    return [...session.reviewQueue].sort((a, b) => {
      const rA = rank[a.confidence] ?? 1;
      const rB = rank[b.confidence] ?? 1;
      if (rA !== rB) return rA - rB;
      return a.change_id - b.change_id;
    });
  }, [session.reviewQueue]);

  const selectedItem = sortedChanges[selectedIndex];

  // Scoped keyboard shortcuts (J/K, A, R, F, N, Space)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement;
      if (
        target?.tagName === "INPUT" ||
        target?.tagName === "TEXTAREA" ||
        target?.tagName === "SELECT" ||
        target?.isContentEditable
      ) {
        return;
      }

      if (e.key === "n" || e.key === "N") {
        if (noteInputRef.current) {
          e.preventDefault();
          noteInputRef.current.focus();
        }
      } else if (e.key === "j" || e.key === "J" || e.key === "ArrowDown") {
        e.preventDefault();
        setSelectedIndex((idx) => Math.min(idx + 1, sortedChanges.length - 1));
      } else if (e.key === "k" || e.key === "K" || e.key === "ArrowUp") {
        e.preventDefault();
        setSelectedIndex((idx) => Math.max(idx - 1, 0));
      } else if (e.key === "a" || e.key === "A") {
        if (selectedItem) {
          e.preventDefault();
          onAcceptChange(selectedItem.change_id);
        }
      } else if (e.key === "r" || e.key === "R") {
        if (selectedItem) {
          e.preventDefault();
          onRejectChange(selectedItem.change_id);
        }
      } else if (e.key === "f" || e.key === "F") {
        if (selectedItem) {
          e.preventDefault();
          setFixInputOpen(selectedItem.change_id);
        }
      } else if (e.key === " ") {
        if (selectedItem) {
          e.preventDefault();
          onJumpToTick(selectedItem.tick_range[0]);
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [selectedItem, sortedChanges.length, onAcceptChange, onRejectChange, onJumpToTick]);

  const confidenceIcon = (conf: ReviewChange["confidence"]) => {
    switch (conf) {
      case "low":
        return <span style={{ color: "var(--accent-text)", fontWeight: 700 }}>● Low</span>;
      case "medium":
        return <span style={{ color: "var(--warn-text, #f59e0b)", fontWeight: 700 }}>▲ Med</span>;
      case "high":
        return <span style={{ color: "var(--accent-text-violet)", fontWeight: 700 }}>◆ High</span>;
    }
  };

  return (
    <div
      className="review-card"
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "var(--gap, 12px)",
        padding: "12px",
        height: "100%",
        overflowY: "auto",
        fontSize: "13px",
      }}
    >
      {/* Whole-Branch Actions Bar */}
      <div
        className="glass"
        data-elev="2"
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "8px 12px",
          borderRadius: "var(--radius-sm, 10px)",
        }}
      >
        <div>
          <div style={{ fontWeight: 600, fontSize: "12px" }}>
            Branch Review: <code>{session.activeBranch}</code>
          </div>
          <div style={{ fontSize: "11px", color: "var(--text-muted, #9ca3af)" }}>
            {session.reviewQueue.length} proposed changes · Shortkeys: J/K, A, R, F, N, Space
          </div>
        </div>

        <div style={{ display: "flex", gap: "6px" }}>
          <InteriorButton
            size="sm"
            variant="primary"
            onClick={onAcceptAllBranch}
            disabled={!session.activeBranch || session.activeBranch === "main"}
            title="Accept all branch edits"
          >
            Accept All
          </InteriorButton>
          <InteriorButton
            size="sm"
            variant="secondary"
            onClick={onRejectAllBranch}
            disabled={!session.activeBranch || session.activeBranch === "main"}
            title="Reject and revert branch edits"
          >
            Reject All
          </InteriorButton>
        </div>
      </div>

      {session.lastError && (
        <div
          role="alert"
          className="glass"
          data-elev="2"
          style={{
            padding: "8px 12px",
            borderRadius: "var(--radius-sm, 10px)",
            borderLeft: "3px solid var(--error-text)",
            color: "var(--error-text)",
            fontSize: "12px",
          }}
        >
          {session.lastError}
        </div>
      )}

      {/* Pro Review Notes & Revision Rounds (TS4) */}
      {session.taskSheet && onAddReviewNote && onResolveReviewNote && onOpenReviewRound && onCloseReviewRound ? (
        <ReviewNotesSection
          taskSheet={session.taskSheet}
          playhead={playhead}
          selectedRoundNumber={selectedRoundNumber}
          onSelectRound={(round) => onSelectRound?.(round)}
          selectedNoteId={selectedNoteId}
          onSelectNote={(noteId) => onSelectNote?.(noteId)}
          onJumpToTick={onJumpToTick}
          onAddNote={onAddReviewNote}
          onResolveNote={onResolveReviewNote}
          onOpenRound={onOpenReviewRound}
          onCloseRound={onCloseReviewRound}
          noteInputRef={noteInputRef}
        />
      ) : (
        <div
          className="glass"
          data-elev="1"
          style={{
            padding: "8px 12px",
            borderRadius: "var(--radius-sm, 10px)",
            fontSize: "12px",
            color: "var(--text-muted)",
          }}
        >
          No task sheet attached. Load a Task Sheet in the Sheet tab to manage review rounds and notes.
        </div>
      )}

      {/* Verification Evidence Summary */}
      {session.verificationReport && (
        <div
          className="glass"
          data-elev="1"
          style={{
            padding: "8px 12px",
            borderRadius: "var(--radius-sm, 10px)",
            display: "flex",
            flexDirection: "column",
            gap: "6px",
          }}
        >
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <span style={{ fontWeight: 600, fontSize: "12px" }}>
              ✓ Verification Report ({session.verificationReport.preset})
            </span>
            <span style={{ fontSize: "11px", color: "var(--text-muted, #9ca3af)" }}>Measured evidence</span>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
            {session.verificationReport.results.map((item) => (
              <div
                key={item.check_id}
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "center",
                  fontSize: "11px",
                  padding: "2px 4px",
                  borderRadius: "4px",
                  background: item.status === "warn" ? "rgba(245, 158, 11, 0.1)" : "transparent",
                }}
              >
                <span style={{ display: "flex", alignItems: "center", gap: "6px" }}>
                  <span>{item.status === "pass" ? "✅" : "⚠️"}</span>
                  <span>{item.check_id}</span>
                </span>
                <span style={{ color: "var(--text-dim, #d1d5db)" }}>
                  {item.measured_value} (target: {item.threshold})
                  {item.tick_ref != null && (
                    <button
                      type="button"
                      onClick={() => onJumpToTick(item.tick_ref!)}
                      style={{
                        background: "transparent",
                        border: "none",
                        color: "var(--accent-text)",
                        cursor: "pointer",
                        marginLeft: "6px",
                        fontSize: "10px",
                      }}
                    >
                      @{item.tick_ref}t
                    </button>
                  )}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Changes Queue */}
      <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
        <div style={{ fontWeight: 600, fontSize: "12px" }}>Proposed Edits (Low-confidence first)</div>
        {sortedChanges.length === 0 && (
          <div
            className="glass"
            data-elev="1"
            style={{
              padding: "10px 12px",
              borderRadius: "var(--radius-sm, 10px)",
              fontSize: "12px",
              color: "var(--text-muted)",
            }}
          >
            No proposed agent changes yet.
          </div>
        )}
        {sortedChanges.map((change, idx) => {
          const isSelected = idx === selectedIndex;
          return (
            <div
              key={change.change_id}
              onClick={() => setSelectedIndex(idx)}
              className="glass"
              data-elev={isSelected ? "3" : "1"}
              style={{
                padding: "8px 12px",
                borderRadius: "var(--radius-sm, 10px)",
                display: "flex",
                flexDirection: "column",
                gap: "6px",
                cursor: "pointer",
                borderLeft: isSelected
                  ? "3px solid var(--accent-text)"
                  : change.accepted
                  ? "3px solid var(--accent-text-violet)"
                  : change.rejected
                  ? "3px solid var(--text-muted)"
                  : "3px solid transparent",
                opacity: change.rejected ? 0.5 : 1,
              }}
            >
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <span style={{ fontWeight: 600, fontSize: "12px" }}>
                  #{change.change_id} · {change.step_title || `Step ${change.step_id}`}
                </span>
                <div style={{ display: "flex", alignItems: "center", gap: "8px", fontSize: "11px" }}>
                  {confidenceIcon(change.confidence)}
                  <span
                    style={{
                      fontFamily: "monospace",
                      color: "var(--accent-text)",
                      cursor: "pointer",
                    }}
                    onClick={(e) => {
                      e.stopPropagation();
                      onJumpToTick(change.tick_range[0]);
                    }}
                    title="Jump playhead to edit range"
                  >
                    [{change.tick_range[0]}t–{change.tick_range[1]}t]
                  </span>
                </div>
              </div>

              <div style={{ fontSize: "12px", color: "var(--text-dim, #d1d5db)" }}>
                {change.rationale}
              </div>

              {change.conflict && (
                <div
                  style={{
                    fontSize: "11px",
                    color: "var(--warn-text)",
                    borderLeft: "2px solid var(--warn-text)",
                    paddingLeft: "6px",
                  }}
                >
                  ⚠ Conflict: {change.conflict_reason || "Subsequent edits prevent selective revert"}
                </div>
              )}

              {/* Action Buttons for this Change */}
              <div
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "center",
                  marginTop: "4px",
                }}
              >
                <span style={{ fontSize: "11px", color: "var(--text-muted, #9ca3af)" }}>
                  {change.actor ? `${change.actor} · ` : ""}
                  Tier: <code>{change.tier}</code>
                </span>

                <div style={{ display: "flex", gap: "6px" }}>
                  <InteriorButton
                    size="sm"
                    variant={change.accepted ? "primary" : "secondary"}
                    onClick={(e) => {
                      e.stopPropagation();
                      onAcceptChange(change.change_id);
                    }}
                  >
                    {change.accepted ? "✓ Accepted" : "Accept (A)"}
                  </InteriorButton>
                  <InteriorButton
                    size="sm"
                    variant="secondary"
                    disabled={Boolean(change.rejected || change.conflict)}
                    title={change.conflict ? change.conflict_reason || "Cannot undo due to conflict" : undefined}
                    onClick={(e) => {
                      e.stopPropagation();
                      onRejectChange(change.change_id);
                    }}
                  >
                    {change.rejected ? "Reverted" : change.conflict ? "Conflict" : "Reject (R)"}
                  </InteriorButton>
                  <InteriorButton
                    size="sm"
                    variant="secondary"
                    onClick={(e) => {
                      e.stopPropagation();
                      setFixInputOpen(fixInputOpen === change.change_id ? null : change.change_id);
                    }}
                  >
                    Fix (F)
                  </InteriorButton>
                </div>
              </div>

              {/* Fix Feedback Input */}
              {fixInputOpen === change.change_id && (
                <div
                  style={{ display: "flex", gap: "6px", marginTop: "6px" }}
                  onClick={(e) => e.stopPropagation()}
                >
                  <input
                    type="text"
                    className="input glass"
                    data-elev="2"
                    placeholder="Feedback (e.g. 'Shift cut +2 frames later')..."
                    value={fixFeedback}
                    onChange={(e) => setFixFeedback(e.target.value)}
                    style={{ flex: 1, padding: "4px 8px", fontSize: "11px", borderRadius: "6px" }}
                  />
                  <InteriorButton
                    size="sm"
                    variant="primary"
                    onClick={() => {
                      if (fixFeedback.trim()) {
                        onFixChange(change.change_id, fixFeedback.trim());
                        setFixFeedback("");
                        setFixInputOpen(null);
                      }
                    }}
                  >
                    Submit Fix
                  </InteriorButton>
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
};
