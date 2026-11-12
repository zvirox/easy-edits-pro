// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
/**
 * Pure TS4 review round, note non-drop, and timeline indicator helpers.
 * Kept framework-free so both UI components and Node unit tests import them directly.
 */

import type { Clip } from "../../app/projectTypes";
import type { ReviewNote, ReviewNoteStatus, ReviewRound, SheetStatus } from "../tasksheet/types";
import type { ReviewChange } from "./types";

export const ENGINE_TICKS_PER_SECOND = 254_016_000_000;

export interface NoteSelectionEntry {
  roundNumber: number;
  roundLabel: string;
  note: ReviewNote;
}

export function roundDisplayLabel(round: Pick<ReviewRound, "round_number" | "label">): string {
  return round.label?.trim() ? round.label.trim() : `v${round.round_number}`;
}

export function countOpenReviewNotes(rounds: readonly ReviewRound[]): number {
  let count = 0;
  for (const round of rounds) {
    for (const note of round.notes ?? []) {
      if (note.status === "open") {
        count += 1;
      }
    }
  }
  return count;
}

export function selectRoundNotes(
  rounds: readonly ReviewRound[],
  selectedRound: number | "all" | null,
): NoteSelectionEntry[] {
  const entries: NoteSelectionEntry[] = [];
  for (const round of rounds) {
    if (selectedRound !== "all" && selectedRound != null && round.round_number !== selectedRound) {
      continue;
    }
    const roundLabel = roundDisplayLabel(round);
    for (const note of round.notes ?? []) {
      entries.push({
        roundNumber: round.round_number,
        roundLabel,
        note,
      });
    }
  }
  return entries;
}

/**
 * Matches whether a timeline clip is touched by an active low-confidence AI change.
 * Prefers explicit entity linkage via `clip.agent_note` and falls back to tick-interval overlap.
 */
export function clipHasLowConfidenceEdit(
  clip: Pick<Clip, "id" | "timeline_start" | "duration" | "agent_note">,
  reviewQueue: readonly ReviewChange[],
): ReviewChange | null {
  if (clip.agent_note?.active && clip.agent_note.confidence === "low") {
    const matched = reviewQueue.find(
      (c) => c.change_id === clip.agent_note?.change_id && !c.accepted && !c.rejected,
    );
    if (matched) return matched;
    if (clip.agent_note.change_id == null) {
      return {
        change_id: 0,
        tick_range: [clip.timeline_start, clip.timeline_start + clip.duration],
        tier: "edit",
        rationale: clip.agent_note.rationale || "Low-confidence AI edit",
        confidence: "low",
        confidence_score: 0.5,
      };
    }
  }

  const clipStart = clip.timeline_start;
  const clipEnd = clipStart + Math.max(0, clip.duration);

  for (const change of reviewQueue) {
    if (change.confidence !== "low" || change.accepted || change.rejected) {
      continue;
    }
    if (clip.id != null && clip.agent_note?.change_id === change.change_id) {
      return change;
    }
    const [rStart, rEnd] = change.tick_range;
    const normStart = Math.min(rStart, rEnd);
    const normEnd = Math.max(rStart, rEnd);
    if (normStart === normEnd) {
      if (normStart >= clipStart && normStart <= clipEnd) {
        return change;
      }
    } else if (clipStart < normEnd && normStart < clipEnd) {
      return change;
    }
  }

  return null;
}

export function formatReviewTick(ticks: number): string {
  const totalSeconds = Math.max(0, ticks / ENGINE_TICKS_PER_SECOND);
  const whole = Math.floor(totalSeconds);
  const mins = Math.floor(whole / 60);
  const secs = whole % 60;
  const tenths = Math.floor((totalSeconds - whole) * 10);
  return `${String(mins).padStart(2, "0")}:${String(secs).padStart(2, "0")}.${tenths}`;
}

export function applyLocalNoteAdd(
  rounds: readonly ReviewRound[],
  params: {
    text: string;
    author?: string;
    targetTick?: number;
    endTick?: number;
    clipId?: number;
    trackId?: number;
    roundNumber?: number;
    nowMs?: number;
  },
): { rounds: ReviewRound[]; roundNumber: number; note: ReviewNote } {
  const trimmed = params.text.trim();
  if (!trimmed) {
    throw new Error("Review note text cannot be empty.");
  }
  if (
    params.endTick != null &&
    params.targetTick != null &&
    params.endTick < params.targetTick
  ) {
    throw new Error("Review note end tick must be greater than or equal to start tick.");
  }

  const nowMs = params.nowMs ?? Date.now();
  const nextRounds: ReviewRound[] = rounds.map((r) => ({ ...r, notes: [...r.notes] }));

  let targetIdx =
    params.roundNumber != null
      ? nextRounds.findIndex((r) => r.round_number === params.roundNumber)
      : nextRounds.findIndex((r) => r.completed_at_unix_ms == null);

  if (targetIdx === -1) {
    if (params.roundNumber != null) {
      throw new Error(`Review round v${params.roundNumber} not found.`);
    }
    const nextNum = nextRounds.reduce((m, r) => Math.max(m, r.round_number), 0) + 1;
    nextRounds.push({
      round_number: nextNum,
      label: `v${nextNum}`,
      started_at_unix_ms: nowMs,
      opened_by: params.author ?? "human:user",
      notes: [],
    });
    targetIdx = nextRounds.length - 1;
  } else if (nextRounds[targetIdx].completed_at_unix_ms != null) {
    throw new Error(`Review round v${nextRounds[targetIdx].round_number} is already closed.`);
  }

  const targetRound = nextRounds[targetIdx];
  let seq = targetRound.notes.length + 1;
  const existingIds = new Set(nextRounds.flatMap((r) => r.notes.map((n) => n.id)));
  while (existingIds.has(`note_r${targetRound.round_number}_${seq}`)) {
    seq += 1;
  }

  const note: ReviewNote = {
    id: `note_r${targetRound.round_number}_${seq}`,
    author: params.author ?? "human:user",
    created_at_unix_ms: nowMs,
    text: trimmed,
    target_tick: params.targetTick,
    end_tick: params.endTick,
    clip_id: params.clipId,
    track_id: params.trackId,
    status: "open",
  };

  targetRound.notes.push(note);
  return { rounds: nextRounds, roundNumber: targetRound.round_number, note };
}

export function applyLocalNoteResolve(
  rounds: readonly ReviewRound[],
  params: {
    noteId: string;
    resolution: string;
    status?: Exclude<ReviewNoteStatus, "open">;
    actor?: string;
    nowMs?: number;
  },
): { rounds: ReviewRound[]; note: ReviewNote; roundNumber: number } {
  const resolution = params.resolution.trim();
  if (!resolution) {
    throw new Error("A non-empty resolution explanation is required; notes cannot be dropped.");
  }

  const nowMs = params.nowMs ?? Date.now();
  const nextRounds: ReviewRound[] = rounds.map((r) => ({ ...r, notes: [...r.notes] }));
  for (const round of nextRounds) {
    const idx = round.notes.findIndex((n) => n.id === params.noteId);
    if (idx !== -1) {
      const updated: ReviewNote = {
        ...round.notes[idx],
        status: params.status ?? "resolved",
        resolution,
        resolved_by: params.actor ?? "human:user",
        resolved_at_unix_ms: nowMs,
      };
      round.notes[idx] = updated;
      return { rounds: nextRounds, note: updated, roundNumber: round.round_number };
    }
  }
  throw new Error(`Review note ${params.noteId} not found.`);
}

export function applyLocalRoundClose(
  rounds: readonly ReviewRound[],
  outcome: "revision_requested" | "approved",
  actor = "human:user",
  nowMs = Date.now(),
): { rounds: ReviewRound[]; sheetStatus: SheetStatus; roundNumber: number } {
  if (outcome === "approved") {
    const openCount = countOpenReviewNotes(rounds);
    if (openCount > 0) {
      throw new Error(
        `Cannot approve cut with ${openCount} open review note(s); resolve or dismiss with an explanation first.`,
      );
    }
  }

  const nextRounds: ReviewRound[] = rounds.map((r) => ({ ...r, notes: [...r.notes] }));
  const openIdx = [...nextRounds].reverse().findIndex((r) => r.completed_at_unix_ms == null);
  if (openIdx === -1) {
    throw new Error("No open review round to close.");
  }
  const actualIdx = nextRounds.length - 1 - openIdx;
  nextRounds[actualIdx] = {
    ...nextRounds[actualIdx],
    completed_at_unix_ms: nowMs,
    closed_by: actor,
    outcome,
  };

  return {
    rounds: nextRounds,
    sheetStatus: outcome === "approved" ? "approved" : "revision_requested",
    roundNumber: nextRounds[actualIdx].round_number,
  };
}
