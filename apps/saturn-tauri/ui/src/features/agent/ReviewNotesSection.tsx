// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import React, { useState } from "react";
import { InteriorButton } from "../../components/interior/interior-button";
import type { TaskSheetState } from "../tasksheet/types";
import {
  countOpenReviewNotes,
  formatReviewTick,
  roundDisplayLabel,
  selectRoundNotes,
} from "./reviewHelpers";

export interface ReviewNotesSectionProps {
  taskSheet: TaskSheetState;
  playhead: number;
  selectedRoundNumber: number | "all";
  onSelectRound: (round: number | "all") => void;
  selectedNoteId: string | null;
  onSelectNote: (noteId: string | null) => void;
  onJumpToTick: (tick: number) => void;
  onAddNote: (params: { text: string; targetTick?: number }) => Promise<void>;
  onResolveNote: (
    noteId: string,
    resolution: string,
    status: "resolved" | "dismissed",
  ) => Promise<void>;
  onOpenRound: (label?: string) => Promise<void>;
  onCloseRound: (outcome: "revision_requested" | "approved", reason?: string) => Promise<void>;
  noteInputRef?: React.RefObject<HTMLInputElement | null>;
}

export const ReviewNotesSection: React.FC<ReviewNotesSectionProps> = ({
  taskSheet,
  playhead,
  selectedRoundNumber,
  onSelectRound,
  selectedNoteId,
  onSelectNote,
  onJumpToTick,
  onAddNote,
  onResolveNote,
  onOpenRound,
  onCloseRound,
  noteInputRef,
}) => {
  const rounds = taskSheet.rounds ?? [];
  const [noteDraft, setNoteDraft] = useState("");
  const [resolveOpenId, setResolveOpenId] = useState<string | null>(null);
  const [resolutionDraft, setResolutionDraft] = useState("");
  const [actionError, setActionError] = useState<string | null>(null);

  const openNotesTotal = countOpenReviewNotes(rounds);
  const activeRound = rounds.find((r) => r.completed_at_unix_ms == null) ?? null;
  const viewedRound =
    typeof selectedRoundNumber === "number"
      ? rounds.find((r) => r.round_number === selectedRoundNumber) ?? rounds[rounds.length - 1]
      : rounds[rounds.length - 1];

  const displayedNotes = selectRoundNotes(rounds, selectedRoundNumber);
  const diffReport = viewedRound?.diff_from_prior ?? null;

  const handleSubmitNote = async (e?: React.FormEvent) => {
    e?.preventDefault();
    const trimmed = noteDraft.trim();
    if (!trimmed) return;
    setActionError(null);
    try {
      await onAddNote({ text: trimmed, targetTick: playhead });
      setNoteDraft("");
    } catch (err) {
      setActionError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleResolveSubmit = async (
    noteId: string,
    status: "resolved" | "dismissed",
  ) => {
    const trimmed = resolutionDraft.trim();
    if (!trimmed) {
      setActionError("Resolution explanation is required before closing a note.");
      return;
    }
    setActionError(null);
    try {
      await onResolveNote(noteId, trimmed, status);
      setResolveOpenId(null);
      setResolutionDraft("");
    } catch (err) {
      setActionError(err instanceof Error ? err.message : String(err));
    }
  };

  return (
    <div className="review-notes-section glass" data-elev="1">
      <div className="review-notes-header">
        <div className="review-round-controls">
          <span style={{ fontWeight: 600, fontSize: "12px" }}>Review Rounds</span>
          <select
            className="review-round-select"
            aria-label="Review round selector"
            value={String(selectedRoundNumber)}
            onChange={(e) => {
              const val = e.target.value;
              onSelectRound(val === "all" ? "all" : Number(val));
            }}
          >
            {rounds.map((r) => (
              <option key={r.round_number} value={String(r.round_number)}>
                {roundDisplayLabel(r)} ({r.completed_at_unix_ms == null ? "Open" : r.outcome ?? "Closed"} · {r.notes.length})
              </option>
            ))}
            <option value="all">All Rounds ({rounds.reduce((sum, r) => sum + r.notes.length, 0)})</option>
          </select>
          <span className={openNotesTotal > 0 ? "review-open-badge" : "review-clean-badge"}>
            {openNotesTotal > 0 ? `${openNotesTotal} open note${openNotesTotal === 1 ? "" : "s"}` : "All notes addressed"}
          </span>
        </div>

        <div className="review-round-controls">
          {activeRound ? (
            <>
              <InteriorButton
                size="sm"
                variant="secondary"
                onClick={() => void onCloseRound("revision_requested", "Revisions requested")}
                title="Close active round and send notes back for revision"
              >
                Request Revisions
              </InteriorButton>
              <InteriorButton
                size="sm"
                variant="primary"
                disabled={openNotesTotal > 0}
                onClick={() => void onCloseRound("approved", "Approved")}
                title={
                  openNotesTotal > 0
                    ? "Resolve or dismiss all open review notes with explanations before approving"
                    : "Approve active review round"
                }
              >
                Approve Cut
              </InteriorButton>
            </>
          ) : (
            <InteriorButton
              size="sm"
              variant="secondary"
              disabled={taskSheet.status === "approved" || taskSheet.status === "delivered"}
              onClick={() => void onOpenRound()}
              title="Open next review round"
            >
              Open Round
            </InteriorButton>
          )}
        </div>
      </div>

      {/* Round-by-Round Diff Summary */}
      {diffReport && (
        <div className="review-diff-box" role="region" aria-label="Round diff summary">
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", fontSize: "11px" }}>
            <strong>
              Diff v{diffReport.from_round} → v{diffReport.to_round}
            </strong>
            <span style={{ color: "var(--text-sub)" }}>{diffReport.summary}</span>
          </div>
          {diffReport.changed_ranges.length > 0 && (
            <div className="review-diff-ranges">
              {diffReport.changed_ranges.map((range, idx) => (
                <button
                  key={`${range.kind}-${range.start_tick}-${idx}`}
                  type="button"
                  className="review-diff-chip"
                  onClick={() => onJumpToTick(range.start_tick)}
                  title={`${range.summary} · Jump to ${formatReviewTick(range.start_tick)}`}
                >
                  <span>{formatReviewTick(range.start_tick)}–{formatReviewTick(range.end_tick)}</span>
                  <span>{range.summary}</span>
                </button>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Add Note at Playhead (N shortcut) */}
      <form className="review-note-compose" onSubmit={(e) => void handleSubmitNote(e)}>
        <input
          ref={noteInputRef}
          type="text"
          className="review-note-input"
          aria-label="Review note text"
          placeholder={`Add review note at ${formatReviewTick(playhead)} (Shortcut: N)...`}
          value={noteDraft}
          onChange={(e) => setNoteDraft(e.target.value)}
        />
        <InteriorButton
          type="submit"
          size="sm"
          variant="primary"
          disabled={!noteDraft.trim()}
          title="Add timecoded review note at current playhead (Shortcut: N)"
        >
          Add Note at Playhead
        </InteriorButton>
      </form>

      {actionError && (
        <div style={{ fontSize: "11px", color: "var(--status-danger-text)" }}>{actionError}</div>
      )}

      {/* Timecoded Notes List */}
      <div className="review-note-list">
        {displayedNotes.length === 0 ? (
          <div style={{ fontSize: "11px", color: "var(--text-sub)" }}>
            No review notes recorded in this selection yet.
          </div>
        ) : (
          displayedNotes.map(({ roundLabel, note }) => {
            const isSelected = selectedNoteId === note.id;
            const statusClass =
              note.status === "resolved"
                ? "is-resolved"
                : note.status === "dismissed"
                ? "is-dismissed"
                : "";
            return (
              <div
                key={note.id}
                className={`review-note-item ${statusClass} ${isSelected ? "is-selected" : ""}`}
                onClick={() => {
                  onSelectNote(note.id);
                  if (note.target_tick != null) onJumpToTick(note.target_tick);
                }}
              >
                <div className="review-note-meta">
                  <span style={{ display: "flex", alignItems: "center", gap: "6px" }}>
                    <code>{roundLabel}</code>
                    {note.target_tick != null && (
                      <button
                        type="button"
                        className="review-note-timecode"
                        onClick={(e) => {
                          e.stopPropagation();
                          onJumpToTick(note.target_tick!);
                        }}
                        title="Jump playhead to note timecode"
                      >
                        {formatReviewTick(note.target_tick)}
                        {note.end_tick != null && note.end_tick > note.target_tick
                          ? `–${formatReviewTick(note.end_tick)}`
                          : ""}
                      </button>
                    )}
                    {note.clip_id != null && <span>Clip #{note.clip_id}</span>}
                  </span>
                  <span>
                    {note.author} · <strong>{note.status}</strong>
                  </span>
                </div>

                <div style={{ fontSize: "12px", color: "var(--text-main)" }}>{note.text}</div>

                {note.resolution && (
                  <div className="review-note-resolution">
                    Resolution{note.resolved_by ? ` (${note.resolved_by})` : ""}: {note.resolution}
                  </div>
                )}

                {note.status === "open" && (
                  <div
                    style={{ display: "flex", gap: "6px", alignItems: "center", flexWrap: "wrap" }}
                    onClick={(e) => e.stopPropagation()}
                  >
                    {resolveOpenId === note.id ? (
                      <>
                        <input
                          type="text"
                          className="review-note-input"
                          aria-label={`Resolution explanation for ${note.id}`}
                          placeholder="Required explanation for resolving/dismissing..."
                          value={resolutionDraft}
                          onChange={(e) => setResolutionDraft(e.target.value)}
                        />
                        <InteriorButton
                          size="sm"
                          variant="primary"
                          disabled={!resolutionDraft.trim()}
                          onClick={() => void handleResolveSubmit(note.id, "resolved")}
                        >
                          Confirm Resolve
                        </InteriorButton>
                        <InteriorButton
                          size="sm"
                          variant="secondary"
                          disabled={!resolutionDraft.trim()}
                          onClick={() => void handleResolveSubmit(note.id, "dismissed")}
                        >
                          Dismiss
                        </InteriorButton>
                        <InteriorButton
                          size="sm"
                          variant="secondary"
                          onClick={() => {
                            setResolveOpenId(null);
                            setResolutionDraft("");
                          }}
                        >
                          Cancel
                        </InteriorButton>
                      </>
                    ) : (
                      <InteriorButton
                        size="sm"
                        variant="secondary"
                        onClick={() => {
                          setResolveOpenId(note.id);
                          setResolutionDraft("");
                        }}
                      >
                        Resolve / Explain
                      </InteriorButton>
                    )}
                  </div>
                )}
              </div>
            );
          })
        )}
      </div>
    </div>
  );
};
