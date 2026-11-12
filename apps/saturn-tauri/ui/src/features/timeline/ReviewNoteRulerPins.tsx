// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import type { CSSProperties } from "react";
import type { ReviewRound } from "../tasksheet/types";
import { TICKS_PER_SECOND } from "./types";
import { formatReviewTick, selectRoundNotes } from "../agent/reviewHelpers";

export interface ReviewNoteRulerPinsProps {
  rounds: readonly ReviewRound[];
  selectedRoundNumber: number | "all";
  selectedNoteId: string | null;
  pixelsPerSecond: number;
  onSelectAndJump: (noteId: string, roundNumber: number, targetTick: number) => void;
}

/**
 * Timecoded review note pins rendered directly along the timeline ruler axis.
 * Keeps timeline surface strictly raw (no glass, no blur, no motion transitions).
 */
export function ReviewNoteRulerPins({
  rounds,
  selectedRoundNumber,
  selectedNoteId,
  pixelsPerSecond,
  onSelectAndJump,
}: ReviewNoteRulerPinsProps) {
  const entries = selectRoundNotes(rounds, selectedRoundNumber).filter(
    (entry) => entry.note.target_tick != null,
  );
  if (entries.length === 0) return null;

  return (
    <div className="timeline-review-pins" aria-label="Review note indicators on ruler">
      {entries.map(({ roundNumber, roundLabel, note }) => {
        const startTick = note.target_tick!;
        const leftPx = (startTick / TICKS_PER_SECOND) * pixelsPerSecond;
        const hasRange = note.end_tick != null && note.end_tick > startTick;
        const widthPx = hasRange
          ? Math.max(10, ((note.end_tick! - startTick) / TICKS_PER_SECOND) * pixelsPerSecond)
          : undefined;
        const style: CSSProperties = {
          left: `${leftPx}px`,
          width: widthPx != null ? `${widthPx}px` : undefined,
        };
        const statusClass =
          note.status === "resolved"
            ? "is-resolved"
            : note.status === "dismissed"
            ? "is-dismissed"
            : "";

        return (
          <button
            key={note.id}
            type="button"
            className={[
              "timeline-review-pin",
              hasRange ? "is-range" : "",
              statusClass,
              note.id === selectedNoteId ? "is-selected" : "",
            ]
              .filter(Boolean)
              .join(" ")}
            style={style}
            title={`Review ${roundLabel} (${note.status}) · ${formatReviewTick(startTick)} · ${note.text}`}
            aria-label={`Review note ${note.id} at ${formatReviewTick(startTick)}: ${note.text}`}
            onPointerDown={(event) => event.stopPropagation()}
            onClick={(event) => {
              event.stopPropagation();
              onSelectAndJump(note.id, roundNumber, startTick);
            }}
          />
        );
      })}
    </div>
  );
}
