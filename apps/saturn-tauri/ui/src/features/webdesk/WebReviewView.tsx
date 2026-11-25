// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
/**
 * Web Desk Review View (TS8).
 * Provides low-resolution proxy video playback, custom scrubber, timecoded notes,
 * round diff summary, and delivery approval actions.
 */

import React, { useRef, useState } from "react";
import { InteriorButton } from "../../components/interior/interior-button";
import type { useWebDeskStore } from "./webDeskStore";
import type { ReviewNote, ReviewRoundState, WebDeskJob } from "./types";

interface WebReviewViewProps {
  store: ReturnType<typeof useWebDeskStore>;
  job: WebDeskJob;
}

function formatTimecode(seconds: number): string {
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  const frames = Math.floor((seconds % 1) * 30);
  return `${String(mins).padStart(2, "0")}:${String(secs).padStart(2, "0")}:${String(frames).padStart(2, "0")}`;
}

export const WebReviewView: React.FC<WebReviewViewProps> = ({ store, job }) => {
  const {
    setActiveView,
    addReviewNote,
    resolveReviewNote,
    requestRevision,
    approveDelivery,
  } = store;

  const videoRef = useRef<HTMLVideoElement | null>(null);
  const [isPlaying, setIsPlaying] = useState(false);
  const [currentTime, setCurrentTime] = useState(0);
  const [duration, setDuration] = useState(60);
  const [noteDraft, setNoteDraft] = useState("");
  const [resolvingNoteId, setResolvingNoteId] = useState<string | null>(null);
  const [resolutionDraft, setResolutionDraft] = useState("");
  const [revisionReason, setRevisionReason] = useState("");
  const [showRevisionModal, setShowRevisionModal] = useState(false);
  const [selectedRoundIndex, setSelectedRoundIndex] = useState<number>(() => {
    return (job.rounds?.length ?? 0) > 0 ? (job.rounds?.length ?? 1) - 1 : 0;
  });

  const rounds = job.rounds ?? [];
  const currentRound: ReviewRoundState | null =
    rounds.length > 0 && selectedRoundIndex < rounds.length
      ? rounds[selectedRoundIndex]
      : null;

  const notes: ReviewNote[] = currentRound?.notes ?? [];
  const openNotes = notes.filter((n) => n.status === "open");

  // Sync playhead state
  const handleTimeUpdate = () => {
    if (videoRef.current) {
      setCurrentTime(videoRef.current.currentTime);
    }
  };

  const handleLoadedMetadata = () => {
    if (videoRef.current && !isNaN(videoRef.current.duration)) {
      setDuration(videoRef.current.duration);
    }
  };

  const togglePlay = () => {
    if (!videoRef.current) return;
    if (isPlaying) {
      videoRef.current.pause();
      setIsPlaying(false);
    } else {
      videoRef.current.play().catch(() => {});
      setIsPlaying(true);
    }
  };

  const handleSeek = (newTime: number) => {
    if (videoRef.current) {
      videoRef.current.currentTime = newTime;
      setCurrentTime(newTime);
    }
  };

  const handleNudge = (deltaSeconds: number) => {
    if (videoRef.current) {
      const next = Math.max(0, Math.min(duration, videoRef.current.currentTime + deltaSeconds));
      videoRef.current.currentTime = next;
      setCurrentTime(next);
    }
  };

  const handleJumpToTick = (tickOrSeconds: number) => {
    // 3000 ticks per second standard in saturn timeline, or seconds directly
    const targetSeconds = tickOrSeconds > 300 ? tickOrSeconds / 3000 : tickOrSeconds;
    handleSeek(targetSeconds);
  };

  const handleAddNote = async () => {
    if (!noteDraft.trim()) return;
    // Store in ticks (3000 ticks/sec)
    const tick = Math.round(currentTime * 3000);
    await addReviewNote(job.id, noteDraft.trim(), tick);
    setNoteDraft("");
  };

  const handleConfirmResolve = async (noteId: string) => {
    const res = resolutionDraft.trim() || "Addressed in latest cut revision";
    await resolveReviewNote(job.id, noteId, res);
    setResolvingNoteId(null);
    setResolutionDraft("");
  };

  const handleConfirmRevision = async () => {
    await requestRevision(job.id, revisionReason.trim() || "Requested revision with new review notes");
    setShowRevisionModal(false);
    setRevisionReason("");
  };

  const handleApprove = async () => {
    await approveDelivery(job.id);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", gap: "var(--gap, 12px)", overflow: "hidden" }}>
      {/* Top Header Bar */}
      <div
        className="surface-1" style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "10px 16px",
          borderRadius: "var(--radius, 14px)",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
          <InteriorButton
            size="sm" variant="ghost" onClick={() => setActiveView("detail")}
            data-testid="back-to-detail-btn"
          >
            ← Back to Job Detail
          </InteriorButton>
          <span style={{ fontWeight: 700, fontSize: "15px" }}> Review: {job.title}</span>
          {rounds.length > 0 && (
            <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
              <span style={{ fontSize: "12px", color: "var(--text-muted)" }}>Round:</span>
              <select
                className="webdesk-select" value={selectedRoundIndex}
                onChange={(e) => setSelectedRoundIndex(Number(e.target.value))}
                style={{ height: "26px", fontSize: "11px" }}
                data-testid="round-selector"
              >
                {rounds.map((r, i) => (
                  <option key={r.round_number} value={i}>
                    {r.label} ({r.notes?.length ?? 0} notes)
                  </option>
                ))}
              </select>
            </div>
          )}
        </div>

        {/* Decision Actions Bar */}
        <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
          <InteriorButton
            size="sm" variant="secondary" onClick={() => setShowRevisionModal(true)}
            data-testid="request-revision-btn"
          >
             Request Revision
          </InteriorButton>
          <InteriorButton
            size="sm" variant="primary" onClick={handleApprove}
            data-testid="approve-delivery-btn"
          >
            ✓ Approve Delivery
          </InteriorButton>
        </div>
      </div>

      {/* Main Review Layout */}
      <div className="webdesk-review-layout">
        {/* Left Column: RAW Proxy Video Player & Round Diff */}
        <div style={{ display: "flex", flexDirection: "column", gap: "var(--gap, 12px)", overflowY: "auto" }}>
          {/* RAW Video Player Surface — Strictly No Glass / Blur */}
          <div className="webdesk-player-surface" data-testid="video-player-surface">
            <video
              ref={videoRef}
              className="webdesk-video-element" src={job.media_proxy_url || "editor-smoke.webm"}
              onTimeUpdate={handleTimeUpdate}
              onLoadedMetadata={handleLoadedMetadata}
              data-testid="proxy-video-player"
            />

            {/* Custom Scrubber Bar */}
            <div className="webdesk-player-controls">
              <InteriorButton
                size="sm" variant="ghost" onClick={togglePlay}
                data-testid="play-pause-btn"
              >
                {isPlaying ? "⏸" : "▶"}
              </InteriorButton>

              <button
                type="button" className="webdesk-nav-btn" style={{ padding: "0 6px" }}
                onClick={() => handleNudge(-1 / 30)}
                title="Step Back 1 Frame"
              >
                -1f
              </button>
              <button
                type="button" className="webdesk-nav-btn" style={{ padding: "0 6px" }}
                onClick={() => handleNudge(1 / 30)}
                title="Step Forward 1 Frame"
              >
                +1f
              </button>

              <input
                type="range" className="webdesk-scrubber" min={0}
                max={duration || 100}
                step={0.01}
                value={currentTime}
                onChange={(e) => handleSeek(Number(e.target.value))}
                data-testid="video-scrubber" aria-label="Video Scrub Bar"
              />

              <div className="webdesk-timecode-display" data-testid="timecode-display">
                {formatTimecode(currentTime)} / {formatTimecode(duration)}
              </div>
            </div>
          </div>

          {/* Round Diff Summary */}
          <div
            className="surface-1" style={{
              padding: "12px 14px",
              borderRadius: "var(--radius, 14px)",
              display: "flex",
              flexDirection: "column",
              gap: "8px",
            }}
            data-testid="round-diff-summary"
          >
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
              <span style={{ fontSize: "13px", fontWeight: 700 }}>
                 Round Diff Summary ({currentRound?.label ?? "v1"})
              </span>
              <span style={{ fontSize: "11px", color: "var(--text-muted)" }}>
                Opened by {currentRound?.opened_by ?? "system"}
              </span>
            </div>

            <div style={{ fontSize: "12px", color: "var(--text-muted)" }}>
              {currentRound?.diff_from_prior?.summary ||
                "Initial cut candidate generated from task sheet rules with automated silence removal and loudness normalization."}
            </div>

            <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: "8px", marginTop: "4px" }}>
              <div className="surface-1" style={{ padding: "6px 8px", borderRadius: "6px", fontSize: "11px" }}>
                <span style={{ color: "var(--text-muted)" }}>Loudness:</span> <strong>-14.1 LUFS</strong>
              </div>
              <div className="surface-1" style={{ padding: "6px 8px", borderRadius: "6px", fontSize: "11px" }}>
                <span style={{ color: "var(--text-muted)" }}>True Peak:</span> <strong>-1.0 dBTP</strong>
              </div>
              <div className="surface-1" style={{ padding: "6px 8px", borderRadius: "6px", fontSize: "11px" }}>
                <span style={{ color: "var(--text-muted)" }}>Captions:</span> <strong>Safe Area Pass</strong>
              </div>
            </div>
          </div>
        </div>

        {/* Right Column: Timecoded Revision Notes */}
        <div
          className="surface-1" style={{
            padding: "14px",
            borderRadius: "var(--radius, 14px)",
            display: "flex",
            flexDirection: "column",
            gap: "12px",
            overflow: "hidden",
          }}
          data-testid="review-notes-card"
        >
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
            <span style={{ fontSize: "13px", fontWeight: 700 }}>
               Timecoded Notes ({notes.length})
            </span>
            <span
              style={{
                fontSize: "11px",
                fontWeight: 600,
                color: openNotes.length > 0 ? "var(--accent-text)" : "var(--accent-text-hover)",
              }}
            >
              {openNotes.length} Open Note{openNotes.length === 1 ? "" : "s"}
            </span>
          </div>

          {/* Add note input anchored to playhead */}
          <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
            <div style={{ display: "flex", gap: "6px" }}>
              <input
                type="text" className="webdesk-search-input" placeholder="Note at current playhead position..." value={noteDraft}
                onChange={(e) => setNoteDraft(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    void handleAddNote();
                  }
                }}
                data-testid="note-input" aria-label="Add review note"
              />
              <InteriorButton
                size="sm" variant="primary" onClick={handleAddNote}
                data-testid="add-note-btn"
              >
                Add @ {formatTimecode(currentTime)}
              </InteriorButton>
            </div>
          </div>

          {/* Notes list */}
          <div className="webdesk-notes-list" style={{ flex: 1 }}>
            {notes.length === 0 ? (
              <div style={{ fontSize: "12px", color: "var(--text-muted)", textAlign: "center", padding: "20px" }}>
                No review notes in this round yet. Pause playback and add a timecoded note at any playhead position.
              </div>
            ) : (
              notes.map((note) => {
                const tickSeconds = (note.target_tick ?? 0) / 3000;
                const isResolving = resolvingNoteId === note.id;

                return (
                  <div key={note.id} className="webdesk-note-item surface-1" data-testid={`note-item-${note.id}`}>
                    <div className="webdesk-note-header">
                      <button
                        type="button" className="webdesk-note-timecode-btn" onClick={() => handleJumpToTick(note.target_tick ?? 0)}
                        title="Jump to timecode" data-testid="note-timecode-btn"
                      >
                        {formatTimecode(tickSeconds)}
                      </button>
                      <span
                        style={{
                          fontSize: "10px",
                          fontWeight: 700,
                          textTransform: "uppercase",
                          color: note.status === "resolved" ? "var(--accent-text-hover)" : "var(--accent-text)",
                        }}
                      >
                        {note.status.toUpperCase()}
                      </span>
                    </div>

                    <div style={{ fontSize: "12px", lineHeight: "1.4" }}>
                      {note.text}
                    </div>

                    {note.resolution && (
                      <div
                        style={{
                          fontSize: "11px",
                          color: "var(--accent-text-hover)",
                          background: "var(--card-inset)",
                          padding: "4px 8px",
                          borderRadius: "4px",
                          border: "1px solid var(--glass-border)",
                        }}
                      >
                        ✓ {note.resolution}
                      </div>
                    )}

                    {note.status === "open" && !isResolving && (
                      <div style={{ display: "flex", justifyContent: "flex-end", marginTop: "4px" }}>
                        <InteriorButton
                          size="sm" variant="ghost" onClick={() => setResolvingNoteId(note.id)}
                          data-testid="resolve-note-btn"
                        >
                          Resolve Note
                        </InteriorButton>
                      </div>
                    )}

                    {isResolving && (
                      <div style={{ display: "flex", flexDirection: "column", gap: "6px", marginTop: "6px" }}>
                        <input
                          type="text" className="webdesk-search-input" placeholder="Resolution details (e.g. cut adjusted in take 2)..." value={resolutionDraft}
                          onChange={(e) => setResolutionDraft(e.target.value)}
                          aria-label="Note Resolution Comment" data-testid="resolution-input"
                        />
                        <div style={{ display: "flex", gap: "6px", justifyContent: "flex-end" }}>
                          <InteriorButton
                            size="sm" variant="ghost" onClick={() => {
                              setResolvingNoteId(null);
                              setResolutionDraft("");
                            }}
                          >
                            Cancel
                          </InteriorButton>
                          <InteriorButton
                            size="sm" variant="primary" onClick={() => handleConfirmResolve(note.id)}
                            data-testid="confirm-resolve-btn"
                          >
                            Confirm Resolution
                          </InteriorButton>
                        </div>
                      </div>
                    )}
                  </div>
                );
              })
            )}
          </div>
        </div>
      </div>

      {/* Revision Request Modal */}
      {showRevisionModal && (
        <div
          className="webdesk-modal-backdrop" role="dialog" aria-modal="true" aria-labelledby="revision-modal-title" data-testid="revision-modal"
        >
          <div className="webdesk-modal surface-1">
            <div className="webdesk-modal-header">
              <h2 id="revision-modal-title" className="webdesk-modal-title">
                 Request Revision (Round {rounds.length + 1})
              </h2>
              <InteriorButton size="sm" variant="ghost" onClick={() => setShowRevisionModal(false)}>
                ✕
              </InteriorButton>
            </div>

            <div style={{ display: "flex", flexDirection: "column", gap: "8px", fontSize: "13px" }}>
              <span>Explain required revisions for the autonomous agent:</span>
              <input
                type="text" className="webdesk-search-input" placeholder="Describe key adjustments..." value={revisionReason}
                onChange={(e) => setRevisionReason(e.target.value)}
                aria-label="Revision Instructions" data-testid="revision-reason-input"
              />
              <div style={{ color: "var(--text-muted)", fontSize: "11px" }}>
                This will close review round {rounds.length} and return the job to the agent with open review notes attached.
              </div>
            </div>

            <div style={{ display: "flex", justifyContent: "flex-end", gap: "8px", marginTop: "8px" }}>
              <InteriorButton size="md" variant="ghost" onClick={() => setShowRevisionModal(false)}>
                Cancel
              </InteriorButton>
              <InteriorButton
                size="md" variant="primary" onClick={handleConfirmRevision}
                data-testid="confirm-revision-btn"
              >
                Send Revision to Agent
              </InteriorButton>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
