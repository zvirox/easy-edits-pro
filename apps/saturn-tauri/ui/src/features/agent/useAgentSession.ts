// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import { useCallback, useContext, useEffect, useSyncExternalStore } from "react";
import { ProjectContext } from "../../app/ProjectProvider";
import type { ProjectDocument } from "../../app/projectTypes";
import type { TaskSheetState } from "../tasksheet/types";
import type { ProjectBriefState } from "./types";
import {
  acceptAllBranchAction,
  acceptChangeAction,
  addReviewNoteAction,
  answerQuestionAction,
  closeReviewRoundAction,
  getAgentStoreSnapshot,
  getHostBridge,
  loadTemplateAction,
  openReviewRoundAction,
  refreshAgentSessionFromEngine,
  rejectAllBranchAction,
  rejectChangeAction,
  resolveReviewNoteAction,
  resumeSessionAction,
  setSelectedNoteId,
  setSelectedRoundNumber,
  stopSessionAction,
  submitTaskSheetAction,
  subscribeAgentStore,
  syncAgentHostBridge,
  updateAgentSessionState,
  updateBriefAction,
  updateTaskSheetAction,
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
  const projectCtx = useContext(ProjectContext);
  const snapshot = useSyncExternalStore(subscribeAgentStore, getAgentStoreSnapshot);
  const { session, playheadTicks, selectedRoundNumber, selectedNoteId } = snapshot;

  useEffect(() => {
    syncAgentHostBridge({
      ...(perform ? { perform } : {}),
      ...(projectCtx?.refreshProject ? { refreshProject: projectCtx.refreshProject } : {}),
      ...(seek ? { seek } : {}),
      ...(typeof setNotice === "function" ? { setNotice } : {}),
      ...(playhead != null ? { playheadTicks: playhead } : {}),
      ...(project !== undefined ? { project } : {}),
    });
  }, [perform, projectCtx?.refreshProject, seek, setNotice, playhead, project]);

  useEffect(() => {
    void refreshAgentSessionFromEngine().catch(() => {
      // Surfacing happens through storeSnapshot.session.lastError & notice
    });
  }, [project]);

  const setSession = updateAgentSessionState;

  const stopSession = useCallback(async () => {
    try {
      await stopSessionAction();
    } catch {
      // Surfaced via session.lastError and notice
    }
  }, []);

  const resumeSession = useCallback(async () => {
    try {
      await resumeSessionAction();
    } catch {
      // Surfaced via session.lastError and notice
    }
  }, []);

  const updateBrief = useCallback(async (newBrief: ProjectBriefState) => {
    try {
      await updateBriefAction(newBrief);
    } catch {
      // Surfaced via session.lastError and notice
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
    acceptChangeAction(changeId);
  }, []);

  const rejectChange = useCallback(async (changeId: number) => {
    try {
      await rejectChangeAction(changeId);
    } catch {
      // Surfaced via session.lastError, change.conflict, and notice
    }
  }, []);

  const fixChange = useCallback((changeId: number, feedback: string) => {
    updateAgentSessionState((s) => ({
      ...s,
      reviewQueue: s.reviewQueue.map((item) =>
        item.change_id === changeId ? { ...item, feedback } : item,
      ),
    }));
    getHostBridge().setNotice?.(`Recorded fix request for change #${changeId}`);
  }, []);

  const acceptAllBranch = useCallback(async () => {
    try {
      await acceptAllBranchAction();
    } catch {
      // Surfaced via session.lastError and notice
    }
  }, []);

  const rejectAllBranch = useCallback(async () => {
    try {
      await rejectAllBranchAction();
    } catch {
      // Surfaced via session.lastError and notice
    }
  }, []);

  const jumpToTick = useCallback((tick: number) => {
    const target = Math.max(0, tick);
    getHostBridge().seek?.(target);
  }, []);

  const updateTaskSheet = useCallback(async (newSheet: TaskSheetState) => {
    try {
      await updateTaskSheetAction(newSheet);
    } catch {
      // Surfaced via session.lastError and notice
    }
  }, []);

  const submitTaskSheet = useCallback(async (reason?: string) => {
    try {
      await submitTaskSheetAction(reason);
    } catch {
      // Surfaced via session.lastError and notice
    }
  }, []);

  const loadTemplate = useCallback(async (templateId: string) => {
    try {
      await loadTemplateAction(templateId);
    } catch {
      // Surfaced via session.lastError and notice
    }
  }, []);

  const answerQuestion = useCallback(async (questionId: string, optionId: string) => {
    try {
      await answerQuestionAction(questionId, optionId);
    } catch {
      // Surfaced via session.lastError and notice
    }
  }, []);

  return {
    session,
    setSession,
    playhead: playheadTicks,
    selectedRoundNumber,
    setSelectedRoundNumber,
    selectedNoteId,
    setSelectedNoteId,
    refreshFromEngine: refreshAgentSessionFromEngine,
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
