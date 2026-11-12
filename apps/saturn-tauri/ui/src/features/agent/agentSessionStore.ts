// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
/**
 * Shared external store backing `useAgentSession` so Dock Review/Agent/Sheet
 * panels, Header status chip, and Timeline overlays render real engine state
 * and confirm engine execution before mutating UI state.
 */

import type { ProjectDocument } from "../../app/projectTypes";
import { FLAGSHIP_TEMPLATES, type TaskSheetState } from "../tasksheet/types.ts";
import { agentBridge, isDemoAgentMode } from "./agentBridge.ts";
import { createDemoAgentSnapshot } from "./agentDemoSeed.ts";
import {
  mapBrief,
  mapPlanSteps,
  mapRawChange,
  parseActor,
  parseVerifyReport,
  sheetHasRealState,
} from "./agentPayloadMappers.ts";
import type {
  AgentSessionState,
  ProjectBriefState,
} from "./types.ts";

export interface AgentHostBridge {
  perform?: (id: string, params?: unknown, successNotice?: string) => Promise<unknown>;
  refreshProject?: () => Promise<unknown>;
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

function createEmptyAgentSnapshot(): AgentStoreSnapshot {
  return {
    session: {
      status: "idle",
      activeSessionId: null,
      activeBranch: "main",
      currentAction: "No active agent session",
      stepCurrent: 0,
      stepTotal: 0,
      elapsedSeconds: 0,
      lastActor: null,
      isStopped: false,
      activeMode: "core",
      loadedProfiles: ["core"],
      grantedTiers: [],
      ghostPlayheadTick: null,
      brief: null,
      plan: [],
      pendingPermissions: [],
      reviewQueue: [],
      verificationReport: null,
      taskSheet: null,
      inbox: [],
      budgetSpent: {
        wall_clock_seconds: 0,
        model_cost_usd: 0,
        render_seconds: 0,
      },
      lastError: null,
      lastTelemetry: null,
    },
    playheadTicks: 0,
    selectedRoundNumber: "all",
    selectedNoteId: null,
  };
}

function createInitialSnapshot(demo = isDemoAgentMode()): AgentStoreSnapshot {
  return demo ? createDemoAgentSnapshot() : createEmptyAgentSnapshot();
}

let storeSnapshot: AgentStoreSnapshot = createInitialSnapshot();
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

export function resetAgentStoreForTest(demo = false): void {
  storeSnapshot = createInitialSnapshot(demo);
  hostBridge = { playheadTicks: 0 };
  lastSyncedSheetJson = "";
  emitChange();
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
            questions: remoteSheet.questions ?? storeSnapshot.session.inbox,
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

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function surfaceActionError(error: unknown): Error {
  const message = errorMessage(error);
  storeSnapshot = {
    ...storeSnapshot,
    session: {
      ...storeSnapshot.session,
      lastError: message,
    },
  };
  emitChange();
  hostBridge.setNotice?.(message);
  return error instanceof Error ? error : new Error(message);
}

export function applyServerTaskSheet(rawSheet: unknown): boolean {
  if (!rawSheet || typeof rawSheet !== "object") return false;
  const sheet = rawSheet as TaskSheetState;
  if (!sheet.id || !sheet.schema) return false;
  lastSyncedSheetJson = JSON.stringify(sheet);
  const latestRound = sheet.rounds?.length
    ? sheet.rounds[sheet.rounds.length - 1].round_number
    : storeSnapshot.selectedRoundNumber;
  const plan = mapPlanSteps(sheet);
  storeSnapshot = {
    ...storeSnapshot,
    selectedRoundNumber: latestRound,
    session: {
      ...storeSnapshot.session,
      taskSheet: sheet,
      inbox: sheet.questions ?? [],
      plan: plan.length > 0 ? plan : storeSnapshot.session.plan,
      lastError: null,
    },
  };
  emitChange();
  return true;
}

export async function refreshAgentSessionFromEngine(options?: {
  forceEvenInDemo?: boolean;
}): Promise<void> {
  if (isDemoAgentMode() && !options?.forceEvenInDemo) {
    return;
  }

  let statusRes;
  try {
    statusRes = await agentBridge.getSessionStatus(storeSnapshot.session.activeSessionId);
  } catch (error) {
    const msg = errorMessage(error);
    // Avoid showing noisy IPC errors when the preview runs in a plain browser without Tauri runtime.
    if (msg.includes("__TAURI_INTERNALS__") || msg.includes("Tauri")) {
      return;
    }
    throw surfaceActionError(error);
  }

  let branchList: { active?: string; branches?: Array<{ name: string }> } | null = null;
  try {
    branchList = await agentBridge.runCommand<{
      active?: string;
      branches?: Array<{ name: string }>;
    }>("project.branch_list", {});
  } catch (error) {
    const msg = errorMessage(error);
    if (msg.includes("No project")) {
      return;
    }
    throw surfaceActionError(error);
  }

  const activeEngineBranch = branchList?.active || statusRes.active_branch || "main";
  const agentBranches = (branchList?.branches ?? [])
    .map((b) => b.name)
    .filter((name) => name !== "main");
  const reviewTargetBranch =
    activeEngineBranch !== "main"
      ? activeEngineBranch
      : statusRes.active_branch && statusRes.active_branch !== "main"
      ? statusRes.active_branch
      : agentBranches[0] ?? "main";

  const [briefRes, sheetRes, changesRes] = await Promise.all([
    agentBridge.runCommand("project.brief_get", {}).catch(() => null),
    agentBridge.runCommand<{ sheet?: TaskSheetState }>("tasksheet.get", {}).catch(() => null),
    agentBridge
      .runCommand<{ changes?: Array<Record<string, unknown>> }>(
        "review.list_changes",
        reviewTargetBranch !== "main" ? { branch: reviewTargetBranch } : {},
      )
      .catch(() => null),
  ]);

  const parsedBrief = mapBrief(briefRes);
  const verifyRes = await agentBridge
    .runCommand("verify.run", { preset: parsedBrief?.platform || "web-1080p" })
    .catch(() => null);
  const { report, telemetry } = parseVerifyReport(verifyRes);

  const rawSheet = sheetRes?.sheet ?? null;
  const keepSheet = sheetHasRealState(rawSheet, Boolean(hostBridge.project?.task_sheet));
  const nextSheet = keepSheet ? rawSheet : null;
  const plan = mapPlanSteps(nextSheet);

  const rawChanges = changesRes?.changes ?? [];
  const reviewQueue = rawChanges
    .filter((item) => item.is_human === false || reviewTargetBranch !== "main")
    .map(mapRawChange);

  const prev = storeSnapshot.session;
  const isStopped = Boolean(statusRes.has_active_session && statusRes.is_stopped);
  const nextStatus = isStopped
    ? "stopped"
    : statusRes.has_active_session
    ? prev.status === "thinking" || prev.status === "editing" || prev.status === "waiting"
      ? prev.status
      : "idle"
    : "idle";

  const latestRound = nextSheet?.rounds?.length
    ? nextSheet.rounds[nextSheet.rounds.length - 1].round_number
    : "all";

  if (nextSheet) {
    lastSyncedSheetJson = JSON.stringify(nextSheet);
  }

  const currentDoneCount = plan.filter((s) => s.status === "done").length;

  storeSnapshot = {
    ...storeSnapshot,
    selectedRoundNumber: latestRound,
    session: {
      ...prev,
      status: nextStatus,
      activeSessionId: statusRes.has_active_session ? statusRes.session_id : null,
      activeBranch: reviewTargetBranch,
      currentAction: isStopped
        ? "Session stopped by user"
        : statusRes.has_active_session
        ? `Session active on ${reviewTargetBranch}`
        : reviewQueue.length > 0
        ? `${reviewQueue.length} change(s) on ${reviewTargetBranch}`
        : "No active agent session",
      stepCurrent: currentDoneCount,
      stepTotal: plan.length,
      lastActor: parseActor(statusRes.actor),
      isStopped,
      ghostPlayheadTick: statusRes.has_active_session ? statusRes.playhead : null,
      brief: parsedBrief,
      plan,
      reviewQueue,
      verificationReport: report,
      taskSheet: nextSheet,
      inbox: nextSheet?.questions ?? [],
      lastError: null,
      lastTelemetry: telemetry,
    },
  };
  emitChange();
}

export async function stopSessionAction(): Promise<void> {
  let sessionId = storeSnapshot.session.activeSessionId;
  try {
    if (!sessionId) {
      const status = await agentBridge.getSessionStatus(null);
      sessionId = status.has_active_session ? status.session_id : null;
    }
    if (!sessionId) {
      throw new Error("No active agent session to stop");
    }
    await agentBridge.stopSession(sessionId);
    storeSnapshot = {
      ...storeSnapshot,
      session: {
        ...storeSnapshot.session,
        activeSessionId: sessionId,
        status: "stopped",
        isStopped: true,
        currentAction: "Session stopped by user",
        lastError: null,
      },
    };
    emitChange();
    hostBridge.setNotice?.("Agent session halted (immediate Stop)");
  } catch (error) {
    throw surfaceActionError(error);
  }
}

export async function resumeSessionAction(): Promise<void> {
  const sessionId = storeSnapshot.session.activeSessionId;
  try {
    if (!sessionId) {
      throw new Error("No active agent session to resume");
    }
    await agentBridge.resumeSession(sessionId);
    storeSnapshot = {
      ...storeSnapshot,
      session: {
        ...storeSnapshot.session,
        status: "idle",
        isStopped: false,
        currentAction: "Agent session resumed",
        lastError: null,
      },
    };
    emitChange();
    hostBridge.setNotice?.("Agent session resumed");
  } catch (error) {
    throw surfaceActionError(error);
  }
}

export async function updateBriefAction(newBrief: ProjectBriefState): Promise<void> {
  try {
    await agentBridge.runCommand("project.brief_set", newBrief);
    storeSnapshot = {
      ...storeSnapshot,
      session: {
        ...storeSnapshot.session,
        brief: newBrief,
        lastError: null,
      },
    };
    emitChange();
    await hostBridge.refreshProject?.();
    hostBridge.setNotice?.("Project brief updated");
  } catch (error) {
    throw surfaceActionError(error);
  }
}

export function acceptChangeAction(changeId: number): void {
  storeSnapshot = {
    ...storeSnapshot,
    session: {
      ...storeSnapshot.session,
      reviewQueue: storeSnapshot.session.reviewQueue.map((item) =>
        item.change_id === changeId ? { ...item, accepted: true, rejected: false } : item,
      ),
      lastError: null,
    },
  };
  emitChange();
  hostBridge.setNotice?.(`Accepted change #${changeId}`);
}

export async function rejectChangeAction(changeId: number): Promise<void> {
  try {
    await agentBridge.undoChange(changeId);
    storeSnapshot = {
      ...storeSnapshot,
      session: {
        ...storeSnapshot.session,
        reviewQueue: storeSnapshot.session.reviewQueue.map((item) =>
          item.change_id === changeId
            ? { ...item, accepted: false, rejected: true, undone: true, conflict: false, conflict_reason: null }
            : item,
        ),
        lastError: null,
      },
    };
    emitChange();
    await hostBridge.refreshProject?.();
    await refreshAgentSessionFromEngine().catch(() => {});
    hostBridge.setNotice?.(`Rejected change #${changeId}`);
  } catch (error) {
    const msg = errorMessage(error);
    if (msg.toLowerCase().includes("conflict")) {
      storeSnapshot = {
        ...storeSnapshot,
        session: {
          ...storeSnapshot.session,
          reviewQueue: storeSnapshot.session.reviewQueue.map((item) =>
            item.change_id === changeId
              ? { ...item, conflict: true, conflict_reason: msg }
              : item,
          ),
        },
      };
    }
    throw surfaceActionError(error);
  }
}

async function resolveTargetMergeBranch(): Promise<{
  targetBranch: string;
  currentActive: string;
}> {
  const branchList = await agentBridge.runCommand<{
    active?: string;
    branches?: Array<{ name: string }>;
  }>("project.branch_list", {});
  const currentActive = branchList.active || "main";
  const branches = (branchList.branches ?? []).map((b) => b.name).filter((n) => n !== "main");
  const uiBranch = storeSnapshot.session.activeBranch;
  const targetBranch =
    uiBranch && uiBranch !== "main" && (branches.includes(uiBranch) || currentActive === uiBranch)
      ? uiBranch
      : currentActive !== "main"
      ? currentActive
      : branches[0] ?? "";

  if (!targetBranch || targetBranch === "main") {
    throw new Error("No active agent branch available to merge or discard");
  }
  return { targetBranch, currentActive };
}

export async function acceptAllBranchAction(options?: { force?: boolean }): Promise<void> {
  try {
    const { targetBranch, currentActive } = await resolveTargetMergeBranch();
    if (currentActive === targetBranch) {
      await agentBridge.runCommand("project.branch_switch", { name: "main" });
    }
    await agentBridge.runCommand("project.branch_merge", {
      branch: targetBranch,
      action: "accept",
      ...(options?.force ? { force: true } : {}),
    });
    storeSnapshot = {
      ...storeSnapshot,
      session: {
        ...storeSnapshot.session,
        activeBranch: "main",
        status: "idle",
        reviewQueue: storeSnapshot.session.reviewQueue.map((item) =>
          item.rejected ? item : { ...item, accepted: true },
        ),
        lastError: null,
      },
    };
    emitChange();
    await hostBridge.refreshProject?.();
    await refreshAgentSessionFromEngine().catch(() => {});
    hostBridge.setNotice?.(`Accepted and merged branch '${targetBranch}' into main`);
  } catch (error) {
    throw surfaceActionError(error);
  }
}

export async function rejectAllBranchAction(): Promise<void> {
  try {
    const { targetBranch, currentActive } = await resolveTargetMergeBranch();
    if (currentActive === targetBranch) {
      await agentBridge.runCommand("project.branch_switch", { name: "main" });
    }
    await agentBridge.runCommand("project.branch_merge", {
      branch: targetBranch,
      action: "reject",
    });
    storeSnapshot = {
      ...storeSnapshot,
      session: {
        ...storeSnapshot.session,
        activeBranch: "main",
        status: "idle",
        reviewQueue: storeSnapshot.session.reviewQueue.map((item) => ({
          ...item,
          accepted: false,
          rejected: true,
        })),
        lastError: null,
      },
    };
    emitChange();
    await hostBridge.refreshProject?.();
    await refreshAgentSessionFromEngine().catch(() => {});
    hostBridge.setNotice?.(`Discarded agent branch '${targetBranch}'`);
  } catch (error) {
    throw surfaceActionError(error);
  }
}

export async function updateTaskSheetAction(newSheet: TaskSheetState): Promise<void> {
  try {
    const res = (await agentBridge.runCommand("tasksheet.update", newSheet)) as
      | { sheet?: TaskSheetState }
      | undefined;
    if (!applyServerTaskSheet(res?.sheet ?? newSheet)) {
      storeSnapshot = {
        ...storeSnapshot,
        session: { ...storeSnapshot.session, taskSheet: newSheet, lastError: null },
      };
      emitChange();
    }
    await hostBridge.refreshProject?.();
  } catch (error) {
    throw surfaceActionError(error);
  }
}

export async function loadTemplateAction(templateId: string): Promise<void> {
  const tmpl = FLAGSHIP_TEMPLATES.find((t) => t.id === templateId);
  if (!tmpl) return;
  const draftSheet: TaskSheetState = {
    ...JSON.parse(JSON.stringify(tmpl.template)),
    status: "draft",
  };
  try {
    const res = (await agentBridge.runCommand("tasksheet.create", draftSheet)) as
      | { sheet?: TaskSheetState }
      | undefined;
    applyServerTaskSheet(res?.sheet ?? draftSheet);
    await hostBridge.refreshProject?.();
    hostBridge.setNotice?.(`Loaded template: ${tmpl.label}`);
  } catch (error) {
    throw surfaceActionError(error);
  }
}

export async function submitTaskSheetAction(reason?: string): Promise<void> {
  const sheet = storeSnapshot.session.taskSheet;
  try {
    if (!sheet) {
      throw new Error("No task sheet loaded to submit");
    }
    const res = (await agentBridge.runCommand("tasksheet.submit", {
      sheet_id: sheet.id,
      reason: reason ?? "Submitted from UI",
    })) as { sheet?: TaskSheetState } | undefined;
    const nextSheet: TaskSheetState = res?.sheet ?? { ...sheet, status: "ready" };
    applyServerTaskSheet(nextSheet);
    await hostBridge.refreshProject?.();
    hostBridge.setNotice?.("Task sheet submitted to ready state");
  } catch (error) {
    throw surfaceActionError(error);
  }
}

export async function answerQuestionAction(questionId: string, optionId: string): Promise<void> {
  try {
    const res = (await agentBridge.runCommand("tasksheet.question_answer", {
      question_id: questionId,
      option_id: optionId,
    })) as { sheet?: TaskSheetState } | undefined;
    if (!applyServerTaskSheet(res?.sheet)) {
      storeSnapshot = {
        ...storeSnapshot,
        session: {
          ...storeSnapshot.session,
          inbox: storeSnapshot.session.inbox.map((q) =>
            q.id === questionId ? { ...q, status: "answered", selected_option_id: optionId } : q,
          ),
          lastError: null,
        },
      };
      emitChange();
    }
    await hostBridge.refreshProject?.();
    hostBridge.setNotice?.("Question answered");
  } catch (error) {
    throw surfaceActionError(error);
  }
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

  try {
    const res = (await agentBridge.runCommand("review.note_add", payload)) as
      | { sheet?: TaskSheetState; note?: { id?: string } }
      | undefined;
    if (res?.sheet) applyServerTaskSheet(res.sheet);
    if (res?.note?.id) setSelectedNoteId(res.note.id);
    await hostBridge.refreshProject?.();
    hostBridge.setNotice?.("Added review note");
  } catch (error) {
    throw surfaceActionError(error);
  }
}

export async function resolveReviewNoteAction(
  noteId: string,
  resolution: string,
  status: "resolved" | "dismissed" = "resolved",
): Promise<void> {
  const trimmed = resolution.trim();
  if (!trimmed) {
    throw surfaceActionError(new Error("Resolution explanation is required"));
  }
  try {
    const res = (await agentBridge.runCommand("review.note_resolve", {
      note_id: noteId,
      resolution: trimmed,
      status,
    })) as { sheet?: TaskSheetState } | undefined;
    if (res?.sheet) applyServerTaskSheet(res.sheet);
    await hostBridge.refreshProject?.();
    hostBridge.setNotice?.(`Marked review note ${status}`);
  } catch (error) {
    throw surfaceActionError(error);
  }
}

export async function openReviewRoundAction(label?: string): Promise<void> {
  try {
    const res = (await agentBridge.runCommand("review.round_open", { label })) as
      | { sheet?: TaskSheetState }
      | undefined;
    if (res?.sheet) applyServerTaskSheet(res.sheet);
    await hostBridge.refreshProject?.();
    hostBridge.setNotice?.("Opened review round");
  } catch (error) {
    throw surfaceActionError(error);
  }
}

export async function closeReviewRoundAction(
  outcome: "revision_requested" | "approved",
  reason?: string,
): Promise<void> {
  try {
    const res = (await agentBridge.runCommand("review.round_close", {
      outcome,
      reason,
    })) as { sheet?: TaskSheetState } | undefined;
    if (res?.sheet) applyServerTaskSheet(res.sheet);
    await hostBridge.refreshProject?.();
    hostBridge.setNotice?.(`Closed review round (${outcome})`);
  } catch (error) {
    throw surfaceActionError(error);
  }
}
