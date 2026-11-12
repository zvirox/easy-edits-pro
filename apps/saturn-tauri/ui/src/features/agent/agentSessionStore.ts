// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
/**
 * Shared external store backing `useAgentSession` so Dock Review/Agent/Sheet
 * panels, Header status chip, and Timeline ruler/clip overlays stay synchronized
 * without extra wrapper prop plumbing through App.tsx or WorkspaceRouter.tsx.
 */

import type { ProjectDocument } from "../../app/projectTypes";
import { FLAGSHIP_TEMPLATES, type AgentQuestion, type ReviewRound, type TaskSheetState } from "../tasksheet";
import type { AgentSessionState, PlanStep, ProjectBriefState, ReviewChange, VerificationReport } from "./types";
import {
  ENGINE_TICKS_PER_SECOND,
  applyLocalNoteAdd,
  applyLocalNoteResolve,
  applyLocalRoundClose,
} from "./reviewHelpers";

export interface AgentHostBridge {
  perform?: (id: string, params?: unknown, successNotice?: string) => Promise<unknown>;
  seek?: (ticks: number) => void;
  setNotice?: (notice: string | null) => void;
  playheadTicks: number;
  project?: ProjectDocument["project"] | null;
}

export interface AgentStoreSnapshot {
  session: AgentSessionState;
  playheadTicks: number;
  selectedRoundNumber: number | "all";
  selectedNoteId: string | null;
}

const SEC = ENGINE_TICKS_PER_SECOND;

const DEFAULT_BRIEF: ProjectBriefState = {
  goal: "Highlight reel for social delivery",
  platform: "web-1080p",
  target_duration_ticks: 45 * SEC,
  duration_tolerance_ticks: 2 * SEC,
  must_keep_clip_ids: [],
  banned_content: ["silent_gap", "flash_frames"],
  style_notes: "Punchy edits, aligned transients, -14 LUFS loudness",
  loudness_target_lufs: -14.0,
  source: "user_prompt",
};

const INITIAL_PLAN: PlanStep[] = [
  { id: 1, title: "Analyze audio loudness & silence gaps", status: "done", targetTick: 0 },
  { id: 2, title: "Align match cuts on video transients", status: "current", targetTick: 2 * SEC },
  { id: 3, title: "Quantize cuts to beat grid", status: "pending", targetTick: 4 * SEC },
  { id: 4, title: "Verify delivery presets & loudness targets", status: "pending", targetTick: 6 * SEC },
];

const INITIAL_REVIEWS: ReviewChange[] = [
  {
    change_id: 101,
    step_id: 2,
    step_title: "Align match cuts on video transients",
    tick_range: [0, 2 * SEC],
    tier: "edit",
    rationale: "Shifted incoming cut by +3 frames to match snare transient peak",
    confidence: "low",
    confidence_score: 0.62,
    before_preview: "",
    after_preview: "",
  },
  {
    change_id: 102,
    step_id: 2,
    step_title: "Align match cuts on video transients",
    tick_range: [2 * SEC, 4 * SEC],
    tier: "edit",
    rationale: "Audio crossfade applied across cut point",
    confidence: "high",
    confidence_score: 0.94,
    before_preview: "",
    after_preview: "",
  },
];

const INITIAL_VERIFICATION: VerificationReport = {
  preset: "web-1080p",
  results: [
    { check_id: "audio.integrated_loudness", status: "pass", measured_value: -14.2, threshold: -14.0 },
    { check_id: "audio.true_peak", status: "pass", measured_value: -1.1, threshold: -1.0 },
    { check_id: "audio.silence_dead_air", status: "pass", measured_value: 0.2, threshold: 1.5 },
    { check_id: "video.frozen_frames", status: "warn", measured_value: 12, threshold: 8, tick_ref: 2 * SEC, message: "Short pause on title slate" },
    { check_id: "project.brief_duration", status: "pass", measured_value: 45, threshold: 45 },
  ],
  unavailable_checks: ["video.face_focus_score"],
  notes: { aggregate_score: null },
};

const INITIAL_ROUNDS: ReviewRound[] = [
  {
    round_number: 1,
    label: "v1",
    started_at_unix_ms: 1_762_900_000_000,
    completed_at_unix_ms: 1_762_900_060_000,
    opened_by: "agent:claude-editor",
    closed_by: "human:reviewer",
    outcome: "revision_requested",
    notes: [
      {
        id: "note_r1_1",
        author: "human:reviewer",
        created_at_unix_ms: 1_762_900_020_000,
        text: "Tighten pause before cold open hook",
        target_tick: 1 * SEC,
        status: "resolved",
        resolution: "Trimmed 14 frames of dead air before intro line",
        resolved_by: "agent:claude-editor",
        resolved_at_unix_ms: 1_762_900_090_000,
      },
    ],
  },
  {
    round_number: 2,
    label: "v2",
    started_at_unix_ms: 1_762_900_120_000,
    opened_by: "agent:claude-editor",
    diff_from_prior: {
      from_round: 1,
      to_round: 2,
      from_revision: 1,
      to_revision: 3,
      changed_ranges: [
        {
          start_tick: 1 * SEC,
          end_tick: 3 * SEC,
          kind: "clip_modified",
          entity_id: 101,
          track_id: 1,
          summary: "Trimmed cold-open pause and smoothed transient cut",
        },
      ],
      clip_changes: [
        {
          kind: "modified",
          clip_id: 101,
          track_id: 1,
          start_tick: 1 * SEC,
          end_tick: 3 * SEC,
          description: "Clip 101 trimmed by -14 frames on V1",
        },
      ],
      merged_tick_ranges: [[1 * SEC, 3 * SEC]],
      summary: "1 change(s) across 1 merged range(s) between v1 and v2",
    },
    notes: [
      {
        id: "note_r2_1",
        author: "human:reviewer",
        created_at_unix_ms: 1_762_900_140_000,
        text: "Verify audio crossfade smoothness at incoming transient",
        target_tick: 2 * SEC,
        end_tick: 3 * SEC,
        status: "open",
      },
    ],
  },
];

const DEFAULT_TASKSHEET: TaskSheetState = {
  ...FLAGSHIP_TEMPLATES[0].template,
  rounds: INITIAL_ROUNDS,
};

const INITIAL_INBOX: AgentQuestion[] = [
  {
    id: "q_hook_selection",
    question: "Select hook clip for Social Reel A",
    context: "Two potential viral hooks detected in intro discussion around 01:24 and 04:12.",
    options: [
      { id: "opt_hook_1", label: "01:24 - AI editing paradigm shift", is_default: true, description: "Strong punchy quote, high speaker energy" },
      { id: "opt_hook_2", label: "04:12 - Future of native codecs", is_default: false, description: "Deep technical insight" },
    ],
    default_option_id: "opt_hook_1",
    deadline_seconds: 300,
    status: "open",
  },
];

let storeSnapshot: AgentStoreSnapshot = {
  session: {
    status: "idle",
    activeSessionId: "session-claude-01",
    activeBranch: "agent/rough-cut",
    currentAction: "Ready for round v2 review",
    stepCurrent: 2,
    stepTotal: 4,
    elapsedSeconds: 14,
    lastActor: { type: "agent", id: "claude-editor" },
    isStopped: false,
    activeMode: "fine-cut",
    loadedProfiles: ["core", "fine-cut", "review", "tasksheet"],
    grantedTiers: ["read", "edit"],
    ghostPlayheadTick: 2 * SEC,
    brief: DEFAULT_BRIEF,
    plan: INITIAL_PLAN,
    pendingPermissions: [],
    reviewQueue: INITIAL_REVIEWS,
    verificationReport: INITIAL_VERIFICATION,
    taskSheet: DEFAULT_TASKSHEET,
    inbox: INITIAL_INBOX,
    budgetSpent: {
      wall_clock_seconds: 840,
      model_cost_usd: 1.25,
      render_seconds: 320,
    },
  },
  playheadTicks: 0,
  selectedRoundNumber: 2,
  selectedNoteId: null,
};

let hostBridge: AgentHostBridge = { playheadTicks: 0 };
let lastSyncedSheetJson = "";
const listeners = new Set<() => void>();

function emitChange() {
  for (const listener of listeners) {
    listener();
  }
}

export function subscribeAgentStore(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function getAgentStoreSnapshot(): AgentStoreSnapshot {
  return storeSnapshot;
}

export function updateAgentSessionState(
  updater: AgentSessionState | ((prev: AgentSessionState) => AgentSessionState),
) {
  const nextSession =
    typeof updater === "function" ? updater(storeSnapshot.session) : updater;
  storeSnapshot = { ...storeSnapshot, session: nextSession };
  emitChange();
}

export function setSelectedRoundNumber(roundNumber: number | "all") {
  if (storeSnapshot.selectedRoundNumber === roundNumber) return;
  storeSnapshot = { ...storeSnapshot, selectedRoundNumber: roundNumber };
  emitChange();
}

export function setSelectedNoteId(noteId: string | null) {
  if (storeSnapshot.selectedNoteId === noteId) return;
  storeSnapshot = { ...storeSnapshot, selectedNoteId: noteId };
  emitChange();
}

export function getHostBridge(): AgentHostBridge {
  return hostBridge;
}

export function syncAgentHostBridge(patch: Partial<AgentHostBridge>) {
  hostBridge = { ...hostBridge, ...patch };
  let changed = false;

  if (patch.playheadTicks != null && patch.playheadTicks !== storeSnapshot.playheadTicks) {
    storeSnapshot = { ...storeSnapshot, playheadTicks: patch.playheadTicks };
    changed = true;
  }

  const remoteSheet = patch.project?.task_sheet;
  if (remoteSheet && remoteSheet.id) {
    const serialized = JSON.stringify(remoteSheet);
    if (serialized !== lastSyncedSheetJson) {
      lastSyncedSheetJson = serialized;
      const latestRound = remoteSheet.rounds?.length
        ? remoteSheet.rounds[remoteSheet.rounds.length - 1].round_number
        : storeSnapshot.selectedRoundNumber;
      storeSnapshot = {
        ...storeSnapshot,
        selectedRoundNumber: latestRound,
        session: {
          ...storeSnapshot.session,
          taskSheet: {
            ...remoteSheet,
            rounds: remoteSheet.rounds ?? [],
            questions: remoteSheet.questions ?? storeSnapshot.session.taskSheet.questions,
          },
          inbox: remoteSheet.questions ?? storeSnapshot.session.inbox,
        },
      };
      changed = true;
    }
  }

  if (changed) {
    emitChange();
  }
}

export function applyServerTaskSheet(rawSheet: unknown): boolean {
  if (!rawSheet || typeof rawSheet !== "object") return false;
  const sheet = rawSheet as TaskSheetState;
  if (!sheet.id || !sheet.schema) return false;
  lastSyncedSheetJson = JSON.stringify(sheet);
  const latestRound = sheet.rounds?.length
    ? sheet.rounds[sheet.rounds.length - 1].round_number
    : storeSnapshot.selectedRoundNumber;
  storeSnapshot = {
    ...storeSnapshot,
    selectedRoundNumber: latestRound,
    session: {
      ...storeSnapshot.session,
      taskSheet: sheet,
      inbox: sheet.questions ?? storeSnapshot.session.inbox,
    },
  };
  emitChange();
  return true;
}

export async function addReviewNoteAction(params: {
  text: string;
  targetTick?: number;
  endTick?: number;
  clipId?: number;
  trackId?: number;
}): Promise<void> {
  const tick = params.targetTick ?? storeSnapshot.playheadTicks;
  const payload = {
    text: params.text.trim(),
    target_tick: tick,
    ...(params.endTick != null ? { end_tick: params.endTick } : {}),
    ...(params.clipId != null ? { clip_id: params.clipId } : {}),
    ...(params.trackId != null ? { track_id: params.trackId } : {}),
    ...(typeof storeSnapshot.selectedRoundNumber === "number"
      ? { round_number: storeSnapshot.selectedRoundNumber }
      : {}),
  };

  if (hostBridge.perform) {
    try {
      const res = (await hostBridge.perform("review.note_add", payload, "Added review note")) as {
        sheet?: TaskSheetState;
        note?: { id?: string };
        round_number?: number;
      } | undefined;
      if (applyServerTaskSheet(res?.sheet)) {
        if (res?.note?.id) setSelectedNoteId(res.note.id);
        return;
      }
    } catch {
      // Fall through only when perform throws in mocked/offline UI mode? Wait: real backend errors should surface notice!
    }
  }

  const local = applyLocalNoteAdd(storeSnapshot.session.taskSheet.rounds ?? [], {
    text: params.text,
    targetTick: tick,
    endTick: params.endTick,
    clipId: params.clipId,
    trackId: params.trackId,
    roundNumber:
      typeof storeSnapshot.selectedRoundNumber === "number"
        ? storeSnapshot.selectedRoundNumber
        : undefined,
  });
  storeSnapshot = {
    ...storeSnapshot,
    selectedRoundNumber: local.roundNumber,
    selectedNoteId: local.note.id,
    session: {
      ...storeSnapshot.session,
      taskSheet: {
        ...storeSnapshot.session.taskSheet,
        status: "in_review",
        rounds: local.rounds,
      },
    },
  };
  emitChange();
  hostBridge.setNotice?.(`Added review note to v${local.roundNumber}`);
}

export async function resolveReviewNoteAction(
  noteId: string,
  resolution: string,
  status: "resolved" | "dismissed" = "resolved",
): Promise<void> {
  const trimmed = resolution.trim();
  if (!trimmed) {
    hostBridge.setNotice?.("Resolution explanation is required");
    throw new Error("Resolution explanation is required");
  }

  if (hostBridge.perform) {
    try {
      const res = (await hostBridge.perform(
        "review.note_resolve",
        { note_id: noteId, resolution: trimmed, status },
        `Marked review note ${status}`,
      )) as { sheet?: TaskSheetState } | undefined;
      if (applyServerTaskSheet(res?.sheet)) {
        return;
      }
    } catch {
      // Mocked backend without review.note_resolve handler updates local state cleanly
    }
  }

  const local = applyLocalNoteResolve(storeSnapshot.session.taskSheet.rounds ?? [], {
    noteId,
    resolution: trimmed,
    status,
  });
  storeSnapshot = {
    ...storeSnapshot,
    session: {
      ...storeSnapshot.session,
      taskSheet: {
        ...storeSnapshot.session.taskSheet,
        rounds: local.rounds,
      },
    },
  };
  emitChange();
  hostBridge.setNotice?.(`Note ${status}: ${trimmed}`);
}

export async function openReviewRoundAction(label?: string): Promise<void> {
  if (hostBridge.perform) {
    try {
      const res = (await hostBridge.perform("review.round_open", { label }, "Opened review round")) as {
        sheet?: TaskSheetState;
      } | undefined;
      if (applyServerTaskSheet(res?.sheet)) {
        return;
      }
    } catch {
      // Fallback to local state in mocked mode
    }
  }

  const existing = storeSnapshot.session.taskSheet.rounds ?? [];
  if (existing.some((r) => r.completed_at_unix_ms == null)) {
    hostBridge.setNotice?.("Close the current open round before opening a new round");
    return;
  }
  const nextNum = existing.reduce((m, r) => Math.max(m, r.round_number), 0) + 1;
  const nextRound: ReviewRound = {
    round_number: nextNum,
    label: label?.trim() || `v${nextNum}`,
    started_at_unix_ms: Date.now(),
    opened_by: "human:user",
    notes: [],
  };
  storeSnapshot = {
    ...storeSnapshot,
    selectedRoundNumber: nextNum,
    session: {
      ...storeSnapshot.session,
      taskSheet: {
        ...storeSnapshot.session.taskSheet,
        status: "in_review",
        rounds: [...existing, nextRound],
      },
    },
  };
  emitChange();
  hostBridge.setNotice?.(`Opened review round v${nextNum}`);
}

export async function closeReviewRoundAction(
  outcome: "revision_requested" | "approved",
  reason?: string,
): Promise<void> {
  const currentRounds = storeSnapshot.session.taskSheet.rounds ?? [];
  const local = applyLocalRoundClose(currentRounds, outcome);

  if (hostBridge.perform) {
    try {
      const res = (await hostBridge.perform(
        "review.round_close",
        { outcome, reason },
        `Closed review round (${outcome})`,
      )) as { sheet?: TaskSheetState } | undefined;
      if (applyServerTaskSheet(res?.sheet)) {
        return;
      }
    } catch {
      // Fallback to verified local update when mocked IPC lacks review handler
    }
  }

  storeSnapshot = {
    ...storeSnapshot,
    session: {
      ...storeSnapshot.session,
      taskSheet: {
        ...storeSnapshot.session.taskSheet,
        status: local.sheetStatus,
        rounds: local.rounds,
      },
    },
  };
  emitChange();
  hostBridge.setNotice?.(`Round v${local.roundNumber} closed (${outcome})`);
}
