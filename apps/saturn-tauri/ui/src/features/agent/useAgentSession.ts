// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import { useCallback, useEffect, useState } from "react";
import type {
  AgentSessionState,
  PlanStep,
  ProjectBriefState,
  ReviewChange,
  VerificationReport,
} from "./types";

const DEFAULT_BRIEF: ProjectBriefState = {
  goal: "Highlight reel for social delivery",
  platform: "web-1080p",
  target_duration_ticks: 3000,
  duration_tolerance_ticks: 120,
  must_keep_clip_ids: [],
  banned_content: ["silent_gap", "flash_frames"],
  style_notes: "Punchy edits, aligned transients, -14 LUFS loudness",
  loudness_target_lufs: -14.0,
  source: "user_prompt",
};

const INITIAL_PLAN: PlanStep[] = [
  { id: 1, title: "Analyze audio loudness & silence gaps", status: "done", targetTick: 0 },
  { id: 2, title: "Align match cuts on video transients", status: "current", targetTick: 1200 },
  { id: 3, title: "Quantize cuts to beat grid", status: "pending", targetTick: 2400 },
  { id: 4, title: "Verify delivery presets & loudness targets", status: "pending", targetTick: 3000 },
];

const INITIAL_REVIEWS: ReviewChange[] = [
  {
    change_id: 101,
    step_id: 2,
    step_title: "Align match cuts on video transients",
    tick_range: [1000, 1500],
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
    tick_range: [1800, 2200],
    tier: "edit",
    rationale: "Audio crossfade 120 ticks applied across cut point",
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
    { check_id: "video.frozen_frames", status: "warn", measured_value: 12, threshold: 8, tick_ref: 1200, message: "Short pause on title slate" },
    { check_id: "project.brief_duration", status: "pass", measured_value: 3020, threshold: 3000 },
  ],
  unavailable_checks: ["video.face_focus_score"],
  notes: { aggregate_score: null },
};

export interface UseAgentSessionProps {
  perform?: (id: string, params?: unknown, successNotice?: string) => Promise<unknown>;
  seek?: (seconds: number) => void;
  setNotice?: (notice: string | null) => void;
}

export function useAgentSession({ perform, seek, setNotice }: UseAgentSessionProps = {}) {
  const [session, setSession] = useState<AgentSessionState>({
    status: "idle",
    activeSessionId: "session-claude-01",
    activeBranch: "agent/rough-cut",
    currentAction: "Aligning match cuts on video transients",
    stepCurrent: 2,
    stepTotal: 4,
    elapsedSeconds: 14,
    lastActor: { type: "agent", id: "claude-editor" },
    isStopped: false,
    activeMode: "fine-cut",
    loadedProfiles: ["core", "fine-cut", "review"],
    grantedTiers: ["read", "edit"],
    ghostPlayheadTick: 1200,
    brief: DEFAULT_BRIEF,
    plan: INITIAL_PLAN,
    pendingPermissions: [],
    reviewQueue: INITIAL_REVIEWS,
    verificationReport: INITIAL_VERIFICATION,
  });

  // Elapsed timer simulation when thinking or editing
  useEffect(() => {
    if (session.status !== "thinking" && session.status !== "editing") return;
    const interval = window.setInterval(() => {
      setSession((s) => ({ ...s, elapsedSeconds: s.elapsedSeconds + 1 }));
    }, 1000);
    return () => window.clearInterval(interval);
  }, [session.status]);

  const stopSession = useCallback(async () => {
    setSession((s) => ({
      ...s,
      status: "stopped",
      isStopped: true,
      currentAction: "Session stopped by user",
    }));
    if (perform && session.activeSessionId) {
      try {
        await perform("session.stop", { session_id: session.activeSessionId }, "Agent session stopped");
      } catch (e) {
        console.warn("Failed to stop session over IPC:", e);
      }
    }
    setNotice?.("Agent session halted (immediate Stop)");
  }, [perform, session.activeSessionId, setNotice]);

  const resumeSession = useCallback(async () => {
    setSession((s) => ({
      ...s,
      status: "editing",
      isStopped: false,
      currentAction: "Resuming task plan...",
    }));
    if (perform && session.activeSessionId) {
      try {
        await perform("session.resume", { session_id: session.activeSessionId }, "Agent session resumed");
      } catch (e) {
        console.warn("Failed to resume session over IPC:", e);
      }
    }
    setNotice?.("Agent session resumed");
  }, [perform, session.activeSessionId, setNotice]);

  const updateBrief = useCallback(async (newBrief: ProjectBriefState) => {
    setSession((s) => ({ ...s, brief: newBrief }));
    if (perform) {
      try {
        await perform("project.brief_set", newBrief, "Project brief updated");
      } catch (e) {
        console.warn("Failed to set brief:", e);
      }
    }
  }, [perform]);

  const allowPermission = useCallback((reqId: string, mode: "once" | "session") => {
    setSession((s) => {
      const req = s.pendingPermissions.find((p) => p.id === reqId);
      const newTiers = req && mode === "session" && !s.grantedTiers.includes(req.tier)
        ? [...s.grantedTiers, req.tier]
        : s.grantedTiers;
      return {
        ...s,
        grantedTiers: newTiers,
        pendingPermissions: s.pendingPermissions.filter((p) => p.id !== reqId),
      };
    });
    setNotice?.(`Granted permission (${mode})`);
  }, [setNotice]);

  const denyPermission = useCallback((reqId: string) => {
    setSession((s) => ({
      ...s,
      pendingPermissions: s.pendingPermissions.filter((p) => p.id !== reqId),
    }));
    setNotice?.("Permission denied");
  }, [setNotice]);

  const acceptChange = useCallback((changeId: number) => {
    setSession((s) => ({
      ...s,
      reviewQueue: s.reviewQueue.map((item) =>
        item.change_id === changeId ? { ...item, accepted: true, rejected: false } : item
      ),
    }));
    setNotice?.(`Accepted change #${changeId}`);
  }, [setNotice]);

  const rejectChange = useCallback((changeId: number) => {
    setSession((s) => ({
      ...s,
      reviewQueue: s.reviewQueue.map((item) =>
        item.change_id === changeId ? { ...item, accepted: false, rejected: true } : item
      ),
    }));
    if (perform) {
      void perform("history.undo_change", { change_id: changeId }).catch(() => {});
    }
    setNotice?.(`Rejected change #${changeId}`);
  }, [perform, setNotice]);

  const fixChange = useCallback((changeId: number, feedback: string) => {
    setSession((s) => ({
      ...s,
      status: "thinking",
      currentAction: `Fixing change #${changeId}: "${feedback}"`,
      reviewQueue: s.reviewQueue.map((item) =>
        item.change_id === changeId ? { ...item, feedback } : item
      ),
    }));
    setNotice?.(`Sent fix request for change #${changeId}`);
  }, [setNotice]);

  const acceptAllBranch = useCallback(async () => {
    if (perform) {
      await perform("project.branch_merge", { resolution: "accept" }, "Merged agent branch changes into main");
    }
    setSession((s) => ({
      ...s,
      activeBranch: "main",
      status: "idle",
      reviewQueue: s.reviewQueue.map((item) => ({ ...item, accepted: true })),
    }));
    setNotice?.("Accepted and merged all branch changes");
  }, [perform, setNotice]);

  const rejectAllBranch = useCallback(async () => {
    if (perform) {
      await perform("project.branch_merge", { resolution: "reject" }, "Discarded agent branch changes");
    }
    setSession((s) => ({
      ...s,
      activeBranch: "main",
      status: "idle",
      reviewQueue: s.reviewQueue.map((item) => ({ ...item, rejected: true })),
    }));
    setNotice?.("Discarded all branch changes");
  }, [perform, setNotice]);

  const jumpToTick = useCallback((tick: number) => {
    seek?.(tick / 1000);
  }, [seek]);

  return {
    session,
    setSession,
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
  };
}
