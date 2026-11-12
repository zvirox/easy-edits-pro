// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import { useCallback, useEffect, useSyncExternalStore } from "react";
import type { ProjectDocument } from "../../app/projectTypes";
import { FLAGSHIP_TEMPLATES, type TaskSheetState } from "../tasksheet";
import type { ProjectBriefState } from "./types";
import {
  addReviewNoteAction,
  closeReviewRoundAction,
  getAgentStoreSnapshot,
  getHostBridge,
  openReviewRoundAction,
  resolveReviewNoteAction,
  setSelectedNoteId,
  setSelectedRoundNumber,
  subscribeAgentStore,
  syncAgentHostBridge,
  updateAgentSessionState,
} from "./agentSessionStore";

export interface UseAgentSessionProps {
  perform?: (id: string, params?: unknown, successNotice?: string) => Promise<unknown>;
  seek?: (ticks: number) => void;
  setNotice?: (notice: string | null) => void;
  playhead?: number;
  project?: ProjectDocument["project"] | null;
}

export function useAgentSession({
  perform,
  seek,
  setNotice,
  playhead,
  project,
}: UseAgentSessionProps = {}) {
  const snapshot = useSyncExternalStore(subscribeAgentStore, getAgentStoreSnapshot);
  const { session, playheadTicks, selectedRoundNumber, selectedNoteId } = snapshot;

  useEffect(() => {
    syncAgentHostBridge({
      ...(perform ? { perform } : {}),
      ...(seek ? { seek } : {}),
      ...(typeof setNotice === "function" ? { setNotice } : {}),
      ...(playhead != null ? { playheadTicks: playhead } : {}),
      ...(project !== undefined ? { project } : {}),
    });
  }, [perform, seek, setNotice, playhead, project]);

  // Elapsed timer simulation when thinking or editing
  useEffect(() => {
    if (session.status !== "thinking" && session.status !== "editing") return;
    const interval = window.setInterval(() => {
      updateAgentSessionState((s) => ({ ...s, elapsedSeconds: s.elapsedSeconds + 1 }));
    }, 1000);
    return () => window.clearInterval(interval);
  }, [session.status]);

  const setSession = updateAgentSessionState;

  const stopSession = useCallback(async () => {
    updateAgentSessionState((s) => ({
      ...s,
      status: "stopped",
      isStopped: true,
      currentAction: "Session stopped by user",
    }));
    const bridge = getHostBridge();
    if (bridge.perform && session.activeSessionId) {
      try {
        await bridge.perform("session.stop", { session_id: session.activeSessionId }, "Agent session stopped");
      } catch (e) {
        console.warn("Failed to stop session over IPC:", e);
      }
    }
    bridge.setNotice?.("Agent session halted (immediate Stop)");
  }, [session.activeSessionId]);

  const resumeSession = useCallback(async () => {
    updateAgentSessionState((s) => ({
      ...s,
      status: "editing",
      isStopped: false,
      currentAction: "Resuming task plan...",
    }));
    const bridge = getHostBridge();
    if (bridge.perform && session.activeSessionId) {
      try {
        await bridge.perform("session.resume", { session_id: session.activeSessionId }, "Agent session resumed");
      } catch (e) {
        console.warn("Failed to resume session over IPC:", e);
      }
    }
    bridge.setNotice?.("Agent session resumed");
  }, [session.activeSessionId]);

  const updateBrief = useCallback(async (newBrief: ProjectBriefState) => {
    updateAgentSessionState((s) => ({ ...s, brief: newBrief }));
    const bridge = getHostBridge();
    if (bridge.perform) {
      try {
        await bridge.perform("project.brief_set", newBrief, "Project brief updated");
      } catch (e) {
        console.warn("Failed to set brief:", e);
      }
    }
  }, []);

  const allowPermission = useCallback((reqId: string, mode: "once" | "session") => {
    updateAgentSessionState((s) => {
      const req = s.pendingPermissions.find((p) => p.id === reqId);
      const newTiers =
        req && mode === "session" && !s.grantedTiers.includes(req.tier)
          ? [...s.grantedTiers, req.tier]
          : s.grantedTiers;
      return {
        ...s,
        grantedTiers: newTiers,
        pendingPermissions: s.pendingPermissions.filter((p) => p.id !== reqId),
      };
    });
    getHostBridge().setNotice?.(`Granted permission (${mode})`);
  }, []);

  const denyPermission = useCallback((reqId: string) => {
    updateAgentSessionState((s) => ({
      ...s,
      pendingPermissions: s.pendingPermissions.filter((p) => p.id !== reqId),
    }));
    getHostBridge().setNotice?.("Permission denied");
  }, []);

  const acceptChange = useCallback((changeId: number) => {
    updateAgentSessionState((s) => ({
      ...s,
      reviewQueue: s.reviewQueue.map((item) =>
        item.change_id === changeId ? { ...item, accepted: true, rejected: false } : item,
      ),
    }));
    getHostBridge().setNotice?.(`Accepted change #${changeId}`);
  }, []);

  const rejectChange = useCallback((changeId: number) => {
    updateAgentSessionState((s) => ({
      ...s,
      reviewQueue: s.reviewQueue.map((item) =>
        item.change_id === changeId ? { ...item, accepted: false, rejected: true } : item,
      ),
    }));
    const bridge = getHostBridge();
    if (bridge.perform) {
      void bridge.perform("history.undo_change", { change_id: changeId }).catch(() => {});
    }
    bridge.setNotice?.(`Rejected change #${changeId}`);
  }, []);

  const fixChange = useCallback((changeId: number, feedback: string) => {
    updateAgentSessionState((s) => ({
      ...s,
      status: "thinking",
      currentAction: `Fixing change #${changeId}: "${feedback}"`,
      reviewQueue: s.reviewQueue.map((item) =>
        item.change_id === changeId ? { ...item, feedback } : item,
      ),
    }));
    getHostBridge().setNotice?.(`Sent fix request for change #${changeId}`);
  }, []);

  const acceptAllBranch = useCallback(async () => {
    const bridge = getHostBridge();
    if (bridge.perform) {
      await bridge.perform("project.branch_merge", { resolution: "accept" }, "Merged agent branch changes into main");
    }
    updateAgentSessionState((s) => ({
      ...s,
      activeBranch: "main",
      status: "idle",
      reviewQueue: s.reviewQueue.map((item) => ({ ...item, accepted: true })),
    }));
    bridge.setNotice?.("Accepted and merged all branch changes");
  }, []);

  const rejectAllBranch = useCallback(async () => {
    const bridge = getHostBridge();
    if (bridge.perform) {
      await bridge.perform("project.branch_merge", { resolution: "reject" }, "Discarded agent branch changes");
    }
    updateAgentSessionState((s) => ({
      ...s,
      activeBranch: "main",
      status: "idle",
      reviewQueue: s.reviewQueue.map((item) => ({ ...item, rejected: true })),
    }));
    bridge.setNotice?.("Discarded all branch changes");
  }, []);

  const jumpToTick = useCallback((tick: number) => {
    const target = Math.max(0, tick);
    getHostBridge().seek?.(target);
  }, []);

  const updateTaskSheet = useCallback((newSheet: TaskSheetState) => {
    updateAgentSessionState((s) => ({
      ...s,
      taskSheet: { ...newSheet, rounds: newSheet.rounds ?? s.taskSheet.rounds },
    }));
    const bridge = getHostBridge();
    if (bridge.perform) {
      void bridge.perform("tasksheet.update", newSheet).catch(() => {});
    }
  }, []);

  const submitTaskSheet = useCallback(async (reason?: string) => {
    updateAgentSessionState((s) => ({
      ...s,
      taskSheet: { ...s.taskSheet, status: "ready" },
      status: "idle",
      currentAction: "Task sheet submitted, awaiting execution start",
    }));
    const bridge = getHostBridge();
    if (bridge.perform) {
      try {
        await bridge.perform(
          "tasksheet.submit",
          { sheet_id: session.taskSheet.id, reason: reason ?? "Submitted from UI" },
          "Task sheet submitted to ready state",
        );
      } catch (e) {
        console.warn("Failed to submit sheet:", e);
      }
    }
    bridge.setNotice?.("Task sheet submitted to ready state");
  }, [session.taskSheet.id]);

  const loadTemplate = useCallback((templateId: string) => {
    const tmpl = FLAGSHIP_TEMPLATES.find((t) => t.id === templateId);
    if (!tmpl) return;
    updateAgentSessionState((s) => ({
      ...s,
      taskSheet: {
        ...JSON.parse(JSON.stringify(tmpl.template)),
        status: "draft",
        rounds: s.taskSheet.rounds,
      },
    }));
    getHostBridge().setNotice?.(`Loaded template: ${tmpl.label}`);
  }, []);

  const answerQuestion = useCallback(async (questionId: string, optionId: string) => {
    updateAgentSessionState((s) => ({
      ...s,
      inbox: s.inbox.map((q) =>
        q.id === questionId ? { ...q, status: "answered", selected_option_id: optionId } : q,
      ),
      status: "editing",
      currentAction: `Resumed with decision for ${questionId}`,
    }));
    const bridge = getHostBridge();
    if (bridge.perform) {
      try {
        await bridge.perform("tasksheet.question_answer", { question_id: questionId, option_id: optionId }, "Question answered");
      } catch (e) {
        console.warn("Failed to report answer over IPC:", e);
      }
    }
    bridge.setNotice?.("Question answered, agent resumed");
  }, []);

  return {
    session,
    setSession,
    playhead: playheadTicks,
    selectedRoundNumber,
    setSelectedRoundNumber,
    selectedNoteId,
    setSelectedNoteId,
    stopSession,
    resumeSession,
    updateBrief,
    allowPermission,
    denyPermission,
    acceptChange,
    rejectChange,
    fixChange,
    acceptAllBranch,
    rejectAllBranch,
    jumpToTick,
    updateTaskSheet,
    submitTaskSheet,
    loadTemplate,
    answerQuestion,
    addReviewNote: addReviewNoteAction,
    resolveReviewNote: resolveReviewNoteAction,
    openReviewRound: openReviewRoundAction,
    closeReviewRound: closeReviewRoundAction,
  };
}
